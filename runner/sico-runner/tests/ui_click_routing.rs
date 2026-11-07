//! Guest-visible `sico:script/ui@1.0.0` **click(handle) routing** evidence
//! (M24 §8.6.1, RFC-0052 §2/§3 as re-homed by Amendment A1, ADR-0018).
//!
//! STEP-0592 delivered the pure layout + point→handle hit-testing
//! (`crates/sico-ui-renderer-d2d/src/layout.rs`); STEP-0597 (this slice) wires
//! the runner to deliver `click(ui-handle)` for a coordinate
//! [`WindowEvent::Clicked`] instead of skipping it. This file is the
//! **G4-capability** e2e evidence: a real hand-encoded guest Component runs
//! through the real [`Runner`] against the production [`WindowedUiBackend`]
//! (via a test-local adapter), and the guest's own `poll-event` observes a
//! `click(handle)` whose handle is routed correctly.
//!
//! # How the click is manufactured (honest statement)
//!
//! The `ui@1` surface has **no** guest-visible way to inject a click, and the
//! shell only captures a click from a real OS `WM_LBUTTONDOWN` (the `HWND` is
//! private to the shell). A real `SendInput` click would need the window to be
//! foreground/focused and is not deterministic in CI, so this slice does **not**
//! drive a real mouse. Instead the host delivers the **client coordinates**
//! directly into the routing seam (`ClickRoutingBackend::inject_click`), and
//! the routing uses the **same production function** the real
//! [`D2dUiRenderer`](sico_runner::ui::D2dUiRenderer) uses
//! (`sico_runner::ui::route_click_event`) against the **real** tree + live
//! client size (`WindowedUiBackend::layout_input`). The evidence is therefore
//! "host delivers coordinates → guest observes `click(handle)`"; the only
//! difference from a real user click is that the OS `WM_LBUTTONDOWN` capture +
//! `translate` step is bypassed (it is a pure, deterministic message→event
//! mapping, tested by the frozen event corpus). The routing — the part this
//! slice adds — is exercised end-to-end.
//!
//! # The three attribution cases
//!
//! - **child**: a click on the button resolves to the button handle (the
//!   deepest node), never to its column or the root;
//! - **root**: a click in the window padding (inside the window, under no
//!   child node) resolves to the **root** handle — the correct `Node(root)`
//!   attribution, NOT the STEP-0585 "attribute any click to the root"
//!   (which had no layout and so could not distinguish root from child);
//! - **outside**: a click outside the client area is a [`Hit::Miss`] — no
//!   event is delivered and the host-side `skipped_click_count` increments.
//!
//! # Authority regression (④)
//!
//! With `UiGrants::default()` every call still fails closed as typed
//! `authority-denied` and the backend is never touched (zero-touch), so the
//! default-deny semantics from STEP-0581 do not regress.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
    ui::{UiError, UiEvent, UiGrants, UiRenderer, route_click_event},
};
use sico_ui_renderer_d2d::layout::compute as layout_compute;
use sico_ui_renderer_d2d::{UiBackendError, WindowedUiBackend};
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use wasm_encoder::{
    BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ComponentTypeRef,
    ComponentValType, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, InstanceType, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeBounds, TypeSection,
    ValType,
};

const TITLE_OFFSET: u32 = 1024;
const TITLE_LEN: u32 = 8; // "ui-click"
const LABEL_OFFSET: u32 = 2048;
const LABEL_LEN: u32 = 2; // "OK"
const RESULT_AREA: u32 = 4096;
const AREA_WINDOW: u32 = 8192;
const AREA_COLUMN: u32 = 10_240;
const AREA_BUTTON: u32 = 12_288;
const AREA_POLL: u32 = 14_336;
const EMPTY_LIST_PTR: u32 = 3072;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;
const TRANSPORT_INSTANCE: &str = "ui-transport";
const UI_INTERFACE: &str = "sico:script/ui@1.0.0";

/// Client size the guest requests (matches the `ui_guest_visible.rs` shape).
const CLIENT_WIDTH: u64 = 320;
const CLIENT_HEIGHT: u64 = 160;

/// Exit codes within the runner's accepted `0..=119` range. Guest A reports the
/// button handle (3) on success; error bases follow `ui_guest_visible.rs`.
/// Guest B reports the **click handle** it observed (1..), or a no-click /
/// unexpected-event code.
const EXIT_WINDOW_ERR_BASE: i64 = 110;
const EXIT_COLUMN_ERR_BASE: i64 = 90;
const EXIT_BUTTON_ERR_BASE: i64 = 80;
const EXIT_NO_CLICK: i64 = 107;
const EXIT_UNEXPECTED_EVENT: i64 = 108;
/// The guest's own `poll-event` must observe the click; `POLL_ROUNDS` empty
/// polls is the honest "no click delivered" bound.
const POLL_ROUNDS: i32 = 64;

