//! STEP-0516 (S5-W1): the 1 MB-scale emission feasibility measurement.
//!
//! This is a **pure measurement** slice (M22 plan §3.3.4): it measures, it
//! does not implement, and it advances no exit gate. The question it answers
//! is the one the plan calls the decision hub of the whole closure ladder --
//! can a guest emit bytes on the order of 1 MiB inside the qualified limits
//! (arena 2 GiB / fuel 800B / timeout 120 s), and is the growth curve
//! near-linear (D6: <=6x "feasible", <=16x "critical", >16x "infeasible")?
//!
//! ## Probe placement (D5)
//!
//! The probe components do **not** live in `selfhost/`: putting them there
//! would change the frozen source set S and disturb the `A = R(S)` baseline
//! bytes. They live here, in the runner test tree, and are compiled
//! separately by the Rust backend. They never enter the A/B/C bootstrap chain
//! and never enter the self-hosting shape frontier.
//!
//! ## What is measured
//!
//! Two byte-assembly strategies at six output sizes:
//!
//! * **S1 -- incremental `sico.text.concat`**: one growing string, `n` times.
//!   This is the shape the guest's own codegen uses today.
//! * **S2 -- chunked `sico.arena.push_chunk`**: push fixed-size chunks into
//!   the arena table, then assemble once at the end.
//! * **S3 -- linear control**: one `sico.text.concat` over two large operands.
//! * **S4 (STEP-0548, P0) -- `sico.list.append` + one `sico.text.join`**: the
//!   amortized builder the backend already ships. P0's decision gate asks
//!   whether *this* shape reaches 1 MiB inside the qualified limits -- it does
//!   (flat growth, ~0.005% of the fuel ceiling), which is what licenses P1.
//!
//! ## Measurement honesty (plan D13)
//!
//! The Runtime ABI has **no fuel-consumption readout** and **no guest
//! peak-memory readout**: the runner only observes the fuel *ceiling* it
//! charges and the memory *ceiling* it enforces. So this slice does not
//! pretend to read consumption. It measures the **required ceiling** by
//! bisection instead: the smallest fuel limit and the smallest memory limit
//! under which the probe still completes. Those are honest, reproducible
//! bounds -- and they are upper bounds on consumption, never a claim that
//! the number *is* the consumption.
//!
//! These numbers are **not** a runtime-support claim and **not** a
//! performance claim: they are one machine's observation of one fixture.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

/// The Script v0 record prelude every probe needs.
const PRELUDE: &str = "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\nrecord ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\nenum ScriptErrorCode:\n  case InvalidInput\n  case ResourceLimit\n  case DomainError\n  case Cancelled\nend enum\n\nrecord ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n";

