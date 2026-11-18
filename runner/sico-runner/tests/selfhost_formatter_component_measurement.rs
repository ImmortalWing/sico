//! STEP-0608 measurement probe (W3-a): whole-`formatter.sico` Component
//! byte-equality. This is a MEASUREMENT slice, not a parity claim.
//!
//! It records, under the standard guest harness budget (800e9 fuel / 120s):
//! the Rust-oracle Component byte size, the guest `--emit-component-hex`
//! outcome (component hex / typed refusal / fuel exhaustion / IR fall-through),
//! fuel consumed, wall time, the guest component byte size, and — when the
//! guest emits a component — whether the bytes are byte-equal to the oracle
//! and pass `wasmparser::Validator`.
//!
//! Two variants are measured:
//!  A. the real `selfhost/formatter.sico` (entry function `main`);
//!  B. the same source with its entry renamed `main` -> `run` (the transform
//!     the production CLI performs in `prepare_script_module`), to isolate
//!     whether the entry-name mapping is the sole blocker.
//!
//! Ignored by default because it compiles the whole selfhost frontend
//! (`compiler.sico` + `parser.sico` + `codegen.sico`) through the Rust
//! compiler first. Run with:
//!   cargo test -p sico-runner --test selfhost_formatter_component_measurement -- --ignored --exact --nocapture
//!
//! This probe edits no frozen artifact; it only reads `selfhost/formatter.sico`.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use sico_runner::{CancelToken, RunOutcome, RunnerLimits, ScriptInput};

#[path = "support/selfhost_common.rs"]
mod selfhost_common;

use selfhost_common::{PreparedPool, cached_component, default_pool_slots, prepare_slot};

const COMPILER_SOURCE: &str = include_str!("../../../selfhost/compiler.sico");
const COMPILER_LEXER_SOURCE: &str = include_str!("../../../selfhost/compiler_lexer.sico");
const COMPILER_PARSER_SOURCE: &str = include_str!("../../../selfhost/compiler_parser.sico");
const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const CODEGEN_SOURCE: &str = include_str!("../../../selfhost/codegen.sico");
const FORMATTER_SOURCE: &str = include_str!("../../../selfhost/formatter.sico");

/// The Script-profile record/enum preamble every control fixture needs so the
/// guest codegen sees a well-formed `ScriptInput`/`ScriptOutput`/`ScriptError`
/// module (mirrors `ECHO_COMPONENT_FIXTURE_SOURCE` in `selfhost_compiler.rs`).
const SCRIPT_PREAMBLE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

const GUEST_FUEL: u64 = 800_000_000_000;
const GUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const GUEST_MEMORY_BYTES: usize = 2560 * 1024 * 1024;
const GUEST_HOSTCALL_FUEL: usize = 8 << 30;

fn compiler_sources() -> [(&'static str, &'static str); 5] {
    [
        ("compiler.sico", COMPILER_SOURCE),
        ("compiler_lexer.sico", COMPILER_LEXER_SOURCE),
        ("compiler_parser.sico", COMPILER_PARSER_SOURCE),
        ("parser.sico", PARSER_SOURCE),
        ("codegen.sico", CODEGEN_SOURCE),
    ]
}

/// The guest `compiler.sico` Component, built through the real in-process
/// production CLI exactly as `selfhost_compiler.rs::compile_guest` does.
fn guest_compiler_component() -> &'static [u8] {
    static COMPONENT: OnceLock<Vec<u8>> = OnceLock::new();
    COMPONENT.get_or_init(|| cached_component("compiler.sico", &compiler_sources()))
}

fn guest_pool() -> &'static PreparedPool {
    static POOL: OnceLock<PreparedPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let component: Vec<u8> = guest_compiler_component().to_vec();
        PreparedPool::new(default_pool_slots(), move || prepare_slot(&component))
    })
}

fn guest_limits() -> RunnerLimits {
    RunnerLimits {
        fuel: GUEST_FUEL,
        timeout: GUEST_TIMEOUT,
        memory_bytes: GUEST_MEMORY_BYTES,
        hostcall_fuel: GUEST_HOSTCALL_FUEL,
    }
}

/// Runs the guest on `compiler.sico` with no `--emit-*` argument, so its
/// general path returns the guest's own canonical IR for `source`. This is the
/// exact IR text the `--emit-component-hex` path would hand to the emitter.
fn guest_canonical_ir(source: &str) -> Option<String> {
    let pool = guest_pool();
    let prepared = pool.acquire();
    let outcome = prepared.run(
        &ScriptInput {
            arguments: Vec::new(),
            stdin: source.as_bytes().to_vec(),
        },
        &guest_limits(),
        &CancelToken::new(),
    );
    pool.release(prepared);
    match outcome.expect("input bounds hold") {
        RunOutcome::Output(output) if output.exit_code == 0 => {
            Some(String::from_utf8(output.stdout).expect("the IR text is utf-8"))
        }
        other => {
            println!("[ir] guest canonical IR unavailable: {other:?}");
            None
        }
    }
}

/// The injection driver: it reads the canonical IR from stdin and answers with
/// the identity of the first emitter refusal (`component_refusal_for_ir`), or
/// with the emitted component length. This names the gap precisely instead of
/// collapsing it into the CLI's generic "unsupported codegen source shape".
const IR_REFUSAL_SOURCE: &str = concat!(
    "record ScriptInput:\n",
    "  field arguments: List[Text]\n",
    "  field stdin: Bytes\n",
    "end record\n\n",
    "record ScriptOutput:\n",
    "  field stdout: Bytes\n",
    "  field stderr: Bytes\n",
    "  field exit_code: I64\n",
    "end record\n\n",
    "enum ScriptErrorCode:\n",
    "  case InvalidInput\n",
    "  case ResourceLimit\n",
    "  case DomainError\n",
    "  case Cancelled\n",
    "end enum\n\n",
    "record ScriptError:\n",
    "  field code: ScriptErrorCode\n",
    "  field message: Text\n",
    "end record\n\n",
    "use codegen.component_refusal_for_ir\n",
    "use codegen.component_hex_for_ir\n\n",
    "function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n",
    "  let refusal = codegen.component_refusal_for_ir(input.stdin)\n",
    "  if U64.equal(sico.text.length(refusal), U64.literal(0)):\n",
    "    let component_hex = codegen.component_hex_for_ir(input.stdin)\n",
    "    return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(sico.text.length(component_hex))), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n",
    "  end if\n",
    "  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: refusal))\n",
    "end function\n",
);

fn refusal_driver_source() -> &'static str {
    static SOURCE: OnceLock<String> = OnceLock::new();
    SOURCE.get_or_init(|| IR_REFUSAL_SOURCE.to_owned())
}

fn refusal_driver_component() -> &'static [u8] {
    static COMPONENT: OnceLock<Vec<u8>> = OnceLock::new();
    COMPONENT.get_or_init(|| {
        let sources = [
            ("refusal.sico", refusal_driver_source()),
            ("codegen.sico", CODEGEN_SOURCE),
        ];
        cached_component("refusal.sico", &sources)
    })
}

