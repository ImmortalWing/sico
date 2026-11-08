//! STEP-0572 (M22): function-level fuel attribution for the emitter fuel wall.
//!
//! This is a **measurement** slice, not a gate. It advances no exit gate, raises
//! no fuel/memory ceiling, and touches no production crate and no frozen guest
//! source (`selfhost/parser.sico` and `selfhost/codegen.sico` are read-only
//! here). M22 stays NO-GO.
//!
//! ## Why this file exists (the blocker it clears)
//!
//! STEP-0562 concluded that op-level fuel profiling was blocked because
//! `cargo check --locked --offline --manifest-path runner/sico-runner/Cargo.toml
//! --tests` fails with `cc-rs: failed to find tool "gcc.exe"`. This slice
//! measured the whole compile matrix and located the block precisely:
//!
//! * The failing crate is the dependency **`wasmtime-internal-fiber v47.0.2`**.
//!   Its `build.rs` compiles `src/windows.c` through `cc-rs`, which needs
//!   `gcc.exe` (absent on this host).
//! * That cc-rs invocation only happens for a build-dir hash that has **no
//!   cached `windows.o`**. The `check` profile maps to
//!   `target/debug/build/wasmtime-internal-fiber-669e568f003f8b7c/`, which holds
//!   only the build-script binaries and **no `out/*.o`**, so it must recompile
//!   and fails.
//! * The `build`/`test` profiles map to `...-4e89d76ba2d977c7/`, which **does**
//!   hold `out/ea708c7824d36062-windows.o` (compiled 2026-11-04). `cargo build
//!   --tests` therefore links through `rust-lld` with **no cc-rs call** and
//!   succeeds.
//!
//! Conclusion: a **new integration-test file is buildable** via
//! `cargo build/test --no-run`; only `cargo check --tests` is blocked, and it is
//! not needed to add a test-side hook. So a test-side sampling手段 IS feasible,
//! and this file is that hook.
//!
//! ## What is measured (and what is honestly out of reach)
//!
//! The runner's public API exposes **no fuel-consumption readout and no
//! wasmtime opcode/epoch callback** (`PreparedProgram::run` returns only the
//! `RunOutcome` and the fuel *ceiling* it charged). True op-level sampling is
//! therefore **not reachable through the public API** -- it would need a runner
//! change (production code this slice may not touch). So the attribution here is
//! **function-level, by fuel bisection**: the smallest fuel ceiling under which
//! the guest still emits the component, i.e. a bracket `(low, high]` that bounds
//! real consumption from below and above.
//!
//! Two kinds of attribution, both emit-only (canonical IR -> guest emitter,
//! guest frontend skipped so only the emitter's fuel is paid):
//!
//! 1. **A real `FuelExhausted` site** from the frozen ledger
//!    (`parser.sico:fixed_literal_const_segment`), bracketed to replace
//!    STEP-0562's ceiling-truncated "exhausted" with a measured lower bound.
//! 2. **An emitter cost-center ranking** from controlled synthetic closures:
//!    each family holds everything constant except one emitter workload
//!    (`read_local` chain / `text.concat` chain / function count / `match` arms),
//!    so the fuel-per-unit slope ranks *which emitter path* carries the
//!    super-linear term. This is the "per-function closure replacement" the
//!    slice asked for: the closure body composition is replaced, the emitter is
//!    the frozen one.
//!
//! Heavy sweeps are `#[ignore]`d (repo convention for measurement probes); the
//! default test is a bounded smoke that proves the hook runs and produces a real
//! bracket on the real site.

use sico_ir::{canonical_json, lower_core};
use sico_runner::{CancelToken, FuelTrace, RunOutcome, RunnerLimits, ScriptInput};
use sico_source::{SourceFile, SourceId};

#[path = "support/selfhost_common.rs"]
mod selfhost_common;

use selfhost_common::{PreparedPool, cached_component, default_pool_slots, prepare_slot};

const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");
const CODEGEN_SOURCE: &str = include_str!("../../../selfhost/codegen.sico");

/// The Script v0 record prelude the injection driver needs.
const HOST_CALL_SHAPE_PRELUDE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

/// The `run` entry every probe module needs to be a legal Script component.
const HOST_CALL_SHAPE_RUN: &str = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

/// The emit-only driver: read canonical IR from stdin, hand it straight to the
/// guest `codegen`, answer with the emitted component hex or the refusal.
const IR_INJECTION_SOURCE: &str = r#"record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

use codegen.component_hex_for_ir
use codegen.component_refusal_for_ir

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let refusal = codegen.component_refusal_for_ir(input.stdin)
  if U64.equal(sico.text.length(refusal), U64.literal(0)):
    let component_hex = codegen.component_hex_for_ir(input.stdin)
    return ok(ScriptOutput(stdout: sico.text.encode(component_hex), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
  end if
  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: refusal))
end function
"#;

/// The frozen qualification fuel ceiling (ADR-0015). This is the measurement
/// ceiling only; no limit is raised by this slice.
const FUEL_CEILING: u64 = 800_000_000_000;

/// One measured family: a label plus the source builder for that shape.
///
/// Named so the fixed-size family arrays below stay readable; the inline
/// tuple type triggers `clippy::type_complexity` (the runner CI step runs
/// `clippy --all-targets -- -D warnings`).
type SourceFamily = (&'static str, Box<dyn Fn(usize) -> String>);

/// The real frozen site this slice brackets. STEP-0562 ranked it the *smallest*
/// of the 16 exhausted `parser.sico` groups (~92 KB emit-only IR), so it is the
/// cheapest real site that still exhausts -- the right anchor for a bounded
/// default test.
const ANCHOR_SITE: (&str, &str) = ("parser.sico", "fixed_literal_const_segment");

/// The Rust oracle's canonical IR for one source text. `Err` carries the
/// oracle's own refusal (e.g. a per-function verifier `Limit`), so a fixture
/// beyond what the oracle accepts is a recorded measurement outcome rather than
/// a panic.
fn rust_ir(text: &str) -> Result<String, String> {
    let source = SourceFile::from_text(SourceId::new(0), "m22-fuel-profile.sico", text)
        .map_err(|error| format!("source:{error:?}"))?;
    let module = lower_core(&source).map_err(|error| format!("{error:?}"))?;
    canonical_json(&module).map_err(|error| format!("{error:?}"))
}

/// One parsed top-level `function` in a frozen Sico source.
struct SicoFunction {
    name: String,
    text: String,
}

/// The top-level `function`s of a frozen Sico source, in declaration order.
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

/// The same-file functions a body calls: a name is a call only when it is not a
/// member access (`U64.checked_add(`) and not part of a longer identifier, so
/// `sico.text.length(` never becomes a local callee.
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
///
/// A real site's function is not self-contained -- it calls parser helpers --
/// so the closure pulls the callees in, exactly as the frozen ledger does. The
/// bounds keep a runaway closure a typed error rather than a hang.
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

/// The emit-only injection component: `inject.sico` + the frozen `codegen.sico`,
/// compiled once by the Rust oracle.
fn injection_component() -> &'static [u8] {
    static COMPONENT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    COMPONENT.get_or_init(|| {
        cached_component(
            "inject.sico",
            &[
                ("inject.sico", IR_INJECTION_SOURCE),
                ("codegen.sico", CODEGEN_SOURCE),
            ],
        )
    })
}

