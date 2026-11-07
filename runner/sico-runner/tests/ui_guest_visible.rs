//! Guest-visible `sico:script/ui@1.0.0` **present/frame/event** evidence
//! (M24 §8.6.1, RFC-0052 §2/§3 as re-homed by Amendment A1, ADR-0018).
//!
//! STEP-0581 delivered G1 (guest `window()` creates a real OS window) but left
//! `run()`/`closed` guest-level observation as "后续切片 #2": the e2e drove
//! `run()`, `close()` and `poll()` through **host-side** affordances on
//! `D2dUiRenderer`, not through guest imports. This file is that slice.
//!
//! The compiler frontend does not emit `ui@1` imports (M14 baseline), so these
//! tests encode real guest Components with `wasm-encoder` and run them through
//! the real `Runner` prepare/run path against the **production** backend. The
//! production backend is [`WindowedUiBackend`] (STEP-0338 renderer core +
//! STEP-0577 window shell, composed in STEP-0581); the runner's `D2dUiRenderer`
//! wraps it but does **not** expose `read_presented_frame()` (only
//! `window_title`/`skipped_click_count`/`close`/`destroy` are host-side
//! affordances, `runner/sico-runner/src/ui.rs`). So this fixture drives the
//! SAME production backend through a thin test-local adapter
//! ([`ReadbackBackend`]) that delegates every `ui@1` op to [`WindowedUiBackend`
//! ] exactly as `D2dUiRenderer` does, and additionally exposes the host-side
//! readback affordance the byte-level G2 check needs. The guest path is
//! unchanged: the guest calls `window`/`text`/`run`/`poll-event` imports that
//! reach the real renderer + OS window.
//!
//! Two guest Components share one backend across two generations:
//!
//! - **Guest A** (`window` → `text` → `run` → `poll-event`): proves the guest
//!   observed the window handle, that its own `run()` presented a frame, and
//!   that `poll-event` returned `none` before any event. The host then reads
//!   back the presented DIB and asserts it is **byte-identical** to an
//!   independently rendered offscreen frame (a fresh [`D2dUiBackend`] oracle) —
//!   the present path is a pure byte copy, so the guest-triggered frame is
//!   exactly the renderer's offscreen output.
//! - **Guest B** (`poll-event` loop): after the host posts `WM_CLOSE` (the
//!   close lifecycle is a Host concern; `ui@1` has **no** guest-visible `close`
//!   — see the STEP's honest downgrade), the guest's own `poll-event` delivers
//!   the `closed` variant.
//!
//! Authority (④): with `UiGrants::default()` every call still fails closed as
//! typed `authority-denied` and the backend is never touched (zero-touch), so
//! the default-deny semantics from STEP-0581 do not regress.
//!
//! Fail-closed / environment honesty: a real OS window needs an interactive
//! desktop. On a headless host the guest's `window()` returns a typed
//! `ui-error` and these tests degrade to asserting that typed refusal plus an
//! explicit `ENV_LIMIT` marker — never a fabricated "presented". On this
//! evidence host the window is real and the full lifecycle runs.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
    ui::{UiError, UiEvent, UiGrants, UiRenderer},
};
use sico_ui_renderer_d2d::events::WindowEvent;
use sico_ui_renderer_d2d::{D2dUiBackend, Frame, UiBackendError, WindowedUiBackend};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use wasm_encoder::{
    BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ComponentTypeRef,
    ComponentValType, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, InstanceType, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeBounds, TypeSection,
    ValType,
};

const TITLE_OFFSET: u32 = 1024;
const TITLE_LEN: u32 = 10; // "ui-fixture"
const TEXT_OFFSET: u32 = 2048;
const TEXT_LEN: u32 = 4; // "Sico"
const RESULT_AREA: u32 = 4096;
const AREA_WINDOW: u32 = 8192;
const AREA_TEXT: u32 = 10_240;
const AREA_RUN: u32 = 12_288;
const AREA_POLL: u32 = 14_336;
const EMPTY_LIST_PTR: u32 = 3072;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;
const TRANSPORT_INSTANCE: &str = "ui-transport";
const UI_INTERFACE: &str = "sico:script/ui@1.0.0";

/// Client size the guest requests and the oracle renders.
const CLIENT_WIDTH: u64 = 320;
const CLIENT_HEIGHT: u64 = 160;

/// Exit codes the guest reports through the script-output `exit-code`, all
/// within the runner's accepted `0..=119` range. Guest A: `window` err →
/// `110 + ui-error disc`; `text` err → `80 + disc`; `run` err → `90 + disc`;
/// unexpected `poll-event` Some → `70 + disc`; full success → `1` (the
/// window-root handle the guest observed). Guest B: `closed` observed → `105`;
/// 64 polls with no event → `106`.
const EXIT_WINDOW_ERR_BASE: i64 = 110;
const EXIT_RUN_ERR_BASE: i64 = 90;
const EXIT_TEXT_ERR_BASE: i64 = 80;
const EXIT_POLL_SOME_BASE: i64 = 70;
const EXIT_CLOSED: i64 = 105;
const EXIT_NO_CLOSED: i64 = 106;
/// The guest's own `poll-event` must observe `none` (option disc 0) before the
/// close; a `Some` here is an unexpected early event (see `EXIT_POLL_SOME_BASE`).
const POLL_ROUNDS: i32 = 64;