/// Feeds `ir` to the guest emitter through the refusal driver and returns the
/// precise refusal identity, or the emitted component hex length.
fn guest_emitter_refusal(ir: &str) -> String {
    let runner = sico_runner::Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(
            refusal_driver_component(),
            &sico_runner::FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .expect("refusal driver component links");
    let started = Instant::now();
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: ir.as_bytes().to_vec(),
            },
            &guest_limits(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    let wall = started.elapsed();
    match outcome {
        RunOutcome::Output(output) => {
            format!(
                "no-refusal: component_hex_chars={} (wall {wall:.3?})",
                output.stdout.len()
            )
        }
        RunOutcome::Domain { code, message } => {
            format!("refusal: code={code:?} message={message:?} (wall {wall:.3?})")
        }
        other => format!("other: {other:?} (wall {wall:.3?})"),
    }
}

/// Builds the Rust-oracle Component through the real production CLI
/// (`sico build --profile script-v0`), exactly as
/// `selfhost_formatter.rs::compile_formatter` does. The CLI requires a `main`
/// entry (it performs the Script-profile `main` -> `run` mapping in
/// `prepare_script_module`), so this is only valid for the unrenamed source.
fn oracle_component_via_cli(source: &str, tag: &str) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0608-{tag}-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join(format!("{tag}.sico"));
    let component_path = directory.join("component.wasm");
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
    assert_eq!(exit, 0, "{tag}: {}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

/// Builds the Rust-oracle Component for a source whose entry is already
/// spelled `run` (so the CLI's main->run mapping is not needed): lower with the
/// Rust frontend and emit with the Rust backend directly.
fn oracle_component_run_entry(source: &str) -> Vec<u8> {
    let source_file =
        sico_source::SourceFile::from_text(sico_source::SourceId::new(0), "formatter.sico", source)
            .expect("formatter source is bounded");
    let module = sico_ir::lower_core(&source_file).expect("Rust oracle lowers formatter source");
    sico_codegen_wasm::compile_script_program(&module)
        .expect("Rust oracle emits the run-entry formatter Component")
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

fn hex_context(bytes: &[u8], at: usize) -> String {
    let start = at.saturating_sub(16);
    let end = (at + 24).min(bytes.len());
    bytes[start..end]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs the guest `compiler.sico --emit-component-hex` over `source` and
/// records the outcome + fuel + wall into `report`. When a component hex is
/// produced, compares byte-equality against `oracle` and validates.
fn measure_variant(report: &mut String, label: &str, source: &str, oracle: &[u8]) {
    let mut line = |text: &str| {
        println!("[{label}] {text}");
        report.push_str(&format!("[{label}] {text}\n"));
    };

    let pool = guest_pool();
    let prepared = pool.acquire();
    let started = Instant::now();
    let outcome = prepared.run(
        &ScriptInput {
            arguments: vec!["--emit-component-hex".to_owned()],
            stdin: source.as_bytes().to_vec(),
        },
        &guest_limits(),
        &CancelToken::new(),
    );
    let wall = started.elapsed();
    pool.release(prepared);
    let outcome = outcome.expect("input bounds hold");

    line(&format!(
        "budget = fuel {GUEST_FUEL} / timeout {GUEST_TIMEOUT:?} / memory_bytes {GUEST_MEMORY_BYTES}"
    ));
    line(&format!("wall = {wall:.3?}"));
    match &outcome {
        RunOutcome::Output(output) => {
            line(&format!("outcome = Output exit_code={}", output.exit_code));
            line(&format!(
                "stderr = {:?}",
                String::from_utf8_lossy(&output.stderr)
            ));
            let prefix_len = output.stdout.len().min(96);
            line(&format!(
                "stdout_prefix = {:?}",
                String::from_utf8_lossy(&output.stdout[..prefix_len])
            ));
            line(&format!("stdout_len = {}", output.stdout.len()));
            if output.exit_code != 0 {
                line("byte_equal = n/a (guest nonzero exit, typed ScriptError)");
                line("validator = n/a (guest nonzero exit)");
            } else if output.stdout.starts_with(b"record ")
                || output.stdout.starts_with(b"function ")
            {
                line(
                    "verdict = IR_FALLTHROUGH (the --emit-component-hex branch did not emit a component)",
                );
                line("byte_equal = n/a (no component emitted)");
                line("validator = n/a (no component emitted)");
            } else {
                let guest_bytes = decode_hex(&output.stdout);
                line(&format!("component_hex_chars = {}", output.stdout.len()));
                line(&format!("component_bytes = {}", guest_bytes.len()));
                line("verdict = component");
                if oracle.is_empty() {
                    line("byte_equal = n/a (ladder probe, no oracle)");
                    line("validator = n/a (ladder probe, no oracle)");
                } else if guest_bytes == oracle {
                    line("byte_equal = yes");
                } else {
                    let first = guest_bytes
                        .iter()
                        .zip(oracle)
                        .position(|(l, r)| l != r)
                        .unwrap_or_else(|| guest_bytes.len().min(oracle.len()));
                    line(&format!(
                        "byte_equal = no first_divergence_at = {first} guest_len={} oracle_len={}",
                        guest_bytes.len(),
                        oracle.len()
                    ));
                    line(&format!(
                        "guest around divergence: {}",
                        hex_context(&guest_bytes, first)
                    ));
                    line(&format!(
                        "oracle around divergence: {}",
                        hex_context(oracle, first)
                    ));
                }
                if !oracle.is_empty() {
                    match wasmparser::Validator::new().validate_all(&guest_bytes) {
                        Ok(_) => line("validator = pass"),
                        Err(error) => line(&format!("validator = fail: {error}")),
                    }
                }
            }
        }
        RunOutcome::Domain { code, message } => {
            line(&format!(
                "outcome = Domain code={code:?} message={message:?} (typed refusal)"
            ));
            line("byte_equal = n/a (guest refused)");
            line("validator = n/a (guest refused)");
        }
        RunOutcome::FuelExhausted => {
            line("outcome = FuelExhausted");
            line("byte_equal = n/a (guest exhausted fuel)");
            line("validator = n/a (guest exhausted fuel)");
        }
        RunOutcome::Timeout => {
            line("outcome = Timeout");
            line("byte_equal = n/a (guest timed out)");
            line("validator = n/a (guest timed out)");
        }
        other => {
            line(&format!("outcome = {other:?}"));
            line("byte_equal = n/a (guest did not complete)");
            line("validator = n/a (guest did not complete)");
        }
    }
    line("----");
}

#[test]
#[ignore = "STEP-0608 measurement probe: builds the whole selfhost frontend; run with --ignored"]
fn step0608_measure_whole_formatter_component_emission() {
    let mut report = String::new();
    // Variant A: the real formatter.sico (entry `main`), oracle via the
    // production CLI (which performs main->run before emitting).
    let oracle_a = oracle_component_via_cli(FORMATTER_SOURCE, "formatterA");
    println!("oracle_bytes_A_main = {}", oracle_a.len());
    report.push_str(&format!("oracle_bytes_A_main = {}\n", oracle_a.len()));
    measure_variant(&mut report, "A:main", FORMATTER_SOURCE, &oracle_a);

    // Variant B: entry renamed main -> run in the source text. The guest
    // emitter keys the entry on "run", so this isolates whether the entry-name
    // mapping is the sole blocker. Oracle lowers the already-renamed module
    // (entry `run`) straight through the Rust backend.
    let renamed = rename_entry_to_run(FORMATTER_SOURCE);
    let oracle_b = oracle_component_run_entry(&renamed);
    println!("oracle_bytes_B_run = {}", oracle_b.len());
    report.push_str(&format!("oracle_bytes_B_run = {}\n", oracle_b.len()));
    measure_variant(&mut report, "B:run", &renamed, &oracle_b);

    // Prefix ladder on the renamed (run-entry) source: find the largest
    // prefix, cut at a top-level `function`/`end function` boundary, that the
    // guest still emits a component for. This localizes how far the emitter
    // gets before it refuses the whole-file shape.
    prefix_ladder(&mut report, &renamed);

    // Precise refusal identity: hand the guest's own canonical IR straight to
    // the emitter's `component_refusal_for_ir`, which names the first shape it
    // refuses. Isolate whether the Script record declarations, the entry
    // position, or the helper-function body is the blocking shape.
    let echo_run_first = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";
    let echo_identity_first = format!(
        "{SCRIPT_PREAMBLE}function identity(value: I64) returns I64:\n  return value\nend function\n\n{echo_run_first}"
    );
    let echo_run_with_preamble = format!("{SCRIPT_PREAMBLE}{echo_run_first}");
    let formatter_run = &renamed[renamed.find("function run(").unwrap()
        ..renamed[renamed.find("function run(").unwrap()..]
            .find("end function")
            .map(|relative| {
                renamed.find("function run(").unwrap() + relative + "end function".len()
            })
            .unwrap()];
    let formatter_run_only_ir = format!("{SCRIPT_PREAMBLE}{formatter_run}");
    for (label, source) in [
        ("echo run only (no records)", echo_run_first),
        ("echo run + Script records", &echo_run_with_preamble),
        ("echo records + I64 helper + run", &echo_identity_first),
        ("formatter run only + records", &formatter_run_only_ir),
        ("whole-formatter run-entry", &renamed),
        ("whole-formatter main-entry", FORMATTER_SOURCE),
    ] {
        if let Some(ir) = guest_canonical_ir(source) {
            let message = guest_emitter_refusal(&ir);
            println!("[refusal] {label} IR -> {message}");
            report.push_str(&format!("[refusal] {label} IR -> {message}\n"));
        }
    }

    // Bisect which construct in the formatter `run` body trips the emitter.
    // Each fixture isolates one feature against the known-good echo `run`.
    let helper_source_has_lex_error = "function source_has_lex_error(source: Text) returns Bool:\n  return false\nend function\n\n";
    for (label, body) in [
        (
            "bisect: run ok-only",
            "  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\n",
        ),
        (
            "bisect: run + bytes.is_utf8 if",
            "  if sico.bytes.is_utf8(input.stdin):\n    return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"no\"))\n",
        ),
        (
            "bisect: run + utf8_decode + text.encode",
            "  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    return ok(ScriptOutput(stdout: sico.text.encode(source), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"no\"))\n",
        ),
        (
            "bisect: run + user call in if condition",
            "  if source_has_lex_error(sico.bytes.utf8_decode(input.stdin)):\n    return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))\n  end if\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\n",
        ),
        (
            "bisect: run + error nested construct only",
            "  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))\n",
        ),
    ] {
        let fixture = format!(
            "{SCRIPT_PREAMBLE}{helper_source_has_lex_error}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n{body}end function\n"
        );
        if let Some(ir) = guest_canonical_ir(&fixture) {
            let message = guest_emitter_refusal(&ir);
            println!("[bisect] {label} -> {message}");
            report.push_str(&format!("[bisect] {label} -> {message}\n"));
        }
    }

    // Cross-check: the two oracle components should differ only by the entry
    // function name embedded in the export; sizes are reported for honesty.
    let delta = oracle_b.len() as i64 - oracle_a.len() as i64;
    println!("oracle_size_delta_B_minus_A = {delta}");
    report.push_str(&format!("oracle_size_delta_B_minus_A = {delta}\n"));

    // Persist the raw measurement log next to the other step0608 artifacts.
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0608");
    std::fs::create_dir_all(&log_directory).unwrap();
    let log_path = log_directory.join("formatter-component-measurement.log");
    std::fs::write(&log_path, &report).unwrap();
    println!("log written to {}", log_path.display());
}

/// Replaces the formatter's `function run(...)` ... `end function` with
/// `run_body`. The formatter declares exactly one top-level `function run(`
/// (after `rename_entry_to_run`); the replacement keeps the whole helper
/// function set so every user call in the run body stays resolvable.
fn replace_run_body(source: &str, run_body: &str) -> String {
    let start = source
        .find("function run(")
        .expect("renamed source declares function run(");
    let relative = source[start..]
        .find("end function")
        .expect("run function terminates");
    let end = start + relative + "end function".len();
    let mut replaced = String::with_capacity(source.len() - (end - start) + run_body.len());
    replaced.push_str(&source[..start]);
    replaced.push_str(run_body);
    replaced.push_str(&source[end..]);
    replaced
}

/// The real formatter `run` body (the G2 target shape): two nested `if`s and
/// three `return` points, with user calls in the inner condition and before the
/// ok return. This is exactly `formatter.sico`'s `main`, entry renamed.
const FORMATTER_RUN_BODY: &str = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    if source_has_lex_error(source):\n      return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))\n    end if\n    let normalized = normalize_source(source)\n    return ok(ScriptOutput(stdout: sico.text.encode(normalized), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))\nend function\n";

/// G2 verify probe: localize the whole-formatter refusal precisely. Confirms
/// the map/trim intrinsic arms are the blocker and isolates which piece of the
/// nested-if fixture the frontend rejects.
#[test]
#[ignore = "G2 verify: localize frontend vs emitter refusal"]
fn g2_verify_nested_if_run_body() {
    let mut report = String::new();
    // Stubs whose shapes the guest frontend accepts (the real formatter's
    // `source_has_lex_error` returns `true`/`false` and `normalize_source`
    // returns a Text param). `U64.equal(...)` in a Bool return position is a
    // frontend `ERR:E-SH-IR-EXPRESSION`, so avoid it here.
    let stub_has_lex =
        "function source_has_lex_error(src: Text) returns Bool:\n  return true\nend function\n";
    let stub_norm =
        "function normalize_source(src: Text) returns Text:\n  return src\nend function\n";

    // The G2 target: the real formatter run body (nested if + 3 returns), with
    // the two user-call helpers stubbed so the shape reaches the emitter.
    let fixture = format!("{SCRIPT_PREAMBLE}{stub_has_lex}\n{stub_norm}\n{FORMATTER_RUN_BODY}");
    let oracle = oracle_component_run_entry(&fixture);
    measure_variant(
        &mut report,
        "G2:nested-if run + bool/text stubs",
        &fixture,
        &oracle,
    );
    if let Some(ir) = guest_canonical_ir(&fixture) {
        let message = guest_emitter_refusal(&ir);
        println!("[g2-verify] refusal -> {message}");
        report.push_str(&format!("[g2-verify] refusal -> {message}\n"));
    } else {
        println!("[g2-verify] IR unavailable");
        report.push_str("[g2-verify] IR unavailable\n");
    }

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0613c");
    std::fs::create_dir_all(&log_directory).unwrap();
    let log_path = log_directory.join("g2-verify.log");
    std::fs::write(&log_path, &report).unwrap();
    println!("g2 verify log written to {}", log_path.display());
}

/// G2 bisect probe: localize the exact emitter refusal on the whole
/// `formatter.sico`. Keeps the full helper set and varies only the `run` body.
#[test]
#[ignore = "G2 bisect probe: localize the whole-formatter emitter refusal"]
fn g2_bisect_whole_formatter_emission() {
    let renamed = rename_entry_to_run(FORMATTER_SOURCE);
    let mut report = String::new();

    let echo_run = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";

    // One-arm outer if (no nested if, no user calls): isolates whether the
    // nested `if`/user-call shapes are the blocker, against the full helper set.
    let no_inner_if = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    let normalized = source\n    return ok(ScriptOutput(stdout: sico.text.encode(normalized), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))\nend function\n";

    // Nested if with an INTRINSIC condition (not a user call): isolates
    // user-call-in-condition.
    let inner_if_intrinsic_cond = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    if sico.bytes.is_utf8(source):\n      return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))\n    end if\n    let normalized = source\n    return ok(ScriptOutput(stdout: sico.text.encode(normalized), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))\nend function\n";

    // Nested if with a user-call condition, but no user call before the ok
    // return: isolates user-call-before-ok.
    let inner_if_user_cond_no_norm = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  if sico.bytes.is_utf8(input.stdin):\n    let source = sico.bytes.utf8_decode(input.stdin)\n    if source_has_lex_error(source):\n      return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"LEXICAL\"))\n    end if\n    let normalized = source\n    return ok(ScriptOutput(stdout: sico.text.encode(normalized), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: \"source is not UTF-8\"))\nend function\n";

    for (label, body) in [
        ("V0: whole formatter (control)", FORMATTER_RUN_BODY),
        ("V1: echo run body", echo_run),
        ("V2: no inner if", no_inner_if),
        ("V3: inner if intrinsic cond", inner_if_intrinsic_cond),
        (
            "V4: inner if user cond, no normalize call",
            inner_if_user_cond_no_norm,
        ),
    ] {
        let source = replace_run_body(&renamed, body);
        if let Some(ir) = guest_canonical_ir(&source) {
            let message = guest_emitter_refusal(&ir);
            println!("[g2-bisect] {label} -> {message}");
            report.push_str(&format!("[g2-bisect] {label} -> {message}\n"));
        } else {
            println!("[g2-bisect] {label} -> IR unavailable");
            report.push_str(&format!("[g2-bisect] {label} -> IR unavailable\n"));
        }
    }

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0613c");
    std::fs::create_dir_all(&log_directory).unwrap();
    let log_path = log_directory.join("g2-bisect.log");
    std::fs::write(&log_path, &report).unwrap();
    println!("g2 bisect log written to {}", log_path.display());
}