/// S1: `reps` incremental `sico.text.concat` calls over a 16-byte chunk.
///
/// The chunk is a literal so the measured work is the concatenation itself,
/// not argument marshalling.
fn concat_probe(reps: u64) -> String {
    format!(
        "{PRELUDE}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let chunk = \"0123456789abcdef\"\n  let reps = I64.literal({reps})\n  let acc = \"\"\n  let i = I64.literal(0)\n  while I64.less_than(i, reps):\n    match I64.checked_add(i, I64.literal(1)):\n      case ok(next_i):\n        set i = next_i\n      case error(_):\n        return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"overflow\"))\n    end match\n    set acc = sico.text.concat(acc, chunk)\n  end while\n  let total = sico.text.length(acc)\n  return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(total)), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    )
}

/// S2: `reps` `sico.arena.push_chunk` calls, then one final assembly.
///
/// The arena treadmill reclaims the scratch zone on every push, so the chunk
/// table is the only thing that grows -- that is the whole point of the
/// chunked strategy.
///
/// The empty `List[Text]` comes from `sico.text.split_lines("")`: the
/// `sico.list.empty` bracket monomorph is declared for `I64`/`U64` only, so
/// `sico.list.empty[Text]()` is not a legal call target.
fn chunk_probe(reps: u64) -> String {
    format!(
        "{PRELUDE}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let chunk = \"0123456789abcdef\"\n  let reps = I64.literal({reps})\n  let chunks = sico.text.split_lines(\"\")\n  let i = I64.literal(0)\n  while I64.less_than(i, reps):\n    match I64.checked_add(i, I64.literal(1)):\n      case ok(next_i):\n        set i = next_i\n      case error(_):\n        return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"overflow\"))\n    end match\n    set chunks = sico.arena.push_chunk(chunks, chunk)\n  end while\n  let total = sico.text.length(sico.text.join(chunks, \"\"))\n  return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(total)), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    )
}

/// S4 (STEP-0548, P0): `reps` `sico.list.append` calls into one `List[Text]`,
/// then one `sico.text.join`.
///
/// S2's `sico.arena.push_chunk` rebuilds its chunk table on every push, so it
/// is quadratic in the number of chunks. `sico.list.append` is the amortized
/// builder the backend already ships: the list carries a private capacity
/// header, and while a spare slot remains the element is written in place, so
/// the table grows geometrically instead of being copied per push
/// (`crates/sico-codegen-wasm/src/stdlib.rs`: `emit_list_append`, and
/// `emit_join` for the single linear `join` pass).
///
/// This is the strategy P0 asks about: the guest emitter already needs a
/// fragment list to assemble at 1 MiB, and this is the only shipped primitive
/// whose cost is not quadratic in the fragment count.
fn list_append_probe(reps: u64) -> String {
    format!(
        "{PRELUDE}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let chunk = \"0123456789abcdef\"\n  let reps = I64.literal({reps})\n  let chunks = sico.text.split_lines(\"\")\n  let i = I64.literal(0)\n  while I64.less_than(i, reps):\n    match I64.checked_add(i, I64.literal(1)):\n      case ok(next_i):\n        set i = next_i\n      case error(_):\n        return error(ScriptError(code: ScriptErrorCode.DomainError, message: \"overflow\"))\n    end match\n    set chunks = sico.list.append(chunks, chunk)\n  end while\n  let total = sico.text.length(sico.text.join(chunks, \"\"))\n  return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(total)), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    )
}

/// The qualified-run limits the plan fixes for W1 (arena 2 GiB / fuel 800B /
/// timeout 120 s) -- the same values the ADR-0015 qualifying run uses.
///
/// Two different ceilings meet here, and the plan's "arena 2 GiB" is the
/// **guest arena** (`canonical::ARENA_LIMIT`, a compile-time constant inside
/// the guest's linear memory), not the runner's linear-memory ceiling. The
/// runner's ceiling must cover the component's **declared** linear memory
/// (33,792 Wasm pages = 2,214,592,512 B = 2112 MiB) or wasmtime cannot
/// satisfy the declared minimum and every instantiation is denied -- the
/// STEP-0497 failure mode. The runner's own default (2560 MiB) is used.
fn qualified_limits() -> RunnerLimits {
    RunnerLimits {
        fuel: 800_000_000_000,
        timeout: std::time::Duration::from_secs(120),
        memory_bytes: 2560 * 1024 * 1024,
        hostcall_fuel: 128 << 20,
    }
}

/// The component's **declared** linear memory, in bytes: 33,792 Wasm pages
/// of 64 KiB -- the guest arena (2 GiB) plus the 64 MiB of linear memory the
/// arena does not own. This is the floor any guest memory ceiling has to
/// clear (STEP-0497): below it, wasmtime cannot satisfy the declared
/// minimum and the instantiation is denied.
const DECLARED_LINEAR_MEMORY_BYTES: u64 = 33_792 * 65_536;

/// One measured (strategy, size) data point.
#[derive(Clone)]
struct Point {
    label: String,
    reps: u64,
    bytes: u64,
    millis: u128,
    /// `ok`, `arena-exhausted`, `fuel-exhausted`, ...
    verdict: String,
    /// The smallest fuel ceiling under which the probe still completed.
    min_fuel: u64,
    /// The component's declared linear memory (also its peak: it never grows).
    declared_memory_bytes: u64,
    /// The smallest runner memory ceiling that let the probe start.
    min_memory_bytes: u64,
}

/// Compiles one probe source into a component through the **Rust backend**.
///
/// The probe goes through the real `sico build --profile script-v0` CLI, so
/// the component bytes are the backend's production output -- the same path
/// the frozen `A = R(S)` baseline uses. Each probe source is distinct, so
/// nothing is cached: the measurement must observe this source's own
/// component.
fn build(source: &str) -> Vec<u8> {
    static BUILD_ID: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-m22-scale-probe-{}-{}",
        std::process::id(),
        BUILD_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root_path = directory.join("probe.sico");
    let component_path = directory.join("component.wasm");
    std::fs::write(&root_path, source).unwrap();
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
            root_path.as_os_str().to_owned(),
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

/// Runs one probe component once under `limits` with `stdin` as the guest's
/// standard input.
///
/// Returns the produced byte count when the probe completed, `None` otherwise.
fn run_once(component: &[u8], limits: &RunnerLimits, stdin: &[u8]) -> Option<u64> {
    run_once_classified(component, limits, stdin).0
}

/// Runs one probe component once and classifies the outcome.
///
/// The classification matters for the measurement: the arena ceiling is
/// enforced by a registered `unreachable` trap (a Wasm `Trap`, not a typed
/// fuel outcome), so "the probe ran out of arena" and "the probe ran out of
/// fuel" are **different** observations and must not be merged.
fn run_once_classified(
    component: &[u8],
    limits: &RunnerLimits,
    stdin: &[u8],
) -> (Option<u64>, String) {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("probe component links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: stdin.to_vec(),
            },
            limits,
            &CancelToken::new(),
        )
        .expect("probe input is within bounds");
    match outcome {
        RunOutcome::Output(output) => {
            let bytes = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u64>()
                .unwrap_or(0);
            (Some(bytes), "ok".to_string())
        }
        RunOutcome::FuelExhausted => (None, "fuel-exhausted".to_string()),
        RunOutcome::MemoryLimit => (None, "memory-limit".to_string()),
        RunOutcome::Timeout => (None, "timeout".to_string()),
        RunOutcome::Trap(message) => {
            let kind = if message.contains("unreachable") {
                "arena-exhausted"
            } else {
                "trap"
            };
            (None, kind.to_string())
        }
        other => (None, format!("other:{other:?}")),
    }
}

/// Finds the smallest fuel ceiling under which the probe still completes.
///
/// The Runtime ABI has no fuel readout, so the required ceiling is the
/// observable quantity (plan D13: an upper bound, never a claimed read).
fn min_fuel(component: &[u8], stdin: &[u8]) -> u64 {
    let mut low = 1_000u64;
    let mut high = 800_000_000_000u64;
    if run_once(
        component,
        &RunnerLimits {
            fuel: high,
            ..qualified_limits()
        },
        stdin,
    )
    .is_none()
    {
        return high;
    }
    while low < high {
        let mid = low + (high - low) / 2;
        if run_once(
            component,
            &RunnerLimits {
                fuel: mid,
                ..qualified_limits()
            },
            stdin,
        )
        .is_some()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}

/// Finds the smallest memory ceiling under which the probe still completes.
///
/// The guest's linear memory is declared with `minimum == maximum`, so it
/// never grows at run time -- but the **ceiling** that lets the program start
/// is not the declaration itself. This bisects the real floor instead of
/// assuming it, and records it as the observed value.
fn min_memory(component: &[u8], stdin: &[u8]) -> u64 {
    let mut low = 1u64;
    let mut high = 2560 * 1024 * 1024u64;
    if run_once(
        component,
        &RunnerLimits {
            memory_bytes: high as usize,
            ..qualified_limits()
        },
        stdin,
    )
    .is_none()
    {
        return high;
    }
    while low < high {
        let mid = low + (high - low) / 2;
        if run_once(
            component,
            &RunnerLimits {
                memory_bytes: mid as usize,
                ..qualified_limits()
            },
            stdin,
        )
        .is_some()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}

/// S3: one `sico.text.concat` over two **large** runtime operands.
///
/// This is the linear control for S1 and S2. S1 grows one string `n` times
/// (so it copies O(n^2) bytes) and S2 rebuilds its chunk table on every push
/// (also O(n^2) in the table). S3 concatenates two operands **once**, so the
/// copied volume is linear in the output size.
///
/// The operands arrive on **stdin** rather than as source literals: a source
/// literal of this size trips the `LineTooLong` source contract, which would
/// measure the source contract instead of the emission scale. The runner's
/// channel budget (8 MiB) covers the sizes this ladder needs.
fn linear_probe(_reps: u64) -> String {
    format!(
        "{PRELUDE}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  let left = sico.bytes.utf8_decode(input.stdin)\n  let total = sico.text.length(sico.text.concat(left, left))\n  return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(total)), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\nend function\n"
    )
}
/// Measures one (strategy, size) point.
fn measure(label: &str, source: &str, reps: u64, stdin: &[u8]) -> Point {
    let component = build(source);
    let limits = qualified_limits();
    let started = Instant::now();
    let (bytes, verdict) = run_once_classified(&component, &limits, stdin);
    let millis = started.elapsed().as_millis();
    Point {
        label: label.to_string(),
        reps,
        bytes: bytes.unwrap_or(0),
        millis,
        verdict,
        min_fuel: if bytes.is_some() {
            min_fuel(&component, stdin)
        } else {
            0
        },
        declared_memory_bytes: DECLARED_LINEAR_MEMORY_BYTES,
        // The smallest ceiling that lets the probe start, measured rather
        // than assumed. Only the successful points pay for it.
        min_memory_bytes: if bytes.is_some() {
            min_memory(&component, stdin)
        } else {
            0
        },
    }
}

/// The measurement ladder (plan §3.3.4 M1): the produced-byte sizes from
/// 4 KiB up to 4 MiB, at a 16-byte chunk.
///
/// The ladder is dense between 256 KiB and 1 MiB on purpose: that is where
/// the arena ceiling bites, and the boundary has to be located rather than
/// assumed.
const LADDER: &[u64] = &[
    256, 2_048, 8_192, 16_384, 24_576, 32_768, 40_960, 49_152, 57_344, 65_536,
];

/// The linear strategy's ladder: it takes **one** operand pair per point, so
/// the ladder is the per-operand size and the produced output is twice it.
const LINEAR_LADDER: &[u64] = &[
    256, 2_048, 16_384, 65_536, 131_072, 262_144, 524_288, 1_048_576, 2_097_152,
];

/// The W1 verdict thresholds (plan D6).
///
/// * `<= 6x` growth from 256 KiB to 1 MiB -> **feasible**
/// * `<= 16x` -> **critical**
/// * `> 16x` -> **infeasible** (triggers the T-R3-a stop-loss)
const D6_FEASIBLE_GROWTH: f64 = 6.0;
const D6_CRITICAL_GROWTH: f64 = 16.0;

/// The 256 KiB-to-1 MiB growth ratio the D6 verdict is read from.
fn growth_ratio_256k_to_1m(points: &[Point]) -> Option<f64> {
    let at_256k = points.iter().find(|point| point.bytes == 256 * 1024)?;
    let at_1m = points.iter().find(|point| point.bytes == 1024 * 1024)?;
    if at_256k.millis == 0 {
        return None;
    }
    Some(at_1m.millis as f64 / at_256k.millis as f64)
}

/// The D6 three-tier verdict for one strategy's growth curve.
fn verdict(growth: f64) -> &'static str {
    if growth <= D6_FEASIBLE_GROWTH {
        "feasible"
    } else if growth <= D6_CRITICAL_GROWTH {
        "critical"
    } else {
        "infeasible"
    }
}

#[test]
fn the_one_megabyte_emission_scale_is_measured() {
    // The evidence file lives under the repository's own `target/tmp`, derived
    // from the manifest directory so the probe is portable across hosts and
    // CI runners (an absolute machine path would fail there).
    // `CARGO_MANIFEST_DIR` is `<repo>/runner/sico-runner`, so two levels up is
    // the repository root.
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root resolves");
    let evidence_dir = repo_root.join("target").join("tmp").join("step0516");
    std::fs::create_dir_all(&evidence_dir).unwrap();
    let evidence_path = evidence_dir.join("scale-measurement.txt");
    // STEP-0548 (P0) reads the same report from its own step directory, so the
    // P0 verdict and the W1 verdict are computed from one observation.
    let p0_dir = repo_root.join("target").join("tmp").join("step0548");
    std::fs::create_dir_all(&p0_dir).unwrap();
    let p0_path = p0_dir.join("assembly-measurement.txt");
    let mut report = String::new();
    let mut per_strategy: Vec<(&str, Vec<Point>)> = Vec::new();

    for (strategy, build_probe, ladder, stdin_of) in [
        ("S1-concat", concat_probe as fn(u64) -> String, LADDER, None),
        ("S2-chunked", chunk_probe, LADDER, None),
        ("S4-list-append", list_append_probe, LADDER, None),
        (
            "S3-linear",
            linear_probe,
            LINEAR_LADDER,
            Some(
                (|reps: u64| -> Vec<u8> {
                    // The operand arrives on stdin: one ASCII byte per rep.
                    vec![b'a'; reps as usize]
                }) as fn(u64) -> Vec<u8>,
            ),
        ),
    ] {
        let mut previous: Option<Point> = None;
        let mut points: Vec<Point> = Vec::new();
        for &reps in ladder {
            let stdin = stdin_of.map_or_else(Vec::new, |make| make(reps));
            let point = measure(strategy, &build_probe(reps), reps, &stdin);
            let ratio = previous.as_ref().map_or(1.0f64, |last| {
                if last.millis == 0 {
                    f64::INFINITY
                } else {
                    point.millis as f64 / last.millis as f64
                }
            });
            report.push_str(&format!(
                "{} reps={} bytes={} wall_ms={} growth={ratio:.2}x verdict={} min_fuel={} declared_mem={} min_mem={}\n",
                point.label,
                point.reps,
                point.bytes,
                point.millis,
                point.verdict,
                point.min_fuel,
                point.declared_memory_bytes,
                point.min_memory_bytes
            ));
            previous = Some(point.clone());
            points.push(point);
        }
        let growth = growth_ratio_256k_to_1m(&points);
        let tier = growth.map_or("unmeasured", verdict);
        report.push_str(&format!(
            "== {strategy}: growth(256KiB->1MiB)={growth:?} tier={tier}\n"
        ));
        per_strategy.push((strategy, points));
    }

    std::fs::write(&evidence_path, &report).unwrap();
    std::fs::write(&p0_path, &report).unwrap();
    println!("{report}");

    // --- The pinned findings (STEP-0516) -------------------------------
    //
    // The measurement is a **pure observation**, but its *conclusion* is
    // pinned so a later change in the runtime's allocation behaviour cannot
    // silently rewrite the W1 verdict. The wall-clock values are not
    // asserted (they are machine-dependent); the structural facts are.

    let points = |name: &str| -> Vec<Point> {
        per_strategy
            .iter()
            .find(|(strategy, _)| *strategy == name)
            .map(|(_, points)| points.clone())
            .expect("every strategy was measured")
    };

    // 1. The 1 MiB target is reachable **with a linear strategy**, inside the
    //    qualified limits, with a near-linear growth curve.
    let linear = points("S3-linear");
    let one_mib = linear
        .iter()
        .find(|point| point.bytes == 1024 * 1024)
        .expect("the linear strategy reached 1 MiB");
    assert_eq!(
        one_mib.verdict, "ok",
        "the linear strategy must reach 1 MiB"
    );
    assert!(
        one_mib.min_fuel <= 800_000_000_000,
        "the 1 MiB emission must fit the qualified fuel ceiling"
    );
    assert!(
        one_mib.min_fuel * 1000 < 800_000_000_000,
        "the 1 MiB emission must use far less than the qualified fuel ceiling"
    );
    let growth = growth_ratio_256k_to_1m(&linear).expect("both growth endpoints exist");
    assert!(
        growth <= D6_FEASIBLE_GROWTH,
        "the linear growth curve must read as feasible under D6, got {growth:.2}x"
    );

    // 2. The quadratic strategies hit the arena ceiling far below 1 MiB --
    //    which is why the verdict has to be read per strategy, not globally.
    let concat = points("S1-concat");
    assert!(
        concat
            .iter()
            .any(|point| point.verdict == "arena-exhausted"),
        "the incremental strategy must still hit the arena ceiling"
    );
    assert!(
        concat.iter().all(|point| point.bytes < 1024 * 1024),
        "the incremental strategy must not reach 1 MiB inside the arena"
    );

    // 3. The memory ceiling that lets a script component start is exactly the
    //    declared linear memory (33,792 pages = the guest arena plus its
    //    64 MiB reserve) -- not the arena alone, and not anything smaller
    //    (STEP-0497's instantiation denial).
    let min_memory = one_mib.min_memory_bytes;
    assert_eq!(
        min_memory, DECLARED_LINEAR_MEMORY_BYTES,
        "the measured memory floor must be the declared linear memory"
    );

    // --- The P0 verdict (STEP-0548, decision gate D1) -------------------
    //
    // The amortized builder (`sico.list.append`) plus one `join` is the only
    // shipped strategy that reaches 1 MiB at all, and it does so with flat
    // growth for ~0.005% of the qualified fuel ceiling. That is what licenses
    // P1 (rewriting the emitter's 461 self-appending `concat` sites into
    // fragment lists). These pins keep a later change in allocation behaviour
    // from silently rewriting the verdict -- the same discipline STEP-0516
    // applied to S1/S3.
    let appended = points("S4-list-append");
    let appended_at_1m = appended
        .iter()
        .find(|point| point.bytes == 1024 * 1024)
        .expect("the amortized builder reached 1 MiB");
    assert_eq!(
        appended_at_1m.verdict, "ok",
        "the amortized builder must reach 1 MiB"
    );
    assert!(
        appended_at_1m.min_fuel * 100 < 800_000_000_000,
        "1 MiB through the amortized builder must cost under 1% of the fuel ceiling, got {}",
        appended_at_1m.min_fuel
    );
    let appended_growth = growth_ratio_256k_to_1m(&appended).expect("both growth endpoints exist");
    assert!(
        appended_growth <= D6_FEASIBLE_GROWTH,
        "the amortized builder's growth must read as feasible, got {appended_growth:.2}x"
    );
    let appended_at_256k = appended
        .iter()
        .find(|point| point.bytes == 256 * 1024)
        .expect("the amortized builder reached 256 KiB");
    let chunked_at_256k = points("S2-chunked")
        .into_iter()
        .find(|point| point.bytes == 256 * 1024)
        .expect("the chunked strategy reached 256 KiB");
    assert!(
        appended_at_256k.min_fuel * 10 < chunked_at_256k.min_fuel,
        "the amortized builder must be at least 10x cheaper than the chunk \
         table at 256 KiB ({} vs {})",
        appended_at_256k.min_fuel,
        chunked_at_256k.min_fuel
    );
}
