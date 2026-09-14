//! M21 §3.2 / RFC-0046 D4 (STEP-0176): `List[I64]`/`List[U64]` numeric list
//! monomorphs end-to-end through the real Component Runtime — the bracket
//! family (empty/append/get/length/sort/min/max), the signed/unsigned
//! ordering comparators, and the for-loop over a scalar list.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const FAMILY_SOURCE: &str = include_str!("../../../tests/end-to-end/list-i64-family.sico");
const SORT_SOURCE: &str = include_str!("../../../tests/end-to-end/list-i64-sort.sico");
const FOR_SOURCE: &str = include_str!("../../../tests/end-to-end/for-loop-i64.sico");

fn compile_source(source: &str, tag: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0176-{}-{}-{}",
        tag,
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("list.sico");
    let component_path = directory.join("list.component.wasm");
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
        .expect("list component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn numeric_list_family_reads_back_elements() {
    let component = compile_source(FAMILY_SOURCE, "family");
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(output.stdout, b"42/2".to_vec(), "family drifted");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn signed_sort_orders_negatives_correctly() {
    // -5 < -1 < 3 with signed I64 compares; min = -5, max = 3.
    let component = compile_source(SORT_SOURCE, "sort");
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(output.stdout, b"-5,3,-53".to_vec(), "sort drifted");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn for_loop_binds_scalar_elements_and_accumulates() {
    // 10 + 20 + 30 = 60; the element stream is emitted per iteration.
    let component = compile_source(FOR_SOURCE, "for");
    match run_component(&component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(output.stdout, b"102030=60".to_vec(), "for drifted");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}
