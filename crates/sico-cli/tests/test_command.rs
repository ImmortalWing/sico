//! M14 STEP-0140: `sico test` self-test through the real CLI binary and
//! the real `sico-runner`. Requires the runner workspace to be built
//! (the standard validation builds both workspaces before running root
//! tests).

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

const GREETING: &str = r#"
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

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: sico.text.encode("test-ok"), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
end function
"#;

const GREEDY: &str = r#"
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

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: sico.text.encode("greedy"), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
end function
"#;

fn sico_exe() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    // target/debug/deps/test_command-... -> target/debug/sico.exe
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("sico.exe");
    assert!(
        path.is_file(),
        "build the sico CLI first ({} missing)",
        path.display()
    );
    path
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

fn run_sico_test(dir: &Path, runner: &Path) -> (i32, String) {
    let mut child = ProcessCommand::new(sico_exe())
        .arg("test")
        .arg(dir)
        .env("SICO_RUNNER", runner)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("sico spawns");
    let mut output = String::new();
    child
        .stdout
        .as_mut()
        .expect("piped stdout")
        .read_to_string(&mut output)
        .expect("stdout read");
    let exit = child.wait().expect("child exits").code().unwrap_or(-1);
    (exit, output)
}

#[test]
fn sico_test_discovers_passes_and_fails_deterministically() {
    let runner = runner_exe();
    let dir = std::env::temp_dir().join(format!("sico-step0140-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // Failing pair (sorted first): wrong expected stdout.
    std::fs::write(dir.join("aa-fail.sico"), GREEDY).unwrap();
    std::fs::write(
        dir.join("aa-fail.test.json"),
        r#"{"expect_stdout": "something-else"}"#,
    )
    .unwrap();
    // Passing pair (sorted second).
    std::fs::write(dir.join("zz-pass.sico"), GREETING).unwrap();
    std::fs::write(
        dir.join("zz-pass.test.json"),
        r#"{"expect_stdout": "test-ok"}"#,
    )
    .unwrap();
    // A .sico without a manifest is not a test and must be ignored.
    std::fs::write(dir.join("untested.sico"), GREETING).unwrap();

    let (exit, output) = run_sico_test(&dir, &runner);
    assert_eq!(exit, 1, "a failing test forces exit 1; output:\n{output}");
    let pass_index = output.find("PASS").expect("PASS line present");
    let fail_index = output.find("FAIL").expect("FAIL line present");
    assert!(
        pass_index > fail_index,
        "sorted discovery must report the failing pair first:\n{output}"
    );
    assert!(output.contains("1 passed, 1 failed"), "{output}");
    assert!(
        !output.contains("untested"),
        "manifest-less sources are ignored: {output}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sico_test_reports_success_and_args_contract() {
    let runner = runner_exe();
    let dir = std::env::temp_dir().join(format!("sico-step0140-ok-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("greeting.sico"), GREETING).unwrap();
    std::fs::write(
        dir.join("greeting.test.json"),
        r#"{"expect_stdout": "test-ok"}"#,
    )
    .unwrap();
    std::fs::write(dir.join("args.sico"), GREEDY).unwrap();
    std::fs::write(
        dir.join("args.test.json"),
        r#"{"args": ["x"], "expect_stdout": "greedy"}"#,
    )
    .unwrap();
    let (exit, output) = run_sico_test(&dir, &runner);
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("2 passed, 0 failed"), "{output}");
    let _ = std::fs::remove_dir_all(&dir);
}
