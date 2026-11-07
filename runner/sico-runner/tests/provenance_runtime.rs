//! M24 gate-4 provenance — **runtime budget enforcement + cancellation**
//! (STEP-0578, RFC-0051 property 3 runtime half + property 4).
//!
//! The install-time half (budget ⊆ package ceiling, A1.3, load-time) is
//! exercised in `crates/sico-package/tests/provenance_install.rs`; this file
//! proves the **run-time** half on a real runner:
//!
//! 1. `RunnerLimits::narrowed_by` maps the frozen fixture budget onto the
//!    existing limit kinds one-to-one and can only narrow (an over-ceiling
//!    budget clamps, never widens — no second ceiling path);
//! 2. a real guest whose execution exceeds the narrowed **memory** ceiling
//!    ends in a typed `MemoryLimit`, and the Host stays reusable;
//! 3. a spinning guest under a narrowed **wall** budget ends in typed
//!    `Timeout`;
//! 4. the deterministic **fuel** bound (the caller's own ceiling — the budget
//!    carries no fuel field) ends in typed `FuelExhausted`;
//! 5. an explicit mid-flight cancel (M9) ends in typed `Cancelled`;
//! 6. the end-to-end chain: a Script package carrying the frozen fixture +
//!    manifest → `verify` → `VerifiedPackage::provenance_budgets` →
//!    `run_program_with_budgets` → the narrowed ceiling is enforced.
//!
//! Guests are hand-built minimal core modules (wasm-encoder) wrapped in the
//! frozen Program Component shell — the runner accepts a small declared
//! memory, which is what makes the 1 MiB fixture budget enforceable (a real
//! Sico Script component declares ~2 GiB and is a separate, documented
//! contract note; see STEP-0578 §Contract-gaps).
//!
//! Evidence level: `internal-fixture` (frozen fixture + hand-built guests, no
//! clean-room consumer, no external pilot).

use std::time::Duration;

use sico_package::provenance::{Budgets, ProvenanceManifest};
use sico_runner::{CancelToken, FsGrants, RunOutcome, Runner, RunnerLimits, ScriptInput};

/// The frozen provenance manifest of the 256-byte fixture model (budget
/// 1 MiB / 5000 ms / 100000 hostcalls), committed in `sico-package`.
const MODEL_MANIFEST: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/sico-package/tests/fixtures/provenance/model-fixture.provenance.json"
));

fn fixture_budgets() -> Budgets {
    ProvenanceManifest::parse(MODEL_MANIFEST)
        .expect("frozen manifest parses")
        .asset
        .budgets
}

fn runner() -> Runner {
    Runner::new().expect("runner starts")
}

fn no_cancel() -> CancelToken {
    CancelToken::new()
}

fn limits() -> RunnerLimits {
    RunnerLimits::default()
}

/// A hand-built minimal core guest, wrapped in the frozen Program Component
/// shell (same structure as `runner.rs::malformed_component`, which is proven
/// to instantiate and run). `mode` selects the `run` body.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GuestMode {
    /// Grows linear memory to 18 pages (1.125 MiB) then traps.
    GrowPastOneMiB,
    /// Spins forever (`loop { br 0 }`), consuming fuel and wall time.
    Spin,
}