/// The production backend with the host-side readback affordance the byte-level
/// G2 check needs. It wraps [`WindowedUiBackend`] — the SAME backend
/// [`D2dUiRenderer`](sico_runner::ui::D2dUiRenderer) wraps — and delegates every
/// `ui@1` operation identically; it is not a new renderer and claims no extra
/// behaviour. `calls` counts every operation that crosses the authority gate so
/// the zero-touch assertion (④) is observable.
struct ReadbackBackend {
    inner: WindowedUiBackend,
    calls: AtomicUsize,
}

impl ReadbackBackend {
    fn new() -> Result<Self, UiBackendError> {
        Ok(Self {
            inner: WindowedUiBackend::new()?,
            calls: AtomicUsize::new(0),
        })
    }

    fn hit(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    /// The title of the open window, if a real window was created.
    fn window_title(&self) -> Option<String> {
        self.inner.window_title()
    }

    /// Reads back the last presented frame from the shell's DIB (byte copy).
    fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        self.inner.read_presented_frame()
    }

    /// Posts a `WM_CLOSE` so the window closes deterministically.
    fn close(&self) -> Result<(), UiBackendError> {
        self.inner.close()
    }

    /// Destroys the window; idempotent.
    fn destroy(&self) -> Result<(), UiBackendError> {
        self.inner.destroy()
    }
}

/// Folds a renderer failure into the closed guest `ui-error` set, mirroring the
/// documented mapping (`ui.rs::fold_error`): `InvalidArgument`→`limit`,
/// `NoWindow|UnsupportedPlatform|RenderFailed`→`no-window`.
fn fold(error: UiBackendError) -> UiError {
    match error {
        UiBackendError::UnknownHandle => UiError::UnknownHandle,
        UiBackendError::WrongParentKind => UiError::WrongParentKind,
        UiBackendError::Limit { .. } => UiError::Limit,
        UiBackendError::InvalidArgument { .. } => UiError::Limit,
        UiBackendError::NoWindow
        | UiBackendError::UnsupportedPlatform
        | UiBackendError::RenderFailed { .. } => UiError::NoWindow,
    }
}

impl UiRenderer for ReadbackBackend {
    fn window(&self, title: &str, width: i64, height: i64) -> Result<u64, UiError> {
        self.hit();
        self.inner.window(title, width, height).map_err(fold)
    }
    fn column(&self, parent: u64) -> Result<u64, UiError> {
        self.hit();
        self.inner.column(parent).map_err(fold)
    }
    fn row(&self, parent: u64) -> Result<u64, UiError> {
        self.hit();
        self.inner.row(parent).map_err(fold)
    }
    fn text(&self, parent: u64, content: &str) -> Result<u64, UiError> {
        self.hit();
        self.inner.text(parent, content).map_err(fold)
    }
    fn button(&self, parent: u64, label: &str) -> Result<u64, UiError> {
        self.hit();
        self.inner.button(parent, label).map_err(fold)
    }
    fn dropdown(&self, parent: u64, options: &[String]) -> Result<u64, UiError> {
        self.hit();
        self.inner.dropdown(parent, options).map_err(fold)
    }
    fn image_preview(&self, parent: u64, w: u64, h: u64) -> Result<u64, UiError> {
        self.hit();
        self.inner.image_preview(parent, w, h).map_err(fold)
    }
    fn progress(&self, parent: u64) -> Result<u64, UiError> {
        self.hit();
        self.inner.progress(parent).map_err(fold)
    }
    fn set_text(&self, handle: u64, content: &str) -> Result<bool, UiError> {
        self.hit();
        self.inner.set_text(handle, content).map_err(fold)
    }
    fn set_progress(&self, handle: u64, done: u64, of: u64) -> Result<bool, UiError> {
        self.hit();
        self.inner.set_progress(handle, done, of).map_err(fold)
    }
    fn draw_image(&self, handle: u64, w: u64, h: u64, pixels: &[u8]) -> Result<bool, UiError> {
        self.hit();
        self.inner.draw_image(handle, w, h, pixels).map_err(fold)
    }
    fn poll_event(&self) -> Option<UiEvent> {
        self.hit();
        // Pump until a deliverable `ui@1` event surfaces, exactly as
        // `D2dUiRenderer::poll_event` does: `Closed`→`closed`; a coordinate
        // `Clicked` is not fabricated into a `click(handle)` (no hit-testing),
        // `Resized`/`Paint` are lifecycle notifications with no `ui@1` variant,
        // and a lost shell thread surfaces as `None`.
        loop {
            match self.inner.poll_event() {
                Ok(Some(WindowEvent::Closed)) => return Some(UiEvent::Closed),
                Ok(Some(WindowEvent::Clicked { .. }))
                | Ok(Some(WindowEvent::Resized { .. }))
                | Ok(Some(WindowEvent::Paint)) => {}
                Ok(None) | Err(_) => return None,
            }
        }
    }
    fn run(&self) -> Result<bool, UiError> {
        self.hit();
        self.inner.run().map_err(fold)
    }
}

