//! M21 §3.1 / RFC-0045 (STEP-0174): stdlib batch 2 end-to-end through the
//! real Component Runtime — byte access, byte equality, byte-order text
//! comparison, char indexing, template formatting, deterministic list
//! ordering and map values, each with a content-asserting expectation.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const BATCH2_SOURCE: &str = include_str!("../../../tests/end-to-end/stdlib-batch2.sico");

fn compile_source(source: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0174-batch2-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("stdlib-batch2.sico");
    let component_path = directory.join("stdlib-batch2.component.wasm");
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
        .expect("batch2 component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn stdlib_batch2_intrinsics_produce_the_frozen_outputs() {
    let component = compile_source(BATCH2_SOURCE);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(
                output.stdout,
                b"at1=98\n\
                  at9=-1\n\
                  eq=true\n\
                  eq2=false\n\
                  cmp=-101\n\
                  char1=\xC3\xA9\n\
                  char9=err\n\
                  fmt=x=a y=b { c {5} }\n\
                  sort=apple banana cherry\n\
                  min=apple max=cherry\n\
                  minempty=none sortempty=0\n\
                  values=v1,v2\n"
                    .to_vec(),
                "stdout drifted"
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_batch2_runs_are_deterministic() {
    let component = compile_source(BATCH2_SOURCE);
    for _ in 0..2 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.exit_code, 0);
                assert!(output.stdout.starts_with(b"at1=98\n"));
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

#[test]
fn map_values_non_executable_value_element_is_refused_at_check() {
    // RFC-0045 D4 as amended by RFC-0046 (STEP-0175): `map.values[Text,I64]`
    // is executable now that `List[I64]` exists; a `Bool` value element
    // (`List[Bool]` is not an executable type) still fails closed at check
    // time with the typed unresolved-target diagnostic, never reaching
    // codegen.
    let header = BATCH2_SOURCE
        .split_once("function main")
        .expect("fixture shape")
        .0;
    let source = format!(
        "{header}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let m = sico.map.empty[Text,Bool]()
  let vs = sico.map.values[Text,Bool](m)
  return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))
end function
"
    );
    let directory =
        std::env::temp_dir().join(format!("sico-step0174-refusal-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("values-refusal.sico");
    std::fs::write(&source_path, source).unwrap();
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("check"),
            source_path.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    let _ = std::fs::remove_dir_all(directory);
    assert_eq!(exit, 1, "values[Bool] must be refused");
    let text = String::from_utf8_lossy(&stderr);
    assert!(
        text.contains("E2031") || text.contains("unresolved call target"),
        "typed refusal expected, got: {text}"
    );
}
