//! Forensic probe (read-only): does the emitter pay a *literal-count* O(N^2)
//! in `cgs_data_offset`'s dedup (`cgs_list_holds`), separate from the
//! `text_concat` assembly term STEP-0606 already named?
//!
//! This is a measurement slice only. It raises no fuel/memory ceiling, touches
//! no production crate, and does NOT edit `selfhost/codegen.sico` (single-writer
//! discipline). M22 stays NO-GO.
//!
//! Discriminator: two families that differ ONLY in whether the N `const_string`
//! literals are distinct or all the same.
//!
//! * `distinct`  -- N different literals => `cgs_list_holds` dedup list grows to
//!   N => per-`const_string` dedup scan is O(N) => O(N^2) if the hypothesis holds.
//! * `repeated`  -- N copies of ONE literal => dedup list stays size 1 => the
//!   dedup term is O(1)/const_string. Any growth here is the *other* terms
//!   (module walk, text_concat assembly), NOT literal dedup.
//!
//! If `distinct` grows ~4x per doubling while `repeated` stays near-flat at equal
//! IR size, the quadratic lives in the dedup. If both grow ~4x, the term is the
//! module walk / assembly, not dedup. Both are measured emit-only through
//! `run_with_fuel` (exact `FuelTrace::consumed`), never a bracket midpoint.
//!
//! `#[ignore]`d per repo convention for measurement probes; run explicitly.

use sico_ir::{canonical_json, lower_core};
use sico_runner::{CancelToken, RunOutcome, RunnerLimits, ScriptInput};
use sico_source::{SourceFile, SourceId};

#[path = "support/selfhost_common.rs"]
mod selfhost_common;

use selfhost_common::{PreparedPool, cached_component, default_pool_slots, prepare_slot};

const CODEGEN_SOURCE: &str = include_str!("../../../selfhost/codegen.sico");

/// The Script v0 record prelude the injection driver needs.
const PRELUDE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

const RUN: &str = "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\nend function\n";

/// The emit-only driver: canonical IR in, emitted component hex out.
const IR_INJECTION_SOURCE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\nuse codegen.component_hex_for_ir\nuse codegen.component_refusal_for_ir\n\nfunction main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let refusal = codegen.component_refusal_for_ir(input.stdin)\n  if U64.equal(sico.text.length(refusal), U64.literal(0)):\n    let component_hex = codegen.component_hex_for_ir(input.stdin)\n    return ok(ScriptOutput(stdout: sico.text.encode(component_hex), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end if\n  return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: refusal))\nend function\n";

/// A generous measurement ceiling. No production limit is raised: this only
/// bounds the probe's own Store so a super-linear family exhausts honestly.
const PROBE_FUEL: u64 = 400_000_000_000;

fn rust_ir(text: &str) -> Result<String, String> {
    let source = SourceFile::from_text(SourceId::new(0), "data-offset-probe.sico", text)
        .map_err(|error| format!("source:{error:?}"))?;
    let module = lower_core(&source).map_err(|error| format!("{error:?}"))?;
    canonical_json(&module).map_err(|error| format!("{error:?}"))
}

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

/// The exact fuel the guest emitter burned to lower one injected IR, and what it
/// emitted. `None` consumed means the run produced no readable Store reading.
fn measure(ir: &str) -> (Option<u64>, String) {
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
            fuel: PROBE_FUEL,
            timeout: std::time::Duration::from_secs(300),
            memory_bytes: 2560 * 1024 * 1024,
            hostcall_fuel: 8 << 30,
        },
        &CancelToken::new(),
    );
    pool.release(prepared);
    let (outcome, trace) = result.expect("the injected IR bounds hold");
    match outcome {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0, "{:?}", output.stderr);
            (
                trace.consumed,
                format!("emitted component_bytes={}", output.stdout.len() / 2),
            )
        }
        RunOutcome::FuelExhausted => (trace.consumed, "FuelExhausted".to_owned()),
        RunOutcome::Domain { code, message } => {
            (trace.consumed, format!("refused:{code}:{message}"))
        }
        RunOutcome::Trap(reason) => (trace.consumed, format!("trap:{reason}")),
        other => panic!("unexpected outcome: {other:?}"),
    }
}