/// Encodes one fixture Component importing the `sico:script/ui@1.0.0` subset
/// (`window`, `text`, `run`, `poll-event`), lowering all four against the
/// shared transport and wiring `guest` as the guest core module.
fn component_with_guest(guest: &[u8]) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();

    let types_ty = builder.type_instance(Some("script-types"), &script_types());
    let types_instance = builder.import("sico:script/types", ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    // The versioned UI host-provider instance (subset: `window`, `text`, `run`,
    // `poll-event`).
    let ui_ty = builder.type_instance(Some("ui1"), &ui_instance_type());
    let ui_instance = builder.import(UI_INTERFACE, ComponentTypeRef::Instance(ui_ty));

    // Transport core module: shared memory + a checked bump allocator and the
    // static title/text strings every canonical lift/lower reads.
    let transport_module_index =
        builder.core_module_raw(Some(TRANSPORT_INSTANCE), &transport_module());
    let transport_instance = builder.core_instantiate(
        Some(TRANSPORT_INSTANCE),
        transport_module_index,
        std::iter::empty::<(&str, ModuleArg)>(),
    );
    let transport_memory = builder.core_alias_export(
        Some("memory"),
        transport_instance,
        "memory",
        ExportKind::Memory,
    );
    let transport_realloc = builder.core_alias_export(
        Some("realloc"),
        transport_instance,
        "realloc",
        ExportKind::Func,
    );
    let transport_exports = builder.core_instantiate_exports(
        Some("transport-exports"),
        [
            ("memory", ExportKind::Memory, transport_memory),
            ("realloc", ExportKind::Func, transport_realloc),
        ],
    );

    // Lower the four `ui@1` funcs against the transport memory.
    let window_function = builder.alias_export(ui_instance, "window", ComponentExportKind::Func);
    let window_lowered = builder.lower_func(
        Some("window"),
        window_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let text_function = builder.alias_export(ui_instance, "text", ComponentExportKind::Func);
    let text_lowered = builder.lower_func(
        Some("text"),
        text_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let run_function = builder.alias_export(ui_instance, "run", ComponentExportKind::Func);
    let run_lowered = builder.lower_func(
        Some("run"),
        run_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    // `poll-event`'s `option<ui-event>` result flattens to more than one core
    // type, so the canonical ABI uses the return-area form (a trailing i32
    // retptr); no owned-heap value is returned, so realloc is never invoked.
    let poll_function = builder.alias_export(ui_instance, "poll-event", ComponentExportKind::Func);
    let poll_lowered = builder.lower_func(
        Some("poll-event"),
        poll_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let lowered_instance = builder.core_instantiate_exports(
        Some("ui1-lowered"),
        [
            ("window", ExportKind::Func, window_lowered),
            ("text", ExportKind::Func, text_lowered),
            ("run", ExportKind::Func, run_lowered),
            ("poll-event", ExportKind::Func, poll_lowered),
        ],
    );

    // Guest core module.
    let core = builder.core_module_raw(Some("guest"), guest);
    let instance = builder.core_instantiate(
        Some("guest"),
        core,
        [
            (TRANSPORT_INSTANCE, ModuleArg::Instance(transport_exports)),
            (UI_INTERFACE, ModuleArg::Instance(lowered_instance)),
        ],
    );
    let memory = builder.core_alias_export(Some("memory"), instance, "memory", ExportKind::Memory);
    let realloc = builder.core_alias_export(
        Some("cabi_realloc"),
        instance,
        "cabi_realloc",
        ExportKind::Func,
    );
    let post_return = builder.core_alias_export(
        Some("cabi_post_run"),
        instance,
        "cabi_post_run",
        ExportKind::Func,
    );
    let run_core = builder.core_alias_export(Some("run"), instance, "run", ExportKind::Func);
    let run_function = builder.lift_func(
        Some("run"),
        run_core,
        run_ty,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(memory),
            wasm_encoder::CanonicalOption::Realloc(realloc),
            wasm_encoder::CanonicalOption::PostReturn(post_return),
        ],
    );
    builder.export("run", ComponentExportKind::Func, run_function, None);
    builder.finish()
}

/// The Guest A Component (imports `window`, `text`, `run`, `poll-event`).
fn fixture_a() -> Vec<u8> {
    component_with_guest(&guest_a_module())
}

/// The Guest B Component (imports the same four; calls only `poll-event`).
fn fixture_b() -> Vec<u8> {
    component_with_guest(&guest_b_module())
}

/// The component-level instance type of the `ui@1.0.0` subset (`window`,
/// `text`, `run`, `poll-event`, width-subtyped over the frozen 13-func
/// surface). Index bookkeeping follows the http2 adapter's proven rules:
/// `ty()` consumes one index, each `Eq` alias export consumes one, `Func`
/// exports consume none. `window`=Func(5), `poll-event`=Func(11),
/// `run`=Func(13), `text`=Func(14).
fn ui_instance_type() -> InstanceType {
    let mut types = InstanceType::new();
    // 0 record ui-handle { id: u64 }; 1 alias ui-handle; 2 enum ui-error;
    // 3 alias ui-error; 4 result<ui-handle, ui-error>; 5 func window.
    types
        .ty()
        .defined_type()
        .record([("id", ComponentValType::Primitive(PrimitiveValType::U64))]);
    types.export("ui-handle", ComponentTypeRef::Type(TypeBounds::Eq(0)));
    types.ty().defined_type().enum_type([
        "unknown-handle",
        "wrong-parent-kind",
        "limit",
        "no-window",
        "event-overflow",
        "authority-denied",
    ]);
    types.export("ui-error", ComponentTypeRef::Type(TypeBounds::Eq(2)));
    types.ty().defined_type().result(
        Some(ComponentValType::Type(1)),
        Some(ComponentValType::Type(3)),
    );
    let mut window = types.ty().function();
    window.params(vec![
        (
            "title",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
        ("width", ComponentValType::Primitive(PrimitiveValType::S64)),
        ("height", ComponentValType::Primitive(PrimitiveValType::S64)),
    ]);
    window.result(Some(ComponentValType::Type(4)));
    types.export("window", ComponentTypeRef::Func(5));

    // Event surface for `poll-event`, mirroring the `ui.rs` derives exactly
    // (case order click/select/closed; select packages handle+index in a
    // record because a Wasmtime variant permits one unnamed payload per case).
    // 6 record ui-selection; 7 alias ui-selection; 8 variant ui-event;
    // 9 alias ui-event; 10 option<ui-event>; 11 func poll-event.
    types.ty().defined_type().record([
        ("handle", ComponentValType::Type(1)),
        ("index", ComponentValType::Primitive(PrimitiveValType::U64)),
    ]);
    types.export("ui-selection", ComponentTypeRef::Type(TypeBounds::Eq(6)));
    types.ty().defined_type().variant([
        ("click", Some(ComponentValType::Type(1))),
        ("select", Some(ComponentValType::Type(7))),
        ("closed", None),
    ]);
    types.export("ui-event", ComponentTypeRef::Type(TypeBounds::Eq(8)));
    types.ty().defined_type().option(ComponentValType::Type(9));
    let mut poll = types.ty().function();
    poll.params(Vec::<(&str, ComponentValType)>::new());
    poll.result(Some(ComponentValType::Type(10)));
    types.export("poll-event", ComponentTypeRef::Func(11));

    // 12 result<bool, ui-error>; 13 func run.
    types.ty().defined_type().result(
        Some(ComponentValType::Primitive(PrimitiveValType::Bool)),
        Some(ComponentValType::Type(3)),
    );
    let mut run = types.ty().function();
    run.params(Vec::<(&str, ComponentValType)>::new());
    run.result(Some(ComponentValType::Type(12)));
    types.export("run", ComponentTypeRef::Func(13));

    // 14 func text (parent ui-handle, content string) -> result<ui-handle,
    // ui-error>.
    let mut text = types.ty().function();
    text.params(vec![
        ("parent", ComponentValType::Type(1)),
        (
            "content",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
    ]);
    text.result(Some(ComponentValType::Type(4)));
    types.export("text", ComponentTypeRef::Func(14));
    types
}

fn script_types() -> InstanceType {
    let mut types = InstanceType::new();
    types.ty().defined_type().list(PrimitiveValType::String);
    types.export("string-list", ComponentTypeRef::Type(TypeBounds::Eq(0)));
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.export("byte-list", ComponentTypeRef::Type(TypeBounds::Eq(2)));
    types.ty().defined_type().record([
        ("arguments", ComponentValType::Type(1)),
        ("stdin", ComponentValType::Type(3)),
    ]);
    types.export("script-input", ComponentTypeRef::Type(TypeBounds::Eq(4)));
    types.ty().defined_type().record([
        ("stdout", ComponentValType::Type(3)),
        ("stderr", ComponentValType::Type(3)),
        (
            "exit-code",
            ComponentValType::Primitive(PrimitiveValType::S64),
        ),
    ]);
    types.export("script-output", ComponentTypeRef::Type(TypeBounds::Eq(6)));
    types.ty().defined_type().enum_type([
        "invalid-input",
        "resource-limit",
        "domain-error",
        "cancelled",
    ]);
    types.export(
        "script-error-code",
        ComponentTypeRef::Type(TypeBounds::Eq(8)),
    );
    types.ty().defined_type().record([
        ("code", ComponentValType::Type(9)),
        (
            "message",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
    ]);
    types.export("script-error", ComponentTypeRef::Type(TypeBounds::Eq(10)));
    types.ty().defined_type().result(
        Some(ComponentValType::Type(7)),
        Some(ComponentValType::Type(11)),
    );
    types.export("script-result", ComponentTypeRef::Type(TypeBounds::Eq(12)));
    types
}

/// Shared transport: one memory + a checked bump allocator and the static
/// title (`ui-fixture`) and text (`Sico`) strings the guest passes.
fn transport_module() -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    let mut functions = FunctionSection::new();
    functions.function(0);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: u64::from(MEMORY_PAGES),
        maximum: Some(u64::from(MEMORY_PAGES)),
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
        &ConstExpr::i32_const(EMPTY_LIST_PTR as i32),
    );
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("realloc", ExportKind::Func, 0);
    let mut realloc = Function::new(vec![(2, ValType::I32)]);
    for instruction in [
        Instruction::GlobalGet(0),
        Instruction::LocalGet(2),
        Instruction::I32Add,
        Instruction::I32Const(-1),
        Instruction::I32Add,
        Instruction::I32Const(0),
        Instruction::LocalGet(2),
        Instruction::I32Sub,
        Instruction::I32And,
        Instruction::LocalTee(4),
        Instruction::LocalGet(3),
        Instruction::I32Add,
        Instruction::LocalTee(5),
        Instruction::LocalGet(4),
        Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::Unreachable,
        Instruction::End,
        Instruction::LocalGet(5),
        Instruction::I32Const(i32::try_from(ARENA_LIMIT).unwrap_or(i32::MAX)),
        Instruction::I32GtU,
        Instruction::If(BlockType::Empty),
        Instruction::Unreachable,
        Instruction::End,
        Instruction::LocalGet(5),
        Instruction::GlobalSet(0),
        Instruction::LocalGet(4),
        Instruction::End,
    ] {
        realloc.instruction(&instruction);
    }
    let mut data = DataSection::new();
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(TITLE_OFFSET).unwrap_or(0)),
        b"ui-fixture".to_vec(),
    );
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(TEXT_OFFSET).unwrap_or(0)),
        b"Sico".to_vec(),
    );
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    let mut code = CodeSection::new();
    code.function(&realloc);
    module.section(&code);
    module.section(&data);
    module.finish()
}

