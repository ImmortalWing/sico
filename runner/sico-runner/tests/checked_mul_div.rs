//! M14 STEP-0134: fixed-width checked multiply/divide end-to-end through
//! the real Component Runtime. The fixture compiles
//! `tests/end-to-end/checked-mul-div.sico` — pure Sico source asserting
//! exact products/quotients, I64 mul overflow and underflow directions,
//! U64 mul overflow, division by zero and the `i64::MIN / -1` overflow —
//! and verifies the deterministic result plus repeated-run isolation.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const MUL_DIV_SOURCE: &str = include_str!("../../../tests/end-to-end/checked-mul-div.sico");

fn compile_source(source: &str, tag: usize) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0134-muldiv-{}-{}", std::process::id(), tag));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("mul-div.sico");
    let component_path = directory.join("mul-div.component.wasm");
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
        .expect("mul-div component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn checked_mul_div_run_deterministically() {
    // The guest asserts 19 mul/div cases (exact values, both overflow
    // directions, division by zero, MIN/-1) itself; stdout carries the
    // fixed marker.
    let component = compile_source(MUL_DIV_SOURCE, 1);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"mul-div-ok".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_checked_mul_div_runs_are_isolated() {
    let component = compile_source(MUL_DIV_SOURCE, 2);
    for _ in 0..3 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.stdout, b"mul-div-ok".to_vec());
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}