/// Renames the formatter entry `main` -> `run` in the source text. The
/// formatter declares exactly one top-level `function main(`; this rewrites
/// only that declaration so the guest codegen entry lookup (`"run"`) fires.
fn rename_entry_to_run(source: &str) -> String {
    let needle = "function main(";
    let position = source
        .find(needle)
        .expect("formatter.sico declares function main(");
    let mut renamed = String::with_capacity(source.len());
    renamed.push_str(&source[..position]);
    renamed.push_str("function run(");
    renamed.push_str(&source[position + needle.len()..]);
    renamed
}

/// Walks an increasing number of leading helper functions, always appending the
/// whole `run` entry function, and reports for each whether the guest emits a
/// component, refuses, or exhausts. This localizes how many of the formatter's
/// helper functions the guest emitter tolerates before it refuses.
fn prefix_ladder(report: &mut String, renamed: &str) {
    let run_start = renamed
        .find("function run(")
        .expect("renamed source declares function run(");
    let run_end = renamed[run_start..]
        .find("end function")
        .map(|relative| run_start + relative + "end function".len())
        .expect("run function terminates");
    let run_function = &renamed[run_start..run_end];
    let helpers = &renamed[..run_start];

    println!("[ladder] begin (helpers + run entry; run occupies bytes {run_start}..{run_end})");
    report.push_str(&format!(
        "[ladder] begin (helpers + run entry; run occupies bytes {run_start}..{run_end})\n"
    ));

    // Control: the run entry alone, and a known-good identity control, to
    // confirm the guest emitter is functional in this very run before we blame
    // any formatter shape.
    let echo_run = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";
    let identity_only = format!(
        "{SCRIPT_PREAMBLE}function identity(value: Int) returns Int:\n  return value\nend function\n\n{echo_run}"
    );
    let mut section = String::new();
    measure_variant(
        &mut section,
        "control:echo-identity+echo-run (known good)",
        &identity_only,
        &[],
    );
    report.push_str(&section);

    let identity_formatter_run = format!(
        "{SCRIPT_PREAMBLE}function identity(value: Int) returns Int:\n  return value\nend function\n\n{run_function}"
    );
    let mut section = String::new();
    measure_variant(
        &mut section,
        "control:identity+formatter-run",
        &identity_formatter_run,
        &[],
    );
    report.push_str(&section);

    let run_only = format!("{}{}", SCRIPT_PREAMBLE, run_function);
    let mut section = String::new();
    measure_variant(&mut section, "control:formatter-run-only", &run_only, &[]);
    report.push_str(&section);

    let helper_bytes = helpers.as_bytes();
    let mut cut = 0usize;
    let mut index = 0usize;
    while let Some(relative) = find_substring(&helper_bytes[cut..], b"end function") {
        let end = cut + relative + b"end function".len();
        cut = end;
        index += 1;
        let prefix = format!("{}{}", &helpers[..end], run_function);
        let mut section = String::new();
        measure_variant(
            &mut section,
            &format!("ladder@{index}(helper_bytes={end})"),
            &prefix,
            &[],
        );
        report.push_str(&section);
        if !section.contains("verdict = component") {
            println!(
                "[ladder] first non-emitting prefix at helper boundary {index} (helper_bytes={end})"
            );
            report.push_str(&format!(
                "[ladder] first non-emitting prefix at helper boundary {index} (helper_bytes={end})\n"
            ));
            break;
        }
    }
}

fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .find(|&start| &haystack[start..start + needle.len()] == needle)
}

/// Feeds `source` through the guest frontend to its canonical IR, then through
/// the refusal driver (`component_refusal_for_ir`) to the emitter, and records
/// both the frontend outcome and the precise refusal identity. Returns a short
/// one-line summary. `guest_canonical_ir` is None when the frontend refuses the
/// source before codegen is ever reached — that message is the real blocker.
fn guest_refusal_identity(source: &str, report: &mut String, label: &str) -> String {
    match guest_canonical_ir(source) {
        Some(ir) => {
            let message = guest_emitter_refusal(&ir);
            println!("[{label}] frontend: ok; refusal identity -> {message}");
            report.push_str(&format!(
                "[{label}] frontend: ok; refusal identity -> {message}\n"
            ));
            format!("frontend=ok refusal={message}")
        }
        None => {
            println!("[{label}] frontend: REFUSED (no IR reached the emitter)");
            report.push_str(&format!(
                "[{label}] frontend: REFUSED (no IR reached the emitter)\n"
            ));
            "frontend=REFUSED".to_owned()
        }
    }
}

/// STEP-0613f diagnostic probe (W3-a): surface the SPECIFIC emitter refusal
/// identity for the co-gap fixture family — a `run` body using
/// `sico.bytes.utf8_decode` + `sico.text.encode` (NO `text.trim`) — that the
/// driver masks as the generic "unsupported codegen source shape".
///
/// The generic message comes from `component_hex_for_ir` returning `""` when
/// `cg_component_hex` yields `"ERR:..."`. This probe feeds the guest's own
/// canonical IR to `codegen.component_refusal_for_ir` through the refusal
/// driver (exactly how STEP-0608 surfaced `ERR:E-SH-CG-IR-EMIT`), so it names
/// the first shape the emitter refuses instead of collapsing it.
///
/// Fixtures (entry `run`; record/enum preamble from the G1 fixture
/// `IS_UTF8_GATE_FIXTURE_SOURCE` in `selfhost_compiler.rs`):
///   F1  utf8_decode + text.encode — the co-gap fixture (single-block, no
///       `if`, no `text.trim`)
///   F2  text.encode only (no utf8_decode); `sico.text.concat` feeds encode
///   F3  utf8_decode only (no text.encode); the decode result feeds the
///       ScriptError message Text field
///
/// For each fixture it records both (a) the driver outcome the runner actually
/// reports (the generic mask) and (b) the specific `ERR:...` refusal identity
/// from `component_refusal_for_ir`. When the frontend refuses before codegen,
/// `guest_canonical_ir` is None and the driver's own Domain message IS the
/// specific code — no emitter is reached.
///
/// A bisect section (B0..B6) varies one construct at a time against a
/// known-good single-block echo `run` to pin down the exact shape that trips
/// the refusal.
///
/// Ignored by default because it builds the whole selfhost frontend through
/// the Rust compiler first (same as the other probes in this file). Run with:
///   cargo test -p sico-runner --test selfhost_formatter_component_measurement -- --ignored --exact --nocapture m22_w3a_cogap_refusal_identity
#[test]
#[ignore = "STEP-0613f diagnostic probe: surface the masked co-gap refusal identity; run with --ignored"]
fn m22_w3a_cogap_refusal_identity() {
    let mut report = String::new();
    // G1 fixture record/enum preamble (single InvalidInput case), entry `run`.
    let g1_preamble = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

    let f1 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let raw = sico.bytes.utf8_decode(input.stdin)\n  return ok(ScriptOutput(stdout: sico.text.encode(raw), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    );
    let f2 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let t = sico.text.concat(\"a\", \"b\")\n  return ok(ScriptOutput(stdout: sico.text.encode(t), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    );
    let f3 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let raw = sico.bytes.utf8_decode(input.stdin)\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: raw))\nend function\n"
    );

    for (label, source) in [
        ("F1:utf8_decode+text.encode (co-gap)", f1),
        ("F2:text.encode only (no utf8_decode)", f2),
        ("F3:utf8_decode only (no text.encode)", f3),
    ] {
        report.push_str(&format!("==== {label} ====\n"));
        println!("==== {label} ====");

        // (a) What the runner reports: the driver outcome, confirming whether
        // the generic "unsupported codegen source shape" masks this fixture.
        measure_variant(&mut report, &format!("{label}:driver"), &source, &[]);

        // (b) The SPECIFIC refusal identity from component_refusal_for_ir.
        let identity = guest_refusal_identity(&source, &mut report, label);
        report.push_str(&format!("---- {identity}\n"));
    }

    // Bisect the exact shape that trips `ERR:E-SH-IR-CONSTRUCT-VALUE`: vary one
    // construct at a time against the known-good single-block echo `run`.
    let echo = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";
    let b1 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: sico.text.encode(\"hi\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    );
    let b2 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let raw = sico.bytes.utf8_decode(input.stdin)\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n"
    );
    let b3 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let t = sico.text.concat(\"a\", \"b\")\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n"
    );
    let b4 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let raw = input.stdin\n  return ok(ScriptOutput(stdout: raw, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n"
    );
    let b5 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: sico.text.encode(sico.bytes.utf8_decode(input.stdin)), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    );
    let b6 = format!(
        "{g1_preamble}function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: sico.text.encode(sico.text.concat(\"a\", \"b\")), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    );
    for (label, source) in [
        ("B0:control echo run", echo.to_owned()),
        ("B1:text.encode(string literal) only", b1),
        ("B2:let utf8_decode (result unused)", b2),
        ("B3:let text.concat (result unused)", b3),
        ("B4:let alias param + bare-word field value", b4),
        ("B5:inline utf8_decode+text.encode (no let)", b5),
        ("B6:inline text.concat+text.encode (no let)", b6),
    ] {
        report.push_str(&format!("==== BISECT {label} ====\n"));
        println!("==== BISECT {label} ====");
        let identity = guest_refusal_identity(&source, &mut report, &format!("BISECT {label}"));
        report.push_str(&format!("---- {identity}\n"));
    }

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0613f");
    std::fs::create_dir_all(&log_directory).unwrap();
    let log_path = log_directory.join("cogap-refusal-identity.log");
    std::fs::write(&log_path, &report).unwrap();
    println!("log written to {}", log_path.display());
}

