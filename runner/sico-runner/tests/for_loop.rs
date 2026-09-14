//! M21 §3.2 / RFC-0046 (STEP-0175): for-loops end-to-end through the real
//! Component Runtime — `for x in expr:` over a `List[Text]` with per-
//! iteration binding, accumulator updates through the checked-arithmetic
//! ceremony, and a post-loop content assertion.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const FOR_SOURCE: &str = include_str!("../../../tests/end-to-end/for-loop-words.sico");

fn compile_source(source: &str, tag: usize) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0175-for-{}-{}", std::process::id(), tag));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("for-loop-words.sico");
    let component_path = directory.join("for-loop-words.component.wasm");
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
        .expect("for-loop component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn for_loop_iterates_and_binds_elements() {
    let component = compile_source(FOR_SOURCE, 1);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(
                output.stdout,
                b"alpha|beta|gamma|delta|".to_vec(),
                "for-loop element stream drifted"
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_for_loop_runs_are_deterministic() {
    let component = compile_source(FOR_SOURCE, 2);
    for _ in 0..2 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.exit_code, 0);
                assert!(output.stdout.starts_with(b"alpha|"));
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}