fn minimal_guest(mode: GuestMode) -> Vec<u8> {
    use wasm_encoder::{
        CodeSection, ConstExpr, ExportKind, ExportSection, Function as WasmFunction,
        FunctionSection, GlobalSection, GlobalType, Instruction as Wasm, MemorySection, MemoryType,
        Module as CoreModule, TypeSection, ValType,
    };
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function([ValType::I32], []);
    let mut functions = FunctionSection::new();
    functions.function(0); // run
    functions.function(0); // cabi_realloc
    functions.function(1); // cabi_post_run
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(1024),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(4096),
    );
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("run", ExportKind::Func, 0);
    exports.export("cabi_realloc", ExportKind::Func, 1);
    exports.export("cabi_post_run", ExportKind::Func, 2);

    let mut run = WasmFunction::new(Vec::new());
    match mode {
        GuestMode::GrowPastOneMiB => {
            // Start at 1 page (64 KiB); grow to 18 pages (1.125 MiB), which is
            // past the 1 MiB fixture budget. If the limiter denies the grow
            // the engine traps; if it returns -1 the `unreachable` traps. Both
            // paths leave `RunState::denied` set → `MemoryLimit`.
            run.instruction(&Wasm::I32Const(17));
            run.instruction(&Wasm::MemoryGrow(0));
            run.instruction(&Wasm::Drop);
            run.instruction(&Wasm::Unreachable);
        }
        GuestMode::Spin => {
            run.instruction(&Wasm::Loop(wasm_encoder::BlockType::Empty));
            run.instruction(&Wasm::Br(0));
            run.instruction(&Wasm::End); // loop
            run.instruction(&Wasm::Unreachable); // unreachable after infinite loop
        }
    }
    run.instruction(&Wasm::End); // function

    let mut realloc = WasmFunction::new(vec![(1, ValType::I32)]);
    realloc.instruction(&Wasm::GlobalGet(0));
    realloc.instruction(&Wasm::LocalGet(2));
    realloc.instruction(&Wasm::I32Add);
    realloc.instruction(&Wasm::I32Const(-1));
    realloc.instruction(&Wasm::I32Add);
    realloc.instruction(&Wasm::I32Const(0));
    realloc.instruction(&Wasm::LocalGet(2));
    realloc.instruction(&Wasm::I32Sub);
    realloc.instruction(&Wasm::I32And);
    realloc.instruction(&Wasm::LocalTee(4));
    realloc.instruction(&Wasm::LocalGet(3));
    realloc.instruction(&Wasm::I32Add);
    realloc.instruction(&Wasm::GlobalSet(0));
    realloc.instruction(&Wasm::LocalGet(4));
    realloc.instruction(&Wasm::End);

    let mut post = WasmFunction::new(Vec::new());
    post.instruction(&Wasm::I32Const(4096));
    post.instruction(&Wasm::GlobalSet(0));
    post.instruction(&Wasm::End);

    let mut code = CodeSection::new();
    code.function(&run);
    code.function(&realloc);
    code.function(&post);
    let mut module = CoreModule::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    sico_codegen_wasm::wrap_script_component(
        &module.finish(),
        sico_codegen_wasm::FsUse::default(),
        sico_codegen_wasm::StreamUse::default(),
        false,
        false,
        &[],
        0,
    )
}

#[test]
fn narrowed_by_maps_the_frozen_budget_and_never_widens() {
    // Property 3, pure narrowing half: the frozen budget maps one-to-one onto
    // the existing limit kinds and the result can only be ≤ the base limits.
    let budgets = fixture_budgets();
    let narrowed = limits().narrowed_by(&budgets);
    assert_eq!(narrowed.memory_bytes, 1_048_576);
    assert_eq!(narrowed.timeout, Duration::from_millis(5_000));
    assert_eq!(narrowed.hostcall_fuel, 100_000);
    // The deterministic wasm fuel bound is deliberately untouched (the budget
    // carries no fuel field; max_hostcalls is the transport/hostcall budget).
    assert_eq!(narrowed.fuel, limits().fuel);

    // A hostile over-ceiling budget clamps to the ceiling, never widens it.
    let over = Budgets {
        max_memory_bytes: u64::MAX,
        max_wall_ms: u64::MAX,
        max_hostcalls: u64::MAX,
    };
    let clamped = limits().narrowed_by(&over);
    assert_eq!(clamped.memory_bytes, limits().memory_bytes);
    assert_eq!(clamped.timeout, limits().timeout);
    assert_eq!(clamped.hostcall_fuel, limits().hostcall_fuel);
}