/// Splits `text` at each `end function` boundary (top-level function ends).
fn function_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::new();
    let mut cut = 0usize;
    while let Some(rel) = find_substring(text.as_bytes()[cut..].as_ref(), b"end function") {
        let end = cut + rel + b"end function".len();
        boundaries.push(end);
        cut = end;
    }
    boundaries
}

/// The name of the i-th (1-based) top-level function in `text`.
fn nth_function_name(text: &str, boundaries: &[usize], index: usize) -> String {
    let start = if index == 1 { 0 } else { boundaries[index - 2] };
    let end = boundaries[index - 1];
    let slice = &text[start..end];
    slice
        .find("function ")
        .and_then(|p| {
            let ns = p + "function ".len();
            slice[ns..]
                .find('(')
                .map(|r| slice[ns..ns + r].trim().to_owned())
        })
        .unwrap_or_else(|| format!("<fn{index}>"))
}

/// G2 helper bisect probe: find the FIRST formatter helper whose body trips the
/// guest emitter (`ERR:E-SH-CG-IR-EMIT`), by feeding the record preamble + a
/// growing prefix of the formatter helper functions + the echo `run` entry.
/// Also isolates the map-using helper family against the full set.
#[test]
#[ignore = "G2 helper bisect probe: localize which formatter helper trips the emitter"]
fn g2_bisect_helper_functions() {
    let mut report = String::new();
    let renamed = rename_entry_to_run(FORMATTER_SOURCE);

    let run_start = renamed
        .find("function run(")
        .expect("renamed source declares function run(");
    let run_end = renamed[run_start..]
        .find("end function")
        .map(|rel| run_start + rel + "end function".len())
        .expect("run function terminates");
    let _run_function = &renamed[run_start..run_end];
    let helpers = &renamed[..run_start];
    let boundaries = function_boundaries(helpers);
    let helper_count = boundaries.len();
    println!("[helpers] count = {helper_count}");
    report.push_str(&format!("[helpers] count = {helper_count}\n"));

    let echo_run = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";
    let control = format!("{SCRIPT_PREAMBLE}{echo_run}");
    if let Some(ir) = guest_canonical_ir(&control) {
        let message = guest_emitter_refusal(&ir);
        println!("[helpers] control: echo run only -> {message}");
        report.push_str(&format!("[helpers] control: echo run only -> {message}\n"));
    }

    // Ladder: preamble + helpers[0..i] + echo run. The emitter processes
    // functions in order and returns the first failure, so the first prefix
    // that refuses names the first offending helper.
    let mut first_refusal: Option<(usize, String, String)> = None;
    for i in 1..=helper_count {
        let end = boundaries[i - 1];
        let name = nth_function_name(helpers, &boundaries, i);
        let prefix = format!("{}{}\n{}", &helpers[..end], "\n", echo_run);
        let label = format!("helpers[0..{i}]");
        match guest_canonical_ir(&prefix) {
            Some(ir) => {
                let message = guest_emitter_refusal(&ir);
                println!("[helpers] {label} ({name}) -> {message}");
                report.push_str(&format!("[helpers] {label} ({name}) -> {message}\n"));
                if message.starts_with("refusal") && first_refusal.is_none() {
                    first_refusal = Some((i, name.clone(), message.clone()));
                    break;
                }
            }
            None => {
                println!("[helpers] {label} ({name}) -> IR unavailable (frontend)");
                report.push_str(&format!(
                    "[helpers] {label} ({name}) -> IR unavailable (frontend)\n"
                ));
                if first_refusal.is_none() {
                    first_refusal = Some((i, name.clone(), "frontend: IR unavailable".to_owned()));
                    break;
                }
            }
        }
    }

    if let Some((index, name, message)) = &first_refusal {
        println!("[helpers] first refusal at helper index {index} ({name}) -> {message}");
        report.push_str(&format!(
            "[helpers] first refusal at helper index {index} ({name}) -> {message}\n"
        ));
    }

    // Map-hypothesis isolation: the map-using helpers (nearest_match /
    // set_nearest_match / match_arm_levels / normalize_source) are the only
    // helpers that use `sico.map.get` / `sico.map.put` / `sico.map.empty`.
    // Replace them with trivial stubs that keep every earlier helper resolved,
    // and check whether the remaining full set + echo run emits.
    let stub_nearest = "function nearest_match(blocks: Map[Text,U64], depth: U64) returns Text:\n  return \"\"\nend function\n";
    let stub_set_nearest = "function set_nearest_match(active: Map[Text,U64], blocks: Map[Text,U64], depth: U64, value: U64) returns Map[Text,U64]:\n  return active\nend function\n";
    let stub_match_arm = "function match_arm_levels(active: Map[Text,U64], depth: U64) returns U64:\n  return U64.literal(0)\nend function\n";
    let stub_normalize =
        "function normalize_source(src: Text) returns Text:\n  return src\nend function\n";
    let mut stub_set = helpers.to_owned();
    for (needle, stub) in [
        (
            "function nearest_match(blocks: Map[Text,U64], depth: U64) returns Text:",
            stub_nearest,
        ),
        (
            "function set_nearest_match(active: Map[Text,U64], blocks: Map[Text,U64], depth: U64, value: U64) returns Map[Text,U64]:",
            stub_set_nearest,
        ),
        (
            "function match_arm_levels(active: Map[Text,U64], depth: U64) returns U64:",
            stub_match_arm,
        ),
        (
            "function normalize_source(src: Text) returns Text:",
            stub_normalize,
        ),
    ] {
        if let Some(start) = stub_set.find(needle) {
            let rel = stub_set[start..]
                .find("end function")
                .expect("function terminates");
            let end = start + rel + "end function".len();
            let mut replaced = String::with_capacity(stub_set.len() - (end - start) + stub.len());
            replaced.push_str(&stub_set[..start]);
            replaced.push_str(stub);
            replaced.push_str(&stub_set[end..]);
            stub_set = replaced;
        }
    }
    let map_stubbed = format!("{}{}\n{}", SCRIPT_PREAMBLE, stub_set, echo_run);
    match guest_canonical_ir(&map_stubbed) {
        Some(ir) => {
            let message = guest_emitter_refusal(&ir);
            println!("[helpers] map-stubbed full set -> {message}");
            report.push_str(&format!("[helpers] map-stubbed full set -> {message}\n"));
        }
        None => {
            println!("[helpers] map-stubbed full set -> IR unavailable (frontend)");
            report.push_str("[helpers] map-stubbed full set -> IR unavailable (frontend)\n");
        }
    }

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0613g2");
    std::fs::create_dir_all(&log_directory).unwrap();
    let log_path = log_directory.join("g2-helper-bisect.log");
    std::fs::write(&log_path, &report).unwrap();
    println!("g2 helper bisect log written to {}", log_path.display());
}

