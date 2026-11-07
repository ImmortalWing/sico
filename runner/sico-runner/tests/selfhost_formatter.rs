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
    // 118 = selfhost/corpus-v0.json `rust_format == "accepted"` partition of the
    // 235-entry corpus refreeze (99 -> 118; STEP-0279..0283), post-merge 7bcdc35.
    assert_eq!(accepted.len(), 118);
    // Refused partition, classified by the frozen corpus's per-entry diagnostic
    // identity so the expectation is derived from the manifest and not from a
    // blanket "every refusal is lexical" assumption.
    //
    // - `diagnostic_ids == ["LEXICAL"]` -> the source is lexically invalid and
    //   the guest formatter's `source_has_lex_error` gate must reject it with
    //   `Domain { invalid-input, "LEXICAL" }`.
    // - any other identity (a semantic/parse refusal such as IND-003 / E1001,
    //   added by STEP-0584) -> the source is lexically valid, and the guest
    //   formatter -- a lexical-only formatter whose `main` only runs
    //   `source_has_lex_error` -- accepts and normalizes it. No `Domain`
    //   message can be derived from the manifest for these, so they are
    //   registered as expectation-pending rather than pinned to a fabricated
    //   expectation. See STEP-0601.
    let mut lexical_refusals: Vec<String> = Vec::new();
    let mut pending_refusals: Vec<String> = Vec::new();
    for entry in manifest["entries"].as_array().unwrap() {
        if entry["rust_format"] != "refused" {
            continue;
        }
        let path = entry["path"].as_str().unwrap().to_owned();
        let diagnostic_ids: Vec<&str> = entry["diagnostic_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        if diagnostic_ids == ["LEXICAL"] {
            lexical_refusals.push(path);
        } else {
            pending_refusals.push(path);
        }
    }
    // 117 = `rust_format == "refused"` partition of the 235-entry corpus
    // refreeze (116 LEXICAL + 1 semantic refusal IND-003/E1001; STEP-0279..0283
    // + STEP-0584), post-merge 7bcdc35.
    assert_eq!(lexical_refusals.len() + pending_refusals.len(), 117);
    // Exactly one refusal is expectation-pending: the semantic-only refusal
    // IND-003 (E1001). Assert the exact set so a future corpus change that adds
    // or moves a semantic refusal fails loudly instead of silently shrinking the
    // pinned coverage below the frozen expectation table.
    let expected_pending: Vec<String> = vec![
        "syntax-candidates/b/indentation/invalid/zero-indent-recovery-declaration.sico".to_owned(),
    ];
    assert_eq!(pending_refusals, expected_pending);

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

    for path in &lexical_refusals {
        let bytes = std::fs::read(repository.join(path)).unwrap();
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

    for path in &pending_refusals {
        // Expectation-pending (STEP-0601): the guest formatter is lexical-only,
        // so a semantically-refused but lexically-valid source is accepted and
        // normalized rather than producing a Domain message. The manifest's
        // `rust_format = "refused"` records the Rust formatter's verdict and does
        // not determine a guest-formatter message here, so we surface the
        // observed outcome as a registered divergence for owner decision instead
        // of pinning a fabricated expectation.
        let bytes = std::fs::read(repository.join(path)).unwrap();
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
        eprintln!("STEP-0601 expectation-pending refused case {path}: observed {outcome:?}");
    }
}
