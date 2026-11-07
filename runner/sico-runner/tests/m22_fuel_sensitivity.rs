//! STEP-0600 (M22): fuel-ceiling sensitivity of the frozen match-site ledger.
//!
//! This is a **measurement** slice, not a gate. It advances no exit gate, raises
//! no fuel/memory/timeout limit, and touches no production crate, no runner
//! limit default, and no frozen guest source (`selfhost/*` and the ledger
//! `match-component-coverage-v1.json` are read-only here). M22 stays NO-GO.
//!
//! ## What is measured (and why)
//!
//! The owner D8 decision (M22 plan §3.3.2) is "raise the qualification fuel
//! ceiling 800e9 -> 1.6T (2x), or narrow the corpus/closure scope". The missing
//! evidence is a **sensitivity**: how many of the 331 `FuelExhausted` ledger
//! sites would flip to green at 2x (1.6T) and 4x (3.2T) fuel. This file
//! measures exactly that, on the **ledger path** (the full guest compiler once,
//! `--emit-component-hex`), because the ledger verdicts were produced that way
//! (`run_guest_with_args` at 800e9 / 120 s). The emit-only injection path is
//! NOT used here: it skips the guest frontend and runs the emitter twice, so it
//! is not comparable to a ledger verdict.
//!
//! "Green" is precise: the guest must emit a component whose SHA-256 equals the
//! ledger's frozen `component_sha256` (the Rust oracle's production output), so
//! a site that emits but diverges from the oracle is `component-mismatch`, not
//! green. The IR axis is already green for every one of these sites (the ledger
//! verdict is `ir-equal+component-other:FuelExhausted`), so the component axis
//! is the only thing fuel can move.
//!
//! ## Interface
//!
//! * `SICO_M22_SENS_GROUPS`  -- `path:function,...` (required). The unit is the
//!   distinct enclosing function: every site inside a function shares one
//!   closure, one fuel demand and one flip outcome, so one measurement per
//!   function maps to all of its sites.
//! * `SICO_M22_SENS_FUELS`   -- comma-separated fuel budgets (default
//!   `800000000000,1600000000000,3200000000000` = 1x / 2x / 4x of 800e9).
//! * `SICO_M22_SENS_TIMEOUT` -- wall-clock seconds per run (default 300; the
//!   ledger used 120, the D8 2x profile is 180; a generous default keeps FUEL
//!   the binding constraint and reports any wall-clock trap distinctly).
//!
//! Output line (one per group per fuel):
//! `SENS_FUEL path:function fuel=X outcome=... component_bytes=... component_equal=... wall_ms=...`
//! where `outcome` is `component` | `refused:<id>` | `fuel-exhausted` | `trap:<reason>`.
//!
//! Heavy sweeps are `#[ignore]`d (repo convention for measurement probes); the
//! default test is a bounded smoke that proves the hook runs on one real group.

use sha2::{Digest, Sha256};
use sico_runner::{CancelToken, RunOutcome, RunnerLimits, ScriptInput};

#[path = "support/selfhost_common.rs"]
mod selfhost_common;

use selfhost_common::{PreparedPool, cached_component, default_pool_slots, prepare_slot};

const COMPILER_SOURCE: &str = include_str!("../../../selfhost/compiler.sico");
const COMPILER_LEXER_SOURCE: &str = include_str!("../../../selfhost/compiler_lexer.sico");
const COMPILER_PARSER_SOURCE: &str = include_str!("../../../selfhost/compiler_parser.sico");
const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const CODEGEN_SOURCE: &str = include_str!("../../../selfhost/codegen.sico");

const LEDGER_JSON: &str = include_str!("../../../selfhost/match-component-coverage-v1.json");

/// The frozen qualification fuel ceiling (ADR-0015). Measurement ceiling only;
/// no limit is raised by this slice.
const BASE_FUEL: u64 = 800_000_000_000;

/// The Script v0 record prelude the closure source needs.
const HOST_CALL_SHAPE_PRELUDE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

/// The `run` entry every closure module needs to be a legal Script component.
const HOST_CALL_SHAPE_RUN: &str = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

const MATCH_CLOSURE_FUNCTION_LIMIT: usize = 256;
const MATCH_CLOSURE_BYTE_LIMIT: usize = 1_048_576;

fn compiler_sources() -> [(&'static str, &'static str); 5] {
    [
        ("compiler.sico", COMPILER_SOURCE),
        ("compiler_lexer.sico", COMPILER_LEXER_SOURCE),
        ("compiler_parser.sico", COMPILER_PARSER_SOURCE),
        ("parser.sico", PARSER_SOURCE),
        ("codegen.sico", CODEGEN_SOURCE),
    ]
}

