//! M14 STEP-0130: general control flow (while / if-else / break / continue /
//! `set` reassignment) end-to-end through the real Component Runtime. The
//! fixture compiles `tests/end-to-end/control-flow-counter.sico` — pure Sico
//! source using loops, cells and fall-through branches — and verifies its
//! deterministic result, plus typed refusals for malformed control flow.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const COUNTER_SOURCE: &str = include_str!("../../../tests/end-to-end/control-flow-counter.sico");

fn compile_source(source: &str, tag: usize) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0130-cfg-{}-{}", std::process::id(), tag));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("control-flow.sico");
    let component_path = directory.join("control-flow.component.wasm");
    std::fs::write(&source_path, source).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
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
        .expect("control-flow component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn while_if_else_break_continue_run_deterministically() {
    // The loop counts 1,2,4 (continue skips 3, break stops at 5) and the
    // guest asserts the count itself; stdout carries the fixed marker.
    let component = compile_source(COUNTER_SOURCE, 1);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"control-flow-ok".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_control_flow_runs_are_isolated() {
    // Cells live in wasm locals per Store; two runs of the same component
    // must produce identical results with no cross-run state.
    let component = compile_source(COUNTER_SOURCE, 2);
    for _ in 0..3 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.stdout, b"control-flow-ok".to_vec());
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

#[test]
fn set_without_prior_let_fails_closed() {
    let broken = COUNTER_SOURCE.replace(
        "      set index = next_index",
        "      set rogue = next_index",
    );
    let directory = std::env::temp_dir().join(format!("sico-step0130-set-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("set-rogue.sico");
    std::fs::write(&source_path, &broken).unwrap();
    let component_path = directory.join("out.wasm");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
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
    std::fs::remove_dir_all(directory).unwrap();
    assert_ne!(exit, 0, "set of an undeclared name must fail closed");
    assert!(
        String::from_utf8_lossy(&stderr).contains("set without prior let"),
        "typed refusal expected: {}",
        String::from_utf8_lossy(&stderr)
    );
}