/// G2 shape isolation probe: pin down the exact shape in `scan_space` that
/// trips the guest emitter's generic `ERR:E-SH-CG-IR-EMIT`. Variants against a
/// minimal base (`add` + the variant + echo run) so each run is cheap.
#[test]
#[ignore = "G2 shape isolation probe: localize the scan_space blocker shape"]
fn g2_isolate_scan_space_shapes() {
    let mut report = String::new();
    let add_fn = "function add(left: U64, right: U64) returns U64:\n  match U64.checked_add(left, right):\n    case ok(value):\n      return value\n    case error(_):\n      return left\n  end match\nend function\n";
    let echo_run = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";

    // The real scan_space body (as in formatter.sico).
    let scan_space = "function scan_space(src: Bytes, start: U64, size: U64) returns U64:\n  let cursor = start\n  while U64.less_than(cursor, size):\n    let b = sico.bytes.at(src, cursor, I64.literal(-1))\n    if I64.equal(b, I64.literal(32)):\n      set cursor = add(cursor, U64.literal(1))\n    else:\n      if I64.equal(b, I64.literal(9)):\n        set cursor = add(cursor, U64.literal(1))\n      else:\n        return cursor\n      end if\n    end if\n  end while\n  return cursor\nend function\n";
    // Straight-line scan_space (no while loop).
    let scan_space_no_while = "function scan_space(src: Bytes, start: U64, size: U64) returns U64:\n  let cursor = start\n  let b = sico.bytes.at(src, cursor, I64.literal(-1))\n  if I64.equal(b, I64.literal(32)):\n    set cursor = add(cursor, U64.literal(1))\n  else:\n    return cursor\n  end if\n  return cursor\nend function\n";
    // While loop with no bytes.at (b is a literal).
    let scan_space_no_bytes_at = "function scan_space(src: Bytes, start: U64, size: U64) returns U64:\n  let cursor = start\n  while U64.less_than(cursor, size):\n    let b = I64.literal(32)\n    if I64.equal(b, I64.literal(32)):\n      set cursor = add(cursor, U64.literal(1))\n    else:\n      if I64.equal(b, I64.literal(9)):\n        set cursor = add(cursor, U64.literal(1))\n      else:\n        return cursor\n      end if\n    end if\n  end while\n  return cursor\nend function\n";
    // While loop with no user call in set (set cursor = cursor).
    let scan_space_no_user_set = "function scan_space(src: Bytes, start: U64, size: U64) returns U64:\n  let cursor = start\n  while U64.less_than(cursor, size):\n    let b = sico.bytes.at(src, cursor, I64.literal(-1))\n    if I64.equal(b, I64.literal(32)):\n      set cursor = cursor\n    else:\n      if I64.equal(b, I64.literal(9)):\n        set cursor = cursor\n      else:\n        return cursor\n      end if\n    end if\n  end while\n  return cursor\nend function\n";
    // Bytes param removed (Text param) to test the Bytes-param hypothesis.
    let _scan_space_text_param = "function scan_space(src: Text, start: U64, size: U64) returns U64:\n  let cursor = start\n  while U64.less_than(cursor, size):\n    let b = sico.bytes.at(sico.text.encode(src), cursor, I64.literal(-1))\n    if I64.equal(b, I64.literal(32)):\n      set cursor = add(cursor, U64.literal(1))\n    else:\n      if I64.equal(b, I64.literal(9)):\n        set cursor = add(cursor, U64.literal(1))\n      else:\n        return cursor\n      end if\n    end if\n  end while\n  return cursor\nend function\n";

    // Focus: is it `bytes.at` or the `I64.literal(-1)` negative constant?
    let neg_only = "function f(src: Bytes, start: U64, size: U64) returns U64:\n  let b = I64.literal(-1)\n  return U64.literal(0)\nend function\n";
    let at_zero = "function f(src: Bytes, start: U64, size: U64) returns U64:\n  let b = sico.bytes.at(src, start, I64.literal(0))\n  return U64.literal(0)\nend function\n";
    let at_neg = "function f(src: Bytes, start: U64, size: U64) returns U64:\n  let b = sico.bytes.at(src, start, I64.literal(-1))\n  return U64.literal(0)\nend function\n";
    let at_neg_no_let = "function f(src: Bytes, start: U64, size: U64) returns U64:\n  let b = sico.bytes.at(src, start, I64.literal(-1))\n  set b = b\n  return U64.literal(0)\nend function\n";

    for (label, variant) in [
        ("control: add + echo run", ""),
        ("scan_space (full)", scan_space),
        ("scan_space (no while)", scan_space_no_while),
        ("scan_space (no bytes.at)", scan_space_no_bytes_at),
        ("scan_space (no user set)", scan_space_no_user_set),
        ("negative const only", neg_only),
        ("bytes.at fallback 0", at_zero),
        ("bytes.at fallback -1", at_neg),
        ("bytes.at fallback -1 (set b)", at_neg_no_let),
    ] {
        let source = format!("{SCRIPT_PREAMBLE}{add_fn}\n{variant}\n{echo_run}");
        report.push_str(&format!("==== {label} ====\n"));
        println!("==== {label} ====");
        match guest_canonical_ir(&source) {
            Some(ir) => {
                let message = guest_emitter_refusal(&ir);
                println!("[{label}] -> {message}");
                report.push_str(&format!("[{label}] -> {message}\n"));
            }
            None => {
                println!("[{label}] -> frontend: IR unavailable");
                report.push_str(&format!("[{label}] -> frontend: IR unavailable\n"));
            }
        }
    }

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0613g2");
    std::fs::create_dir_all(&log_directory).unwrap();
    let log_path = log_directory.join("g2-scan-space-isolate.log");
    std::fs::write(&log_path, &report).unwrap();
    println!(
        "g2 scan-space isolate log written to {}",
        log_path.display()
    );
}

/// G2 main-entry probe: dump the main-entry and run-entry canonical IRs and
/// report which function names they declare, to test whether the main-entry
/// `ERR:E-SH-CG-IR-EMIT` is the entry-name mismatch (no function named `run`).
#[test]
#[ignore = "G2 main-entry probe: dump entry IR function names; run with --ignored"]
fn g2_dump_entry_ir_names() {
    let renamed = rename_entry_to_run(FORMATTER_SOURCE);
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let log_directory = repository.join("target/tmp/step0613g2");
    std::fs::create_dir_all(&log_directory).unwrap();

    for (label, source) in [
        ("main-entry", FORMATTER_SOURCE),
        ("run-entry", renamed.as_str()),
    ] {
        match guest_canonical_ir(source) {
            Some(ir) => {
                let mut names: Vec<&str> = ir
                    .match_indices("\"name\":\"")
                    .filter_map(|(idx, _)| {
                        let rest = &ir[idx + 9..];
                        let end = rest.find('"')?;
                        Some(&rest[..end])
                    })
                    .collect();
                names.dedup();
                println!("[{label}] IR len = {}", ir.len());
                println!("[{label}] function name entries = {names:?}");
                println!("[{label}] has_run_name = {}", names.contains(&"run"));
                println!("[{label}] has_main_name = {}", names.contains(&"main"));
                let path = log_directory.join(format!("{label}-ir.txt"));
                std::fs::write(&path, &ir).unwrap();
                println!("[{label}] IR written to {}", path.display());
            }
            None => println!("[{label}] frontend refused (no IR)"),
        }
    }
}

/// G2 main-entry probe: feed ONLY the whole-formatter main-entry IR to the
/// emitter and report the outcome. This is the focused probe for the remaining
/// `ERR:E-SH-CG-IR-EMIT` gap: it isolates the entry-name blocker (the emitter
/// requires `run`, the main-entry IR names its entry `main`) from the
/// independent fuel wall and any later shape refusal.
#[test]
#[ignore = "G2 main-entry probe: emitter outcome on whole-formatter main-entry IR; run with --ignored"]
fn g2_main_entry_emitter_outcome() {
    match guest_canonical_ir(FORMATTER_SOURCE) {
        Some(ir) => {
            let message = guest_emitter_refusal(&ir);
            println!("[main-entry] whole-formatter main-entry IR -> {message}");
        }
        None => println!("[main-entry] frontend refused (no IR)"),
    }
}

/// G2 main-entry byte-correctness probe: a SMALL main-entry echo fixture must
/// emit byte-identically to the Rust oracle (which maps main -> run before
/// lowering). This proves the emitter's acceptance of a `main`-named entry is
/// byte-correct, independent of the whole-formatter fuel wall.
#[test]
#[ignore = "G2 main-entry probe: byte-equality of a small main-entry fixture; run with --ignored"]
fn g2_main_entry_byte_correctness() {
    let mut report = String::new();
    let echo_main = "function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";
    let fixture = format!("{SCRIPT_PREAMBLE}{echo_main}");
    let oracle = oracle_component_via_cli(&fixture, "echo-main");
    println!("oracle_bytes_main_echo = {}", oracle.len());
    report.push_str(&format!("oracle_bytes_main_echo = {}\n", oracle.len()));
    measure_variant(&mut report, "main-echo", &fixture, &oracle);
    println!("log: {report}");
}

/// Dump the canonical IR around the `sico.bytes.at` intrinsic for a minimal
/// scan_space, so the exact IR shape the emitter sees can be inspected.
#[test]
#[ignore = "G2 IR dump probe: print the bytes.at instruction IR shape"]
fn g2_dump_bytes_at_ir() {
    let add_fn = "function add(left: U64, right: U64) returns U64:\n  match U64.checked_add(left, right):\n    case ok(value):\n      return value\n    case error(_):\n      return left\n  end match\nend function\n";
    let echo_run = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n";
    let scan_space_no_while = "function scan_space(src: Bytes, start: U64, size: U64) returns U64:\n  let cursor = start\n  let b = sico.bytes.at(src, cursor, I64.literal(-1))\n  if I64.equal(b, I64.literal(32)):\n    set cursor = add(cursor, U64.literal(1))\n  else:\n    return cursor\n  end if\n  return cursor\nend function\n";
    let source = format!("{SCRIPT_PREAMBLE}{add_fn}\n{scan_space_no_while}\n{echo_run}");
    if let Some(ir) = guest_canonical_ir(&source) {
        // Find the bytes.at intrinsic region.
        if let Some(at) = ir.find("bytes.at") {
            let start = at.saturating_sub(120);
            let end = (at + 200).min(ir.len());
            println!("--- IR around bytes.at ---");
            println!("{}", &ir[start..end]);
            println!("--- end ---");
            // Also print the whole scan_space function's instructions region.
            let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let log_directory = repository.join("target/tmp/step0613g2");
            std::fs::create_dir_all(&log_directory).unwrap();
            let log_path = log_directory.join("g2-bytes-at-ir.txt");
            std::fs::write(&log_path, &ir).unwrap();
            println!("full IR written to {}", log_path.display());
        } else {
            println!("no bytes.at in IR");
        }
    } else {
        println!("frontend refused scan_space fixture");
    }
}

/// W3-a fuel probe: whole-formatter `main`-entry emission under the standard
/// budget with fuel tracing. Records the consumed fuel, the exhaustion frame
/// (dominant op) when the run exhausts, and byte-equality against the
/// Rust-oracle component when it emits. Measurement only.
#[test]
#[ignore = "W3-a fuel probe: whole-formatter main-entry fuel trace; run with --ignored"]
fn m22_w3a_whole_formatter_main_entry_fuel_trace() {
    let oracle = oracle_component_via_cli(FORMATTER_SOURCE, "formatterA");
    println!("oracle_bytes_A_main = {}", oracle.len());

    let pool = guest_pool();
    let prepared = pool.acquire();
    let started = Instant::now();
    let result = prepared.run_with_fuel(
        &ScriptInput {
            arguments: vec!["--emit-component-hex".to_owned()],
            stdin: FORMATTER_SOURCE.as_bytes().to_vec(),
        },
        &guest_limits(),
        &CancelToken::new(),
    );
    let wall = started.elapsed();
    pool.release(prepared);
    let (outcome, trace) = result.expect("input bounds hold");
    println!("fuel_consumed = {:?}", trace.consumed);
    println!("wall = {wall:.3?}");
    match &outcome {
        RunOutcome::Output(output) => {
            println!("outcome = Output exit_code={}", output.exit_code);
            if output.exit_code == 0 && !output.stdout.is_empty() {
                let guest_bytes = decode_hex(&output.stdout);
                println!("component_bytes = {}", guest_bytes.len());
                if guest_bytes == oracle {
                    println!("byte_equal = yes");
                } else {
                    println!(
                        "byte_equal = no guest_len={} oracle_len={}",
                        guest_bytes.len(),
                        oracle.len()
                    );
                }
            }
        }
        RunOutcome::FuelExhausted => {
            println!("outcome = FuelExhausted");
            for (index, frame) in trace.exhaustion.iter().enumerate() {
                println!(
                    "exhaustion_frame[{index}] func_idx={:?} name={:?} offset={:?} module={:?}",
                    frame.function_index,
                    frame.function_name,
                    frame.instruction_offset,
                    frame.module
                );
            }
        }
        other => println!("outcome = {other:?}"),
    }
}

