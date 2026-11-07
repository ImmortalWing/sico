//! M24 gate-2 **provider-crash isolation (免宿主重启)** — the fifth and final
//! piece of the gate-2 five-piece set (STEP-0587).
//!
//! Property under test: when a **provider** path fails — either by exhausting a
//! runtime budget (resource exhaustion, consistent with the frozen provenance
//! budget narrowing) or by an internal provider fault (a provider call
//! returning an error, or a provider-internal panic being intercepted) — the
//! **host process must not crash**, the **Store must remain reusable** (a
//! subsequent normal guest completes), and the failure must surface as a
//! **typed outcome** (never a panic, never a swallowed error, never a
//! fabricated success).
//!
//! Fault injection seam (STEP-0587): the `sico:script/streams@0.1.0` provider's
//! transport is injected through [`sico_runner::StreamTransportFactory`] via
//! [`sico_runner::Runner::with_stream_transport`]. The default
//! [`Runner::new()`] uses the real stdio transport and is unchanged; the
//! failing/panicking implementations live **here in the test**, not in the
//! production crate, so no fault branch is compiled into the provider path
//! (dependency injection, not a production switch).
//!
//! Coverage matrix (STEP-0587):
//! 1. **resource exhaustion** — a guest whose linear memory grows past the
//!    frozen provenance budget's 1 MiB ceiling under `run_program_with_budgets`
//!    ends in typed `MemoryLimit`, and the same Host then runs a normal guest
//!    to completion;
//! 2. **provider call returns an error** — a failing `write_output` transport
//!    turns `sico.stream.write` into a typed `HostProviderFailure`, and the
//!    same Host then runs a normal guest to completion;
//! 3. **provider-internal panic intercepted** — a panicking `write_output`
//!    transport is caught (wasmtime host-panic interception, or the runner's
//!    guest-worker `join` fallback), surfacing as a typed trap outcome, and the
//!    same Host then runs a normal guest to completion.
//!
//! Guests: the provider-touching guest is a real `.sico` program compiled by
//! the frontend (so the provider call is genuine compiler output); the
//! budget-exhausting guest is the hand-built minimal core module from
//! STEP-0578 (the runner accepts a small declared memory, which is what makes
//! the 1 MiB fixture budget enforceable — see STEP-0578 §Contract-gaps).
//!
//! Evidence level: `internal-fixture` (frozen fixture + real compiler-built
//! guest + hand-built minimal guest; no clean-room consumer, no external
//! pilot).

use std::sync::Arc;

use sico_package::provenance::{Budgets, ProvenanceManifest};
use sico_runner::{
    CancelToken, FsGrants, HostStreamError, RunOutcome, Runner, RunnerLimits, ScriptInput,
    StreamTransport, StreamTransportFactory,
};

/// The frozen provenance manifest of the 256-byte fixture model (budget
/// 1 MiB / 5000 ms / 100000 hostcalls), committed in `sico-package`.
const MODEL_MANIFEST: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/sico-package/tests/fixtures/provenance/model-fixture.provenance.json"
));

const SCRIPT_PRELUDE: &str = "record ScriptInput:
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

";

/// A real compiler-built guest that calls `sico.stream.write` on stdout. With
/// the default (real) transport it writes a marker to the process stdout; with
/// the failing transport the provider call fails and the whole run surfaces a
/// typed provider failure.
fn writer_source() -> String {
    format!(
        "{SCRIPT_PRELUDE}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  \
         let out = sico.stream.stdout()\n  \
         match sico.stream.write(out, sico.text.encode(\"provider-crash-isolation\")):\n    \
         case ok(_):\n      \
         return ok(ScriptOutput(stdout: sico.text.encode(\"\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n    \
         case error(message):\n      \
         return error(ScriptError(code: ScriptErrorCode.DomainError, message: message))\n  end match\n\
         end function\n"
    )
}

/// A real compiler-built guest that returns an empty `ScriptOutput` without
/// touching any provider — the "normal guest" that must complete after a
/// provider fault (proving the Host/Store is not left unusable).
fn plain_source() -> String {
    format!(
        "{SCRIPT_PRELUDE}function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  \
         return ok(ScriptOutput(stdout: sico.text.encode(\"\"), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n\
         end function\n"
    )
}

