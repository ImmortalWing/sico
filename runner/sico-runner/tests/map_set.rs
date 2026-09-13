//! M14 STEP-0131: insertion-ordered Map/Set collections end-to-end through
//! the real Component Runtime. The fixture compiles
//! `tests/end-to-end/map-set-frequency.sico` — pure Sico source counting
//! word frequencies in a `Map[Text,I64]` and tracking distinct words in a
//! `Set[Text]`, with deterministic first-insertion order — and verifies the
//! deterministic result plus a typed miss on `map.get`.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const MAP_SET_SOURCE: &str = include_str!("../../../tests/end-to-end/map-set-frequency.sico");
const TRAVERSAL_SOURCE: &str = include_str!("../../../tests/end-to-end/map-keys-traversal.sico");

fn compile_source(source: &str, tag: usize) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0131-map-{}-{}", std::process::id(), tag));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("map-set.sico");
    let component_path = directory.join("map-set.component.wasm");
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

fn run_component(component: &[u8], stdin: &[u8]) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("map-set component links");
    prepared
        .run(
            &ScriptInput {
                stdin: stdin.to_vec(),
                ..ScriptInput::default()
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn map_set_word_frequency_runs_deterministically() {
    // six words, four distinct; the guest asserts counts, set membership and
    // a phantom-key miss itself; stdout carries the fixed marker.
    let component = compile_source(MAP_SET_SOURCE, 1);
    match run_component(&component, b"alpha beta gamma alpha beta delta gamma omega") {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"map-set-ok".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn repeated_map_set_runs_are_isolated() {
    // Copy-on-write collections live in wasm locals per Store; repeated runs
    // must produce identical results with no cross-run state.
    let component = compile_source(MAP_SET_SOURCE, 2);
    for _ in 0..3 {
        match run_component(&component, b"alpha beta gamma alpha beta delta gamma omega") {
            RunOutcome::Output(output) => {
                assert_eq!(output.stdout, b"map-set-ok".to_vec());
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

#[test]
fn map_keys_and_set_to_list_preserve_element_contents() {
    // STEP-0133 regression: `map.keys` must compact the 16-byte map entries
    // into the 8-byte `List[Text]` stride — the pre-fix passthrough read the
    // value slot of entry i as key i+1, so "alpha|beta" joined as "alpha|".
    // The guest asserts exact join order and length, content round-trips of
    // materialized keys back into the owning collection, empty and singleton
    // maps, and an interleaved arena allocation between `keys` and its use.
    let component = compile_source(TRAVERSAL_SOURCE, 3);
    for _ in 0..2 {
        match run_component(&component, b"") {
            RunOutcome::Output(output) => {
                assert_eq!(output.stdout, b"traversal-ok".to_vec());
                assert_eq!(output.exit_code, 0);
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }
}

#[test]
fn unregistered_collection_instantiation_fails_closed() {
    // `map.get` with a non-fixed-width value stays outside the v0 executable
    // surface: check accepts the suffix syntactically (the declared matrix
    // row marks it check-accepted), and build refuses it with a typed
    // unsupported-call error before any artifact is produced.
    let broken = MAP_SET_SOURCE.replace(
        "sico.map.get[Text,I64](counts, word)",
        "sico.map.get[Text,Text](counts, word)",
    );
    let directory = std::env::temp_dir().join(format!("sico-step0131-get-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("map-get-text.sico");
    std::fs::write(&source_path, &broken).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    // STEP-0144: the off-surface instantiation is now refused at CHECK time
    // (E2031 unresolved-call-target) instead of deferring to the build
    // backend's "unsupported call target" refusal.
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
    std::fs::remove_dir_all(directory).unwrap();
    assert_ne!(exit, 0, "map.get[Text,Text] must fail closed at check");
    assert!(
        String::from_utf8_lossy(&stderr).contains("E2031")
            && String::from_utf8_lossy(&stderr).contains("sico.map.get[Text,Text]"),
        "typed refusal expected: {}",
        String::from_utf8_lossy(&stderr)
    );
}
