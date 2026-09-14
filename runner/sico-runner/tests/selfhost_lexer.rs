//! M22 S3 (STEP-0182): the Sico-written lexer's classification walk runs
//! end-to-end through the real Component Runtime, counting identifiers,
//! integer literals, and punctuation of a Sico source by byte class.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const LEXER_SOURCE: &str = include_str!("../../../selfhost/lexer.sico");
const INPUT: &[u8] = b"function main() returns Int:\n  return 42\nend function\n";

fn compile_lexer() -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0182-lexer-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("lexer.sico");
    let component_path = directory.join("lexer.component.wasm");
    std::fs::write(&source_path, LEXER_SOURCE).unwrap();
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
fn sico_lexer_classifies_source_bytes() {
    let component = compile_lexer();
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("lexer component links");
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
            assert_eq!(
                output.stdout,
                b"ident=7 int=2 punct=4".to_vec(),
                "lexer classification drifted"
            );
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}