/// Compiles one `.sico` source to a runnable `script-v0` Program Component
/// through the real frontend (same pattern as `stream_transform.rs`).
fn compile_source(tag: u32, source: &str) -> Vec<u8> {
    let directory =
        std::env::temp_dir().join(format!("sico-step0587-guest-{}-{tag}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("guest.sico");
    let component_path = directory.join("guest.component.wasm");
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
    assert_eq!(
        exit,
        0,
        "sico build failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

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

/// A hand-built minimal core guest (STEP-0578) that grows linear memory past
/// the 1 MiB fixture budget then traps — the deterministic resource-exhaustion
/// probe whose MemoryLimit is attributable to the budget narrowing, not a
/// default limit.
fn grow_past_one_mib() -> Vec<u8> {
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
    // Start at 1 page (64 KiB); grow to 18 pages (1.125 MiB), past the 1 MiB
    // fixture budget. The limiter denies the grow → `RunState::denied` →
    // typed `MemoryLimit`.
    run.instruction(&Wasm::I32Const(17));
    run.instruction(&Wasm::MemoryGrow(0));
    run.instruction(&Wasm::Drop);
    run.instruction(&Wasm::Unreachable);
    run.instruction(&Wasm::End);

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

/// Fault-injection transport: every `write_output` fails with a typed
/// provider I/O fault (the "provider call returns an error" class). Lives in
/// the test, never in the production crate.
struct FailingWriteTransport;

impl StreamTransport for FailingWriteTransport {
    fn read_stdin(&self, _max: u64, _cancel: &CancelToken) -> Result<Vec<u8>, HostStreamError> {
        // Not exercised by the writer guest; fail closed rather than hang.
        Err(HostStreamError::Io)
    }

    fn write_output(
        &self,
        _stderr: bool,
        _bytes: Vec<u8>,
        _cancel: &CancelToken,
    ) -> Result<(), HostStreamError> {
        Err(HostStreamError::Io)
    }

    fn flush_output(&self, _stderr: bool, _cancel: &CancelToken) -> Result<(), HostStreamError> {
        Err(HostStreamError::Io)
    }

    fn try_flush_output(&self, _stderr: bool) {}
}

struct FailingWriteFactory;

impl StreamTransportFactory for FailingWriteFactory {
    fn spawn_transport(&self) -> Box<dyn StreamTransport> {
        Box::new(FailingWriteTransport)
    }
}

/// Fault-injection transport: `write_output` panics — the "provider-internal
/// panic intercepted" class. The panic is either caught by wasmtime's host
/// panic interception (`catch_unwind_and_record_trap`) or, if it propagates,
/// by the runner's guest-worker `join` fallback (`RunOutcome::Trap("guest
/// worker panicked")`); either way the host process survives.
struct PanickingWriteTransport;

impl StreamTransport for PanickingWriteTransport {
    fn read_stdin(&self, _max: u64, _cancel: &CancelToken) -> Result<Vec<u8>, HostStreamError> {
        Err(HostStreamError::Io)
    }

    fn write_output(
        &self,
        _stderr: bool,
        _bytes: Vec<u8>,
        _cancel: &CancelToken,
    ) -> Result<(), HostStreamError> {
        panic!("provider internal panic (fault injection)");
    }

    fn flush_output(&self, _stderr: bool, _cancel: &CancelToken) -> Result<(), HostStreamError> {
        Err(HostStreamError::Io)
    }

    fn try_flush_output(&self, _stderr: bool) {}
}

struct PanickingWriteFactory;

impl StreamTransportFactory for PanickingWriteFactory {
    fn spawn_transport(&self) -> Box<dyn StreamTransport> {
        Box::new(PanickingWriteTransport)
    }
}

/// Asserts the outcome is a member of the typed outcome family and is **not**
/// a fabricated success.
fn assert_typed_failure(outcome: &RunOutcome) {
    assert!(
        !matches!(outcome, RunOutcome::Output(_)),
        "a provider fault must never fabricate a success: {outcome:?}"
    );
    assert!(
        matches!(
            outcome,
            RunOutcome::Domain { .. }
                | RunOutcome::Cancelled
                | RunOutcome::Timeout
                | RunOutcome::FuelExhausted
                | RunOutcome::StackLimit
                | RunOutcome::MemoryLimit
                | RunOutcome::HostProviderFailure { .. }
                | RunOutcome::Trap(_)
                | RunOutcome::Launch(_)
                | RunOutcome::Incompatible(_)
        ),
        "outcome must be a typed family member: {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// 1. Resource exhaustion (narrowed provenance budget) + host reuse
// ---------------------------------------------------------------------------

#[test]
fn resource_exhaustion_is_typed_and_host_stays_reusable() {
    let grow = grow_past_one_mib();
    let budgets = fixture_budgets();
    let host = runner();

    // Under the frozen 1 MiB budget the grow past 1 MiB is denied → typed
    // MemoryLimit (the resource-exhaustion class, consistent with the
    // narrowed limits of `run_program_with_budgets`).
    let outcome = host
        .run_program_with_budgets(
            &grow,
            &ScriptInput::default(),
            &limits(),
            &budgets,
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(outcome, RunOutcome::MemoryLimit, "{outcome:?}");

    // The Host survives the typed resource exhaustion and is immediately
    // reusable: a normal guest (no provider, default limits) runs to
    // completion on the same runner.
    let plain = compile_source(0, &plain_source());
    let again = host
        .run_program(
            &plain,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert!(
        matches!(again, RunOutcome::Output(_)),
        "host must remain reusable after resource exhaustion: {again:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Provider call returns an error + host reuse
// ---------------------------------------------------------------------------

#[test]
fn provider_error_is_typed_host_provider_failure_and_host_stays_reusable() {
    let writer = compile_source(1, &writer_source());
    let plain = compile_source(2, &plain_source());
    let host = runner().with_stream_transport(Arc::new(FailingWriteFactory));

    // The guest calls `sico.stream.write`; the injected provider transport
    // returns `HostStreamError::Io` → the provider fault is classified into
    // the typed `HostProviderFailure` outcome (never a panic, never a
    // swallowed error, never a fabricated success).
    let outcome = host
        .run_program(
            &writer,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_eq!(
        outcome,
        RunOutcome::HostProviderFailure {
            provider_id: "sico.streams.stdout".into(),
        },
        "{outcome:?}"
    );

    // The Host survives the provider fault and is immediately reusable: the
    // normal guest completes on the same runner.
    let again = host
        .run_program(
            &plain,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert!(
        matches!(again, RunOutcome::Output(_)),
        "host must remain reusable after a provider fault: {again:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Provider-internal panic intercepted + host reuse
// ---------------------------------------------------------------------------

#[test]
fn provider_panic_is_intercepted_typed_and_host_stays_reusable() {
    let writer = compile_source(3, &writer_source());
    let plain = compile_source(4, &plain_source());
    let host = runner().with_stream_transport(Arc::new(PanickingWriteFactory));

    // The provider panics inside `write_output`; the panic is intercepted
    // (wasmtime host-panic interception, or the runner's guest-worker join
    // fallback) into a typed outcome. The host process does not crash.
    let outcome = host
        .run_program(
            &writer,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert_typed_failure(&outcome);

    // The Host survives the intercepted panic and is immediately reusable.
    let again = host
        .run_program(
            &plain,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert!(
        matches!(again, RunOutcome::Output(_)),
        "host must remain reusable after an intercepted provider panic: {again:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. Positive control: the default runner still uses the real stdio transport
// ---------------------------------------------------------------------------

#[test]
fn default_runner_still_uses_the_real_stream_transport() {
    // The seam must not have changed the default: `Runner::new()` runs the
    // writer guest to completion (the real stdio write succeeds, exit 0).
    let writer = compile_source(5, &writer_source());
    let outcome = runner()
        .run_program(
            &writer,
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .expect("input bounds are valid");
    assert!(
        matches!(outcome, RunOutcome::Output(_)),
        "default runner must still complete a provider-touching guest: {outcome:?}"
    );
}
