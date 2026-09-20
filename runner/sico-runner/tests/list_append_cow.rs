//! Regression for the amortized persistent-list append helper used by the
//! M22 self-host frontend.  Linear builders reuse spare capacity, while an
//! append from an older alias must fork without changing the newer value.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const SOURCE: &str = r#"record ScriptInput:
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

function number_at(values: List[U64], index: U64) returns U64:
  match sico.list.get[U64](values, index):
    case ok(value):
      return value
    case error(_):
      return U64.literal(0)
  end match
end function

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let text0 = sico.text.split_lines("")
  let text1 = sico.list.append(text0, "one")
  let text2 = sico.list.append(text1, "two")
  let text_branch = sico.list.append(text1, "left")
  let number0 = sico.list.empty[U64]()
  let number1 = sico.list.append[U64](number0, U64.literal(1))
  let number2 = sico.list.append[U64](number1, U64.literal(2))
  let number_branch = sico.list.append[U64](number1, U64.literal(3))
  let text_result = sico.text.concat(sico.text.join(text2, ","), sico.text.concat("|", sico.text.join(text_branch, ",")))
  let number_result = sico.text.concat(sico.u64.to_text(number_at(number2, U64.literal(1))), sico.text.concat("|", sico.u64.to_text(number_at(number_branch, U64.literal(1)))))
  let result = sico.text.concat(text_result, sico.text.concat("|", number_result))
  return ok(ScriptOutput(stdout: sico.text.encode(result), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
end function
"#;

#[test]
fn append_reuses_linear_capacity_and_forks_stale_aliases() {
    let directory =
        std::env::temp_dir().join(format!("sico-list-append-cow-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    std::fs::create_dir(&directory).unwrap();
    let source = directory.join("list_append_cow.sico");
    let component = directory.join("list_append_cow.component.wasm");
    std::fs::write(&source, SOURCE).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component.as_os_str().to_owned(),
            source.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));

    let bytes = std::fs::read(component).unwrap();
    let runner = Runner::new().unwrap();
    let prepared = runner
        .prepare_program_with_net(&bytes, &FsGrants::default(), &NetGrants::default())
        .unwrap();
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .unwrap();
    let RunOutcome::Output(output) = outcome else {
        panic!("list append probe failed: {outcome:?}")
    };
    assert_eq!(output.stdout, b"one,two|one,left|2|3");
    std::fs::remove_dir_all(directory).unwrap();
}