fn load8(offset: u32) -> Instruction<'static> {
    Instruction::I32Load8U(MemArg {
        offset: u64::from(offset),
        align: 0,
        memory_index: 0,
    })
}

fn store(offset: u32) -> Instruction<'static> {
    Instruction::I32Store(MemArg {
        offset: u64::from(offset),
        align: 2,
        memory_index: 0,
    })
}

/// Emits a zero-tag (ok) script-output with empty stdout/stderr and
/// `exit = base + <ui-error discriminant>` read from `disc_area_offset`, then
/// returns the area pointer. Area layout: tag byte @0; stdout @8/12;
/// stderr @16/20; exit-code i64 @24; 32 bytes total.
fn push_error_exit(
    instructions: &mut Vec<Instruction<'static>>,
    exit_local: u32,
    disc_area_offset: u32,
    base: i64,
) {
    for instruction in [
        Instruction::I32Const(disc_area_offset as i32),
        load8(0),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(base),
        Instruction::I64Add,
        Instruction::LocalSet(exit_local),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        Instruction::I32Store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        store(8),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        store(12),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        store(16),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        store(20),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::LocalGet(exit_local),
        Instruction::I64Store(MemArg {
            offset: 24,
            align: 3,
            memory_index: 0,
        }),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::Return,
    ] {
        instructions.push(instruction);
    }
}