/// The guest compiler component, built once through the shared on-disk cache.
fn compile_guest() -> &'static [u8] {
    static COMPONENT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    COMPONENT.get_or_init(|| cached_component("compiler.sico", &compiler_sources()))
}

/// The ledger path answer for one source under an explicit fuel budget and
/// wall-clock timeout: the full guest compiler once with `--emit-component-hex`.
fn guest_component_with_fuel(source: &str, fuel: u64, timeout_secs: u64) -> GuestFuelEmission {
    static POOL: std::sync::OnceLock<PreparedPool> = std::sync::OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let component: Vec<u8> = compile_guest().to_vec();
        PreparedPool::new(default_pool_slots(), move || prepare_slot(&component))
    });
    let prepared = pool.acquire();
    let outcome = prepared.run(
        &ScriptInput {
            arguments: vec!["--emit-component-hex".into()],
            stdin: source.as_bytes().to_vec(),
        },
        &RunnerLimits {
            fuel,
            timeout: std::time::Duration::from_secs(timeout_secs),
            memory_bytes: 2560 * 1024 * 1024,
            hostcall_fuel: 8 << 30,
        },
        &CancelToken::new(),
    );
    pool.release(prepared);
    match outcome.expect("input bounds hold") {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            GuestFuelEmission::Component(
                String::from_utf8(output.stdout).expect("the hex is utf-8"),
            )
        }
        RunOutcome::Domain { code, message } => {
            assert_eq!(code, "invalid-input", "the compiler refuses");
            GuestFuelEmission::Refusal(message)
        }
        RunOutcome::FuelExhausted => GuestFuelEmission::FuelExhausted,
        RunOutcome::Trap(reason) => GuestFuelEmission::Trap { reason },
        other => panic!("unexpected outcome under a fuel budget: {:?}", other),
    }
}

enum GuestFuelEmission {
    Component(String),
    Refusal(String),
    FuelExhausted,
    Trap { reason: String },
}

/// The guest compiler's answer in **default (lower-only) mode** for one source
/// under an explicit fuel budget. The default mode lowers the input to canonical
/// IR: it prints the IR JSON (`"schema":"sico.ir.v0"`) when it succeeds
/// (`LOWERED`), prints an `ERR:` identity when it refuses, or exhausts/traps.
/// This is the path the W4 corpus-parity probe runs (`sico run compiler.sico`,
/// source on stdin, no `--emit-component-hex`), so a before/after fuel
/// comparison is like for like with the parity record.
enum GuestLowerEmission {
    Lowered,
    Refusal(String),
    FuelExhausted,
    Trap { reason: String },
}

fn guest_lower_with_fuel(source: &str, fuel: u64, timeout_secs: u64) -> GuestLowerEmission {
    static POOL: std::sync::OnceLock<PreparedPool> = std::sync::OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let component: Vec<u8> = compile_guest().to_vec();
        PreparedPool::new(default_pool_slots(), move || prepare_slot(&component))
    });
    let prepared = pool.acquire();
    let outcome = prepared.run(
        &ScriptInput {
            arguments: Vec::new(),
            stdin: source.as_bytes().to_vec(),
        },
        &RunnerLimits {
            fuel,
            timeout: std::time::Duration::from_secs(timeout_secs),
            memory_bytes: 2560 * 1024 * 1024,
            hostcall_fuel: 8 << 30,
        },
        &CancelToken::new(),
    );
    pool.release(prepared);
    match outcome.expect("input bounds hold") {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            let stdout = String::from_utf8(output.stdout).expect("the IR text is utf-8");
            if stdout.contains("\"schema\":\"sico.ir.v0\"") {
                GuestLowerEmission::Lowered
            } else {
                GuestLowerEmission::Refusal(stdout)
            }
        }
        RunOutcome::Domain { code, message } => {
            assert_eq!(code, "invalid-input", "the compiler refuses");
            GuestLowerEmission::Refusal(message)
        }
        RunOutcome::FuelExhausted => GuestLowerEmission::FuelExhausted,
        RunOutcome::Trap(reason) => GuestLowerEmission::Trap { reason },
        other => panic!("unexpected outcome under a fuel budget: {:?}", other),
    }
}

/// One parsed top-level `function` in a frozen Sico source.
struct SicoFunction {
    name: String,
    text: String,
}

fn sico_functions(text: &str) -> Vec<SicoFunction> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<SicoFunction> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        if let Some(rest) = trimmed.strip_prefix("function ") {
            let name: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            let mut end = index;
            while end < lines.len() && lines[end].trim() != "end function" {
                end += 1;
            }
            if end < lines.len() && !name.is_empty() {
                out.push(SicoFunction {
                    name,
                    text: lines[index..=end].join("\n"),
                });
                index = end + 1;
                continue;
            }
        }
        index += 1;
    }
    out
}

