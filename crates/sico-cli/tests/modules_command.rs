//! RFC-0039 (STEP-0143): CLI-level module assembly — check/build/run/test
//! across an entry file plus `<module>.sico` imports, and the fail-closed
//! E8xxx link diagnostics.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

const ENTRY: &str = "\
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

use math_util.twice

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match I64.equal(math_util.twice(I64.literal(21)), I64.literal(42)):
    case true:
      return ok(ScriptOutput(stdout: sico.text.encode(\"modules-ok\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))
    case false:
      return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"wrong value\"))
  end match
end function
";

const MATH_OK: &str = "\
module math_util

function twice(x: I64) returns I64:
  match I64.checked_mul(x, I64.literal(2)):
    case ok(value):
      return value
    case error(_):
      return I64.literal(-1)
  end match
end function
";

const MATH_BROKEN: &str = "\
module math_util

function twice(x: I64) returns I64:
  match I64.checked_mul(x, I64.literal(3)):
    case ok(value):
      return value
    case error(_):
      return I64.literal(-1)
  end match
end function
";

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sico-modules-{tag}-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_sico")
}

fn runner_exe() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../runner/sico-runner/target/debug/sico-runner.exe");
    assert!(
        path.is_file(),
        "build the runner workspace first ({} missing)",
        path.display()
    );
    path
}

fn run<const N: usize>(args: [&str; N]) -> Output {
    let mut command = Command::new(binary());
    command.args(args).stdin(Stdio::null());
    if let Ok(runner) = std::env::var("SICO_RUNNER_CANDIDATE") {
        command.env("SICO_RUNNER", runner);
    } else {
        let runner = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../runner/sico-runner/target/debug/sico-runner.exe");
        if runner.is_file() {
            command.env("SICO_RUNNER", runner);
        }
    }
    command.output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn modules_check_build_run_and_test() {
    let dir = temp_root("happy");
    write(&dir.join("main.sico"), ENTRY);
    write(&dir.join("math_util.sico"), MATH_OK);

    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("check ok:"));

    let output = run(["run", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(output.stdout, b"modules-ok");

    // `sico test` picks up the NAME.sico + NAME.test.json pair and builds
    // through the module set.
    write(
        &dir.join("main.test.json"),
        "{\"expect_stdout\": \"modules-ok\"}",
    );
    let output = run(["test", &dir.display().to_string()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("1 passed"));

    let manifest = dir.join("main.test.json");
    fs::remove_file(&manifest).unwrap();
}

#[test]
fn module_edits_invalidate_the_run_cache() {
    let dir = temp_root("cache");
    write(&dir.join("main.sico"), ENTRY);
    write(&dir.join("math_util.sico"), MATH_OK);
    let source = dir.join("main.sico").display().to_string();

    let first = run(["run", &source]);
    assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
    assert_eq!(first.stdout, b"modules-ok");

    // Same entry bytes, changed module bytes: the cache key must move, so
    // the stale "modules-ok" component is not reused (twice(21) becomes 63
    // and the value guard in the entry turns the run into an error).
    write(&dir.join("math_util.sico"), MATH_BROKEN);
    let second = run(["run", &source]);
    assert_ne!(second.status.code(), Some(0), "{}", stderr(&second));
}

#[test]
fn module_semantic_failure_is_attributed_to_the_module_file() {
    let dir = temp_root("semantic");
    write(&dir.join("main.sico"), ENTRY);
    write(
        &dir.join("math_util.sico"),
        "module math_util\n\nfunction twice(x: I64) returns Result[I64, NumericError]:\n  return \"boom\"\nend function\n",
    );
    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    // The semantic diagnostic renders with the module file's own name.
    assert!(stderr(&output).contains("E2001"), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("math_util.sico"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn link_failures_carry_stable_e8xxx_identities() {
    let cases: &[(&str, &str, &str)] = &[
        // (tag, math_util.sico content, expected code)
        (
            "missing-decl",
            "function twice(x: I64) returns I64:\n  return I64.literal(1)\nend function\n",
            "E8001",
        ),
        (
            "name-mismatch",
            "module other\n\nfunction twice(x: I64) returns I64:\n  return I64.literal(1)\nend function\n",
            "E8002",
        ),
        (
            "unknown-item",
            "module math_util\n\nfunction triple(x: I64) returns I64:\n  return I64.literal(1)\nend function\n",
            "E8005",
        ),
        (
            "import-kind",
            "module math_util\n\nrecord twice:\n  field x: I64\nend record\n",
            "E8011",
        ),
    ];
    for (tag, module_source, code) in cases {
        let dir = temp_root(tag);
        write(&dir.join("main.sico"), ENTRY);
        write(&dir.join("math_util.sico"), module_source);
        let output = run(["check", &dir.join("main.sico").display().to_string()]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "case {tag}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains(code),
            "case {tag}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn missing_module_file_fails_closed() {
    let dir = temp_root("unknown-module");
    write(&dir.join("main.sico"), ENTRY);
    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("E8004"), "{}", stderr(&output));
}

#[test]
fn duplicate_import_fails_closed() {
    let dir = temp_root("duplicate");
    let mut entry = ENTRY.to_owned();
    entry.push_str("use math_util.twice\n");
    write(&dir.join("main.sico"), &entry);
    write(&dir.join("math_util.sico"), MATH_OK);
    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("E8006"), "{}", stderr(&output));
}

#[test]
fn import_cycle_fails_closed() {
    let dir = temp_root("cycle");
    write(&dir.join("main.sico"), ENTRY);
    write(
        &dir.join("math_util.sico"),
        "module math_util\n\nuse left.half\n\nfunction twice(x: I64) returns I64:\n  return I64.literal(1)\nend function\n",
    );
    write(
        &dir.join("left.sico"),
        "module left\n\nuse math_util.twice\n\nfunction half(x: I64) returns I64:\n  return I64.literal(1)\nend function\n",
    );
    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("E8003"), "{}", stderr(&output));
}

#[test]
fn entry_module_declaration_fails_closed() {
    let dir = temp_root("entry-decl");
    let mut entry = String::from("module main\n\n");
    entry.push_str(ENTRY);
    write(&dir.join("main.sico"), &entry);
    write(&dir.join("math_util.sico"), MATH_OK);
    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("E8010"), "{}", stderr(&output));
}

#[test]
fn stdin_module_use_fails_closed() {
    let output = Command::new(binary())
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child
                .stdin
                .as_mut()
                .expect("piped stdin")
                .write_all(b"use math.twice\n")
                .unwrap();
            child.wait_with_output()
        })
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("E8009"), "{}", stderr(&output));
}

#[test]
fn checked_result_returning_function_is_typed_refused() {
    // STEP-0143 defect record: a user function returning a checked
    // fixed-width Result used to emit invalid WebAssembly (no frozen
    // corpus ever exercised the seam). It is now a typed build refusal
    // until the dedicated ABI-repair step lands.
    let dir = temp_root("checked-refusal");
    write(
        &dir.join("main.sico"),
        "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nfunction twice(x: I64) returns Result[I64, NumericError]:\n  return I64.checked_mul(x, I64.literal(2))\nend function\n\nfunction main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: sico.text.encode(\"ok\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n",
    );
    let output = run([
        "build",
        "--profile",
        "script-v0",
        &dir.join("main.sico").display().to_string(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    let err = stderr(&output);
    assert!(err.contains("checked fixed-width Result"), "{}", err);
    assert!(err.contains("recorded defect"), "{}", err);
}

#[test]
fn malformed_use_declaration_is_e1015() {
    let dir = temp_root("e1015");
    let mut entry = ENTRY.to_owned();
    entry.push_str("use math_util\n");
    write(&dir.join("main.sico"), &entry);
    write(&dir.join("math_util.sico"), MATH_OK);
    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("E1015"), "{}", stderr(&output));
}
