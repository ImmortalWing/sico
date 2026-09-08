//! M14 STEP-0138: the offline block-game solver port end-to-end through the
//! real Component Runtime. The fixture compiles
//! `tests/end-to-end/block-solver.sico` — a pure-Sico 8x8 bitboard solver
//! (bit operations, general recursion, checked arithmetic, seeded nothing:
//! fully deterministic) — and compares its stdout byte-exactly against the
//! frozen Python-oracle corpus `案例项目/俄罗斯方块消除/oracle/corpus.json`
//! (RFC-0038 §3.1: Python serves only as the fixed comparison oracle).
//!
//! The search budget (200k node enumerations) is a guest-side constant
//! mirrored by the oracle; exceeding it yields the typed application-level
//! `"limit":true` outcome inside the JSON plan, never a trap.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const SOLVER_SOURCE: &str = include_str!("../../../tests/end-to-end/block-solver.sico");

/// (name, board, tray, expected stdout) — frozen oracle corpus outputs.
const CASES: &[(&str, &str, &str, &str)] = &[
    (
        "trivial",
        "................................................................",
        "###|###|###",
        "{\"placed\":3,\"score\":80530,\"moves\":\"0:0,0;1:0,3;2:1,0\",\"limit\":false}",
    ),
    (
        "medium",
        "#..T......#.........##.......#..##............#..#..........#...",
        "##;.#|###|#",
        "{\"placed\":3,\"score\":84026,\"moves\":\"0:4,3;1:4,5;2:4,2\",\"limit\":true}",
    ),
    (
        "hard",
        "#########......##.TT...##......##...#..##..##..##......#########",
        "#|##;##|...#;###",
        "{\"placed\":3,\"score\":280098,\"moves\":\"0:5,5;1:4,6;2:4,0\",\"limit\":false}",
    ),
    (
        "limit",
        "................................................................",
        "#|##|###|####",
        "{\"placed\":4,\"score\":98561,\"moves\":\"0:0,0;1:1,0;2:0,1;3:0,4\",\"limit\":true}",
    ),
];

fn compile_source(tag: u32) -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!(
        "sico-step0138-solver-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("block-solver.sico");
    let component_path = directory.join("block-solver.component.wasm");
    std::fs::write(&source_path, SOLVER_SOURCE).unwrap();
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

fn solver_limits() -> RunnerLimits {
    RunnerLimits {
        fuel: 200_000_000_000,
        timeout: std::time::Duration::from_secs(600),
        ..RunnerLimits::default()
    }
}

fn run_case(component: &[u8], board: &str, tray: &str) -> Vec<u8> {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("solver component links");
    let document = format!(r#"{{"board":"{board}","tray":"{tray}"}}"#);
    match prepared
        .run(
            &ScriptInput {
                stdin: document.into_bytes(),
                ..ScriptInput::default()
            },
            &solver_limits(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
    {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "solver exit for {board}");
            output.stdout
        }
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn solver_matches_frozen_oracle_corpus_byte_exact() {
    let component = compile_source(0);
    for (name, board, tray, expected) in CASES {
        let stdout = run_case(&component, board, tray);
        assert_eq!(
            String::from_utf8_lossy(&stdout),
            *expected,
            "fixture {name} must match the frozen oracle output"
        );
    }
}