#[test]
fn memory_budget_is_enforced_and_host_stays_reusable() {
    let guest = minimal_guest(GuestMode::GrowPastOneMiB);
    let budgets = fixture_budgets();
    let host = runner();

    // Under the frozen 1 MiB budget the grow past 1 MiB is denied → typed
    // MemoryLimit (a launch-failure would be an opaque leak; a clean typed
    // outcome is the property under test).
    let outcome = host
        .run_program_with_budgets(
            &guest,
            &ScriptInput::default(),
            &limits(),
            &budgets,
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(outcome, RunOutcome::MemoryLimit, "{outcome:?}");

    // The Host survives the typed outcome and is immediately reusable; and
    // without the budget the same guest does NOT hit the memory ceiling (the
    // grow succeeds, then the explicit `unreachable` traps) — so the
    // MemoryLimit is attributable to the budget narrowing, not the default.
    let unbudgeted = host
        .run_program(
            &guest,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert!(matches!(unbudgeted, RunOutcome::Trap(_)), "{unbudgeted:?}");
}

#[test]
fn wall_budget_is_enforced_as_typed_timeout() {
    let guest = minimal_guest(GuestMode::Spin);
    // A tightened wall budget (30 ms) so the test stays fast; the frozen
    // fixture's own 5000 ms is the subset boundary pinned in the pure test and
    // in the install-time subset test. The memory/hostcall fields stay at the
    // frozen values, so this is still a fixture-derived budget.
    let tight = Budgets {
        max_wall_ms: 30,
        ..fixture_budgets()
    };
    // The base limits keep an unbounded fuel bound so the wall deadline (the
    // narrowed budget) is the first boundary to trip; the budget carries no
    // fuel field, so `narrowed_by` leaves the caller's own fuel untouched.
    let base = RunnerLimits {
        fuel: u64::MAX,
        ..limits()
    };
    let outcome = runner()
        .run_program_with_budgets(
            &guest,
            &ScriptInput::default(),
            &base,
            &tight,
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(outcome, RunOutcome::Timeout, "{outcome:?}");
}

#[test]
fn fuel_bound_is_enforced_as_typed_fuel_exhausted() {
    let guest = minimal_guest(GuestMode::Spin);
    // The budget carries no fuel field, so the non-termination bound is the
    // caller's own ceiling; a small fuel is the deterministic non-termination
    // bound the runner enforces (M9 typed outcome).
    let tight = RunnerLimits {
        fuel: 1_000,
        ..limits()
    };
    let outcome = runner()
        .run_program(
            &guest,
            &ScriptInput::default(),
            &tight,
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(outcome, RunOutcome::FuelExhausted, "{outcome:?}");
}

#[test]
fn explicit_mid_flight_cancel_is_a_typed_cancellation() {
    let guest = minimal_guest(GuestMode::Spin);
    let cancel = CancelToken::new();
    let canceller = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        canceller.cancel();
    });
    // Unbounded fuel + long wall deadline so the cancel is the first boundary
    // to trip; the watchdog observes it at the next epoch tick (M9 semantics).
    let roomy = RunnerLimits {
        fuel: u64::MAX,
        timeout: Duration::from_secs(30),
        ..limits()
    };
    let outcome = runner()
        .run_program(
            &guest,
            &ScriptInput::default(),
            &roomy,
            &cancel,
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(outcome, RunOutcome::Cancelled, "{outcome:?}");
}

#[test]
fn package_budget_reaches_the_runner_end_to_end() {
    // The full chain: a Script package carrying the frozen fixture + manifest
    // installs, its provenance budget is extracted via the mount point, and
    // the runner enforces the narrowed memory ceiling on the package's own
    // component.
    use sico_package::{
        BuildScriptInput, ResourceInput, build_script_v1, script_profile_limits, verify,
    };

    let package = build_script_v1(BuildScriptInput {
        app_id: "dev.sico.fixture".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: minimal_guest(GuestMode::GrowPastOneMiB),
        resources: vec![
            ResourceInput {
                path: "model-fixture".to_owned(),
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../crates/sico-package/tests/fixtures/provenance/model-fixture.bin"
                ))
                .to_vec(),
            },
            ResourceInput {
                path: "model-fixture.provenance.json".to_owned(),
                bytes: MODEL_MANIFEST.to_vec(),
            },
        ],
        semantics: "sico.ir.v0".to_owned(),
        wit_sha256: "a".repeat(64),
        adapter_sha256: "b".repeat(64),
        limits: script_profile_limits(),
    })
    .expect("the fixture Script package builds");

    let verified = verify(&package).expect("the package installs (budget ⊆ ceiling)");
    let budgets = verified
        .provenance_budgets()
        .expect("mount point resolves")
        .expect("the package carries a provenance budget");

    let outcome = runner()
        .run_program_with_budgets(
            &verified.component,
            &ScriptInput::default(),
            &limits(),
            &budgets,
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(outcome, RunOutcome::MemoryLimit, "{outcome:?}");
}