/// The production backend with the host-side click-delivery affordance.
///
/// It wraps [`WindowedUiBackend`] — the SAME backend
/// [`D2dUiRenderer`](sico_runner::ui::D2dUiRenderer) wraps — and delegates
/// every tree op identically; it is not a new renderer and claims no extra
/// behaviour. `poll_event` routes a host-delivered coordinate click through the
/// production [`route_click_event`] against the real tree + live client size,
/// exactly as `D2dUiRenderer::poll_event` does, and counts a [`Hit::Miss`] the
/// same way. `calls` counts every operation that crosses the authority gate so
/// the zero-touch assertion (④) is observable.
struct ClickRoutingBackend {
    inner: WindowedUiBackend,
    pending: Mutex<VecDeque<(i32, i32)>>,
    skipped: AtomicU64,
    calls: AtomicU64,
}

impl ClickRoutingBackend {
    fn new() -> Result<Self, UiBackendError> {
        Ok(Self {
            inner: WindowedUiBackend::new()?,
            pending: Mutex::new(VecDeque::new()),
            skipped: AtomicU64::new(0),
            calls: AtomicU64::new(0),
        })
    }

    fn hit(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    /// Delivers a coordinate click into the routing seam (host-side).
    ///
    /// This is the honest "host delivers coordinates" mechanism: the shell's
    /// real `WM_LBUTTONDOWN` capture is bypassed, but the routing is the same
    /// production function the real renderer uses (see the module docs).
    fn inject_click(&self, x: i32, y: i32) {
        self.pending.lock().unwrap().push_back((x, y));
    }

    /// The number of routed clicks that were an explicit [`Hit::Miss`].
    fn skipped(&self) -> u64 {
        self.skipped.load(Ordering::SeqCst)
    }

    /// The current client size + tree snapshot (delegated, read-only).
    fn layout_input(
        &self,
    ) -> Result<(u32, u32, sico_ui_renderer_d2d::tree::UiTree), UiBackendError> {
        self.inner.layout_input()
    }

    /// The title of the open window, if a real window was created.
    fn window_title(&self) -> Option<String> {
        self.inner.window_title()
    }

    /// Destroys the window; idempotent.
    fn destroy(&self) -> Result<(), UiBackendError> {
        self.inner.destroy()
    }
}

/// Folds a renderer failure into the closed guest `ui-error` set, mirroring the
/// documented mapping (`ui.rs::fold_error`).
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

impl UiRenderer for ClickRoutingBackend {
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
    fn image_preview(&self, parent: u64, width: u64, height: u64) -> Result<u64, UiError> {
        self.hit();
        self.inner
            .image_preview(parent, width, height)
            .map_err(fold)
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
    fn draw_image(
        &self,
        handle: u64,
        width: u64,
        height: u64,
        pixels: &[u8],
    ) -> Result<bool, UiError> {
        self.hit();
        self.inner
            .draw_image(handle, width, height, pixels)
            .map_err(fold)
    }
    fn poll_event(&self) -> Option<UiEvent> {
        // Route the oldest host-delivered coordinate click through the real
        // production routing against the real tree + live client size. A node
        // hit delivers `click(handle)`; a `Hit::Miss` is counted as a skip; a
        // layout failure surfaces as `None` (never a fabricated event). This
        // mirrors `D2dUiRenderer::poll_event` exactly for the click path.
        while let Some((x, y)) = self.pending.lock().unwrap().pop_front() {
            let (width, height, tree) = self.inner.layout_input().ok()?;
            match route_click_event(x, y, &tree, width, height) {
                Ok(Some(event)) => return Some(event),
                Ok(None) => {
                    self.skipped.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => return None,
            }
        }
        None
    }
    fn run(&self) -> Result<bool, UiError> {
        self.hit();
        self.inner.run().map_err(fold)
    }
}

/// Encodes one fixture Component importing the `sico:script/ui@1.0.0` subset
/// (`window`, `column`, `button`, `poll-event`), lowering all four against the
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

    // The versioned UI host-provider instance (subset: `window`, `column`,
    // `button`, `poll-event`).
    let ui_ty = builder.type_instance(Some("ui1"), &ui_instance_type());
    let ui_instance = builder.import(UI_INTERFACE, ComponentTypeRef::Instance(ui_ty));

    // Transport core module: shared memory + a checked bump allocator and the
    // static title/label strings every canonical lift/lower reads.
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
    let column_function = builder.alias_export(ui_instance, "column", ComponentExportKind::Func);
    let column_lowered = builder.lower_func(
        Some("column"),
        column_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let button_function = builder.alias_export(ui_instance, "button", ComponentExportKind::Func);
    let button_lowered = builder.lower_func(
        Some("button"),
        button_function,
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
            ("column", ExportKind::Func, column_lowered),
            ("button", ExportKind::Func, button_lowered),
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

/// The Guest A Component (builds window > column > button; reports the button
/// handle).
fn fixture_a() -> Vec<u8> {
    component_with_guest(&guest_a_module())
}

/// The Guest B Component (polls `poll-event` for a `click(handle)`).
fn fixture_b() -> Vec<u8> {
    component_with_guest(&guest_b_module())
}

/// The component-level instance type of the `ui@1.0.0` subset (`window`,
/// `column`, `button`, `poll-event`). Index bookkeeping follows the http2
/// adapter's proven rules: `ty()` consumes one index, each `Eq` alias export
/// consumes one, `Func` exports consume none. `window`=Func(5),
/// `poll-event`=Func(11), `column`=Func(12), `button`=Func(13).
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

    // 12 func column (parent ui-handle) -> result<ui-handle, ui-error>.
    let mut column = types.ty().function();
    column.params([("parent", ComponentValType::Type(1))]);
    column.result(Some(ComponentValType::Type(4)));
    types.export("column", ComponentTypeRef::Func(12));

    // 13 func button (parent ui-handle, label string) -> result<ui-handle,
    // ui-error>.
    let mut button = types.ty().function();
    button.params(vec![
        ("parent", ComponentValType::Type(1)),
        (
            "label",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
    ]);
    button.result(Some(ComponentValType::Type(4)));
    types.export("button", ComponentTypeRef::Func(13));
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
/// title (`ui-click`) and label (`OK`) strings the guest passes.
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
        b"ui-click".to_vec(),
    );
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(LABEL_OFFSET).unwrap_or(0)),
        b"OK".to_vec(),
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
/// 2 = imported lowered `column`; 3 = imported lowered `button`;
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
    // 2: column — (parent i64, result area) -> ()
    types.ty().function([ValType::I64, ValType::I32], []);
    // 3: button — (parent i64, label ptr, label len, result area) -> ()
    types
        .ty()
        .function([ValType::I64, ValType::I32, ValType::I32, ValType::I32], []);
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
    imports.import(UI_INTERFACE, "column", EntityType::Function(2));
    imports.import(UI_INTERFACE, "button", EntityType::Function(3));
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

/// Guest A core module: `window` → `column` → `button`, reporting the button
/// handle through the exit code.
fn guest_a_module() -> Vec<u8> {
    guest_module(guest_a_body())
}

/// Guest B core module: `poll-event` loop until a `click(handle)` (or
/// `POLL_ROUNDS` empty polls).
fn guest_b_module() -> Vec<u8> {
    guest_module(guest_b_body())
}

/// Guest A `run` body. Locals: params 0..3 (input record), 4 = i32 tag/temp,
/// 5 = i64 exit. On the granted path the guest builds window(1) > column(2) >
/// button(3) and exits with the button handle (3).
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
        // ok: root = handle id (i64 @ AREA_WINDOW + 8)
        Instruction::I32Const(AREA_WINDOW as i32),
        Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(5),
        // column(root, AREA_COLUMN)
        Instruction::LocalGet(5),
        Instruction::I32Const(AREA_COLUMN as i32),
        Instruction::Call(2),
        Instruction::I32Const(AREA_COLUMN as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(&mut instructions, 5, AREA_COLUMN + 8, EXIT_COLUMN_ERR_BASE);
    for instruction in [
        Instruction::End,
        // ok: column handle (i64 @ AREA_COLUMN + 8)
        Instruction::I32Const(AREA_COLUMN as i32),
        Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(5),
        // button(column, "OK", AREA_BUTTON)
        Instruction::LocalGet(5),
        Instruction::I32Const(LABEL_OFFSET as i32),
        Instruction::I32Const(LABEL_LEN as i32),
        Instruction::I32Const(AREA_BUTTON as i32),
        Instruction::Call(3),
        Instruction::I32Const(AREA_BUTTON as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(&mut instructions, 5, AREA_BUTTON + 8, EXIT_BUTTON_ERR_BASE);
    // ok: exit = button handle (i64 @ AREA_BUTTON + 8).
    for instruction in [
        Instruction::End,
        Instruction::I32Const(AREA_BUTTON as i32),
        Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(5),
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

/// Guest B `run` body: poll until a `click(handle)` (option disc 1 + variant
/// disc 0) or `POLL_ROUNDS` empty polls. Locals: params 0..3 (input record),
/// 4 = i32 counter, 5 = i64 exit (0 = not yet, h = click handle, 107 = no
/// click, 108 = select/closed).
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
        // if variant disc == 0 (click)
        Instruction::I32Const((AREA_POLL + 8) as i32),
        load8(0),
        Instruction::I32Const(0),
        Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        // click: exit = handle id (i64 @ AREA_POLL + 16)
        Instruction::I32Const((AREA_POLL + 16) as i32),
        Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(5),
        Instruction::Else,
        // select / closed: unexpected
        Instruction::I64Const(EXIT_UNEXPECTED_EVENT),
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
        // if exit == 0: exit = 107 (no click in POLL_ROUNDS)
        Instruction::LocalGet(5),
        Instruction::I64Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::I64Const(EXIT_NO_CLICK),
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
fn run_guest(component: &[u8], backend: &Arc<ClickRoutingBackend>, deny: bool) -> RunOutcome {
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

/// The real Direct2D + window-shell backend click-routing evidence (G4). These
/// run the hand-encoded guest Components through the real runner against the
/// production [`WindowedUiBackend`] backend (via [`ClickRoutingBackend`]).
#[cfg(windows)]
mod real_backend {
    use super::*;

    fn real_backend() -> Arc<ClickRoutingBackend> {
        Arc::new(ClickRoutingBackend::new().expect("the production backend builds on Windows"))
    }

    /// Runs guest A (window > column > button) and returns the observed exit
    /// code; on a headless host the window is refused as a typed ui-error.
    fn build_tree(backend: &Arc<ClickRoutingBackend>) -> i64 {
        let (_, exit) = exit_of(run_guest(&fixture_a(), backend, false));
        if backend.window_title().is_none() {
            assert!(
                (111..=115).contains(&exit),
                "expected a typed ui-error refusal on headless, got exit {exit}"
            );
            eprintln!(
                "ENV_LIMIT: no interactive desktop; click routing is honestly skipped (typed refusal above)"
            );
            return -1;
        }
        assert_eq!(exit, 3, "guest A built window+column+button (handle 3)");
        3
    }

    #[test]
    fn guest_observes_click_on_the_button_child() {
        let backend = real_backend();
        if build_tree(&backend) != 3 {
            return;
        }
        // Host computes the button centre from the real layout and delivers the
        // coordinate click; the routing (the part this slice adds) is the same
        // production function the real renderer uses.
        let (w, h, tree) = backend.layout_input().expect("layout input");
        let layout = layout_compute(&tree, w, h).expect("layout");
        let (cx, cy) = layout.rect(3).expect("button rect").center();
        backend.inject_click(cx as i32, cy as i32);

        // Guest B polls: must observe click(button handle 3).
        let (_, exit_b) = exit_of(run_guest(&fixture_b(), &backend, false));
        assert_eq!(exit_b, 3, "the guest observed click(button handle 3)");
        assert_eq!(backend.skipped(), 0, "a node hit is not a skipped click");
        let _ = backend.destroy();
    }

    #[test]
    fn guest_observes_click_on_the_root_when_no_child_covers_the_point() {
        let backend = real_backend();
        if build_tree(&backend) != 3 {
            return;
        }
        // A point in the window padding (8,8) — inside the window (320x160),
        // under no child node — resolves to the ROOT (handle 1). This is the
        // correct `Node(root)` attribution (STEP-0592), NOT the STEP-0585
        // "attribute any click to the root".
        backend.inject_click(8, 8);
        let (_, exit_b) = exit_of(run_guest(&fixture_b(), &backend, false));
        assert_eq!(exit_b, 1, "the guest observed click(root handle 1)");
        assert_eq!(backend.skipped(), 0, "a root hit is not a skipped click");
        let _ = backend.destroy();
    }

    #[test]
    fn a_click_outside_the_window_is_a_skip_not_a_delivery() {
        let backend = real_backend();
        if build_tree(&backend) != 3 {
            return;
        }
        // (400, 400) is outside the 320x160 client area: an explicit
        // `Hit::Miss` — no event delivered, the host-side skipped count
        // increments (the STEP-0585 observable channel).
        backend.inject_click(400, 400);
        let (_, exit_b) = exit_of(run_guest(&fixture_b(), &backend, false));
        assert_eq!(
            exit_b, EXIT_NO_CLICK,
            "no click(handle) is delivered for an outside point"
        );
        assert_eq!(
            backend.skipped(),
            1,
            "the miss is counted as a skipped click, not delivered"
        );
        let _ = backend.destroy();
    }

    #[test]
    fn denied_guest_is_typed_authority_refusal_with_zero_backend_touch() {
        // ④: the default-deny gate fires before the backend; the guest sees the
        // typed `authority-denied` (discriminant 5 → exit 115) and the
        // production backend is never touched (zero-touch), so the STEP-0581
        // semantics do not regress.
        let backend = real_backend();
        let (_, exit) = exit_of(run_guest(&fixture_a(), &backend, true));
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