/// Emits a zero-tag (ok) script-output with empty stdout/stderr and
/// `exit = exit_local`, then leaves the area pointer for the function return.
fn push_ok_return(instructions: &mut Vec<Instruction<'static>>, exit_local: u32) {
    for instruction in [
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        Instruction::I32Store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        store(8),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        store(12),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        store(16),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        store(20),
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::LocalGet(exit_local),
        Instruction::I64Store(MemArg {
            offset: 24,
            align: 3,
            memory_index: 0,
        }),
        Instruction::I32Const(RESULT_AREA as i32),
    ] {
        instructions.push(instruction);
    }
}

/// The shared guest core-module function index space:
/// 0 = imported realloc; 1 = imported lowered `window`;
/// 2 = imported lowered `text`; 3 = imported lowered `run`;
/// 4 = imported lowered `poll-event`; 5 = guest `run`; 6 = no-op post-return.
fn guest_module(run_body: Function) -> Vec<u8> {
    let mut types = TypeSection::new();
    // 0: realloc
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 1: window — (title ptr, title len, width i64, height i64, result area)
    //     -> (); the result<record{u64}, enum> flattens to two core types, so
    //     the canonical ABI uses the return-area form.
    types.ty().function(
        [
            ValType::I32,
            ValType::I32,
            ValType::I64,
            ValType::I64,
            ValType::I32,
        ],
        [],
    );
    // 2: text — (parent i64, content ptr, content len, result area) -> ()
    types
        .ty()
        .function([ValType::I64, ValType::I32, ValType::I32, ValType::I32], []);
    // 3: run — (result area) -> ()
    types.ty().function([ValType::I32], []);
    // 4: poll-event — (result area) -> ()
    types.ty().function([ValType::I32], []);
    // 5: run — (input record) -> (result area ptr)
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 6: post-run
    types.ty().function([ValType::I32], []);

    let mut imports = ImportSection::new();
    imports.import(
        TRANSPORT_INSTANCE,
        "memory",
        EntityType::Memory(MemoryType {
            minimum: u64::from(MEMORY_PAGES),
            maximum: Some(u64::from(MEMORY_PAGES)),
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    imports.import(TRANSPORT_INSTANCE, "realloc", EntityType::Function(0));
    imports.import(UI_INTERFACE, "window", EntityType::Function(1));
    imports.import(UI_INTERFACE, "text", EntityType::Function(2));
    imports.import(UI_INTERFACE, "run", EntityType::Function(3));
    imports.import(UI_INTERFACE, "poll-event", EntityType::Function(4));

    let mut function_section = FunctionSection::new();
    function_section.function(5);
    function_section.function(6);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, 5);
    exports.export("cabi_post_run", ExportKind::Func, 6);

    let mut code = CodeSection::new();
    code.function(&run_body);
    let mut nop = Function::new(vec![]);
    nop.instruction(&Instruction::End);
    code.function(&nop);

    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&function_section);
    module.section(&exports);
    module.section(&code);
    module.finish()
}

/// Guest A core module: `window` → `text` → `run` → `poll-event`, reporting
/// through the exit code (see the exit-code constants).
fn guest_a_module() -> Vec<u8> {
    guest_module(guest_a_body())
}

/// Guest B core module: `poll-event` loop until `closed` (or `POLL_ROUNDS`
/// empty polls).
fn guest_b_module() -> Vec<u8> {
    guest_module(guest_b_body())
}

/// Guest A `run` body. Locals: params 0..3 (input record), 4 = i32 tag/temp,
/// 5 = i64 exit. On the granted path the guest observes the window-root handle
/// (exit = 1), a successful `run()` present, and a `none` poll — in that order.
fn guest_a_body() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32), (1, ValType::I64)]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // window(title_ptr, title_len, 320, 160, AREA_WINDOW)
        Instruction::I32Const(TITLE_OFFSET as i32),
        Instruction::I32Const(TITLE_LEN as i32),
        Instruction::I64Const(CLIENT_WIDTH as i64),
        Instruction::I64Const(CLIENT_HEIGHT as i64),
        Instruction::I32Const(AREA_WINDOW as i32),
        Instruction::Call(1),
        // tag = load8(AREA_WINDOW + 0)
        Instruction::I32Const(AREA_WINDOW as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ];
    // error discriminant is at AREA_WINDOW + 8 (union align 8).
    push_error_exit(&mut instructions, 5, AREA_WINDOW + 8, EXIT_WINDOW_ERR_BASE);
    for instruction in [
        Instruction::End,
        // ok: exit = handle id (i64 @ AREA_WINDOW + 8) — the guest observed it.
        Instruction::I32Const(AREA_WINDOW as i32),
        Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(5),
        // text(handle, "Sico", AREA_TEXT)
        Instruction::LocalGet(5),
        Instruction::I32Const(TEXT_OFFSET as i32),
        Instruction::I32Const(TEXT_LEN as i32),
        Instruction::I32Const(AREA_TEXT as i32),
        Instruction::Call(2),
        Instruction::I32Const(AREA_TEXT as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(&mut instructions, 5, AREA_TEXT + 8, EXIT_TEXT_ERR_BASE);
    for instruction in [
        Instruction::End,
        // run(AREA_RUN)
        Instruction::I32Const(AREA_RUN as i32),
        Instruction::Call(3),
        Instruction::I32Const(AREA_RUN as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    // `run() -> result<bool, ui-error>`: tag @0, payload @4 (bool / err disc).
    push_error_exit(&mut instructions, 5, AREA_RUN + 4, EXIT_RUN_ERR_BASE);
    for instruction in [
        Instruction::End,
        // poll-event(AREA_POLL)
        Instruction::I32Const(AREA_POLL as i32),
        Instruction::Call(4),
        // option disc @ AREA_POLL + 0 (0 = none, 1 = some)
        Instruction::I32Const(AREA_POLL as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    // none: exit stays = handle (local 5).
    for instruction in [
        Instruction::Else,
        // some (unexpected): exit = 140 + variant disc @ AREA_POLL + 8
        Instruction::I32Const((AREA_POLL + 8) as i32),
        load8(0),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(EXIT_POLL_SOME_BASE),
        Instruction::I64Add,
        Instruction::LocalSet(5),
        Instruction::End,
    ] {
        instructions.push(instruction);
    }
    push_ok_return(&mut instructions, 5);
    instructions.push(Instruction::End);
    for instruction in instructions {
        body.instruction(&instruction);
    }
    body
}

/// Guest B `run` body: poll until `closed` (option disc 1 + variant disc 2) or
/// `POLL_ROUNDS` empty polls. Locals: params 0..3 (input record), 4 = i32
/// counter, 5 = i64 exit (0 = not yet, 201 = closed, 202 = gave up).
fn guest_b_body() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32), (1, ValType::I64)]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // counter = 0; exit = 0
        Instruction::I32Const(0),
        Instruction::LocalSet(4),
        Instruction::I64Const(0),
        Instruction::LocalSet(5),
        Instruction::Loop(BlockType::Empty),
        // poll-event(AREA_POLL)
        Instruction::I32Const(AREA_POLL as i32),
        Instruction::Call(4),
        // if option disc == 1 (some)
        Instruction::I32Const(AREA_POLL as i32),
        load8(0),
        Instruction::I32Const(1),
        Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        // if variant disc == 2 (closed)
        Instruction::I32Const((AREA_POLL + 8) as i32),
        load8(0),
        Instruction::I32Const(2),
        Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::I64Const(EXIT_CLOSED),
        Instruction::LocalSet(5),
        Instruction::End,
        Instruction::End,
        // counter += 1
        Instruction::LocalGet(4),
        Instruction::I32Const(1),
        Instruction::I32Add,
        Instruction::LocalSet(4),
        // continue while counter < POLL_ROUNDS AND exit == 0
        Instruction::LocalGet(4),
        Instruction::I32Const(POLL_ROUNDS),
        Instruction::I32LtU,
        Instruction::LocalGet(5),
        Instruction::I64Eqz,
        Instruction::I32And,
        Instruction::BrIf(0),
        Instruction::End,
        // if exit == 0: exit = 202 (no closed in POLL_ROUNDS)
        Instruction::LocalGet(5),
        Instruction::I64Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::I64Const(EXIT_NO_CLOSED),
        Instruction::LocalSet(5),
        Instruction::End,
    ];
    push_ok_return(&mut instructions, 5);
    instructions.push(Instruction::End);
    for instruction in instructions {
        body.instruction(&instruction);
    }
    body
}

fn exit_of(outcome: RunOutcome) -> (Vec<u8>, i64) {
    match outcome {
        RunOutcome::Output(output) => (output.stdout, output.exit_code),
        other => panic!("expected guest output, got {other:?}"),
    }
}

/// Runs a guest Component against the shared backend (granted or denied) and
/// returns the outcome. Each call prepares a fresh program; the backend Arc is
/// shared so the window/shell state persists across generations.
fn run_guest(component: &[u8], backend: &Arc<ReadbackBackend>, deny: bool) -> RunOutcome {
    let grants = if deny {
        UiGrants::default()
    } else {
        UiGrants { window: true }
    };
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(grants, backend.clone());
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("guest component links");
    prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

/// The independent offscreen oracle: a fresh [`D2dUiBackend`] building the SAME
/// tree Guest A builds (window root + one text child), rendered offscreen.
/// Because the renderer is a pure function of the tree (ADR-0018 §3) and the
/// present path is a byte copy, the oracle's frame must equal the DIB the
/// guest-triggered `run()` left in the shell.
#[cfg(windows)]
fn offscreen_oracle() -> Frame {
    let oracle = D2dUiBackend::new().expect("offscreen renderer builds on Windows");
    let root = oracle
        .window("ui-fixture", CLIENT_WIDTH as i64, CLIENT_HEIGHT as i64)
        .expect("oracle window root");
    oracle.text(root, "Sico").expect("oracle text child");
    oracle.render_frame().expect("oracle frame")
}

/// The real Direct2D + window-shell backend guest-level evidence (G2/G3 and
/// the ④ authority regression). These run the hand-encoded guest Components
/// through the real runner against the production [`WindowedUiBackend`]
/// backend (via [`ReadbackBackend`]).
#[cfg(windows)]
mod real_backend {
    use super::*;

    fn real_backend() -> Arc<ReadbackBackend> {
        Arc::new(ReadbackBackend::new().expect("the production backend builds on Windows"))
    }

    #[test]
    fn guest_run_presents_a_real_frame_byte_identical_to_offscreen_oracle() {
        // G2: the guest's OWN `run()` presents a frame; the host reads the DIB
        // back and asserts it is byte-identical to an independently rendered
        // offscreen frame. This also re-verifies G1 (a real OS window exists).
        let backend = real_backend();
        let outcome = run_guest(&fixture_a(), &backend, false);
        let (stdout, exit) = exit_of(outcome);
        assert!(stdout.is_empty());
        if backend.window_title().is_some() {
            // Real window: the guest observed the window-root handle (1), its
            // `text` child built, its own `run()` presented, and a `none` poll.
            assert_eq!(exit, 1, "the guest observed handle 1 and run() presented");
            assert_eq!(
                backend.window_title().as_deref(),
                Some("ui-fixture"),
                "the host confirms a real OS window with the requested title"
            );
            let readback = backend
                .read_presented_frame()
                .expect("a presented frame is readable back");
            assert_eq!(
                (readback.width(), readback.height()),
                (CLIENT_WIDTH as u32, CLIENT_HEIGHT as u32),
                "the presented frame has the requested client size"
            );
            let oracle = offscreen_oracle();
            assert_eq!(
                readback.pixels(),
                oracle.pixels(),
                "the guest-triggered presented frame is byte-identical to the offscreen frame"
            );
            assert_eq!(
                readback.sha256(),
                oracle.sha256(),
                "the frame digests agree (deterministic render)"
            );
        } else {
            // Headless / no interactive desktop: the guest saw a typed ui-error
            // (any discriminant), never a trap and never a fabricated frame.
            assert!(
                (111..=115).contains(&exit),
                "expected a typed ui-error refusal on headless, got exit {exit}"
            );
            eprintln!(
                "ENV_LIMIT: no interactive desktop; real window creation refused as a typed ui-error"
            );
        }
    }

    #[test]
    fn guest_poll_event_delivers_closed_after_host_close() {
        // G3: the guest's OWN `poll-event` delivers `closed`. `ui@1` has no
        // guest-visible `close`, so the close is a Host lifecycle action
        // (posting WM_CLOSE — the same affordance a user closing the window
        // triggers); the guest observes the resulting `closed` through its own
        // `poll-event`. This is "guest observes" with a host-triggered close —
        // the honest downgrade documented in the STEP.
        let backend = real_backend();
        let out_a = run_guest(&fixture_a(), &backend, false);
        let (_, exit_a) = exit_of(out_a);
        if backend.window_title().is_none() {
            // Headless: guest A already got the typed refusal; the close and
            // closed-delivery drive need a real window, so it is honestly
            // skipped.
            assert!(
                (111..=115).contains(&exit_a),
                "expected a typed ui-error refusal on headless, got exit {exit_a}"
            );
            eprintln!(
                "ENV_LIMIT: headless; the closed-delivery drive is skipped (typed refusal above)"
            );
            return;
        }
        assert_eq!(exit_a, 1, "guest A created the real window (handle 1)");

        // The host closes the window (posts WM_CLOSE); the guest's poll-event
        // must then deliver `closed`.
        backend.close().expect("close posts WM_CLOSE");
        let out_b = run_guest(&fixture_b(), &backend, false);
        let (stdout_b, exit_b) = exit_of(out_b);
        assert!(stdout_b.is_empty());
        assert_eq!(
            exit_b, EXIT_CLOSED,
            "the guest's poll-event delivered `closed` (exit {EXIT_CLOSED})"
        );
        // Tear down idempotently (the OS already destroyed the window on close).
        let _ = backend.destroy();
    }

    #[test]
    fn denied_guest_is_typed_authority_refusal_with_zero_backend_touch() {
        // ④: the default-deny gate fires before the backend; the guest sees
        // the typed `authority-denied` (discriminant 5 → exit 115) and the
        // production backend is never touched (zero-touch), so the STEP-0581
        // semantics do not regress.
        let backend = real_backend();
        let outcome = run_guest(&fixture_a(), &backend, true);
        let (stdout, exit) = exit_of(outcome);
        assert!(stdout.is_empty());
        assert_eq!(
            exit, 115,
            "authority-denied is discriminant 5: exit = 110 + 5 → {exit}"
        );
        assert_eq!(
            backend.calls.load(Ordering::SeqCst),
            0,
            "a denied call must never reach the production backend"
        );
        assert_eq!(
            backend.window_title(),
            None,
            "no window was created on the denied path"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn windowed_backend_refuses_typed_off_windows() {
    // The Direct2D/win32 backend is a Windows implementation; on any other host
    // it must refuse with the typed `no-window` (folded from UnsupportedPlatform),
    // never panic and never pretend to own a window.
    let error = WindowedUiBackend::new().expect_err("the backend must refuse off Windows");
    assert_eq!(error, UiBackendError::UnsupportedPlatform);
}
