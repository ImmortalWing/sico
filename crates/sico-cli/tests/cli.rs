use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::Value;

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn check_freezes_file_stdin_text_json_and_semantic_boundary() {
    let valid = root().join("syntax-candidates/b/nominal-invariants/valid/complete-record.sico");
    let semantic_reject =
        root().join("syntax-candidates/b/nominal-invariants/invalid/missing-field.sico");
    let mutation = root().join("syntax-mutations/b/MUT-001-missing-function-close.sico");

    let output = run(["check", path(&valid)], None);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("syntax ok:"));
    assert!(stdout(&output).contains("type checker: unavailable (M2)"));
    assert!(output.stderr.is_empty());

    let output = run(["check", "--json", path(&valid)], None);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["errors"], 0);
    assert_eq!(json["status"]["syntax"], "ok");
    assert_eq!(json["status"]["type_checker"], "unavailable");
    assert_eq!(json["status"]["semantic_checks_performed"], false);

    let output = run(["check", path(&semantic_reject)], None);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("type checker: unavailable (M2)"));

    let bytes = fs::read(&valid).unwrap();
    let output = run(["check", "-"], Some(&bytes));
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("syntax ok: <stdin>"));

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
    assert_eq!(json["status"]["type_checker"], "unavailable");
    assert_eq!(json["status"]["semantic_checks_performed"], false);
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
    assert_eq!(json["type_checker"], "unavailable");
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
fn usage_io_and_unimplemented_commands_are_tool_errors() {
    let output = run::<0>([], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Usage:"));

    let output = run(["run", "app.sico"], None);
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
