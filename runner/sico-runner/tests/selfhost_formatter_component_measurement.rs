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
