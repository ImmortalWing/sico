//! M14 STEP-0132: fixed-width bit operations end-to-end through the real
//! Component Runtime. The fixture compiles
//! `tests/end-to-end/bit-ops-bitboard.sico` — pure Sico source computing a
//! popcount via `shr`/`bit_and`, plus and/or/xor/shl identities and the
//! I64 arithmetic vs U64 logical right-shift distinction — and verifies the
//! deterministic result and repeated-run isolation.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const BIT_OPS_SOURCE: &str = include_str!("../../../tests/end-to-end/bit-ops-bitboard.sico");

fn compile_source(source: &str, tag: usize) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0132-bit-{}-{}", std::process::id(), tag));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("bit-ops.sico");
    let component_path = directory.join("bit-ops.component.wasm");
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
        .expect("bit-ops component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn bit_operations_run_deterministically() {
    // The guest asserts popcount, and/or/xor identities, shift results and
    // the I64-vs-U64 right-shift distinction itself; stdout carries the
    // fixed marker.
    let component = compile_source(BIT_OPS_SOURCE, 1);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"bit-ops-ok".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_bit_ops_runs_are_isolated() {
    // Bit operations are pure instructions on wasm locals per Store;
    // repeated runs must produce identical results with no cross-run state.
    let component = compile_source(BIT_OPS_SOURCE, 2);
    for _ in 0..3 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.stdout, b"bit-ops-ok".to_vec());
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}