/// N DISTINCT literals concatenated in one `text.concat` chain. Every literal is
/// a separate `const_string`, and the dedup list in `cgs_data_offset` grows to N.
fn distinct_source(n: usize) -> String {
    let mut body = String::from("function probe() returns Text:\n  let acc = \"l00\"\n");
    for i in 1..n {
        body.push_str(&format!("  set acc = sico.text.concat(acc, \"l{i:02}\")\n"));
    }
    body.push_str("  return acc\nend function\n");
    format!("{PRELUDE}{body}\n{RUN}")
}

/// N COPIES of ONE literal. Same IR shape and text_concat count, but the dedup
/// list stays size 1 (the literal is always already seen). Control family.
fn repeated_source(n: usize) -> String {
    let mut body = String::from("function probe() returns Text:\n  let acc = \"same\"\n");
    for _ in 1..n {
        body.push_str("  set acc = sico.text.concat(acc, \"same\")\n");
    }
    body.push_str("  return acc\nend function\n");
    format!("{PRELUDE}{body}\n{RUN}")
}

/// Measures one family across sizes and prints the exact-consumed growth ratio.
fn run_family(name: &str, build: fn(usize) -> String, sizes: &[usize]) {
    let mut previous: Option<(usize, u64)> = None;
    for &n in sizes {
        let source = build(n);
        let ir = match rust_ir(&source) {
            Ok(ir) => ir,
            Err(reason) => {
                println!("DATA_OFFSET_PROBE family={name} n={n} oracle_refused={reason}");
                previous = None;
                continue;
            }
        };
        let (consumed, verdict) = measure(&ir);
        match (consumed, previous) {
            (Some(fuel), Some((pn, pfuel))) => {
                let ratio = fuel as f64 / pfuel as f64;
                println!(
                    "DATA_OFFSET_PROBE family={name} n={n} ir_bytes={} consumed={fuel} verdict={verdict} ratio_vs_prev={ratio:.2} (prev n={pn} consumed={pfuel})",
                    ir.len()
                );
            }
            (Some(fuel), None) => {
                println!(
                    "DATA_OFFSET_PROBE family={name} n={n} ir_bytes={} consumed={fuel} verdict={verdict}",
                    ir.len()
                );
            }
            (None, _) => {
                println!(
                    "DATA_OFFSET_PROBE family={name} n={n} ir_bytes={} consumed=unavailable verdict={verdict}",
                    ir.len()
                );
            }
        }
        previous = consumed.map(|fuel| (n, fuel));
    }
}

/// N DISTINCT literals returned as a tuple, with NO `text.concat`. This removes
/// STEP-0606's text_concat assembly quadratic from the measurement entirely, so
/// any remaining growth is the emitter's per-`const_string` data-offset work
/// (`cgs_data_offset` module walk + `cgs_list_holds` dedup).
fn tuple_distinct_source(n: usize) -> String {
    let mut body = String::from("function probe() returns (Text, Text):\n  return (");
    for i in 0..n {
        if i > 0 {
            body.push_str(", ");
        }
        body.push_str(&format!("\"t{i:02}\""));
    }
    body.push_str(")\nend function\n");
    format!("{PRELUDE}{body}\n{RUN}")
}

/// N COPIES of ONE literal as a tuple, NO `text.concat`. Dedup list stays size 1.
fn tuple_repeated_source(n: usize) -> String {
    let mut body = String::from("function probe() returns (Text, Text):\n  return (");
    for i in 0..n {
        if i > 0 {
            body.push_str(", ");
        }
        body.push_str("\"same\"");
    }
    body.push_str(")\nend function\n");
    format!("{PRELUDE}{body}\n{RUN}")
}

#[test]
#[ignore = "a data-offset literal-dedup forensic probe; run it explicitly"]
fn data_offset_literal_dedup_is_quadratic_in_distinct_literals() {
    let sizes = [8usize, 16, 32, 64];
    println!("DATA_OFFSET_PROBE ceiling={PROBE_FUEL} (emit-only, exact consumed)");
    println!("DATA_OFFSET_PROBE == concat families (text.concat quadratic present) ==");
    run_family("distinct", distinct_source, &sizes);
    run_family("repeated", repeated_source, &sizes);
    println!("DATA_OFFSET_PROBE == tuple families (NO text.concat; isolates cgs_data_offset) ==");
    run_family("tuple_distinct", tuple_distinct_source, &sizes);
    run_family("tuple_repeated", tuple_repeated_source, &sizes);
}