/// The same-file functions a body calls (mirrors the ledger's closure builder).
fn sico_callees(text: &str, known: &[String]) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found: Vec<String> = Vec::new();
    for index in 0..bytes.len() {
        if bytes[index] != b'(' {
            continue;
        }
        let mut start = index;
        while start > 0 {
            let byte = bytes[start - 1];
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                start -= 1;
            } else {
                break;
            }
        }
        if start == index || start == 0 {
            continue;
        }
        let before = bytes[start - 1];
        if before == b'.' || before.is_ascii_alphanumeric() || before == b'_' {
            continue;
        }
        let name = &text[start..index];
        if known.iter().any(|candidate| candidate == name)
            && !found.iter().any(|candidate| candidate == name)
        {
            found.push(name.to_string());
        }
    }
    found
}

/// The transitive same-file dependency closure of `root`, in file order.
fn sico_closure(
    text: &str,
    root: &str,
    function_limit: usize,
    byte_limit: usize,
) -> Result<String, String> {
    let functions = sico_functions(text);
    let names: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
    let mut selected = vec![false; functions.len()];
    let mut stack = vec![root.to_string()];
    let mut count = 0usize;
    let mut bytes = 0usize;
    while let Some(name) = stack.pop() {
        let Some(position) = functions.iter().position(|f| f.name == name) else {
            continue;
        };
        if selected[position] {
            continue;
        }
        selected[position] = true;
        count += 1;
        bytes += functions[position].text.len();
        if count > function_limit || bytes > byte_limit {
            return Err(format!("closure-too-large:{count}fn/{bytes}B"));
        }
        for callee in sico_callees(&functions[position].text, &names) {
            stack.push(callee);
        }
    }
    let mut out = String::new();
    for (position, function) in functions.iter().enumerate() {
        if selected[position] {
            out.push_str(&function.text);
            out.push('\n');
        }
    }
    Ok(out)
}

fn frozen_source(path: &str) -> Option<&'static str> {
    Some(match path {
        "codegen.sico" => CODEGEN_SOURCE,
        "compiler.sico" => COMPILER_SOURCE,
        "compiler_lexer.sico" => COMPILER_LEXER_SOURCE,
        "compiler_parser.sico" => COMPILER_PARSER_SOURCE,
        "parser.sico" => PARSER_SOURCE,
        _ => return None,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_hex(text: &[u8]) -> Vec<u8> {
    fn nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("non-canonical hex byte"),
        }
    }
    assert_eq!(text.len() % 2, 0);
    let (pairs, remainder) = text.as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect()
}

/// The frozen expected component SHA-256 for a (path, function), or `None` when
/// the ledger has no such site. The ledger records the Rust oracle's component
/// digest for every site, even the FuelExhausted ones.
fn expected_component_sha(path: &str, function: &str) -> Option<String> {
    let ledger: serde_json::Value =
        serde_json::from_str(LEDGER_JSON).expect("the frozen ledger parses");
    ledger["sites"].as_array()?.iter().find_map(|site| {
        if site["path"].as_str() == Some(path) && site["function"].as_str() == Some(function) {
            site["component_sha256"].as_str().map(str::to_owned)
        } else {
            None
        }
    })
}

