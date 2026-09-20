//! M22 S2: the Sico-written checker runs end-to-end. STEP-0242 reuses the
//! integrated lossless lexer and closes the frozen corpus's exact 116/99
//! lexical accept/refuse partition without pretending semantic parity.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits,
    ScriptInput,
};

const CHECKER_SOURCE: &str = include_str!("../../../selfhost/checker.sico");
const COMPILER_LEXER_SOURCE: &str = include_str!("../../../selfhost/compiler_lexer.sico");
const OK_SOURCE: &str = "function main() returns Int:\n  return 42\nend function\n";

fn compile_checker() -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0180-checker-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("checker.sico");
    let component_path = directory.join("checker.component.wasm");
    std::fs::write(&source_path, CHECKER_SOURCE).unwrap();
    std::fs::write(directory.join("compiler_lexer.sico"), COMPILER_LEXER_SOURCE).unwrap();
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

fn run_checker(prepared: &PreparedProgram, stdin: &[u8]) -> RunOutcome {
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
fn sico_checker_matches_rust_on_well_formed_program() {
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
    match run_checker(&prepared, OK_SOURCE.as_bytes()) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            assert!(
                output.stdout.starts_with(b"check ok"),
                "expected check ok, got {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn sico_checker_catches_missing_colon_where_rust_refuses() {
    // STEP-0181: the Sico checker now reports the missing-colon function
    // header (the declaration colon-shape check via `sico.text.ends_with`),
    // matching the Rust checker's refusal on the same input.
    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
    let bad = b"function main() returns Int\n  return 42\nend function\n";
    match run_checker(&prepared, bad) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0);
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(
                text.contains("line 0"),
                "expected a line report, got {text}"
            );
            assert!(text.contains("function main() returns Int"), "got {text}");
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn sico_checker_matches_the_frozen_lexical_partition() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(repository.join("selfhost/corpus-v0.json")).unwrap())
            .unwrap();
    let entries = manifest["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 215);

    let component = compile_checker();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
    let limits = RunnerLimits {
        fuel: 5_000_000_000,
        timeout: std::time::Duration::from_secs(30),
        ..RunnerLimits::default()
    };
    let mut lexical = 0;
    let mut non_lexical = 0;

    for entry in entries {
        let path = entry["path"].as_str().unwrap();
        let expected_lexical = entry["diagnostic_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "LEXICAL");
        let source = std::fs::read(repository.join(path)).unwrap();
        let outcome = prepared
            .run(
                &ScriptInput {
                    stdin: source,
                    ..ScriptInput::default()
                },
                &limits,
                &CancelToken::new(),
            )
            .expect("frozen source fits runner input bounds");
        if expected_lexical {
            lexical += 1;
            assert_eq!(
                outcome,
                RunOutcome::Domain {
                    code: "invalid-input".to_owned(),
                    message: "LEXICAL".to_owned(),
                },
                "{path}"
            );
        } else {
            non_lexical += 1;
            assert!(
                matches!(outcome, RunOutcome::Output(_)),
                "non-lexical source must not be classified as lexical: {path}: {outcome:?}"
            );
        }
    }

    assert_eq!((lexical, non_lexical), (116, 99));
}
