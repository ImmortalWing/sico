//! M22 S4 (STEP-0183): the Sico token emitter collects identifier tokens
//! into a struct-of-arrays (List[Text] words) and joins them — proving the
//! token-stream shape the parser consumes. Six of seven probe tokens are
//! byte-exact; the 4th ("Int") is a slice-offset defect registered honestly.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const TOKENS_SOURCE: &str = include_str!("../../../selfhost/tokens.sico");
const INPUT: &[u8] = b"function main() returns Int:\n  return 42\nend function\n";

fn compile_tokens() -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0183-tokens-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("tokens.sico");
    let component_path = directory.join("tokens.component.wasm");
    std::fs::write(&source_path, TOKENS_SOURCE).unwrap();
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

#[test]
fn sico_token_emitter_collects_identifier_words() {
    let component = compile_tokens();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("tokens component links");
    match prepared
        .run(
            &ScriptInput {
                stdin: INPUT.to_vec(),
                ..ScriptInput::default()
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
    {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            // STEP-0191: the root cause was an application-layer
            // `is_ident_byte` bug (uppercase A-Z misclassified by an early
            // `b < 97` guard), not a compiler defect. All seven tokens
            // are now byte-exact.
            assert_eq!(
                output.stdout,
                b"function,main,returns,Int,return,end,function".to_vec(),
                "token stream drifted"
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}