#[test]
#[ignore = "a fuel-ceiling sensitivity measurement probe; run it explicitly"]
fn the_fuel_sensitivity_of_real_match_groups_is_measured() {
    let raw = std::env::var("SICO_M22_SENS_GROUPS").unwrap_or_default();
    let groups: Vec<(String, String)> = raw
        .split(',')
        .filter_map(|part| {
            let mut fields = part.trim().split(':');
            let path = fields.next()?;
            let function = fields.next()?;
            Some((path.to_owned(), function.to_owned()))
        })
        .collect();
    assert!(
        !groups.is_empty(),
        "set SICO_M22_SENS_GROUPS=path:function,..."
    );
    let fuels: Vec<u64> = std::env::var("SICO_M22_SENS_FUELS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| part.trim().parse::<u64>().ok())
                .collect()
        })
        .filter(|list: &Vec<u64>| !list.is_empty())
        .unwrap_or_else(|| vec![BASE_FUEL, 2 * BASE_FUEL, 4 * BASE_FUEL]);
    let timeout = std::env::var("SICO_M22_SENS_TIMEOUT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(300);

    println!("SENS_FUEL fuels={fuels:?} timeout={timeout}s");
    for (path, function) in groups {
        let Some(text) = frozen_source(&path) else {
            panic!("the frozen source of {path} is loaded");
        };
        let closure = match sico_closure(
            text,
            &function,
            MATCH_CLOSURE_FUNCTION_LIMIT,
            MATCH_CLOSURE_BYTE_LIMIT,
        ) {
            Ok(closure) => closure,
            Err(reason) => {
                println!("SENS_FUEL {path}:{function} closure {reason}");
                continue;
            }
        };
        let source = format!("{HOST_CALL_SHAPE_PRELUDE}{closure}\n{HOST_CALL_SHAPE_RUN}");
        let expected_sha = expected_component_sha(&path, &function);
        for &fuel in &fuels {
            let started = std::time::Instant::now();
            let outcome = guest_component_with_fuel(&source, fuel, timeout);
            let wall_ms = started.elapsed().as_millis();
            let (outcome_text, component_bytes, component_equal) = match outcome {
                GuestFuelEmission::Component(hex) => {
                    let equal = expected_sha
                        .as_deref()
                        .map(|expected| sha256_hex(&decode_hex(hex.as_bytes())) == expected)
                        .unwrap_or(false);
                    (
                        "component".to_owned(),
                        hex.len() / 2,
                        if equal { "yes" } else { "no" },
                    )
                }
                GuestFuelEmission::Refusal(identity) => (format!("refused:{identity}"), 0, "-"),
                GuestFuelEmission::FuelExhausted => ("fuel-exhausted".to_owned(), 0, "-"),
                GuestFuelEmission::Trap { reason } => (format!("trap:{reason}"), 0, "-"),
            };
            println!(
                "SENS_FUEL {path}:{function} fuel={fuel} outcome={outcome_text} component_bytes={component_bytes} component_equal={component_equal} wall_ms={wall_ms} source_bytes={}",
                source.len()
            );
        }
    }
}

/// STEP-0600 W4 arm: the 3 remaining corpus-parity mismatches
/// (`scope-nesting-limit.sico`, all a FuelExhausted wall in the guest) are
/// re-run at higher fuel. "Pass" means the guest reaches a typed refusal
/// (matching the Rust `REFUSED:LEXICAL`) instead of exhausting. `SICO_M22_SENS_FILES`
/// selects the corpus files (comma-separated paths); the default is the 3 known
/// scope-nesting-limit cases.
#[test]
#[ignore = "a corpus-refusal fuel sensitivity probe; run it explicitly"]
fn the_fuel_sensitivity_of_corpus_refusals_is_measured() {
    let files: Vec<String> = std::env::var("SICO_M22_SENS_FILES")
        .ok()
        .map(|value| value.split(',').map(str::to_owned).collect())
        .filter(|list: &Vec<String>| !list.is_empty())
        .unwrap_or_else(|| {
            vec![
                "semantic-cases/future-task/invalid/scope-nesting-limit.sico".to_owned(),
                "syntax-candidates/b/future-task/invalid/scope-nesting-limit.sico".to_owned(),
                "syntax-candidates/c/future-task/invalid/scope-nesting-limit.sico".to_owned(),
            ]
        });
    let fuels: Vec<u64> = std::env::var("SICO_M22_SENS_FUELS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| part.trim().parse::<u64>().ok())
                .collect()
        })
        .filter(|list: &Vec<u64>| !list.is_empty())
        .unwrap_or_else(|| vec![BASE_FUEL, 2 * BASE_FUEL, 4 * BASE_FUEL]);
    let timeout = std::env::var("SICO_M22_SENS_TIMEOUT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(300);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");

    println!("SENS_CORPUS fuels={fuels:?} timeout={timeout}s");
    for file in files {
        let path = root.join(&file);
        let content = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(reason) => {
                println!("SENS_CORPUS {file} unreadable={reason}");
                continue;
            }
        };
        let source = String::from_utf8_lossy(&content).into_owned();
        for &fuel in &fuels {
            let started = std::time::Instant::now();
            let outcome = guest_lower_with_fuel(&source, fuel, timeout);
            let wall_ms = started.elapsed().as_millis();
            let outcome_text = match outcome {
                GuestLowerEmission::Lowered => "lowered".to_owned(),
                GuestLowerEmission::Refusal(identity) => {
                    let id = identity
                        .lines()
                        .find(|line| line.contains("ERR:"))
                        .unwrap_or(&identity)
                        .trim();
                    format!("refused:{id}")
                }
                GuestLowerEmission::FuelExhausted => "fuel-exhausted".to_owned(),
                GuestLowerEmission::Trap { reason } => format!("trap:{reason}"),
            };
            println!(
                "SENS_CORPUS {file} fuel={fuel} outcome={outcome_text} wall_ms={wall_ms} source_bytes={}",
                source.len()
            );
        }
    }
}
