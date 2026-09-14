//! M22 S2 (STEP-0180): the Sico-written checker runs end-to-end and is
//! differentially compared against `sico check`. The simple well-formed
//! program matches (`check ok` both sides); the missing-colon negative case
//! is the honest coverage gap the Sico checker does not yet catch.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const CHECKER_SOURCE: &str = include_str!("../../../selfhost/checker.sico");
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

fn run_checker(component: &[u8], stdin: &[u8]) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("checker component links");
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
    match run_checker(&component, OK_SOURCE.as_bytes()) {
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
    let bad = b"function main() returns Int\n  return 42\nend function\n";
    match run_checker(&component, bad) {
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