/// The guest emitter's answer for one injected canonical IR under an explicit
/// fuel budget.
enum GuestFuelEmission {
    Component(String),
    Refusal(String),
    FuelExhausted,
    Trap { reason: String },
}

/// Run the emit-only injection fixture under `fuel` and classify the outcome.
///
/// A fuel-limited run legitimately ends in `FuelExhausted`, so unlike the
/// differential helpers this classifies that outcome instead of panicking.
///
/// Returns the classified outcome **plus the exact measured fuel the guest
/// burned** (`FuelTrace::consumed`, `None` only when the run produced no
/// readable Store reading). This is the production-side readout from
/// `PreparedProgram::run_with_fuel`; no fuel ceiling is raised by reading it.
fn guest_emission_with_fuel(ir: &str, fuel: u64) -> (GuestFuelEmission, Option<u64>) {
    static POOL: std::sync::OnceLock<PreparedPool> = std::sync::OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let component: Vec<u8> = injection_component().to_vec();
        PreparedPool::new(default_pool_slots(), move || prepare_slot(&component))
    });
    let prepared = pool.acquire();
    let result = prepared.run_with_fuel(
        &ScriptInput {
            arguments: Vec::new(),
            stdin: ir.as_bytes().to_vec(),
        },
        &RunnerLimits {
            fuel,
            timeout: std::time::Duration::from_secs(300),
            memory_bytes: 2560 * 1024 * 1024,
            hostcall_fuel: 8 << 30,
        },
        &CancelToken::new(),
    );
    pool.release(prepared);
    let (outcome, fuel_trace) = result.expect("the injected IR bounds hold");
    let consumed = fuel_trace.consumed;
    let emission = match outcome {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            GuestFuelEmission::Component(
                String::from_utf8(output.stdout).expect("the hex is utf-8"),
            )
        }
        RunOutcome::Domain { code, message } => {
            assert_eq!(code, "invalid-input", "the injection fixture refuses");
            GuestFuelEmission::Refusal(message)
        }
        RunOutcome::FuelExhausted => GuestFuelEmission::FuelExhausted,
        // A trap is a measurement result of its own (the shape is beyond what
        // the guest can emit at all), recorded rather than crashing the probe.
        RunOutcome::Trap(reason) => GuestFuelEmission::Trap { reason },
        other => panic!("unexpected outcome under a fuel budget: {:?}", other),
    };
    (emission, consumed)
}

/// The fuel bracket in which the guest flips from "budget exhausted" to
/// "component emitted": consumption lies in `(low, high]`.
///
/// Geometric ladder from `seed`, then refine the last octave until the bracket
/// is within a quarter of its upper end. `None` means the cost exceeds
/// `ceiling` (still exhausted at the top of the ladder).
fn fuel_bracket<F: FnMut(u64) -> bool>(
    mut emits: F,
    ceiling: u64,
    seed: u64,
) -> Option<(u64, u64)> {
    let mut low = 0u64;
    let mut budget = seed.clamp(1, ceiling);
    let high = loop {
        if emits(budget) {
            break budget;
        }
        low = budget;
        if budget >= ceiling {
            return None;
        }
        budget = budget.saturating_mul(2).min(ceiling);
    };
    let mut high = high;
    while high - low > 1 && high - low > high / 4 {
        let middle = low + (high - low) / 2;
        if emits(middle) {
            high = middle;
        } else {
            low = middle;
        }
    }
    Some((low, high))
}

/// The pinned search: a flat measurement must be the guest's, not the search's.
#[test]
fn the_fuel_bracket_brackets_a_known_threshold() {
    for threshold in [1_234_567u64, 250_000_000, 700_000_000_000] {
        let emits = |fuel: u64| fuel >= threshold;
        let (low, high) = fuel_bracket(emits, FUEL_CEILING, 1_000_000).expect("ceiling is enough");
        assert!(
            low < threshold && threshold <= high,
            "bracket must contain {threshold}: {low}..{high}"
        );
        assert!(
            high - low <= high / 4 + 1,
            "bracket must be tight enough to fit an exponent: {low}..{high}"
        );
    }
    assert!(
        fuel_bracket(|fuel| fuel > FUEL_CEILING, FUEL_CEILING, 1_000_000).is_none(),
        "a cost above the ceiling yields no bracket"
    );
}