/// One embedded core module's code-section layout, plus any name-section
/// function names the module carries.
struct ModuleLayout {
    index: usize,
    name: Option<String>,
    import_count: u32,
    defined: u32,
    /// (func_index, body_start, body_end) per defined function, in code order.
    bodies: Vec<(u32, usize, usize)>,
    func_names: std::collections::BTreeMap<u32, String>,
    custom_sections: Vec<String>,
    /// Exported function name -> core func index (from the export section).
    exports: std::collections::BTreeMap<String, u32>,
}

/// Parses every embedded core module of the five-file compiler component with
/// `wasmparser`: code-section body ranges, import/defined counts, and any
/// `name` custom section. This is the five-file analogue of STEP-0611's
/// emit-only layout probe (which does NOT apply to the full compiler).
fn five_file_layouts() -> Vec<ModuleLayout> {
    let component: &[u8] = guest_compiler_component();
    let mut layouts = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(component) {
        let Ok(wasmparser::Payload::ModuleSection {
            unchecked_range, ..
        }) = payload
        else {
            continue;
        };
        let module_bytes = &component[unchecked_range];
        let mut layout = ModuleLayout {
            index: layouts.len(),
            name: None,
            import_count: 0,
            defined: 0,
            bodies: Vec::new(),
            func_names: std::collections::BTreeMap::new(),
            custom_sections: Vec::new(),
            exports: std::collections::BTreeMap::new(),
        };
        for payload in wasmparser::Parser::new(0).parse_all(module_bytes) {
            match payload {
                Ok(wasmparser::Payload::CustomSection(custom)) => {
                    layout.custom_sections.push(custom.name().to_owned());
                    if custom.name() != "name" {
                        continue;
                    }
                    let names = wasmparser::NameSectionReader::new(wasmparser::BinaryReader::new(
                        custom.data(),
                        0,
                    ));
                    for entry in names {
                        let Ok(entry) = entry else {
                            continue;
                        };
                        match entry {
                            wasmparser::Name::Module { name, .. } => {
                                layout.name = Some(name.to_owned());
                            }
                            wasmparser::Name::Function(map) => {
                                for naming in map.into_iter().flatten() {
                                    layout
                                        .func_names
                                        .insert(naming.index, naming.name.to_owned());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(wasmparser::Payload::ImportSection(reader)) => {
                    for group in reader.into_iter().flatten() {
                        for import in group {
                            if let Ok((
                                _,
                                wasmparser::Import {
                                    ty: wasmparser::TypeRef::Func(_),
                                    ..
                                },
                            )) = import
                            {
                                layout.import_count += 1;
                            }
                        }
                    }
                }
                Ok(wasmparser::Payload::FunctionSection(reader)) => {
                    layout.defined = reader.count();
                }
                Ok(wasmparser::Payload::ExportSection(reader)) => {
                    for export in reader.into_iter().flatten() {
                        if let wasmparser::ExternalKind::Func = export.kind {
                            layout.exports.insert(export.name.to_owned(), export.index);
                        }
                    }
                }
                Ok(wasmparser::Payload::CodeSectionEntry(body)) => {
                    let range = body.range();
                    let func_index = layout.import_count + layout.bodies.len() as u32;
                    layout.bodies.push((func_index, range.start, range.end));
                }
                _ => {}
            }
        }
        layouts.push(layout);
    }
    layouts
}

/// W3-a layout probe: dump the five-file compiler component's core-module
/// code-section layout and any name section, so the exhaustion func# indices /
/// offsets can be resolved to Sico functions. Measurement only.
#[test]
#[ignore = "W3-a layout probe: five-file code-section layout + name resolution; run with --ignored"]
fn m22_w3a_five_file_code_section_layout() {
    let layouts = five_file_layouts();
    println!("M22_W3A_LAYOUT module_count={}", layouts.len());
    for layout in &layouts {
        println!(
            "M22_W3A_LAYOUT module={} name={:?} import_count={} defined={} bodies={} name_section_funcs={} custom={:?}",
            layout.index,
            layout.name,
            layout.import_count,
            layout.defined,
            layout.bodies.len(),
            layout.func_names.len(),
            layout.custom_sections
        );
        // Resolve the known exhaustion targets for this module.
        for (target_index, target_offset) in [
            (34u32, None),
            (67, None),
            (68, None),
            (69, None),
            (74, Some(56632usize)),
            (79, None),
            (80, None),
            (81, None),
            (87, None),
            (244, None),
            (245, None),
        ] {
            let named = layout
                .func_names
                .get(&target_index)
                .map(|name| format!("name={name}"))
                .unwrap_or_else(|| "no-name-section-name".to_owned());
            let body = layout
                .bodies
                .iter()
                .find(|(index, _, _)| *index == target_index)
                .map(|(_, start, end)| format!("body[{start}..{end}]"))
                .unwrap_or_else(|| "no-body".to_owned());
            println!(
                "M22_W3A_LAYOUT module={} func#{target_index} {named} {body}",
                layout.index
            );
            if let Some(offset) = target_offset {
                let hit = layout
                    .bodies
                    .iter()
                    .find(|(_, start, end)| *start <= offset && offset < *end)
                    .map(|(index, start, end)| format!("func#{index} body[{start}..{end}]"))
                    .unwrap_or_else(|| "no-body".to_owned());
                println!(
                    "M22_W3A_LAYOUT module={} offset={offset} contained_in={hit}",
                    layout.index
                );
            }
        }
    }
    // Per-file Sico declaration order (the 1:1 hypothesis cross-check).
    for (name, text) in compiler_sources() {
        let functions = sico_top_functions(text);
        println!(
            "M22_W3A_SICO_FILE file={name} top_functions={}",
            functions.len()
        );
        for (position, function) in functions.iter().enumerate() {
            println!(
                "M22_W3A_SICO_FILE file={name} order={position} function={}",
                function.name
            );
        }
    }
}

/// The top-level `function`s of a Sico source, in declaration order (same
/// extraction as `m22_fuel_profile.rs::sico_functions`).
fn sico_top_functions(text: &str) -> Vec<SicoTopFunction> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = Vec::new();
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
                out.push(SicoTopFunction {
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

struct SicoTopFunction {
    name: String,
    #[allow(dead_code)]
    text: String,
}

/// Disassembles the exhaustion-frame core functions and prints their direct
/// wasm call targets, so the inner scan frames can be identified against the
/// IR call graph instead of relying on the (closure-shifted) index mapping.
#[test]
#[ignore = "W3-a disassembly probe: exhaustion-frame call targets; run with --ignored"]
fn m22_w3a_exhaustion_frame_call_targets() {
    let layouts = five_file_layouts();
    let _layout = layouts
        .first()
        .expect("the five-file component has one core module");
    let module_bytes = guest_compiler_component();
    // The core module bytes (single embedded module).
    let core: &[u8] = {
        let mut found = None;
        for payload in wasmparser::Parser::new(0).parse_all(module_bytes) {
            if let Ok(wasmparser::Payload::ModuleSection {
                unchecked_range, ..
            }) = payload
            {
                found = Some(&module_bytes[unchecked_range]);
                break;
            }
        }
        found.expect("one embedded core module")
    };
    // Frames: (frame_index, core_func_index) from the offset containment.
    let frames: [(usize, u32); 15] = [
        (0, 75),
        (1, 80),
        (2, 80),
        (3, 82),
        (4, 88),
        (5, 143),
        (6, 146),
        (7, 170),
        (8, 173),
        (9, 245),
        (10, 245),
        (11, 67),
        (12, 68),
        (13, 70),
        (14, 34),
    ];
    let mut seen: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    for (frame_index, core_index) in frames {
        if !seen.insert(core_index) {
            continue;
        }
        let callees = wasm_call_targets(core, core_index);
        println!("M22_W3A_CALL frame={frame_index} core#={core_index} callees={callees:?}");
    }
}

/// Dumps the body range and direct call targets of core functions 70..95, so
/// the inner exhaustion frames can be identified against the IR call graph.
#[test]
#[ignore = "W3-a layout probe: scan-family call graph; run with --ignored"]
fn m22_w3a_scan_family_call_graph() {
    let layouts = five_file_layouts();
    let layout = layouts
        .first()
        .expect("the five-file component has one core module");
    let module_bytes = guest_compiler_component();
    let core: &[u8] = {
        let mut found = None;
        for payload in wasmparser::Parser::new(0).parse_all(module_bytes) {
            if let Ok(wasmparser::Payload::ModuleSection {
                unchecked_range, ..
            }) = payload
            {
                found = Some(&module_bytes[unchecked_range]);
                break;
            }
        }
        found.expect("one embedded core module")
    };
    for index in 70..=95u32 {
        let body = layout
            .bodies
            .iter()
            .find(|(func, _, _)| *func == index)
            .map(|(_, start, end)| format!("[{start}..{end}]"))
            .unwrap_or_else(|| "<none>".to_owned());
        let callees = wasm_call_targets(core, index);
        println!("M22_W3A_SCAN core#={index} body={body} callees={callees:?}");
    }
    // Code-section byte range, to test whether `module_offset` is
    // module-absolute or code-section-relative.
    for payload in wasmparser::Parser::new(0).parse_all(core) {
        if let Ok(wasmparser::Payload::CodeSectionStart { range, .. }) = payload {
            println!("M22_W3A_SCAN code_section_range={range:?}");
        }
    }
    // The first body's range (where the defined-function code begins).
    for payload in wasmparser::Parser::new(0).parse_all(core) {
        if let Ok(wasmparser::Payload::CodeSectionEntry(body)) = payload {
            println!("M22_W3A_SCAN first_body_range={:?}", body.range());
            break;
        }
    }
}
fn wasm_call_targets(core: &[u8], func_index: u32) -> Vec<u32> {
    let mut targets = Vec::new();
    let mut body_index = 0u32;
    for payload in wasmparser::Parser::new(0).parse_all(core) {
        if let Ok(wasmparser::Payload::CodeSectionEntry(body)) = payload {
            if body_index != func_index {
                body_index += 1;
                continue;
            }
            if let Ok(operators) = body.get_operators_reader() {
                for operator in operators {
                    let Ok(operator) = operator else {
                        break;
                    };
                    match operator {
                        wasmparser::Operator::Call { function_index } => {
                            targets.push(function_index);
                        }
                        wasmparser::Operator::ReturnCall { function_index } => {
                            // Tail call: the frame still nests in a backtrace.
                            targets.push(function_index);
                        }
                        wasmparser::Operator::CallIndirect { type_index, .. } => {
                            // Indirect calls carry a type, not a direct target;
                            // record it as a sentinel so the shape is visible.
                            targets.push(u32::MAX - type_index);
                        }
                        wasmparser::Operator::ReturnCallIndirect { type_index, .. } => {
                            targets.push(u32::MAX - type_index);
                        }
                        _ => {}
                    }
                }
            }
            break;
        }
    }
    targets
}

/// Fuel-vs-size curve for the whole-formatter emission path: the guest compiles
/// progressively larger sources and the consumed fuel is recorded. If fuel
/// grows faster than linearly in the IR size, the emission is superlinear.
#[test]
#[ignore = "W3-a fuel curve probe: whole-formatter fuel vs source size; run with --ignored"]
fn m22_w3a_fuel_vs_size_curve() {
    for function_count in [1usize, 2, 4, 8] {
        let mut source = String::from(SCRIPT_PREAMBLE);
        for index in 0..function_count {
            source.push_str(&format!(
                "function fn{index}(x: U64) returns U64:\n  let s = \"literal-{index}-bytes\"\n  let y = add(x, U64.literal({index}))\n  return y\nend function\n"
            ));
        }
        source.push_str(
            "function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let r = fn0(U64.literal(1))\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))\nend function\n",
        );
        let pool = guest_pool();
        let prepared = pool.acquire();
        let started = Instant::now();
        let result = prepared.run_with_fuel(
            &ScriptInput {
                arguments: vec!["--emit-component-hex".to_owned()],
                stdin: source.as_bytes().to_vec(),
            },
            &guest_limits(),
            &CancelToken::new(),
        );
        let wall = started.elapsed();
        pool.release(prepared);
        let (outcome, trace) = result.expect("input bounds hold");
        println!(
            "M22_W3A_CURVE functions={function_count} outcome={outcome:?} fuel={:?} wall={wall:.2?}",
            trace.consumed
        );
    }
}

/// instruction counts, const_string literals, and the canonical IR text size.
/// This bounds the `cgs_data_offset` O(n²) rescan cost.
#[test]
#[ignore = "W3-a quantification probe: formatter IR size; run with --ignored"]
fn m22_w3a_formatter_ir_size() {
    let source = sico_source::SourceFile::from_text(
        sico_source::SourceId::new(0),
        "formatter.sico",
        FORMATTER_SOURCE,
    )
    .expect("formatter.sico is a legal source");
    let module = sico_ir::lower_core(&source).expect("formatter.sico lowers");
    let functions = module.functions.len();
    let mut blocks = 0usize;
    let mut instructions = 0usize;
    let mut const_strings = 0usize;
    let mut const_bytes = 0usize;
    for function in &module.functions {
        blocks += function.blocks.len();
        for block in &function.blocks {
            instructions += block.instructions.len();
            for instruction in &block.instructions {
                if matches!(instruction.operation, sico_ir::Operation::ConstString(_)) {
                    const_strings += 1;
                }
                if matches!(instruction.operation, sico_ir::Operation::ConstBytes(_)) {
                    const_bytes += 1;
                }
            }
        }
    }
    let ir_text = sico_ir::canonical_json(&module).expect("formatter IR verifies");
    println!(
        "M22_W3A_SIZE functions={functions} blocks={blocks} instructions={instructions} const_strings={const_strings} const_bytes={const_bytes} ir_text_bytes={}",
        ir_text.len()
    );
}

#[test]
#[ignore = "W3-a name probe: resolve exhaustion offsets to Sico functions; run with --ignored"]
fn m22_w3a_whole_formatter_resolve_exhaustion_names() {
    let layouts = five_file_layouts();
    let layout = layouts
        .first()
        .expect("the five-file component has one core module");
    println!(
        "M22_W3A_NAME module=0 defined={} bodies={}",
        layout.defined,
        layout.bodies.len()
    );
    let sources = compiler_sources();
    let (entry_name, entry_text) = sources[0];
    let entry =
        sico_source::SourceFile::from_text(sico_source::SourceId::new(0), entry_name, entry_text)
            .expect("compiler.sico is a legal source");
    let mut import_order: Vec<(&str, &str)> = sources[1..].to_vec();
    import_order.sort_by(|left, right| left.0.cmp(right.0));
    let import_files: Vec<(&str, sico_source::SourceFile)> = import_order
        .iter()
        .map(|(file_name, text)| {
            let module_name = file_name.trim_end_matches(".sico");
            (
                module_name,
                sico_source::SourceFile::from_text(
                    sico_source::SourceId::new(0),
                    *file_name,
                    *text,
                )
                .expect("module source is legal"),
            )
        })
        .collect();
    let imports: Vec<sico_ir::ModuleImport> = import_files
        .iter()
        .map(|(name, source)| sico_ir::ModuleImport { name, source })
        .collect();
    let module =
        sico_ir::lower_core_modules(&entry, &imports, &[]).expect("the five-file program lowers");
    let mut by_id: Vec<&sico_ir::Function> = module.functions.iter().collect();
    by_id.sort_by_key(|function| function.id.0);
    let names: Vec<&str> = by_id
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    println!(
        "M22_W3A_NAME ir_functions={} names={}",
        names.len(),
        names.len()
    );
    // Anchor check: the exported entry function's core index must equal the IR
    // index of that function IF the wasm emits in FunctionId order with ids
    // 1..N mapping to core indices 0..N-1.
    for (export_name, export_index) in &layout.exports {
        let ir_position = names.iter().position(|name| *name == export_name.as_str());
        println!(
            "M22_W3A_NAME export name={export_name} core_index={export_index} ir_position={ir_position:?}"
        );
    }

    // Exhaustion frames as captured by the fuel trace probe (frame[0] is the
    // innermost exhausted op).
    let frames: [(usize, u64); 15] = [
        (74, 56686),
        (79, 58472),
        (80, 58865),
        (81, 60139),
        (87, 65166),
        (143, 123827),
        (145, 125620),
        (170, 164652),
        (173, 168508),
        (244, 239109),
        (245, 242950),
        (67, 54073),
        (68, 55191),
        (69, 55949),
        (34, 40145),
    ];
    for (frame_index, (reported, offset)) in frames.iter().enumerate() {
        let hit = layout
            .bodies
            .iter()
            .find(|(_, start, end)| (*start as u64) <= *offset && *offset < (*end as u64));
        let (core_index, body) = match hit {
            Some((index, start, end)) => (*index, format!("[{start}..{end}]")),
            None => {
                println!(
                    "M22_W3A_NAME frame[{frame_index}] reported#={reported} offset={offset} -> NO_BODY"
                );
                continue;
            }
        };
        // Hypothesis: core func index == IR index (FunctionId order, ids 1..N
        // so IR index = id - 1). Resolve when the index is in the Sico range.
        let name = names
            .get(core_index as usize)
            .map(|name| (*name).to_owned())
            .unwrap_or_else(|| "<beyond-ir>".to_owned());
        println!(
            "M22_W3A_NAME frame[{frame_index}] reported#={reported} offset={offset} core#={core_index} body{body} ir_name={name}"
        );
    }
    // Direct index lookup of the reported component-function indices, for the
    // cross-check that the reported index is NOT the core-module index.
    for reported in [34usize, 67, 68, 69, 74, 79, 80, 81, 87, 244, 245] {
        let name = names
            .get(reported)
            .map(|name| (*name).to_owned())
            .unwrap_or_else(|| "<beyond-ir>".to_owned());
        println!("M22_W3A_NAME direct_index reported#={reported} ir_name={name}");
    }
}

/// W3-a IR-order probe: rebuild the five-file program's IR through the same
/// `lower_core_modules` path the CLI uses, and print the function order so the
/// wasm func index (FunctionId order) can be resolved to Sico names.
#[test]
#[ignore = "W3-a IR-order probe: five-file IR function order; run with --ignored"]
fn m22_w3a_five_file_ir_function_order() {
    let sources = compiler_sources();
    let (entry_name, entry_text) = sources[0];
    let entry =
        sico_source::SourceFile::from_text(sico_source::SourceId::new(0), entry_name, entry_text)
            .expect("compiler.sico is a legal source");
    // Imports in the CLI's deterministic order. The linker stores discovered
    // files in a `BTreeMap` keyed by module name, so the CLI's `imports` order
    // is the sorted module-name order, not the `use` statement order.
    let mut import_order: Vec<(&str, &str)> = sources[1..].to_vec();
    import_order.sort_by(|left, right| left.0.cmp(right.0));
    let import_files: Vec<(&str, sico_source::SourceFile)> = import_order
        .iter()
        .map(|(file_name, text)| {
            // The linker resolves `use <module>` to `<module>.sico`, so the
            // module name is the file name without the `.sico` extension.
            let module_name = file_name.trim_end_matches(".sico");
            (
                module_name,
                sico_source::SourceFile::from_text(
                    sico_source::SourceId::new(0),
                    *file_name,
                    *text,
                )
                .expect("module source is legal"),
            )
        })
        .collect();
    let imports: Vec<sico_ir::ModuleImport> = import_files
        .iter()
        .map(|(name, source)| sico_ir::ModuleImport { name, source })
        .collect();
    let module =
        sico_ir::lower_core_modules(&entry, &imports, &[]).expect("the five-file program lowers");
    println!("M22_W3A_IR functions={}", module.functions.len());
    // FunctionId order (authoritative for the wasm layout if codegen emits in
    // FunctionId order); sorted so the probe is robust to Vec order.
    let mut by_id: Vec<&sico_ir::Function> = module.functions.iter().collect();
    by_id.sort_by_key(|function| function.id.0);
    for (index, function) in by_id.iter().enumerate() {
        println!(
            "M22_W3A_IR index={index} id={} function={}",
            function.id.0, function.name
        );
    }
}

/// W3-a fuel probe: a **budget ladder** over the whole-formatter emission.
///
/// The guest is deterministic, so running the same input under a rising fuel
/// budget always exhausts at the same *op index*; each rung therefore samples
/// a different point of the run and the set of rungs is a (sparse) histogram
/// of where the fuel actually goes.  The guest core module carries no name
/// section, so the exhaustion frame's function index is resolved against the
/// `selfhost/codegen.sico` declaration order (the core module lists the
/// `codegen.sico` functions first, indices 0..N-1; verified against the
/// `component_hex_for_ir` / `cgs_add` / `cgs_sub` / `cgs_mul` anchors).
///
/// Measurement only: no source changes, no gate flips.
#[test]
#[ignore = "W3-a fuel ladder probe: whole-formatter emission fuel distribution; run with --ignored"]
fn m22_w3a_whole_formatter_fuel_ladder() {
    let names: Vec<String> = sico_top_functions(CODEGEN_SOURCE)
        .into_iter()
        .map(|function| function.name)
        .collect();
    let budgets: [u64; 6] = [
        25_000_000_000,
        50_000_000_000,
        100_000_000_000,
        200_000_000_000,
        400_000_000_000,
        800_000_000_000,
    ];
    let mut histogram: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for budget in budgets {
        let mut limits = guest_limits();
        limits.fuel = budget;
        let pool = guest_pool();
        let prepared = pool.acquire();
        let started = Instant::now();
        let result = prepared.run_with_fuel(
            &ScriptInput {
                arguments: vec!["--emit-component-hex".to_owned()],
                stdin: FORMATTER_SOURCE.as_bytes().to_vec(),
            },
            &limits,
            &CancelToken::new(),
        );
        let wall = started.elapsed();
        pool.release(prepared);
        let (outcome, trace) = result.expect("input bounds hold");
        let mut top = String::from("none");
        if let RunOutcome::FuelExhausted = outcome
            && let Some(frame) = trace.exhaustion.first()
        {
            let index = frame.function_index as usize;
            let name = names
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("core#{index}"));
            top = format!(
                "{name} (core#{index}) offset={:?}",
                frame.instruction_offset
            );
            *histogram.entry(name).or_insert(0) += 1;
        }
        println!(
            "M22_W3A_LADDER budget={budget} consumed={:?} wall={wall:.3?} outcome={} top={top}",
            trace.consumed,
            match &outcome {
                RunOutcome::Output(output) => format!("Output({})", output.exit_code),
                RunOutcome::FuelExhausted => "FuelExhausted".to_owned(),
                other => format!("{other:?}"),
            }
        );
    }
    for (name, count) in histogram {
        println!("M22_W3A_LADDER_HIST name={name} samples={count}");
    }
}

/// W3-a fuel probe: **total** fuel for emitting a medium-sized module.
///
/// The whole-formatter emission exceeds the 800e9 default budget, so its
/// consumption can only be read as a lower bound.  This probe emits a module
/// that *completes* inside the budget and reports the exact consumed fuel,
/// which makes it the measurement anchor for any emitter optimization: run it
/// before and after a change and compare the ratio.
///
/// Measurement only: no source changes, no gate flips.
#[test]
#[ignore = "W3-a emission fuel probe: total fuel for a completing module; run with --ignored"]
fn m22_w3a_emission_fuel_for_completing_module() {
    let cases: [(&str, &str); 2] = [
        ("compiler_lexer", COMPILER_LEXER_SOURCE),
        ("compiler_parser", COMPILER_PARSER_SOURCE),
    ];
    for (name, source) in cases {
        let mut limits = guest_limits();
        limits.fuel = 4_000_000_000_000;
        limits.timeout = std::time::Duration::from_secs(600);
        let pool = guest_pool();
        let prepared = pool.acquire();
        let started = Instant::now();
        let result = prepared.run_with_fuel(
            &ScriptInput {
                arguments: vec!["--emit-component-hex".to_owned()],
                stdin: source.as_bytes().to_vec(),
            },
            &limits,
            &CancelToken::new(),
        );
        let wall = started.elapsed();
        pool.release(prepared);
        let (outcome, trace) = result.expect("input bounds hold");
        let emitted = match &outcome {
            RunOutcome::Output(output) if output.exit_code == 0 => decode_hex(&output.stdout).len(),
            _ => 0,
        };
        println!(
            "M22_W3A_EMIT source={name} consumed={:?} wall={wall:.3?} bytes={emitted} outcome={}",
            trace.consumed,
            match &outcome {
                RunOutcome::Output(output) => format!("Output({})", output.exit_code),
                RunOutcome::FuelExhausted => "FuelExhausted".to_owned(),
                other => format!("{other:?}"),
            }
        );
    }
}

/// W3-a fuel probe: the **exact** total fuel of the whole-formatter emission
/// under a raised budget (the default 800e9 only yields a lower bound).
/// Measurement only: no source changes, no gate flips, no limit change in the
/// product (this probe's own budget is a measurement parameter).
#[test]
#[ignore = "W3-a whole-formatter total-fuel probe; run with --ignored"]
fn m22_w3a_whole_formatter_total_fuel() {
    let mut limits = guest_limits();
    limits.fuel = 8_000_000_000_000;
    limits.timeout = std::time::Duration::from_secs(1800);
    let pool = guest_pool();
    let prepared = pool.acquire();
    let started = Instant::now();
    let result = prepared.run_with_fuel(
        &ScriptInput {
            arguments: vec!["--emit-component-hex".to_owned()],
            stdin: FORMATTER_SOURCE.as_bytes().to_vec(),
        },
        &limits,
        &CancelToken::new(),
    );
    let wall = started.elapsed();
    pool.release(prepared);
    let (outcome, trace) = result.expect("input bounds hold");
    let emitted = match &outcome {
        RunOutcome::Output(output) if output.exit_code == 0 => decode_hex(&output.stdout).len(),
        _ => 0,
    };
    println!(
        "M22_W3A_TOTAL consumed={:?} wall={wall:.3?} bytes={emitted} outcome={}",
        trace.consumed,
        match &outcome {
            RunOutcome::Output(output) => format!("Output({})", output.exit_code),
            RunOutcome::FuelExhausted => "FuelExhausted".to_owned(),
            other => format!("{other:?}"),
        }
    );
}

/// W3-a fuel probe: **phase split** of the whole-formatter pipeline.
///
/// Runs the same source through three guest entry points and reports the fuel
/// of each: the default path (canonical IR only ⇒ the *front end*: parse +
/// lower), `--emit-core-hex` (the *core sections* of the emission), and
/// `--emit-component-hex` (the full component).  No source stubbing, so no
/// early-refusal confound: the phases are the guest's own modes.
///
/// Measurement only: no source changes, no gate flips.
#[test]
#[ignore = "W3-a phase-split fuel probe; run with --ignored"]
fn m22_w3a_whole_formatter_phase_split() {
    let modes: [(&str, Option<&str>); 3] = [
        ("frontend_ir", None),
        ("core_hex", Some("--emit-core-hex")),
        ("component_hex", Some("--emit-component-hex")),
    ];
    for (name, mode) in modes {
        let mut limits = guest_limits();
        limits.fuel = 8_000_000_000_000;
        limits.timeout = std::time::Duration::from_secs(1800);
        let mut arguments = Vec::new();
        if let Some(flag) = mode {
            arguments.push(flag.to_owned());
        }
        let pool = guest_pool();
        let prepared = pool.acquire();
        let started = Instant::now();
        let result = prepared.run_with_fuel(
            &ScriptInput {
                arguments,
                stdin: FORMATTER_SOURCE.as_bytes().to_vec(),
            },
            &limits,
            &CancelToken::new(),
        );
        let wall = started.elapsed();
        pool.release(prepared);
        let (outcome, trace) = result.expect("input bounds hold");
        let produced = match &outcome {
            RunOutcome::Output(output) => output.stdout.len(),
            _ => 0,
        };
        println!(
            "M22_W3A_PHASE phase={name} consumed={:?} wall={wall:.3?} stdout_bytes={produced} outcome={}",
            trace.consumed,
            match &outcome {
                RunOutcome::Output(output) => format!("Output({})", output.exit_code),
                RunOutcome::FuelExhausted => "FuelExhausted".to_owned(),
                other => format!("{other:?}"),
            }
        );
    }
}

/// W3-a fuel probe: the **scaling law** of the guest's component emission.
///
/// Emits truncated copies of `formatter.sico` (cut at `end function`
/// boundaries, so every copy is valid Sico) and reports the fuel of each.
/// The fitted exponent says whether the emission cost is a constant factor
/// per byte (linear ⇒ shave constants) or a complexity problem (super-linear
/// ⇒ the representation/assembly must change).
///
/// Measurement only: no source changes, no gate flips.
#[test]
#[ignore = "W3-a emission scaling-law probe; run with --ignored"]
fn m22_w3a_emission_scaling_law() {
    let mut cuts: Vec<usize> = Vec::new();
    let mut seen = 0usize;
    for (index, line) in FORMATTER_SOURCE.lines().enumerate() {
        if line.trim() == "end function" {
            seen += 1;
            if seen == 5 || seen == 10 || seen == 20 {
                cuts.push(index + 1);
            }
        }
    }
    cuts.push(FORMATTER_SOURCE.lines().count());
    let mut sizes: Vec<usize> = Vec::new();
    for cut in &cuts {
        let source: String = FORMATTER_SOURCE
            .lines()
            .take(*cut)
            .collect::<Vec<&str>>()
            .join("\n");
        sizes.push(source.len());
        let mut limits = guest_limits();
        limits.fuel = 8_000_000_000_000;
        limits.timeout = std::time::Duration::from_secs(1800);
        let pool = guest_pool();
        let prepared = pool.acquire();
        let started = Instant::now();
        let result = prepared.run_with_fuel(
            &ScriptInput {
                arguments: vec!["--emit-component-hex".to_owned()],
                stdin: source.as_bytes().to_vec(),
            },
            &limits,
            &CancelToken::new(),
        );
        let wall = started.elapsed();
        pool.release(prepared);
        let (outcome, trace) = result.expect("input bounds hold");
        println!(
            "M22_W3A_SCALE functions_upto={:?} source_bytes={} consumed={:?} wall={wall:.3?} outcome={}",
            cut,
            source.len(),
            trace.consumed,
            match &outcome {
                RunOutcome::Output(output) => format!("Output({})", output.exit_code),
                RunOutcome::FuelExhausted => "FuelExhausted".to_owned(),
                other => format!("{other:?}"),
            }
        );
    }
    println!("M22_W3A_SCALE sizes={sizes:?}");
}
