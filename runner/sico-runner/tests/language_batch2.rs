//! M21 §3.2 / RFC-0046 (STEP-0175): language v1 batch 2 end-to-end through
//! the real Component Runtime — for-loops over every executable iterable,
//! `expr?` error propagation (happy and propagation paths), and the
//! `List[I64]`/`List[U64]` element extension, each with content-asserting
//! expectations plus the typed refusal corpus.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const FOR_WORDS_SOURCE: &str = include_str!("../../../tests/end-to-end/for-loop-words.sico");
const FOR_COLLECTIONS_SOURCE: &str =
    include_str!("../../../tests/end-to-end/for-loop-collections.sico");
const PROPAGATION_SOURCE: &str = include_str!("../../../tests/end-to-end/error-propagation.sico");
const LIST_NUMERIC_SOURCE: &str = include_str!("../../../tests/end-to-end/list-numeric.sico");

fn compile_source(tag: &str, source: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0175-{tag}-{}-{}",
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
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

fn run_component(component: &[u8]) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("batch-2 component links");
    prepared
        .run(
            &ScriptInput::default(),
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

fn expect_output(component: &[u8], expected: &[u8]) {
    match run_component(component) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert_eq!(output.stdout, expected.to_vec(), "stdout drifted");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn for_loops_iterate_lists_maps_and_sets_in_frozen_order() {
    expect_output(
        &compile_source("for-words", FOR_WORDS_SOURCE),
        b"alpha|beta|gamma|delta|",
    );
    expect_output(
        &compile_source("for-collections", FOR_COLLECTIONS_SOURCE),
        b"ac\nxyz\n",
    );
}

#[test]
fn error_propagation_runs_happy_and_error_paths() {
    // half=8 (9/2*2), inv=0 (integer division), div0=caught (the
    // division-by-zero NumericError propagates through `?` verbatim).
    expect_output(
        &compile_source("propagation", PROPAGATION_SOURCE),
        b"half=8\ninv=0\ndiv0=caught\n",
    );
}

#[test]
fn numeric_list_operations_produce_the_frozen_outputs() {
    expect_output(
        &compile_source("list-numeric", LIST_NUMERIC_SOURCE),
        b"len=3\n\
          first=-7\n\
          min=-7 max=3\n\
          -7,1,2,3,\n\
          sum=-1\n\
          ulen=2 umin=1 umax=18446744073709551615\n",
    );
}

#[test]
fn repeated_batch2_runs_are_deterministic() {
    let component = compile_source("list-numeric-determinism", LIST_NUMERIC_SOURCE);
    for _ in 0..2 {
        match run_component(&component) {
            RunOutcome::Output(output) => {
                assert_eq!(output.exit_code, 0);
                assert!(output.stdout.starts_with(b"len=3\n"));
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

fn check_refused(tag: &str, source: &str, needle: &str) {
    let directory = std::env::temp_dir().join(format!("sico-step0175-refusal-{tag}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join(format!("{tag}.sico"));
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
    assert_eq!(exit, 1, "{tag} must be refused");
    let text = String::from_utf8_lossy(&stderr);
    assert!(
        text.contains(needle),
        "{tag}: typed refusal expected, got: {text}"
    );
}

const HEADER: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

fn main_wrapping(body: &str) -> String {
    format!(
        "{HEADER}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n{body}  return ok(ScriptOutput(stdout: sico.text.encode(\"x\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    )
}

#[test]
fn for_over_text_is_refused_at_check() {
    check_refused(
        "for-text",
        &main_wrapping("  for c in \"abc\":\n    set out = c\n  end for\n"),
        "E2001",
    );
}

#[test]
fn question_mark_in_non_numeric_error_function_is_refused_at_check() {
    // main returns Result[ScriptOutput, ScriptError]; `?` freezes both
    // error types to NumericError (RFC-0046 D2, E3101).
    check_refused(
        "propagate-wrong-error",
        &main_wrapping("  let v = I64.checked_div(I64.literal(1), I64.literal(0))?\n"),
        "E3101",
    );
}

#[test]
fn list_bool_elements_are_refused_at_check() {
    check_refused(
        "list-bool",
        &main_wrapping(
            "  let b = sico.list.get[Bool](sico.text.split_words(\"\"), U64.literal(0))\n",
        ),
        "E2031",
    );
}
