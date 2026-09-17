//! M22 accepted-source formatter differential. The Sico implementation must
//! match the Rust formatter byte-for-byte and be idempotent on the fixed corpus.

use sico_format::format;
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits,
    ScriptInput,
};
use sico_source::{SourceFile, SourceId};

const FORMATTER_SOURCE: &str = include_str!("../../../selfhost/formatter.sico");

fn compile_formatter() -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0204-formatter-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("formatter.sico");
    let component_path = directory.join("formatter.component.wasm");
    std::fs::write(&source_path, FORMATTER_SOURCE).unwrap();
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

fn run_formatter(
    prepared: &PreparedProgram,
    limits: &RunnerLimits,
    path: &str,
    bytes: Vec<u8>,
) -> Vec<u8> {
    let outcome = prepared
        .run(
            &ScriptInput {
                stdin: bytes,
                ..ScriptInput::default()
            },
            limits,
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    let RunOutcome::Output(output) = outcome else {
        panic!("Sico formatter failed for {path}: {outcome:?}")
    };
    assert_eq!(output.exit_code, 0, "{path}: {:?}", output.stderr);
    output.stdout
}

#[test]
fn sico_formatter_matches_rust_on_every_accepted_source_and_is_idempotent() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(repository.join("selfhost/corpus-v0.json")).unwrap())
            .unwrap();
    let accepted: Vec<_> = manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["rust_format"] == "accepted")
        .map(|entry| entry["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(accepted.len(), 99);
    let refused: Vec<_> = manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["rust_format"] == "refused")
        .map(|entry| entry["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(refused.len(), 116);

    let component = compile_formatter();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("formatter component links");
    let limits = RunnerLimits {
        fuel: 5_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        ..RunnerLimits::default()
    };

    for (index, path) in accepted.iter().enumerate() {
        let bytes = std::fs::read(repository.join(path)).unwrap();
        let source =
            SourceFile::from_bytes(SourceId::new(u32::try_from(index).unwrap()), path, &bytes)
                .unwrap();
        let expected = format(&source)
            .unwrap_or_else(|error| panic!("manifest says formatter accepted {path}: {error:?}"));
        let actual = run_formatter(&prepared, &limits, path, bytes);
        assert_eq!(
            String::from_utf8(actual.clone()).unwrap(),
            expected,
            "{path}"
        );
        assert_eq!(
            run_formatter(&prepared, &limits, path, actual),
            expected.as_bytes(),
            "{path} idempotence"
        );
    }

    for path in refused {
        let bytes = std::fs::read(repository.join(&path)).unwrap();
        let outcome = prepared
            .run(
                &ScriptInput {
                    stdin: bytes,
                    ..ScriptInput::default()
                },
                &limits,
                &CancelToken::new(),
            )
            .expect("input bounds hold");
        assert_eq!(
            outcome,
            RunOutcome::Domain {
                code: "invalid-input".to_owned(),
                message: "LEXICAL".to_owned(),
            },
            "{path}"
        );
    }
}
