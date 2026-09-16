//! M22 S5 (STEP-0192): the Sico-written parser consumes the token stream
//! (struct-of-arrays `List[Text]` words from STEP-0191) and counts
//! declaration keywords — the first parser-level shape over the token
//! stream.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const INPUT: &[u8] = b"function main() returns Int:\n  return 42\nend function\n";
const INPUT_TWO_PARAMS: &[u8] =
    b"function add(a: I64, b: I64) returns I64:\n  return a\nend function\n";

fn compile_parser(tag: &str) -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!("sico-step0192-parser-{tag}-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("parser.sico");
    let component_path = directory.join("parser.component.wasm");
    std::fs::write(&source_path, PARSER_SOURCE).unwrap();
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

fn run_parser(tag: &str, input: &[u8]) -> Vec<u8> {
    let component = compile_parser(tag);
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("parser component links");
    match prepared
        .run(
            &ScriptInput {
                stdin: input.to_vec(),
                ..ScriptInput::default()
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
    {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            output.stdout
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn sico_parser_extracts_function_names() {
    // STEP-0194: the parser extracts function name + param count
    // (the IR-signature shape): `function main()` yields `fn:main/0`.
    assert_eq!(
        run_parser("main0", INPUT),
        b"fn:main/0".to_vec(),
        "parser summary drifted"
    );
}

#[test]
fn sico_parser_counts_parameters() {
    // STEP-0195: the arity walk counts the full parameter list; the
    // punctuation-free word stream carries each `name: Type` parameter as
    // two words, so `function add(a: I64, b: I64)` yields `fn:add/2`.
    assert_eq!(
        run_parser("add2", INPUT_TWO_PARAMS),
        b"fn:add/2".to_vec(),
        "param count drifted"
    );
}