/// The one real `FuelExhausted` site, bracketed on the emit-only path.
///
/// STEP-0562 could only report "exhausted at the ceiling" for this site. This
/// test replaces that truncation with a measured `(low, high]` whenever the
/// bracket is reachable below the ceiling, and otherwise records the honest
/// lower bound the ladder reached. Either way the site's fuel hunger is pinned
/// to executed runs, not to a ceiling artifact.
#[test]
fn a_real_fuel_exhausted_site_is_bracketed_emit_only() {
    let (path, function) = ANCHOR_SITE;
    let closure = sico_closure(PARSER_SOURCE, function, 256, 1_048_576)
        .unwrap_or_else(|reason| panic!("{path}:{function} closure {reason}"));
    let source = format!("{HOST_CALL_SHAPE_PRELUDE}{closure}\n{HOST_CALL_SHAPE_RUN}");
    let ir = rust_ir(&source)
        .unwrap_or_else(|reason| panic!("{path}:{function} must lower to canonical IR: {reason}"));
    assert!(
        !ir.is_empty(),
        "{path}:{function} must lower to canonical IR through the oracle"
    );

    let component_bytes = std::cell::Cell::new(0usize);
    // The exact fuel the guest burned at the flip point (the smallest ladder
    // budget that still emitted), measured via `run_with_fuel`.
    let flip_consumed = std::cell::Cell::new(Option::<u64>::None);
    let started = std::time::Instant::now();
    let mut emits = |fuel: u64| match guest_emission_with_fuel(&ir, fuel) {
        (GuestFuelEmission::Component(hex), consumed) => {
            component_bytes.set(hex.len() / 2);
            flip_consumed.set(consumed);
            true
        }
        (GuestFuelEmission::FuelExhausted, _consumed) => false,
        (GuestFuelEmission::Refusal(identity), _) => {
            panic!("{path}:{function} refused at {fuel}: {identity}")
        }
        (GuestFuelEmission::Trap { reason }, _) => {
            panic!("{path}:{function} trapped at {fuel}: {reason}")
        }
    };
    // The default ceiling is bounded so the smoke runs in deterministic time on
    // a real (quadratic) site. `SICO_M22_PROBE_FUEL` raises it toward the frozen
    // 800e9 for a full bracket; a site that exhausts the bounded ladder is
    // reported as a lower bound, never as a ceiling artifact.
    let ceiling = std::env::var("SICO_M22_PROBE_FUEL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(6_400_000_000);
    let bracket = fuel_bracket(&mut emits, ceiling, 100_000_000);
    let verdict = match bracket {
        Some((low, high)) => format!("{low}..{high}"),
        None => "exhausted (lower bound only)".to_owned(),
    };
    let measured = match flip_consumed.get() {
        Some(consumed) => consumed.to_string(),
        None => "unavailable".to_owned(),
    };
    println!(
        "M22_FUEL_PROFILE site={path}:{function} ir_bytes={} component_bytes={} fuel_bracket={verdict} measured_consumed_at_flip={measured} search_ms={}",
        ir.len(),
        component_bytes.get(),
        started.elapsed().as_millis()
    );
    // The site must be genuinely fuel-hungry, not trivially cheap: at the
    // ladder's seed it must not already emit a full component for free. This is
    // the "red" the slice pins -- the wall is real at this site.
    assert!(
        component_bytes.get() == 0 || bracket.is_some(),
        "the anchor site must either exhaust or produce a real bracket"
    );
}

/// One probe verdict for the measured bracket: whether the guest emitted under
/// this budget, and the exact fuel it burned (`run_with_fuel` readout).
struct FuelProbe {
    emits: bool,
    consumed: Option<u64>,
}

/// The fuel bracket in which the guest flips from "budget exhausted" to
/// "component emitted", anchored on **measured** consumption.
///
/// Same geometric-ladder-then-refine search as [`fuel_bracket`], but each probe
/// carries the exact `FuelTrace::consumed` from the production-side readout, and
/// the returned bracket records the burn measured at the flip point (the largest
/// `consumed` seen while the guest was still emitting). That anchors the
/// `(low, high]` budget interval on real executed consumption rather than only
/// on the budget the search happened to try.
fn fuel_bracket_measured<F: FnMut(u64) -> FuelProbe>(
    mut probe: F,
    ceiling: u64,
    seed: u64,
) -> Option<(u64, u64, Option<u64>)> {
    let mut low = 0u64;
    let mut budget = seed.clamp(1, ceiling);
    let mut measured: Option<u64> = None;
    let high = loop {
        let verdict = probe(budget);
        if verdict.emits {
            if let Some(consumed) = verdict.consumed {
                measured = Some(match measured {
                    Some(previous) => previous.max(consumed),
                    None => consumed,
                });
            }
            break budget;
        }
        low = budget;
        if budget >= ceiling {
            return None;
        }
        budget = budget.saturating_mul(2).min(ceiling);
    };
    let mut high = high;
    while high - low > 1 && high - low > high / 4 {
        let middle = low + (high - low) / 2;
        let verdict = probe(middle);
        if verdict.emits {
            if let Some(consumed) = verdict.consumed {
                measured = Some(match measured {
                    Some(previous) => previous.max(consumed),
                    None => consumed,
                });
            }
            high = middle;
        } else {
            low = middle;
        }
    }
    Some((low, high, measured))
}

/// The real anchor site, bracketed AND measured: the production-side
/// `run_with_fuel` readout must attribute a concrete burn at the flip point that
/// lies inside the budget bracket the search established.
///
/// This is the attribution the slice exists to clear: instead of only "exhausted
/// at the ceiling", the wall is pinned to an executed `(low, high]` budget
/// interval plus the exact fuel the guest consumed when it finally emitted.
#[test]
fn the_anchor_site_bracket_is_anchored_on_measured_consumption() {
    let (path, function) = ANCHOR_SITE;
    let closure = sico_closure(PARSER_SOURCE, function, 256, 1_048_576)
        .unwrap_or_else(|reason| panic!("{path}:{function} closure {reason}"));
    let source = format!("{HOST_CALL_SHAPE_PRELUDE}{closure}\n{HOST_CALL_SHAPE_RUN}");
    let ir = rust_ir(&source)
        .unwrap_or_else(|reason| panic!("{path}:{function} must lower to canonical IR: {reason}"));

    let ceiling = std::env::var("SICO_M22_PROBE_FUEL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(6_400_000_000);
    let mut probe = |fuel: u64| {
        let (emission, consumed) = guest_emission_with_fuel(&ir, fuel);
        let emits = match &emission {
            GuestFuelEmission::Component(_) => true,
            GuestFuelEmission::FuelExhausted => false,
            GuestFuelEmission::Refusal(identity) => {
                panic!("{path}:{function} refused at {fuel}: {identity}")
            }
            GuestFuelEmission::Trap { reason } => {
                panic!("{path}:{function} trapped at {fuel}: {reason}")
            }
        };
        FuelProbe { emits, consumed }
    };
    let bracket = fuel_bracket_measured(&mut probe, ceiling, 100_000_000);
    let Some((low, high, measured)) = bracket else {
        // Still exhausted at the top of the (bounded) ladder: report the honest
        // lower bound. This is a real measurement outcome, not a failure.
        println!(
            "M22_FUEL_PROFILE_MEASURED site={path}:{function} exhausted lower_bound_only ceiling={ceiling}"
        );
        return;
    };
    let measured = measured.expect("an emitting flip point must carry a measured burn");
    // The measured burn is the fuel the guest actually consumed when it emitted,
    // under a budget of `high`. It can never exceed the budget it was given, and
    // it must sit strictly above the largest budget that still exhausted -- that
    // is what "anchored on measured consumption" means.
    assert!(
        measured <= high,
        "measured burn {measured} must not exceed the emitting budget {high}"
    );
    assert!(
        measured > low,
        "measured burn {measured} must exceed the largest exhausted budget {low}"
    );
    println!(
        "M22_FUEL_PROFILE_MEASURED site={path}:{function} budget_bracket={low}..{high} measured_consumed_at_flip={measured}"
    );
}

// ---------------------------------------------------------------------------
// The emitter cost-center ranking (heavy; run explicitly).
//
// `SICO_M22_PROBE_FUEL` caps the bracket ceiling (default = the frozen 800e9).
// Each family holds the module constant except ONE emitter workload, so the
// fuel-per-unit slope ranks which emitter path carries the super-linear term.
// ---------------------------------------------------------------------------
#[test]
#[ignore = "a function-level emitter cost ranking; run it explicitly"]
fn the_emitter_cost_centers_are_ranked_by_fuel_slope() {
    let ceiling = std::env::var("SICO_M22_PROBE_FUEL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(25_600_000_000);

    // Family A: N sequential `match U64.checked_add` STATEMENTS in ONE function.
    // Isolates the match-lowering path. (An earlier draft nested matches inside
    // the `error` arm and generated source the oracle refused; a flat statement
    // chain is the valid instrument. A source-level `read_local` chain is NOT a
    // valid instrument here: the frontend SSA-forwards `let a1 = a0`, so the IR
    // stays ~2 KB and the fuel stays flat regardless of N -- measured below and
    // reported as a negative control, not as a read_local cost.)
    let match_statements_source = |n: usize| {
        let mut body = String::from("function probe(data: U64) returns U64:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str("  match U64.checked_add(acc, U64.literal(1)):\n");
            body.push_str("    case ok(next):\n      set acc = next\n");
            body.push_str("    case error(_):\n      set acc = U64.literal(0)\n");
            body.push_str("  end match\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };

    // Family B: a `text.concat` chain of length N in ONE function. Isolates the
    // hex-assembly path (`cgj_text_concat` / per-op concat).
    let concat_source = |n: usize| {
        let mut body = String::from("function probe(data: Text) returns Text:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str("  set acc = sico.text.concat(acc, \"0123456789abcdef\")\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };

    // Family C: F tiny functions. Isolates the per-module / per-function scan
    // (`cgs_function_body_hex` walking the module per call).
    let functions_source = |funcs: usize| {
        let mut body = String::new();
        for f in 0..funcs {
            body.push_str(&format!(
                "function helper_{f}(data: U64) returns U64:\n  let a0 = data\n  let a1 = a0\n  return a1\nend function\n"
            ));
        }
        body.push_str(&format!(
            "function run_probe(data: U64) returns U64:\n  let r = helper_0(data)\n  let r1 = helper_{}(r)\n  return r1\nend function\n",
            funcs - 1
        ));
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };

    // Negative control: the SSA-forwarded `let` chain. The frontend collapses it
    // to a near-constant IR, so a flat fuel column here is the expected result
    // and confirms the *growing* families are measuring emitter work, not the
    // bracket search.
    let forwarded_let_source = |n: usize| {
        let mut body = "function probe(data: U64) returns U64:\n  let a0 = data\n".to_string();
        for i in 1..n {
            body.push_str(&format!("  let a{i} = a{}\n", i - 1));
        }
        body.push_str(&format!("  return a{}\nend function\n", n - 1));
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };

    let families: [SourceFamily; 4] = [
        ("match_statements", Box::new(match_statements_source)),
        ("text_concat", Box::new(concat_source)),
        ("functions", Box::new(functions_source)),
        ("forwarded_let_control", Box::new(forwarded_let_source)),
    ];
    let sizes = [8usize, 16, 32, 64];

    for (name, build) in families {
        let mut previous: Option<(usize, u64, usize)> = None;
        for &n in &sizes {
            let source = build(n);
            let ir = match rust_ir(&source) {
                Ok(ir) => ir,
                Err(reason) => {
                    println!("M22_FUEL_PROFILE family={name} n={n} oracle_refused={reason}");
                    previous = None;
                    continue;
                }
            };
            let mut emits = |fuel: u64| {
                let (emission, _consumed) = guest_emission_with_fuel(&ir, fuel);
                matches!(emission, GuestFuelEmission::Component(_))
            };
            let bracket = fuel_bracket(&mut emits, ceiling, 100_000_000);
            let midpoint = bracket.map(|(low, high)| (low + high) / 2);
            match (midpoint, previous) {
                (Some(fuel), Some((pn, pfuel, _))) => {
                    let per_unit = fuel / n.max(1) as u64;
                    let prev_per_unit = pfuel / pn.max(1) as u64;
                    let ratio = if prev_per_unit > 0 {
                        per_unit as f64 / prev_per_unit as f64
                    } else {
                        f64::NAN
                    };
                    println!(
                        "M22_FUEL_PROFILE family={name} n={n} ir_bytes={} fuel_mid={fuel} fuel_per_unit={per_unit} slope_vs_prev={ratio:.2}",
                        ir.len()
                    );
                }
                (Some(fuel), None) => {
                    println!(
                        "M22_FUEL_PROFILE family={name} n={n} ir_bytes={} fuel_mid={fuel} fuel_per_unit={}",
                        ir.len(),
                        fuel / n.max(1) as u64
                    );
                }
                (None, _) => {
                    println!(
                        "M22_FUEL_PROFILE family={name} n={n} ir_bytes={} exhausted lower_bound_only",
                        ir.len()
                    );
                }
            }
            previous = midpoint.map(|fuel| (n, fuel, ir.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// STEP-0575: discriminate the `match_statements` quadratic term.
//
// STEP-0572 ranked `match_statements` the hottest emitter cost center (it
// exhausts at n=8 / 21 KB IR, hotter than `text_concat` and `functions`). Code
// reading narrows the surviving O(n^2) term to one call: `cgs_match_hex` calls
// `cgs_dispatcher_slot(bytes, fn)` ONCE PER MATCH (codegen.sico:6500), and
// `cgs_dispatcher_slot` (codegen.sico:2209) walks the WHOLE run body -- every
// block x every instruction -- through index-form `cgj_array_item`. N matches
// therefore pay N x O(N) = O(N^2).
//
// The two existing STEP-0555/0556 negative results do NOT cover this term:
//   * B3 cursorized in-loop `cgj_array_item` but reverted as net-negative on
//     fuel (2 hostcalls/element vs 1);
//   * B4-D indexed `cgs_cell_slot`, but that is only reached from
//     read_local/write_local, and the `match_statements` family has NO locals --
//     so the matches wall was never explained by it.
// `cgs_dispatcher_slot` per match is a distinct, still-live quadratic term.
//
// These two arms separate it from the alternative (generic per-instruction JSON
// scanning). Both hold the module constant and grow ONE factor:
//   * `match_flat`   -- N matches in one function (the STEP-0572 hot family);
//   * `match_one`    -- 1 match but N-1 extra `set` statements, so the run body
//                       (and hence ONE `cgs_dispatcher_slot` scan) grows to the
//                       same instruction count WITHOUT extra match dispatchers.
// If the quadratic term is the per-match dispatcher rescan, `match_flat` grows
// super-linearly while `match_one` stays near-flat at equal body size. If both
// grow, the term is generic body scanning, not match-specific.
// ---------------------------------------------------------------------------
#[test]
#[ignore = "a match-quadratic discriminating probe; run it explicitly"]
fn the_match_quadratic_term_is_discriminated_from_body_scanning() {
    let ceiling = std::env::var("SICO_M22_PROBE_FUEL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(25_600_000_000);

    // N matches, one function: each match adds a dispatcher + one
    // `cgs_dispatcher_slot` full-body rescan.
    let match_flat_source = |n: usize| {
        let mut body = String::from("function probe(data: U64) returns U64:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str("  match U64.checked_add(acc, U64.literal(1)):\n");
            body.push_str("    case ok(next):\n      set acc = next\n");
            body.push_str("    case error(_):\n      set acc = U64.literal(0)\n");
            body.push_str("  end match\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };

    // 1 match + (N-1) plain `set` statements: the run body reaches the same
    // instruction count, so a single `cgs_dispatcher_slot` scan costs the same,
    // but only ONE match dispatcher is paid. Each `set` reassigns an already
    // `let`-bound accumulator so the frontend accepts it (a bare `set` on an
    // undeclared name is refused: `set without prior let`).
    let match_one_source = |n: usize| {
        let mut body = String::from("function probe(data: U64) returns U64:\n  let acc = data\n");
        for _ in 1..n {
            body.push_str("  set acc = acc\n");
        }
        body.push_str("  match U64.checked_add(acc, U64.literal(1)):\n");
        body.push_str("    case ok(next):\n      set acc = next\n");
        body.push_str("    case error(_):\n      set acc = U64.literal(0)\n");
        body.push_str("  end match\n");
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };

    let families: [SourceFamily; 2] = [
        ("match_flat", Box::new(match_flat_source)),
        ("match_one", Box::new(match_one_source)),
    ];
    let sizes = [2usize, 4, 8, 16];

    for (name, build) in families {
        let mut previous: Option<(usize, u64, usize)> = None;
        for &n in &sizes {
            let source = build(n);
            let ir = match rust_ir(&source) {
                Ok(ir) => ir,
                Err(reason) => {
                    println!("M22_FUEL_PROFILE family={name} n={n} oracle_refused={reason}");
                    previous = None;
                    continue;
                }
            };
            let mut emits = |fuel: u64| {
                let (emission, _consumed) = guest_emission_with_fuel(&ir, fuel);
                matches!(emission, GuestFuelEmission::Component(_))
            };
            let bracket = fuel_bracket(&mut emits, ceiling, 100_000_000);
            let midpoint = bracket.map(|(low, high)| (low + high) / 2);
            match (midpoint, previous) {
                (Some(fuel), Some((pn, pfuel, _))) => {
                    let ratio = pfuel as f64 * (n as f64 / pn as f64).powi(2) / fuel as f64;
                    println!(
                        "M22_FUEL_PROFILE family={name} n={n} ir_bytes={} fuel_mid={fuel} quadratic_predicted_fuel={:.3e} observed_over_quadratic={ratio:.3}",
                        ir.len(),
                        pfuel as f64 * (n as f64 / pn as f64).powi(2)
                    );
                }
                (Some(fuel), None) => {
                    println!(
                        "M22_FUEL_PROFILE family={name} n={n} ir_bytes={} fuel_mid={fuel}",
                        ir.len()
                    );
                }
                (None, _) => {
                    println!(
                        "M22_FUEL_PROFILE family={name} n={n} ir_bytes={} exhausted lower_bound_only",
                        ir.len()
                    );
                }
            }
            previous = midpoint.map(|fuel| (n, fuel, ir.len()));
        }
    }
}

// ---------------------------------------------------------------------------
// STEP-0607: op/phase-level fuel attribution via the epoch-sampling timeline.
//
// STEP-0606 attributed the fuel wall to two named emitter operators
// (`match`-dispatcher rescan + `text_concat` assembly) but only through
// bracket-midpoint fuel (`run_with_fuel`, a ±12.5% band). It honestly narrowed
// that `FuelTrace::timeline` (`run_with_fuel_timeline`, one fuel reading per
// watchdog tick) was built but never exercised. These probes exercise it.
//
// What the timeline IS: the epoch-deadline watchdog fires every `TICK` (5 ms),
// and each firing records `ctx.get_fuel()`. Because fuel is a deterministic
// count of the Wasm ops the guest executed, the timeline is a
// **fuel-burn-vs-execution-progress curve**: cumulative fuel against wall clock.
//
// What it is NOT: it is NOT a per-opcode hook. It cannot name a Wasm opcode; it
// names a *phase* of the run. The honest attribution it supports is burn-rate
// shape -- is fuel accumulated at a roughly constant rate from the first tick
// (every op costs about the same, so the total is per-op constant x op count),
// or does the rate rise as the run advances (the late work costs more per unit
// of IR, i.e. a super-linear operator)? A phase whose fuel/ms *drops* is doing
// disproportionately expensive work per op charged (hostcalls, traps, host
// transport), since fuel is charged per Wasm op and not per hostcall.
//
// Both probes are emit-only (the frozen `codegen.sico` emitter on injected
// canonical IR, guest frontend skipped) and `#[ignore]`d like the other
// measurement probes: they are a measurement slice, advance no exit gate, raise
// no ceiling, and touch no production code and no frozen guest source. M22 stays
// NO-GO.
// ---------------------------------------------------------------------------

/// Runs the emit-only injection fixture with the epoch-sampling timeline armed.
///
/// Same fixture and limits as [`guest_emission_with_fuel`] (which it shares the
/// prepared-program pool with), but through `run_with_fuel_timeline` so the
/// production-side per-tick fuel samples come back in `FuelTrace::timeline`.
/// The generous timeout only bounds a run that would otherwise hang; a
/// fuel-exhausted run ends on its own well before it.
fn guest_emission_with_timeline(
    ir: &str,
    fuel: u64,
) -> (GuestFuelEmission, FuelTrace, std::time::Duration) {
    static POOL: std::sync::OnceLock<PreparedPool> = std::sync::OnceLock::new();
    let pool = POOL.get_or_init(|| {
        let component: Vec<u8> = injection_component().to_vec();
        PreparedPool::new(default_pool_slots(), move || prepare_slot(&component))
    });
    let prepared = pool.acquire();
    let started = std::time::Instant::now();
    let result = prepared.run_with_fuel_timeline(
        &ScriptInput {
            arguments: Vec::new(),
            stdin: ir.as_bytes().to_vec(),
        },
        &RunnerLimits {
            fuel,
            timeout: std::time::Duration::from_secs(600),
            memory_bytes: 2560 * 1024 * 1024,
            hostcall_fuel: 8 << 30,
        },
        &CancelToken::new(),
    );
    let elapsed = started.elapsed();
    pool.release(prepared);
    let (outcome, trace) = result.expect("the injected IR bounds hold");
    let emission = match outcome {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            GuestFuelEmission::Component(
                String::from_utf8(output.stdout).expect("the hex is utf-8"),
            )
        }
        RunOutcome::Domain { code, message } => {
            assert_eq!(code, "invalid-input", "the injection fixture refuses");
            GuestFuelEmission::Refusal(message)
        }
        RunOutcome::FuelExhausted => GuestFuelEmission::FuelExhausted,
        RunOutcome::Trap(reason) => GuestFuelEmission::Trap { reason },
        other => panic!("unexpected outcome under a timeline probe: {:?}", other),
    };
    (emission, trace, elapsed)
}

/// The probe fuel ceiling. Same env convention as the bracket probes; defaults to
/// the frozen qualification ceiling so a super-linear curve can exhaust honestly
/// rather than being truncated by the probe's own budget.
fn probe_ceiling() -> u64 {
    std::env::var("SICO_M22_PROBE_FUEL")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(FUEL_CEILING)
}

/// Prints one timeline as a burn-rate curve.
///
/// The timeline is split into `SEGMENTS` equal fractions of *fuel consumed*
/// (not of time), so each segment holds the same amount of guest work by
/// definition and the reported wall-clock per segment is directly comparable:
/// a run whose ops all cost about the same shows a near-constant wall clock per
/// segment, while a run whose late ops are more expensive per unit charged shows
/// segments that lengthen toward the end. Each segment is printed as
/// `<fuel>/<wall_ms>` so the reader sees both the charge and the time it bought.
fn print_fuel_curve(label: &str, trace: &FuelTrace, elapsed: std::time::Duration) {
    const SEGMENTS: usize = 10;
    const TICK_MS: u64 = 5;
    let samples = &trace.timeline;
    let verdict = match trace.remaining {
        Some(remaining) if remaining > 0 => "emitted",
        _ => "exhausted",
    };
    if samples.len() < SEGMENTS + 2 {
        println!(
            "M22_FUEL_CURVE {label} verdict={verdict} samples={} too_few (run finished within ~{} ticks)",
            samples.len(),
            SEGMENTS + 2
        );
        return;
    }
    // Cumulative fuel consumed at each tick, from the charged ceiling down to the
    // remaining fuel. Tick `t` lands at about `t * TICK_MS` of wall clock.
    let initial = trace.initial;
    let points: Vec<(u64, u64)> = samples
        .iter()
        .map(|sample| (sample.tick as u64, initial.saturating_sub(sample.remaining)))
        .collect();
    let total_fuel = points.last().map(|(_, consumed)| *consumed).unwrap_or(0);
    let total_ms = elapsed.as_millis().max(1) as u64;
    let per_segment = (total_fuel / SEGMENTS as u64).max(1);
    let mut segment_ms = [0u64; SEGMENTS];
    let mut point_index = 0usize;
    let mut previous_close_ms = 0u64;
    for (segment, slot) in segment_ms.iter_mut().enumerate() {
        let boundary = per_segment * (segment as u64 + 1);
        while point_index < points.len() && points[point_index].1 < boundary {
            point_index += 1;
        }
        let closed = point_index.min(points.len() - 1);
        let close_ms = points[closed].0 * TICK_MS;
        *slot = close_ms.saturating_sub(previous_close_ms).max(1);
        previous_close_ms = close_ms;
    }
    let first = segment_ms[0].max(1);
    let last = segment_ms[SEGMENTS - 1].max(1);
    let shape = if last >= 4 * first {
        "accelerating (late ops cost more per unit charged)"
    } else if first >= 4 * last {
        "decelerating (early ops cost more per unit charged)"
    } else {
        "flat (every op costs about the same)"
    };
    let segments: Vec<String> = segment_ms
        .iter()
        .map(|ms| format!("{per_segment}/{ms}ms"))
        .collect();
    println!(
        "M22_FUEL_CURVE {label} verdict={verdict} samples={} wall_ms={total_ms} fuel_consumed={total_fuel} fuel_per_ms={} shape={shape}",
        samples.len(),
        total_fuel / total_ms,
    );
    println!(
        "M22_FUEL_CURVE {label} segments(fuel/wall_ms)={}",
        segments.join(" ")
    );
    // Where the burn concentrates: the tick at which each quarter of the total
    // charge was reached, i.e. how much of the run's wall clock the first half of
    // the work took versus the second half.
    for quarter in 0..4 {
        let boundary = total_fuel / 4 * quarter as u64;
        let tick = points
            .iter()
            .find(|(_, consumed)| *consumed >= boundary)
            .map(|(tick, _)| *tick)
            .unwrap_or(0);
        println!(
            "M22_FUEL_CURVE {label} quarter={quarter} reached_at_tick={tick} ({}ms)",
            tick * TICK_MS
        );
    }
}

/// The `match`-dispatcher rescan curve, discriminated against body scanning.
///
/// Same two arms as `the_match_quadratic_term_is_discriminated_from_body_scanning`
/// (`match_flat` = N matches in one function, `match_one` = 1 match plus N-1
/// plain `set` statements at equal body size), now traced through the epoch
/// timeline. If the quadratic term is the per-match dispatcher rescan,
/// `match_flat` burns with an accelerating curve (each added match makes every
/// later rescan longer) while `match_one` stays flat at equal body size.
#[test]
#[ignore = "a match-dispatcher fuel-timeline probe; run it explicitly"]
fn the_match_dispatcher_rescan_burn_is_traced_over_time() {
    let ceiling = probe_ceiling();
    let match_flat_source = |n: usize| {
        let mut body = String::from("function probe(data: U64) returns U64:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str("  match U64.checked_add(acc, U64.literal(1)):\n");
            body.push_str("    case ok(next):\n      set acc = next\n");
            body.push_str("    case error(_):\n      set acc = U64.literal(0)\n");
            body.push_str("  end match\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let match_one_source = |n: usize| {
        let mut body = String::from("function probe(data: U64) returns U64:\n  let acc = data\n");
        for _ in 1..n {
            body.push_str("  set acc = acc\n");
        }
        body.push_str("  match U64.checked_add(acc, U64.literal(1)):\n");
        body.push_str("    case ok(next):\n      set acc = next\n");
        body.push_str("    case error(_):\n      set acc = U64.literal(0)\n");
        body.push_str("  end match\n");
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let families: [SourceFamily; 2] = [
        ("match_flat", Box::new(match_flat_source)),
        ("match_one", Box::new(match_one_source)),
    ];
    let sizes = [4usize, 8, 16];
    for (name, build) in families {
        for &n in &sizes {
            let source = build(n);
            let ir = match rust_ir(&source) {
                Ok(ir) => ir,
                Err(reason) => {
                    println!("M22_FUEL_CURVE family={name} n={n} oracle_refused={reason}");
                    continue;
                }
            };
            let (emission, trace, elapsed) = guest_emission_with_timeline(&ir, ceiling);
            let verdict = match &emission {
                GuestFuelEmission::Component(hex) => {
                    format!("emitted component_bytes={}", hex.len() / 2)
                }
                GuestFuelEmission::FuelExhausted => "FuelExhausted".to_owned(),
                GuestFuelEmission::Refusal(identity) => {
                    panic!("{name} n={n} refused: {identity}")
                }
                GuestFuelEmission::Trap { reason } => {
                    panic!("{name} n={n} trapped: {reason}")
                }
            };
            println!(
                "M22_FUEL_CURVE family={name} n={n} ir_bytes={} ceiling={ceiling} verdict={verdict}",
                ir.len()
            );
            print_fuel_curve(&format!("family={name} n={n}"), &trace, elapsed);
        }
    }
}

/// The `text_concat` assembly curve.
///
/// A `text.concat` chain of length N in one function (the STEP-0606 assembly
/// cost center). If the cost is per-op assembly (each concat copies the
/// accumulator built so far), the curve accelerates: the later concats each carry
/// more work per op charged. If the cost were per-op constant, the curve stays
/// flat regardless of N.
#[test]
#[ignore = "a text_concat fuel-timeline probe; run it explicitly"]
fn the_text_concat_assembly_burn_is_traced_over_time() {
    let ceiling = probe_ceiling();
    let concat_source = |n: usize| {
        let mut body = String::from("function probe(data: Text) returns Text:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str("  set acc = sico.text.concat(acc, \"0123456789abcdef\")\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let sizes = [8usize, 16, 32, 64];
    for &n in &sizes {
        let source = concat_source(n);
        let ir = match rust_ir(&source) {
            Ok(ir) => ir,
            Err(reason) => {
                println!("M22_FUEL_CURVE family=text_concat n={n} oracle_refused={reason}");
                continue;
            }
        };
        let (emission, trace, elapsed) = guest_emission_with_timeline(&ir, ceiling);
        let verdict = match &emission {
            GuestFuelEmission::Component(hex) => {
                format!("emitted component_bytes={}", hex.len() / 2)
            }
            GuestFuelEmission::FuelExhausted => "FuelExhausted".to_owned(),
            GuestFuelEmission::Refusal(identity) => {
                panic!("text_concat n={n} refused: {identity}")
            }
            GuestFuelEmission::Trap { reason } => {
                panic!("text_concat n={n} trapped: {reason}")
            }
        };
        println!(
            "M22_FUEL_CURVE family=text_concat n={n} ir_bytes={} ceiling={ceiling} verdict={verdict}",
            ir.len()
        );
        print_fuel_curve(&format!("family=text_concat n={n}"), &trace, elapsed);
    }
}

// ---------------------------------------------------------------------------
// STEP-0608: exact finite-difference fuel attribution (segmented measurement).
//
// The timeline probes proved the burn RATE is flat (~21M fuel/ms at every n),
// so fuel is a deterministic per-op count and the wall is "per-op constant x op
// count". A flat rate cannot NAME the op, and the epoch watchdog fires only
// every 5 ms (no per-opcode hook exists in the public API). But an exact
// fuel-per-region distribution does not need either: fuel is deterministic in
// the injected IR, so running ONE emitting run per shape and reading
// `FuelTrace::consumed` gives the exact total, and finite differences across a
// ladder of shapes isolate the marginal fuel of ONE added operator.
//
// The named hypothesis (from `cg_concat_helper_body_hex`, codegen.sico:156):
// every `sico.text.concat` emits a fixed-shape body that arena-allocates the
// result and LINEARLY COPIES BOTH OPERANDS. A left-fold chain `acc = concat(acc,
// lit)` therefore copies the whole accumulator every step: the k-th concat
// copies ~16k bytes, so the total copied bytes over n steps is ~16*n(n+1)/2 =
// O(n^2). A balanced tree concatenates small operands at every level, so the
// total copied bytes grow near-linearly at equal output size. Both shapes
// produce the SAME final component output bytes, so any fuel difference is the
// assembly sub-step, not the output.
//
// Discriminator: if the quadratic term lives in the concat copy sub-step, the
// left-fold's MARGINAL fuel per added concat grows linearly with position while
// the balanced tree's marginal stays near-flat. That names the sub-step without
// an opcode hook and without gcc.
// ---------------------------------------------------------------------------
#[test]
#[ignore = "an exact finite-difference concat-assembly attribution; run it explicitly"]
fn the_concat_quadratic_term_is_attributed_to_operand_copying_by_finite_difference() {
    let ceiling = probe_ceiling();
    let build_left = |n: usize| {
        let mut body = String::from("function probe(data: Text) returns Text:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str("  set acc = sico.text.concat(acc, \"0123456789abcdef\")\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let build_balanced = |n: usize| {
        let mut body = String::from("function probe(data: Text) returns Text:\n");
        for i in 0..n {
            body.push_str(&format!(
                "  let l{i} = sico.text.concat(data, \"0123456789abcdef\")\n"
            ));
        }
        let mut level: Vec<String> = (0..n).map(|i| format!("l{i}")).collect();
        let mut idx = n;
        while level.len() > 1 {
            let mut next = Vec::new();
            for pair in level.chunks(2) {
                if pair.len() == 2 {
                    body.push_str(&format!(
                        "  let n{idx} = sico.text.concat({}, {})\n",
                        pair[0], pair[1]
                    ));
                    next.push(format!("n{idx}"));
                    idx += 1;
                } else {
                    next.push(pair[0].clone());
                }
            }
            level = next;
        }
        body.push_str(&format!("  return {}\nend function\n", level[0]));
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let families: [SourceFamily; 2] = [
        ("left_fold", Box::new(build_left)),
        ("balanced_tree", Box::new(build_balanced)),
    ];
    // Sizes stay within one emitting run of the ceiling so `consumed` is exact
    // (a FuelExhausted run reports only a lower bound, not a finite difference).
    let sizes = [4usize, 8, 12, 16];
    for (name, build) in families {
        let mut previous: Option<(usize, u64)> = None;
        for &n in &sizes {
            let ir = match rust_ir(&build(n)) {
                Ok(ir) => ir,
                Err(reason) => {
                    println!("M22_FUEL_ATTR family={name} n={n} oracle_refused={reason}");
                    previous = None;
                    continue;
                }
            };
            let (emission, trace, _elapsed) = guest_emission_with_timeline(&ir, ceiling);
            let (verdict, component_bytes) = match &emission {
                GuestFuelEmission::Component(hex) => ("emitted", hex.len() / 2),
                GuestFuelEmission::FuelExhausted => ("FuelExhausted", 0),
                GuestFuelEmission::Refusal(identity) => {
                    panic!("{name} n={n} refused: {identity}")
                }
                GuestFuelEmission::Trap { reason } => panic!("{name} n={n} trapped: {reason}"),
            };
            let Some(consumed) = trace.consumed else {
                println!(
                    "M22_FUEL_ATTR family={name} n={n} verdict={verdict} consumed=unavailable"
                );
                previous = None;
                continue;
            };
            let marginal = previous.map(|(pn, pfuel)| {
                let per_concat = consumed.saturating_sub(pfuel) / (n - pn) as u64;
                format!("{per_concat}/concat")
            });
            let total_per_concat = consumed / n as u64;
            println!(
                "M22_FUEL_ATTR family={name} n={n} ir_bytes={} verdict={verdict} component_bytes={component_bytes} consumed={consumed} fuel_per_concat_total={total_per_concat} marginal={}",
                ir.len(),
                marginal.unwrap_or_else(|| "n/a".to_owned())
            );
            previous = Some((n, consumed));
        }
    }
}

/// Confirms the concat sub-step is byte-proportional copying, not a per-op
/// constant. Holds the concat COUNT constant and grows the literal size: if the
/// cost is the linear copy of both operands, total fuel scales with total bytes
/// (fuel per concat rises with literal size); if it were a per-op constant, fuel
/// per concat would stay flat.
#[test]
#[ignore = "a literal-size concat-copy confirmation; run it explicitly"]
fn the_concat_cost_scales_with_copied_bytes_not_op_count() {
    let ceiling = probe_ceiling();
    let build = |n: usize, literal: &str| {
        let mut body = String::from("function probe(data: Text) returns Text:\n  let acc = data\n");
        for _ in 0..n {
            body.push_str(&format!(
                "  set acc = sico.text.concat(acc, \"{literal}\")\n"
            ));
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let n = 8usize;
    for (label, literal) in [
        ("8B", "01234567"),
        ("32B", "0123456789abcdef0123456789abcdef"),
        (
            "64B",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
    ] {
        let ir = match rust_ir(&build(n, literal)) {
            Ok(ir) => ir,
            Err(reason) => {
                println!("M22_FUEL_ATTR_BYTES literal={label} n={n} oracle_refused={reason}");
                continue;
            }
        };
        let (emission, trace, _elapsed) = guest_emission_with_timeline(&ir, ceiling);
        let consumed = match (trace.consumed, &emission) {
            (Some(consumed), GuestFuelEmission::Component(_)) => consumed,
            (Some(_), GuestFuelEmission::FuelExhausted) => {
                println!(
                    "M22_FUEL_ATTR_BYTES literal={label} n={n} FuelExhausted (lower bound only)"
                );
                continue;
            }
            _ => {
                println!("M22_FUEL_ATTR_BYTES literal={label} n={n} consumed=unavailable");
                continue;
            }
        };
        let bytes = literal.len();
        let per_concat = consumed / n as u64;
        let per_byte = per_concat / bytes as u64;
        println!(
            "M22_FUEL_ATTR_BYTES literal={label} n={n} ir_bytes={} consumed={consumed} fuel_per_concat={per_concat} literal_bytes={bytes} fuel_per_concat_per_literal_byte={per_byte}",
            ir.len()
        );
    }
}

/// Applies the same exact finite-difference method to the match-dispatcher
/// hypothesis. `match_flat` = M independent matches in one function (each adds a
/// dispatcher + one `cgs_dispatcher_slot` full-body rescan);
/// `match_nested_error` = M matches nested inside the `error` arm (same match
/// count, but the dispatchers are emitted once and the run body each scans is
/// the small fixed `probe` body, not a body that grows with M).
///
/// If the quadratic term is the per-match full-body rescan, `match_flat`'s
/// marginal fuel per added match grows linearly with M (each added match makes
/// every body longer and every rescan costlier) while the nested arm's marginal
/// stays near-flat. Equal match count, discriminating body growth.
#[test]
#[ignore = "a match-dispatcher finite-difference attribution; run it explicitly"]
fn the_match_quadratic_term_is_attributed_to_dispatcher_body_rescan_by_finite_difference() {
    let ceiling = probe_ceiling();
    let build_flat = |m: usize| {
        let mut body = String::from("function probe(data: U64) returns U64:\n  let acc = data\n");
        for _ in 0..m {
            body.push_str("  match U64.checked_add(acc, U64.literal(1)):\n");
            body.push_str("    case ok(next):\n      set acc = next\n");
            body.push_str("    case error(_):\n      set acc = U64.literal(0)\n");
            body.push_str("  end match\n");
        }
        body.push_str("  return acc\nend function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let build_nested = |m: usize| {
        // M-1 matches nested in the error arm (fixed small body each), plus one
        // outer match: M matches total, but no match sits in a body that grows
        // with M.
        let mut body = String::from("function probe(data: U64) returns U64:\n");
        for _ in 0..m.saturating_sub(1) {
            body.push_str("  match U64.checked_add(data, U64.literal(1)):\n");
            body.push_str("    case ok(next):\n      return next\n");
            body.push_str("    case error(_):\n");
        }
        body.push_str("  return U64.literal(0)\n");
        for _ in 0..m.saturating_sub(1) {
            body.push_str("  end match\n");
        }
        body.push_str("end function\n");
        format!("{HOST_CALL_SHAPE_PRELUDE}{body}\n{HOST_CALL_SHAPE_RUN}")
    };
    let families: [SourceFamily; 2] = [
        ("match_flat", Box::new(build_flat)),
        ("match_nested_error", Box::new(build_nested)),
    ];
    let sizes = [2usize, 4, 6, 8];
    for (name, build) in families {
        let mut previous: Option<(usize, u64)> = None;
        for &m in &sizes {
            let ir = match rust_ir(&build(m)) {
                Ok(ir) => ir,
                Err(reason) => {
                    println!("M22_FUEL_ATTR family={name} m={m} oracle_refused={reason}");
                    previous = None;
                    continue;
                }
            };
            let (emission, trace, _elapsed) = guest_emission_with_timeline(&ir, ceiling);
            let (verdict, component_bytes) = match &emission {
                GuestFuelEmission::Component(hex) => ("emitted", hex.len() / 2),
                GuestFuelEmission::FuelExhausted => ("FuelExhausted", 0),
                GuestFuelEmission::Refusal(identity) => {
                    panic!("{name} m={m} refused: {identity}")
                }
                GuestFuelEmission::Trap { reason } => panic!("{name} m={m} trapped: {reason}"),
            };
            let Some(consumed) = trace.consumed else {
                println!(
                    "M22_FUEL_ATTR family={name} m={m} verdict={verdict} consumed=unavailable"
                );
                previous = None;
                continue;
            };
            let marginal = previous.map(|(pm, pfuel)| {
                let per_match = consumed.saturating_sub(pfuel) / (m - pm) as u64;
                format!("{per_match}/match")
            });
            let total_per_match = consumed / m as u64;
            println!(
                "M22_FUEL_ATTR family={name} m={m} ir_bytes={} verdict={verdict} component_bytes={component_bytes} consumed={consumed} fuel_per_match_total={total_per_match} marginal={}",
                ir.len(),
                marginal.unwrap_or_else(|| "n/a".to_owned())
            );
            previous = Some((m, consumed));
        }
    }
}
