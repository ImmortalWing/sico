use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::Value;
use sico_observability::verify_debug_artifacts;

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
fn top_level_execution_candidates_fail_closed_with_e1013() {
    for candidate in [
        "stdout.write(stdin.read_all())\n",
        "script:\n  return input.stdin\nend script\n",
    ] {
        let output = run(["check", "--json", "-"], Some(candidate.as_bytes()));
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stderr.is_empty());
        let json: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(json["diagnostics"].as_array().unwrap().len(), 1);
        assert_eq!(json["diagnostics"][0]["code"], "E1013");
        assert_eq!(json["diagnostics"][0]["key"], "SYNTAX_UNEXPECTED_TOP_LEVEL");
        assert_eq!(json["status"]["semantic_checks_performed"], false);

        let output = run(["format", "-"], Some(candidate.as_bytes()));
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(stderr(&output).contains("E1013"));
    }
}

#[test]
fn check_matches_all_58_semantic_oracles() {
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
    assert_eq!(sources.len(), 58);
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
    assert_eq!((accepted, rejected), (25, 33));
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
fn build_emits_a_deterministic_component() {
    let source = root().join("tests/end-to-end/answer.sico");
    let first = temp_file("answer-1.component.wasm");
    let second = temp_file("answer-2.component.wasm");
    let third = temp_file("answer-stdin.component.wasm");

    let output = run(["build", "--output", path(&first), path(&source)], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("built "));
    assert!(output.stderr.is_empty());
    let first_bytes = fs::read(&first).unwrap();
    wasmparser::Validator::new()
        .validate_all(&first_bytes)
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

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(third).unwrap();
    fs::remove_file(no_main).unwrap();
}

#[test]
fn debug_build_emits_a_deterministic_bound_triplet_atomically() {
    let source = root().join("tests/end-to-end/answer.sico");
    let first = temp_file("debug-answer-1.component.wasm");
    let second = temp_file("debug-answer-2.component.wasm");
    let first_map = suffix(&first, ".debug-map.json");
    let first_identity = suffix(&first, ".debug-identity.json");
    let second_map = suffix(&second, ".debug-map.json");
    let second_identity = suffix(&second, ".debug-identity.json");

    for output in [&first, &second] {
        let result = run(
            [
                "build",
                "--debug-info",
                "--output",
                path(output),
                path(&source),
            ],
            None,
        );
        assert_eq!(result.status.code(), Some(0), "{}", stderr(&result));
        assert!(stdout(&result).contains("debug-map "));
        assert!(stdout(&result).contains("debug-identity "));
    }
    let first_component = fs::read(&first).unwrap();
    let first_map_bytes = fs::read(&first_map).unwrap();
    let first_identity_bytes = fs::read(&first_identity).unwrap();
    verify_debug_artifacts(&first_component, &first_map_bytes, &first_identity_bytes).unwrap();
    assert_eq!(first_component, fs::read(&second).unwrap());
    assert_eq!(first_map_bytes, fs::read(&second_map).unwrap());
    assert_eq!(first_identity_bytes, fs::read(&second_identity).unwrap());

    let blocked = temp_file("debug-blocked.component.wasm");
    let blocked_map = suffix(&blocked, ".debug-map.json");
    fs::write(&blocked_map, b"owned-by-caller").unwrap();
    let result = run(
        [
            "build",
            "--debug-info",
            "--output",
            path(&blocked),
            path(&source),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(&result).contains("refusing to overwrite"));
    assert!(!blocked.exists());
    assert_eq!(fs::read(&blocked_map).unwrap(), b"owned-by-caller");
    assert!(!suffix(&blocked, ".debug-identity.json").exists());

    let script_source = root().join("tests/end-to-end/script-word-count.sico");
    let script = temp_file("debug-script.component.wasm");
    let script_map = suffix(&script, ".debug-map.json");
    let script_identity = suffix(&script, ".debug-identity.json");
    let result = run(
        [
            "build",
            "--profile",
            "script-v0",
            "--debug-info",
            "--output",
            path(&script),
            path(&script_source),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(0), "{}", stderr(&result));
    let (map, _) = verify_debug_artifacts(
        &fs::read(&script).unwrap(),
        &fs::read(&script_map).unwrap(),
        &fs::read(&script_identity).unwrap(),
    )
    .unwrap();
    assert!(map.functions.iter().any(|function| {
        function.core_module == "sico-script-core" && function.id.starts_with("generated.helper.")
    }));
    assert!(map.mappings.iter().any(|mapping| {
        mapping.generated
            && mapping.source.is_none()
            && mapping.function_id.starts_with("generated.")
    }));

    for path in [
        first,
        first_map,
        first_identity,
        second,
        second_map,
        second_identity,
        blocked_map,
        script,
        script_map,
        script_identity,
    ] {
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn build_script_profile_emits_deterministic_valid_components() {
    let echo = root().join("tests/end-to-end/script-echo.sico");
    let reject = root().join("tests/end-to-end/script-reject.sico");
    let first = temp_file("script-echo-1.component.wasm");
    let second = temp_file("script-echo-2.component.wasm");
    let reject_out = temp_file("script-reject.component.wasm");

    for output in [&first, &second] {
        let result = run(
            [
                "build",
                "--profile",
                "script-v0",
                "--output",
                path(output),
                path(&echo),
            ],
            None,
        );
        assert_eq!(result.status.code(), Some(0), "{}", stderr(&result));
    }
    let first_bytes = fs::read(&first).unwrap();
    assert_eq!(first_bytes, fs::read(&second).unwrap());
    wasmparser::Validator::new()
        .validate_all(&first_bytes)
        .unwrap();

    let result = run(
        [
            "build",
            "--profile",
            "script-v0",
            "--output",
            path(&reject_out),
            path(&reject),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(0), "{}", stderr(&result));
    wasmparser::Validator::new()
        .validate_all(&fs::read(&reject_out).unwrap())
        .unwrap();

    let unknown_profile = temp_file("unknown-profile.component.wasm");
    let result = run(
        [
            "build",
            "--profile",
            "script-v9",
            "--output",
            path(&unknown_profile),
            path(&echo),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(&result).contains("unknown build profile"));
    assert!(!unknown_profile.exists());

    let scalar = root().join("tests/end-to-end/answer.sico");
    let scalar_out = temp_file("script-scalar.component.wasm");
    let result = run(
        [
            "build",
            "--profile",
            "script-v0",
            "--output",
            path(&scalar_out),
            path(&scalar),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(&result).contains("explicit record ScriptInput"));
    assert!(!scalar_out.exists());

    let missing_fields = temp_file("script-missing-fields.sico");
    fs::write(
        &missing_fields,
        b"record ScriptInput:\n  field stdin: Bytes\nend record\n",
    )
    .unwrap();
    let result = run(
        [
            "build",
            "--profile",
            "script-v0",
            "--output",
            path(&scalar_out),
            path(&missing_fields),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(&result).contains("does not match the frozen Script ABI"));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(reject_out).unwrap();
    fs::remove_file(missing_fields).unwrap();
}

#[test]
fn build_script_profile_emits_valid_http_component_import() {
    let source = root().join("tests/end-to-end/script-http-request.sico");
    let output = temp_file("script-http.component.wasm");
    let result = run(
        [
            "build",
            "--profile",
            "script-v0",
            "--output",
            path(&output),
            path(&source),
        ],
        None,
    );
    assert_eq!(result.status.code(), Some(0), "{}", stderr(&result));
    let bytes = fs::read(&output).unwrap();
    wasmparser::Validator::new().validate_all(&bytes).unwrap();
    assert!(
        bytes
            .windows(b"sico:script/http@0.1.0".len())
            .any(|window| window == b"sico:script/http@0.1.0")
    );
    fs::remove_file(output).unwrap();
}

#[test]
fn repl_replays_resets_and_exports_bounded_cells() {
    let export = temp_file("repl-session.json");
    let input = format!(
        "40 + 2\nunknown\n41 + 1\n:history\n:reset\n40 + 2\n:export {}\n:quit\n",
        export.display()
    );
    let output = run(["repl"], Some(input.as_bytes()));
    assert_eq!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("eval expression is not valid"));
    let stdout = stdout(&output);
    let cells: Vec<_> = stdout
        .lines()
        .filter(|line| line.len() > 66 && line.as_bytes()[64] == b' ')
        .collect();
    assert!(cells.len() >= 3, "{stdout}");
    assert_eq!(&cells[0][..64], &cells[cells.len() - 1][..64]);
    let session: Value = serde_json::from_slice(&fs::read(&export).unwrap()).unwrap();
    assert_eq!(session["schema"], "sico.repl.session.v0");
    assert_eq!(session["cells"].as_array().unwrap().len(), 1);
    assert_eq!(session["cells"][0]["value"], 42);
    fs::remove_file(export).unwrap();
}

#[test]
fn explain_reports_action_hints_for_stable_codes() {
    // M21 §3.3: `sico explain <code>` surfaces the action-oriented hint
    // for a stable diagnostic code; unknown codes are a typed diagnostic.
    let output = run(["explain", "E1013"], None);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("E1013"));
    assert!(stdout(&output).contains("function main"));

    let output = run(["explain", "E1001"], None);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("end function"));

    let output = run(["explain", "E9999"], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("no action hint registered"));
}

#[test]
fn usage_io_and_unimplemented_commands_are_tool_errors() {
    let output = run::<0>([], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Usage:"));

    // STEP-0140 made `test` a real subcommand; an unknown spelling is still
    // a tool error, and `sico test` on a missing path reports typed CLI text.
    let output = run(["test-old", "app.sico"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unrecognized subcommand"));
    let output = run(["test", "does-not-exist-dir"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("path not found"));

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

fn suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
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
