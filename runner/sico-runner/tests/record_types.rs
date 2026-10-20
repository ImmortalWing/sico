//! RFC-0047 (STEP-0271/0272/0274): record types minimal closed set — bare
//! `name: Type` field declarations, named literals, dot field access,
//! nominal typing, the typed refusal corpus, and the RFC-0047 D5
//! `List[record]` column-layout monomorphs, end-to-end through the real
//! Component Runtime.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const RECORD_BASIC_SOURCE: &str = include_str!("../../../tests/end-to-end/record-basic.sico");
const RECORD_NESTED_SOURCE: &str = include_str!("../../../tests/end-to-end/record-nested.sico");
const RECORD_LIST_FIELD_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-list-field.sico");
const REFUSAL_EMPTY_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-empty.sico");
const REFUSAL_DUPLICATE_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-duplicate-field.sico");
const REFUSAL_UNKNOWN_ACCESS_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-unknown-access.sico");
const REFUSAL_NOMINAL_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-nominal.sico");
const REFUSAL_MISSING_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-missing-field.sico");
const REFUSAL_FIELD_SET_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-field-set.sico");
const REFUSAL_EQUALITY_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-refusal-equality.sico");
const RECORD_COW_SOURCE: &str = include_str!("../../../tests/end-to-end/record-copy-on-write.sico");
const RECORD_LIST_OF_RECORDS_SOURCE: &str =
    include_str!("../../../tests/end-to-end/record-list-of-records.sico");

fn compile_source(tag: &str, source: &str) -> Result<Vec<u8>, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0272-{tag}-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join(format!("{tag}.sico"));
    let component_path = directory.join(format!("{tag}.component.wasm"));
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
    let component = std::fs::read(&component_path).ok();
    std::fs::remove_dir_all(directory).unwrap();
    if exit == 0 {
        component.ok_or_else(|| "component missing after successful build".to_string())
    } else {
        Err(String::from_utf8_lossy(&stderr).into_owned())
    }
}

fn run_component(component: &[u8]) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("record component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("record component runs")
}

fn expect_stdout(tag: &str, source: &str, marker: &str) {
    let component = compile_source(tag, source).expect("record source builds");
    let outcome = run_component(&component);
    let RunOutcome::Output(output) = outcome else {
        panic!("{tag}: expected output, got {outcome:?}");
    };
    assert_eq!(
        output.stdout,
        marker.as_bytes(),
        "{tag}: stdout marker mismatch"
    );
}

fn expect_refusal(tag: &str, source: &str, code: &str) {
    let error = compile_source(tag, source).expect_err("record refusal source must not build");
    assert!(
        error.contains(code),
        "{tag}: expected {code} in refusal, got: {error}"
    );
}

#[test]
fn record_basic_declaration_literal_access_runs() {
    expect_stdout("basic", RECORD_BASIC_SOURCE, "record-basic-ok");
}

#[test]
fn record_nested_fields_and_access_chain_run() {
    expect_stdout("nested", RECORD_NESTED_SOURCE, "record-nested-ok");
}

#[test]
fn record_with_list_and_bool_fields_runs() {
    expect_stdout(
        "list-field",
        RECORD_LIST_FIELD_SOURCE,
        "record-list-field-ok",
    );
}

#[test]
fn record_empty_declaration_is_refused() {
    expect_refusal("empty", REFUSAL_EMPTY_SOURCE, "E2022");
}

#[test]
fn record_duplicate_literal_field_is_refused() {
    expect_refusal("duplicate", REFUSAL_DUPLICATE_SOURCE, "E2021");
}

#[test]
fn record_unknown_dot_access_is_refused() {
    expect_refusal("unknown-access", REFUSAL_UNKNOWN_ACCESS_SOURCE, "E2011");
}

#[test]
fn record_nominal_mismatch_is_refused() {
    expect_refusal("nominal", REFUSAL_NOMINAL_SOURCE, "E2001");
}

#[test]
fn record_missing_literal_field_is_refused() {
    expect_refusal("missing", REFUSAL_MISSING_SOURCE, "E2010");
}

#[test]
fn record_field_set_is_refused() {
    expect_refusal("field-set", REFUSAL_FIELD_SET_SOURCE, "E2023");
}

#[test]
fn record_equality_is_refused() {
    expect_refusal("equality", REFUSAL_EQUALITY_SOURCE, "E2024");
}

#[test]
fn record_copy_on_write_aliasing_runs() {
    expect_stdout("cow", RECORD_COW_SOURCE, "record-cow-ok");
}

#[test]
fn record_list_of_records_column_layout_runs() {
    expect_stdout(
        "list-of-records",
        RECORD_LIST_OF_RECORDS_SOURCE,
        "record-list-of-records-ok",
    );
}
