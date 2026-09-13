//! STEP-0148 (RFC-0039 amendment A6 repair): a user function returning a
//! checked fixed-width `Result` compiles and runs end to end. This seam
//! emitted invalid WebAssembly before the repair and no frozen corpus ever
//! exercised it — this test is the frozen corpus.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

const PROGRAM: &str = "\
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

use checked.twice

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let label = \"seam\"
  match checked.twice(I64.literal(21)):
    case ok(value):
      if I64.equal(value, I64.literal(42)):
        set label = \"ok-42\"
      else:
        set label = \"ok-wrong\"
      end if
    case error(_):
      set label = \"err-surprise\"
  end match
  match checked.twice(I64.literal(4611686018427387904)):
    case ok(_):
      set label = sico.text.concat(label, \"|err-surprise\")
    case error(_):
      set label = sico.text.concat(label, \"|overflow\")
  end match
  return ok(ScriptOutput(stdout: sico.text.encode(label), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))
end function
";

const CHECKED: &str = "\
module checked

function twice(x: I64) returns Result[I64, NumericError]:
  return I64.checked_mul(x, I64.literal(2))
end function
";

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sico-a6-{tag}-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_sico")
}

fn run<const N: usize>(args: [&str; N]) -> Output {
    let mut command = Command::new(binary());
    command.args(args).stdin(Stdio::null());
    if std::env::var_os("SICO_RUNNER").is_none() {
        let runner = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../runner/sico-runner/target/debug/sico-runner.exe");
        if runner.is_file() {
            command.env("SICO_RUNNER", runner);
        }
    }
    command.output().unwrap()
}

#[test]
fn checked_result_user_function_runs() {
    let dir = temp_root("e2e");
    fs::write(dir.join("main.sico"), PROGRAM).unwrap();
    fs::write(dir.join("checked.sico"), CHECKED).unwrap();

    let output = run(["check", &dir.join("main.sico").display().to_string()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run(["run", &dir.join("main.sico").display().to_string()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"ok-42|overflow");
}
