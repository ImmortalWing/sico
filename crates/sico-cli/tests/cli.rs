use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::Value;
use sico_package::verify;

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn check_runs_semantics_for_file_stdin_text_and_json() {
    let valid = root().join("syntax-candidates/b/nominal-invariants/valid/complete-record.sico");
    let semantic_reject =
        root().join("syntax-candidates/b/nominal-invariants/invalid/missing-field.sico");
    let mutation = root().join("syntax-mutations/b/MUT-001-missing-function-close.sico");

    let output = run(["check", path(&valid)], None);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("check ok:"));
    assert!(stdout(&output).contains("syntax + semantics"));
    assert!(output.stderr.is_empty());

    let output = run(["check", "--json", path(&valid)], None);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["errors"], 0);
    assert_eq!(json["status"]["syntax"], "ok");
    assert_eq!(json["status"]["type_checker"], "ok");
    assert_eq!(json["status"]["semantic_checks_performed"], true);

    let output = run(["check", path(&semantic_reject)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("E2010"));

    let output = run(["check", "--json", path(&semantic_reject)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["diagnostics"][0]["code"], "E2010");
    assert_eq!(json["diagnostics"][0]["key"], "MISSING_FIELD");
    assert_eq!(json["status"]["syntax"], "ok");
    assert_eq!(json["status"]["type_checker"], "error");
    assert_eq!(json["status"]["semantic_checks_performed"], true);

    let bytes = fs::read(&valid).unwrap();
    let output = run(["check", "-"], Some(&bytes));
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("check ok: <stdin>"));

    let output = run(["check", path(&mutation)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("E1001"));

    let output = run(["check", "--json", path(&mutation)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema"], "sico.diagnostics.v0");
    assert_eq!(json["diagnostics"][0]["code"], "E1001");
    assert_eq!(json["status"]["syntax"], "error");
    assert_eq!(json["status"]["type_checker"], "not-run");
    assert_eq!(json["status"]["semantic_checks_performed"], false);
}

#[test]
fn check_matches_all_54_semantic_oracles() {
    let repository = root();
    let map: Value = serde_json::from_str(
        &fs::read_to_string(repository.join("diagnostics/semantic-case-map.json")).unwrap(),
    )
    .unwrap();
    let expected: std::collections::BTreeMap<_, _> = map["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| (case["case"].as_str().unwrap().to_owned(), case.clone()))
        .collect();
    let mut sources = Vec::new();
    collect_sico(&repository.join("syntax-candidates/b"), &mut sources);
    sources.sort();
    assert_eq!(sources.len(), 54);
    let mut accepted = 0;
    let mut rejected = 0;
    for source in sources {
        let text = fs::read_to_string(&source).unwrap();
        let output = run(["check", "--json", path(&source)], None);
        let json: Value = serde_json::from_slice(&output.stdout).unwrap();
        if text.contains("// expect: accept") {
            accepted += 1;
            assert_eq!(output.status.code(), Some(0), "{}", source.display());
            assert_eq!(json["diagnostics"].as_array().unwrap().len(), 0);
            assert_eq!(json["status"]["type_checker"], "ok");
        } else {
            rejected += 1;
            assert_eq!(output.status.code(), Some(1), "{}", source.display());
            assert_eq!(json["diagnostics"].as_array().unwrap().len(), 1);
            let oracle = &expected[metadata(&text, "case")];
            assert_eq!(json["diagnostics"][0]["code"], oracle["code"]);
            assert_eq!(json["diagnostics"][0]["key"], oracle["key"]);
            assert_eq!(
                json["diagnostics"][0]["message"],
                oracle["expected_message"]
            );
            assert_eq!(json["diagnostics"][0]["arguments"], oracle["arguments"]);
            assert_eq!(json["status"]["type_checker"], "error");
        }
        assert_eq!(json["status"]["semantic_checks_performed"], true);
    }
    assert_eq!((accepted, rejected), (25, 29));
}

#[test]
fn format_freezes_stdout_check_write_and_error_refusal() {
    let noncanonical = b"function  main ( ) returns Int :\n return  1\nend function\n";
    let canonical = "function main() returns Int:\n  return 1\nend function\n";

    let output = run(["format", "-"], Some(noncanonical));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), canonical);
    assert!(output.stderr.is_empty());

    let temporary = temp_file("format.sico");
    fs::write(&temporary, noncanonical).unwrap();
    let output = run(["format", "--check", path(&temporary)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("not canonically formatted"));
    let output = run(["format", "--write", path(&temporary)], None);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_to_string(&temporary).unwrap(), canonical);
    let output = run(["format", "--check", path(&temporary)], None);
    assert_eq!(output.status.code(), Some(0));
    fs::remove_file(&temporary).unwrap();

    let mutation = root().join("syntax-mutations/b/MUT-006-missing-call-close.sico");
    let output = run(["format", path(&mutation)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("E1006"));

    let output = run(["format", "--write", "-"], Some(canonical.as_bytes()));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("requires a file path"));
}

#[test]
fn outline_freezes_order_kinds_ranges_and_json() {
    let valid = root().join("syntax-candidates/b/nominal-invariants/valid/complete-record.sico");
    let output = run(["outline", path(&valid)], None);
    assert_eq!(output.status.code(), Some(0));
    let lines: Vec<_> = stdout(&output).lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("record\tFeature\t"));
    assert!(lines[1].starts_with("function\tmain\t"));

    let output = run(["outline", "--json", path(&valid)], None);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema"], "sico.outline.v0");
    assert_eq!(json["type_checker"], "not-run");
    assert_eq!(json["declarations"][0]["kind"], "record");
    assert_eq!(json["declarations"][0]["name"], "Feature");
    assert_eq!(json["declarations"][1]["kind"], "function");
    assert!(
        json["declarations"][1]["range"]["end_byte"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn build_emits_deterministic_package_and_preserves_raw_boundary() {
    let source = root().join("tests/end-to-end/answer.sico");
    let first = temp_file("answer-1.sapp");
    let second = temp_file("answer-2.sapp");
    let third = temp_file("answer-stdin.sapp");

    let output = run(["build", "--output", path(&first), path(&source)], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("built "));
    assert!(output.stderr.is_empty());
    let first_bytes = fs::read(&first).unwrap();
    let package = verify(&first_bytes).unwrap();
    wasmparser::Validator::new()
        .validate_all(&package.component)
        .unwrap();

    let output = run(["build", "--output", path(&second), path(&source)], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let second_bytes = fs::read(&second).unwrap();
    assert_eq!(first_bytes, second_bytes);

    let source_bytes = fs::read(&source).unwrap();
    let output = run(
        ["build", "--output", path(&third), "-"],
        Some(&source_bytes),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(fs::read(&third).unwrap(), first_bytes);

    let output = run(["build", "--output", path(&first), path(&source)], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("refusing to overwrite"));
    assert_eq!(fs::read(&first).unwrap(), first_bytes);

    let invalid = root().join("syntax-candidates/b/numbers-units/invalid/text-as-int.sico");
    let rejected = temp_file("rejected.component.wasm");
    let output = run(["build", "--output", path(&rejected), path(&invalid)], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("E2001"));
    assert!(!rejected.exists());

    let unsupported =
        root().join("syntax-candidates/b/numbers-units/valid/int-arbitrary-precision.sico");
    let refused = temp_file("refused.component.wasm");
    let output = run(
        ["build", "--output", path(&refused), path(&unsupported)],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("outside the proven i64 probe"));
    assert!(!refused.exists());

    let output = run(
        ["build", "-"],
        Some(b"function main() returns Int:\n  return 1\nend function\n"),
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("requires --output"));

    let no_main = temp_file("no-main.sico");
    fs::write(
        &no_main,
        b"function answer() returns Int:\n  return 42\nend function\n",
    )
    .unwrap();
    let no_entry = temp_file("no-entry.component.wasm");
    let output = run(["build", "--output", path(&no_entry), path(&no_main)], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("entry function main() is missing"));
    assert!(!no_entry.exists());

    let raw = temp_file("answer-raw.component.wasm");
    let output = run(
        [
            "build",
            "--raw-component",
            "--output",
            path(&raw),
            path(&source),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    wasmparser::Validator::new()
        .validate_all(&fs::read(&raw).unwrap())
        .unwrap();

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(third).unwrap();
    fs::remove_file(raw).unwrap();
    fs::remove_file(no_main).unwrap();
}

#[test]
fn inspect_signing_trust_and_argument_contract() {
    let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") else {
        return;
    };
    let runtime = runtime.to_str().unwrap();
    let source = root().join("tests/end-to-end/answer.sico");
    let seed = temp_file("development-seed.hex");
    fs::write(
        &seed,
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\n",
    )
    .unwrap();
    let package = temp_file("signed-answer.sapp");
    let output = run(
        [
            "build",
            "--sign-key",
            path(&seed),
            "--output",
            path(&package),
            path(&source),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));

    let output = run(["inspect", "--json", path(&package)], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let inspected: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(inspected["schema"], "sico.sapp.inspect.v0");
    assert_eq!(inspected["app"]["id"], "dev.sico.app");
    assert_eq!(inspected["trust"]["status"], "development-valid-untrusted");
    let public_key = inspected["trust"]["public_key"].as_str().unwrap();
    let trusted_key = temp_file("trusted-public-key.hex");
    fs::write(&trusted_key, format!("{public_key}\n")).unwrap();

    let output = run(["run", "--runtime", runtime, path(&package)], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("requires --trusted-key"));

    let output = run(
        [
            "run",
            "--runtime",
            runtime,
            "--trusted-key",
            path(&trusted_key),
            path(&package),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim(), "42");

    let output = run(
        [
            "run",
            "--runtime",
            runtime,
            "--trusted-key",
            path(&trusted_key),
            path(&package),
            "--",
            "argument",
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("does not accept application arguments"));

    let mut tampered = fs::read(&package).unwrap();
    let index = tampered.len() / 2;
    tampered[index] ^= 1;
    let broken = temp_file("tampered.sapp");
    fs::write(&broken, tampered).unwrap();
    let output = run(["inspect", path(&broken)], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("package verification failed"));

    fs::remove_file(seed).unwrap();
    fs::remove_file(package).unwrap();
    fs::remove_file(trusted_key).unwrap();
    fs::remove_file(broken).unwrap();
}

#[test]
fn source_run_cache_is_auditable_and_corruption_is_not_executed() {
    let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") else {
        return;
    };
    let runtime = runtime.to_str().unwrap();
    let source = root().join("tests/end-to-end/answer.sico");
    let cache = temp_file("source-cache");
    let output = run(
        [
            "run",
            "--runtime",
            runtime,
            "--cache-dir",
            path(&cache),
            path(&source),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let cached: Vec<_> = fs::read_dir(&cache)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(cached.len(), 1);
    fs::write(&cached[0], b"corrupt cache").unwrap();
    let output = run(
        [
            "run",
            "--runtime",
            runtime,
            "--cache-dir",
            path(&cache),
            path(&source),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("refusing corrupt source cache"));
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn usage_io_and_unimplemented_commands_are_tool_errors() {
    let output = run::<0>([], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Usage:"));

    let output = run(["test", "app.sico"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unrecognized subcommand"));

    let output = run(["check", "does-not-exist.sico"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("cannot read"));

    let invalid_source = temp_file("invalid-source.sico");
    fs::write(&invalid_source, [0xff]).unwrap();
    let output = run(["check", path(&invalid_source)], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("source contract error InvalidUtf8"));
    fs::remove_file(&invalid_source).unwrap();

    let lexical_error = temp_file("lexical-error.sico");
    fs::write(&lexical_error, b"!").unwrap();
    let output = run(["check", path(&lexical_error)], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unregistered lexical error"));
    fs::remove_file(&lexical_error).unwrap();
}

fn run<const N: usize>(args: [&str; N], input: Option<&[u8]>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sico"));
    command
        .current_dir(root())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().unwrap();
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    child.wait_with_output().unwrap()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn temp_file(suffix: &str) -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("sico-cli-{}-{id}-{suffix}", std::process::id()))
}

fn collect_sico(root: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_sico(&path, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "sico")
        {
            output.push(path);
        }
    }
}

fn metadata<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("// {key}: ")))
        .unwrap()
}
