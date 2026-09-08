//! M14 STEP-0135: source-level deterministic PRNG end-to-end through the
//! real Component Runtime. The fixture compiles
//! `tests/end-to-end/prng-xorshift.sico` — a xorshift64 (13/7/17) written
//! in pure Sico source on `U64` bit operations — and pins the first eight
//! outputs of seed 88172645463325252 to the reference sequence generated
//! once from the equivalent Python implementation (recorded in
//! `docs/steps/STEP-0135-source-level-prng.md`). No ambient randomness is
//! involved anywhere.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const PRNG_SOURCE: &str = include_str!("../../../tests/end-to-end/prng-xorshift.sico");

fn compile_source(source: &str, tag: usize) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0135-prng-{}-{}", std::process::id(), tag));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("prng.sico");
    let component_path = directory.join("prng.component.wasm");
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
        .expect("prng component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn xorshift64_matches_pinned_reference_sequence() {
    // The guest compares eight outputs against the pinned reference vector
    // itself; stdout carries the fixed marker.
    let component = compile_source(PRNG_SOURCE, 1);
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"prng-ok".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_prng_runs_are_isolated() {
    // The PRNG state lives in wasm locals per Store; repeated runs must
    // reproduce the pinned sequence with no cross-run state.
    let component = compile_source(PRNG_SOURCE, 2);
    for _ in 0..3 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.stdout, b"prng-ok".to_vec());
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}
