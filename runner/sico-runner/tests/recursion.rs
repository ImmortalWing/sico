//! M14 STEP-0137: general recursion with a typed stack-budget exhaustion
//! outcome, end-to-end through the real Component Runtime. Bounded
//! recursion (`tests/end-to-end/recursion-depth.sico`) returns exact
//! results; unbounded recursion
//! (`tests/end-to-end/recursion-unbounded.sico`) yields
//! `RunOutcome::StackLimit` — never a native stack overflow — and the
//! runner stays reusable for a subsequent program afterwards
//! (RFC-0038 profile item 2; M14 exit gate 4).

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const DEPTH_SOURCE: &str = include_str!("../../../tests/end-to-end/recursion-depth.sico");
const UNBOUNDED_SOURCE: &str = include_str!("../../../tests/end-to-end/recursion-unbounded.sico");

fn compile_source(source: &str, name: &str, tag: usize) -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!(
        "sico-step0137-{}-{}-{}",
        name,
        std::process::id(),
        tag
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join(format!("{name}.sico"));
    let component_path = directory.join(format!("{name}.component.wasm"));
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

fn run_component(component: &[u8], tag: usize) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .unwrap_or_else(|error| panic!("component {tag} links: {error:?}"));
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn bounded_recursion_returns_exact_results() {
    // 10,000-deep countdown plus fib(12): the guest asserts both itself;
    // stdout carries the fixed marker.
    let component = compile_source(DEPTH_SOURCE, "depth", 1);
    match run_component(&component, 1) {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"recursion-ok".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn unbounded_recursion_is_a_typed_stack_limit_and_host_stays_reusable() {
    let unbounded = compile_source(UNBOUNDED_SOURCE, "unbounded", 2);
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&unbounded, &FsGrants::default(), &NetGrants::default())
        .expect("unbounded component links");
    let outcome = prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    assert_eq!(outcome, RunOutcome::StackLimit);
    assert_eq!(outcome.exit_code(), 125);
    assert_eq!(outcome.class(), "resource-limit.stack");

    // The exhausted run must leave the runner reusable: the same Runner
    // prepares and completes a normal program afterwards (exit gate 4).
    let depth = compile_source(DEPTH_SOURCE, "depth", 3);
    let prepared = runner
        .prepare_program_with_net(&depth, &FsGrants::default(), &NetGrants::default())
        .expect("depth component links after exhaustion");
    match prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
    {
        RunOutcome::Output(output) => assert_eq!(output.stdout, b"recursion-ok".to_vec()),
        other => panic!("expected guest output after exhaustion, got {other:?}"),
    }
}
