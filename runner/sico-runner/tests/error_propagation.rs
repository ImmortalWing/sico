//! M21 §3.2 / RFC-0046 (STEP-0175): error propagation (`?`) end-to-end
//! through the real Component Runtime — the `?` suffix forwards the
//! `NumericError` payload of a helper `Result[I64, NumericError]`.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const TRY_SOURCE: &str = r#"
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

function helper(a: I64, b: I64) returns Result[I64, NumericError]:
  return I64.checked_add(a, b)
end function

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match helper(I64.literal(2), I64.literal(3)):
    case ok(sum):
      return ok(ScriptOutput(stdout: sico.text.encode(sico.i64.to_text(sum)), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
    case error(_):
      return error(ScriptError(code: ScriptErrorCode.DomainError, message: "overflow"))
  end match
end function
"#;

fn compile_source(source: &str) -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!("sico-step0175-try-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("try.sico");
    let component_path = directory.join("try.component.wasm");
    std::fs::write(&source_path, source).unwrap();
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component_path.as_os_str().to_owned(),
            source_path.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

fn run_component(component: &[u8]) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("try component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn checked_result_helpers_flow_through_match() {
    let component = compile_source(TRY_SOURCE);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(output.stdout, b"5".to_vec(), "helper result drifted");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}
