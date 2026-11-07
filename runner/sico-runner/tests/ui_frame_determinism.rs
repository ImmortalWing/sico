//! Guest-level **frame determinism** evidence (M24 §8.6.1 acceptance clause
//! G5, STEP-0590 / STEP-0591).
//!
//! G5 (STEP-0590): *guest 同树渲染 N 次 → N 帧逐字节一致*. STEP-0591 already
//! delivered the guest-level G2/G3 evidence and explicitly recorded that its
//! cross-instance byte identity (`read_presented_frame() == offscreen oracle`)
//! is *incidental* and does **not** constitute a formal G5. This file is that
//! formal G5专测.
//!
//! Design (all against the production backend [`WindowedUiBackend`], the same
//! object [`D2dUiRenderer`] wraps):
//!
//! - The guest tree is **exactly** the frozen-corpus `text` scene
//!   (`window("Text",320,160) → column → text("Sico") →
//!   text("Fluent-subset renderer")`, `crates/sico-ui-renderer-d2d/tests/frame_corpus.rs`).
//!   Because the present path is a pure byte copy (STEP-0577) and the renderer
//!   is a pure function of the tree (ADR-0018 §3), every guest-triggered frame
//!   must equal the **frozen** corpus digest `0bd324c8…` — anchoring the
//!   guest-level claim to the authoritative STEP-0338 evidence, not only to
//!   N-frame internal consistency.
//! - Boundary ① (cross-Store): `N = 6` fully independent
//!   (Runner, `PreparedProgram::run` Store, [`WindowedUiBackend`], OS window)
//!   generations each render the same guest tree once and read the presented
//!   frame back; all six frames must be byte- and digest-identical.
//! - Boundary ② (cross-process): the same tree is rendered in a **separate
//!   process** (the test binary re-executed as a child, `--exact
//!   emit_offscreen_text_digest`, mirroring the STEP-0338 `frame_digest`
//!   example pattern) and its digest must equal the in-process guest-readback
//!   digest and the frozen corpus digest.
//! - Supplementary: one backend presents the *same tree* `PRESENT_ROUNDS + 1`
//!   times (the guest's `run()` then host-driven `run()` + read-back each
//!   time); every frame byte-identical — the literal "same tree, repeated
//!   present" reading.
//!
//! Determinism argument (why every input that could change a frame byte is
//! fixed or excluded): the tree (nodes, content, client size), the frozen
//! token table, the frozen type ramp, the fixed client size and the
//! DirectWrite rasteriser of this host are the only inputs; there is no timer,
//! no animation, no random source, no environment-derived value (theme, DPI
//! scale, locale, accent) and no thread/process dependency in the renderer
//! core. The present path is a byte copy into the shell DIB (STEP-0577), so
//! the read-back is exactly the offscreen frame.
//!
//! Fail-closed / environment honesty (same as STEP-0591): a real OS window
//! needs an interactive desktop. On a headless host the guest's `window()`
//! returns a typed `ui-error` and these tests degrade to asserting that typed
//! refusal plus an explicit `ENV_LIMIT` marker — never a fabricated frame. On
//! this evidence host the window is real and the full path runs.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
    ui::{UiError, UiEvent, UiGrants, UiRenderer},
};
use sico_ui_renderer_d2d::events::WindowEvent;
use sico_ui_renderer_d2d::{D2dUiBackend, Frame, UiBackendError, WindowedUiBackend};
use std::sync::Arc;
use wasm_encoder::{
    BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ComponentTypeRef,
    ComponentValType, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, InstanceType, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeBounds, TypeSection,
    ValType,
};

/// The guest tree is the frozen-corpus `text` scene. These constants MUST match
/// `crates/sico-ui-renderer-d2d/tests/frame_corpus.rs::scene_text` exactly;
/// the digest below is the frozen STEP-0338 value for that scene.
const TITLE_OFFSET: u32 = 1024;
const TITLE_LEN: u32 = 4; // "Text"
const TEXT1_OFFSET: u32 = 2048;
const TEXT1_LEN: u32 = 4; // "Sico"
const TEXT2_OFFSET: u32 = 3072;
const TEXT2_LEN: u32 = 22; // "Fluent-subset renderer"
const EMPTY_LIST_PTR: u32 = 4096;
const RESULT_AREA: u32 = 8192;
const AREA_WINDOW: u32 = 16_384;
const AREA_COLUMN: u32 = 17_408;
const AREA_TEXT1: u32 = 18_432;
const AREA_TEXT2: u32 = 19_456;
const AREA_RUN: u32 = 20_480;
const AREA_POLL: u32 = 21_504;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;
const TRANSPORT_INSTANCE: &str = "ui-transport";
const UI_INTERFACE: &str = "sico:script/ui@1.0.0";

/// The frozen corpus `text` scene client size.
const CLIENT_WIDTH: u64 = 320;
const CLIENT_HEIGHT: u64 = 160;
/// The frozen STEP-0338 digest of the corpus `text` scene
/// (`frame_corpus.rs::CORPUS[1]`, `docs/evidence/m24/ui-frame-corpus.json`).
const FROZEN_TEXT_DIGEST: &str = "0bd324c87c0f4e41c3f61d62d7a10bd8f3b2bc15b7443a791b234673ef592868";

/// Boundary ①: how many independent (Store, backend, OS window) generations
/// render the same guest tree. N ≥ 5 per the G5 clause; 6 gives margin.
const N_RUNS: usize = 6;
/// Supplementary same-backend repeated-present count (host-driven `run()`s
/// after the guest's own first present).
const PRESENT_ROUNDS: usize = 5;

/// Error-exit bases the guest reports through `exit-code` (runner-accepted
/// `0..=119`). `window` err → `110 + disc`; `column` err → `100 + disc`;
/// `text` err → `80 + disc`; `run` err → `90 + disc`; unexpected `poll-event`
/// Some → `70 + disc`. Success → `1`.
const EXIT_WINDOW_ERR_BASE: i64 = 110;
const EXIT_COLUMN_ERR_BASE: i64 = 100;
const EXIT_RUN_ERR_BASE: i64 = 90;
const EXIT_TEXT_ERR_BASE: i64 = 80;
const EXIT_POLL_SOME_BASE: i64 = 70;
const EXIT_SUCCESS: i64 = 1;

/// The production backend with the host-side readback affordance the G5 byte
/// check needs. It wraps [`WindowedUiBackend`] — the SAME backend
/// [`D2dUiRenderer`] wraps — and delegates every `ui@1` operation identically;
/// it is not a new renderer and claims no extra behaviour.
struct ReadbackBackend {
    inner: WindowedUiBackend,
}

impl ReadbackBackend {
    fn new() -> Result<Self, UiBackendError> {
        Ok(Self {
            inner: WindowedUiBackend::new()?,
        })
    }

    /// The title of the open window, if a real window was created.
    fn window_title(&self) -> Option<String> {
        self.inner.window_title()
    }

    /// Reads back the last presented frame from the shell's DIB (byte copy).
    fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        self.inner.read_presented_frame()
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
        self.inner.window(title, width, height).map_err(fold)
    }
    fn column(&self, parent: u64) -> Result<u64, UiError> {
        self.inner.column(parent).map_err(fold)
    }
    fn row(&self, parent: u64) -> Result<u64, UiError> {
        self.inner.row(parent).map_err(fold)
    }
    fn text(&self, parent: u64, content: &str) -> Result<u64, UiError> {
        self.inner.text(parent, content).map_err(fold)
    }
    fn button(&self, parent: u64, label: &str) -> Result<u64, UiError> {
        self.inner.button(parent, label).map_err(fold)
    }
    fn dropdown(&self, parent: u64, options: &[String]) -> Result<u64, UiError> {
        self.inner.dropdown(parent, options).map_err(fold)
    }
    fn image_preview(&self, parent: u64, w: u64, h: u64) -> Result<u64, UiError> {
        self.inner.image_preview(parent, w, h).map_err(fold)
    }
    fn progress(&self, parent: u64) -> Result<u64, UiError> {
        self.inner.progress(parent).map_err(fold)
    }
    fn set_text(&self, handle: u64, content: &str) -> Result<bool, UiError> {
        self.inner.set_text(handle, content).map_err(fold)
    }
    fn set_progress(&self, handle: u64, done: u64, of: u64) -> Result<bool, UiError> {
        self.inner.set_progress(handle, done, of).map_err(fold)
    }
    fn draw_image(&self, handle: u64, w: u64, h: u64, pixels: &[u8]) -> Result<bool, UiError> {
        self.inner.draw_image(handle, w, h, pixels).map_err(fold)
    }
    fn poll_event(&self) -> Option<UiEvent> {
        // Pump until a deliverable `ui@1` event surfaces, exactly as
        // `D2dUiRenderer::poll_event` does.
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
        self.inner.run().map_err(fold)
    }
}

/// Encodes one fixture Component importing the `sico:script/ui@1.0.0` subset
/// (`window`, `column`, `text`, `run`, `poll-event`), lowering all five
/// against the shared transport and wiring `guest` as the guest core module.
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

    let ui_ty = builder.type_instance(Some("ui1"), &ui_instance_type());
    let ui_instance = builder.import(UI_INTERFACE, ComponentTypeRef::Instance(ui_ty));

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

    let lowered_options = [
        wasm_encoder::CanonicalOption::UTF8,
        wasm_encoder::CanonicalOption::Memory(transport_memory),
        wasm_encoder::CanonicalOption::Realloc(transport_realloc),
    ];
    let mut lowered_names = Vec::with_capacity(5);
    for (lowered_name, export_name) in [
        ("window", "window"),
        ("column", "column"),
        ("text", "text"),
        ("run", "run"),
        ("poll-event", "poll-event"),
    ] {
        let function = builder.alias_export(ui_instance, export_name, ComponentExportKind::Func);
        let lowered = builder.lower_func(Some(lowered_name), function, lowered_options);
        lowered_names.push((lowered_name, ExportKind::Func, lowered));
    }
    let lowered_instance = builder.core_instantiate_exports(Some("ui1-lowered"), lowered_names);

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

/// The component-level instance type of the `ui@1.0.0` subset (`window`,
/// `column`, `text`, `run`, `poll-event`). Index bookkeeping follows the http2
/// adapter's proven rules: `ty()` consumes one index, each `Eq` alias export
/// consumes one, `Func` exports consume none. `window`=Func(5),
/// `poll-event`=Func(11), `run`=Func(13), `text`=Func(14), `column`=Func(15).
fn ui_instance_type() -> InstanceType {
    let mut types = InstanceType::new();
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

    types.ty().defined_type().result(
        Some(ComponentValType::Primitive(PrimitiveValType::Bool)),
        Some(ComponentValType::Type(3)),
    );
    let mut run = types.ty().function();
    run.params(Vec::<(&str, ComponentValType)>::new());
    run.result(Some(ComponentValType::Type(12)));
    types.export("run", ComponentTypeRef::Func(13));

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

    let mut column = types.ty().function();
    column.params(vec![("parent", ComponentValType::Type(1))]);
    column.result(Some(ComponentValType::Type(4)));
    types.export("column", ComponentTypeRef::Func(15));
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

/// Shared transport: one memory + a checked bump allocator and the three static
/// strings the guest passes (the corpus `text` scene: `Text`, `Sico`,
/// `Fluent-subset renderer`).
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
        b"Text".to_vec(),
    );
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(TEXT1_OFFSET).unwrap_or(0)),
        b"Sico".to_vec(),
    );
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(TEXT2_OFFSET).unwrap_or(0)),
        b"Fluent-subset renderer".to_vec(),
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

/// Writes a zero-tag (ok) script-output with empty stdout/stderr and
/// `exit = exit_local`, then returns the area pointer. Area layout: tag byte
/// @0; stdout @8/12; stderr @16/20; exit-code i64 @24; 32 bytes total.
fn write_output_return(instructions: &mut Vec<Instruction<'static>>, exit_local: u32) {
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
        Instruction::Return,
    ] {
        instructions.push(instruction);
    }
}

/// Emits a zero-tag (ok) script-output with empty stdout/stderr and
/// `exit = base + <ui-error discriminant>` read from `disc_area_offset`, then
/// returns the area pointer.
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
    ] {
        instructions.push(instruction);
    }
    write_output_return(instructions, exit_local);
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

/// The guest core-module function index space: 0 = imported realloc;
/// 1 = imported `window`; 2 = imported `column`; 3 = imported `text`;
/// 4 = imported `run`; 5 = imported `poll-event`; 6 = guest `run`;
/// 7 = no-op post-return.
fn guest_module(run_body: Function) -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // window: (title ptr, title len, width i64, height i64, result area) -> ()
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
    // column: (parent i64, result area) -> ()
    types.ty().function([ValType::I64, ValType::I32], []);
    // text: (parent i64, content ptr, content len, result area) -> ()
    types
        .ty()
        .function([ValType::I64, ValType::I32, ValType::I32, ValType::I32], []);
    // run: (result area) -> ()
    types.ty().function([ValType::I32], []);
    // poll-event: (result area) -> ()
    types.ty().function([ValType::I32], []);
    // guest run: (input record) -> (result area ptr)
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // post-run
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
    imports.import(UI_INTERFACE, "text", EntityType::Function(3));
    imports.import(UI_INTERFACE, "run", EntityType::Function(4));
    imports.import(UI_INTERFACE, "poll-event", EntityType::Function(5));

    let mut function_section = FunctionSection::new();
    function_section.function(6);
    function_section.function(7);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, 6);
    exports.export("cabi_post_run", ExportKind::Func, 7);

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

/// The fixture Component (imports `window`, `column`, `text`, `run`,
/// `poll-event`; builds the frozen corpus `text` scene).
fn fixture() -> Vec<u8> {
    component_with_guest(&guest_module(guest_body()))
}

/// Guest `run` body. Locals: params 0..3 (input record), 4 = i32 tag/temp,
/// 5 = i64 exit, 6 = i64 current handle. On the granted path the guest builds
/// the corpus `text` scene (window → column → text → text), presents once and
/// observes a `none` poll — exit `EXIT_SUCCESS`.
fn guest_body() -> Function {
    let mut body = Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I64),
        (1, ValType::I64),
    ]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // window("Text", 320, 160, AREA_WINDOW)
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
    push_error_exit(&mut instructions, 5, AREA_WINDOW + 8, EXIT_WINDOW_ERR_BASE);
    for instruction in [
        Instruction::End,
        // ok: handle = i64 @ AREA_WINDOW + 8
        Instruction::I32Const(AREA_WINDOW as i32),
        Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(6),
        // column(handle, AREA_COLUMN)
        Instruction::LocalGet(6),
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
        Instruction::I32Const(AREA_COLUMN as i32),
        Instruction::I64Load(MemArg {
            offset: 8,
            align: 3,
            memory_index: 0,
        }),
        Instruction::LocalSet(6),
        // text(handle, "Sico", AREA_TEXT1)
        Instruction::LocalGet(6),
        Instruction::I32Const(TEXT1_OFFSET as i32),
        Instruction::I32Const(TEXT1_LEN as i32),
        Instruction::I32Const(AREA_TEXT1 as i32),
        Instruction::Call(3),
        Instruction::I32Const(AREA_TEXT1 as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(&mut instructions, 5, AREA_TEXT1 + 8, EXIT_TEXT_ERR_BASE);
    for instruction in [
        Instruction::End,
        // local 6 stays the COLUMN handle (not the first text handle) so the
        // second text call uses the same container.
        // text(handle, "Fluent-subset renderer", AREA_TEXT2)
        Instruction::LocalGet(6),
        Instruction::I32Const(TEXT2_OFFSET as i32),
        Instruction::I32Const(TEXT2_LEN as i32),
        Instruction::I32Const(AREA_TEXT2 as i32),
        Instruction::Call(3),
        Instruction::I32Const(AREA_TEXT2 as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(&mut instructions, 5, AREA_TEXT2 + 8, EXIT_TEXT_ERR_BASE);
    for instruction in [
        Instruction::End,
        // run(AREA_RUN)
        Instruction::I32Const(AREA_RUN as i32),
        Instruction::Call(4),
        // run() -> result<bool, ui-error>: tag @0, payload @4.
        Instruction::I32Const(AREA_RUN as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(&mut instructions, 5, AREA_RUN + 4, EXIT_RUN_ERR_BASE);
    for instruction in [
        Instruction::End,
        // poll-event(AREA_POLL)
        Instruction::I32Const(AREA_POLL as i32),
        Instruction::Call(5),
        // option disc @ AREA_POLL + 0 (0 = none, 1 = some)
        Instruction::I32Const(AREA_POLL as i32),
        load8(0),
        Instruction::LocalSet(4),
        Instruction::LocalGet(4),
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        // none: exit = success
        Instruction::I64Const(EXIT_SUCCESS),
        Instruction::LocalSet(5),
        Instruction::Else,
        // some (unexpected): exit = 70 + variant disc @ AREA_POLL + 8
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

fn exit_of(outcome: RunOutcome) -> (Vec<u8>, i64) {
    match outcome {
        RunOutcome::Output(output) => (output.stdout, output.exit_code),
        other => panic!("expected guest output, got {other:?}"),
    }
}

/// Runs the guest Component once against `backend` (granted) and returns the
/// outcome. A fresh [`Runner`] and a fresh `PreparedProgram::run` Store are
/// created for each call, so every call is an independent generation.
fn run_guest(component: &[u8], backend: &Arc<ReadbackBackend>) -> RunOutcome {
    let grants = UiGrants { window: true };
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

/// A fresh production backend (real OS window + shell thread).
fn real_backend() -> Arc<ReadbackBackend> {
    Arc::new(ReadbackBackend::new().expect("the production backend builds on Windows"))
}

/// The independent offscreen frame of the frozen corpus `text` scene, built
/// through a fresh [`D2dUiBackend`] (no OS window). Because the renderer is a
/// pure function of the tree (ADR-0018 §3) and the present path is a byte copy
/// (STEP-0577), this frame must equal every guest-triggered presented frame.
#[cfg(windows)]
fn offscreen_text_frame() -> Frame {
    let oracle = D2dUiBackend::new().expect("offscreen renderer builds on Windows");
    let root = oracle
        .window("Text", CLIENT_WIDTH as i64, CLIENT_HEIGHT as i64)
        .expect("oracle window root");
    let column = oracle.column(root).expect("oracle column");
    oracle.text(column, "Sico").expect("oracle text 1");
    oracle
        .text(column, "Fluent-subset renderer")
        .expect("oracle text 2");
    oracle.render_frame().expect("oracle frame")
}

/// Runs the guest once against a fresh backend, reads the presented frame back
/// and returns the exit code and frame. Returns `None` when the host has no
/// interactive desktop and the guest got a typed `ui-error`.
#[cfg(windows)]
fn guest_present_once() -> Option<(i64, Frame)> {
    let backend = real_backend();
    let (stdout, exit) = exit_of(run_guest(&fixture(), &backend));
    assert!(stdout.is_empty());
    if backend.window_title().is_none() {
        assert!(
            (111..=115).contains(&exit),
            "expected a typed ui-error refusal on headless, got exit {exit}"
        );
        return None;
    }
    assert_eq!(exit, EXIT_SUCCESS, "the guest built the tree and presented");
    let frame = backend
        .read_presented_frame()
        .expect("a presented frame is readable back");
    assert_eq!(
        (frame.width(), frame.height()),
        (CLIENT_WIDTH as u32, CLIENT_HEIGHT as u32),
        "the presented frame has the requested client size"
    );
    Some((exit, frame))
}

#[cfg(windows)]
mod real_backend {
    use super::*;

    /// Boundary ① + the frozen-corpus anchor: the same guest tree is rendered
    /// `N_RUNS` times across fully independent (Runner, Store, backend, OS
    /// window) generations; every presented frame is byte- and digest-identical
    /// to every other and to the frozen STEP-0338 corpus digest.
    #[test]
    fn same_guest_tree_renders_n_byte_identical_frames() {
        let mut frames = Vec::with_capacity(N_RUNS);
        for index in 0..N_RUNS {
            match guest_present_once() {
                Some((exit, frame)) => {
                    assert_eq!(exit, EXIT_SUCCESS);
                    frames.push(frame);
                }
                None => {
                    assert!(
                        frames.is_empty(),
                        "inconsistent host: generation {index} lost the interactive desktop"
                    );
                    eprintln!(
                        "ENV_LIMIT: no interactive desktop; the guest refused window creation as a typed ui-error"
                    );
                    return;
                }
            }
        }
        assert_eq!(frames.len(), N_RUNS, "all generations presented a frame");

        let first = &frames[0];
        for (index, frame) in frames.iter().enumerate() {
            assert_eq!(frame.sha256(), first.sha256(), "frame {index} digest");
            assert_eq!(frame.pixels(), first.pixels(), "frame {index} bytes");
        }

        // The frozen corpus anchor: every guest-triggered frame equals the
        // authoritative STEP-0338 digest for the same tree.
        assert_eq!(
            first.sha256(),
            FROZEN_TEXT_DIGEST,
            "the guest-triggered frame must match the frozen corpus digest"
        );

        // In-process offscreen oracle cross-check.
        let oracle = offscreen_text_frame();
        assert_eq!(
            first.pixels(),
            oracle.pixels(),
            "the guest-triggered frame equals the independent offscreen frame"
        );
        assert_eq!(oracle.sha256(), FROZEN_TEXT_DIGEST, "oracle digest");
    }

    /// Boundary ② (cross-process): the same tree rendered in a **separate
    /// process** (this test binary re-executed as a child) produces the same
    /// digest as the in-process guest-readback frame and the frozen corpus.
    #[test]
    fn guest_frame_digest_matches_across_processes() {
        let Some((_, frame)) = guest_present_once() else {
            eprintln!("ENV_LIMIT: no interactive desktop; the cross-process check is skipped");
            return;
        };
        let in_process = frame.sha256();

        let child = std::process::Command::new(std::env::current_exe().expect("current exe"))
            .arg("--exact")
            .arg("emit_offscreen_text_digest")
            .arg("--nocapture")
            .env("SICO_G5_DIGEST_CHILD", "1")
            .output()
            .expect("spawn the child process");
        assert!(
            child.status.success(),
            "child process must exit successfully (stderr: {})",
            String::from_utf8_lossy(&child.stderr)
        );
        let stdout = String::from_utf8_lossy(&child.stdout);
        let child_digest = stdout
            .lines()
            .find_map(|line| line.strip_prefix("G5_DIGEST "))
            .expect("the child prints its digest")
            .trim()
            .to_owned();
        assert_eq!(
            child_digest, in_process,
            "the cross-process digest must equal the in-process guest-readback digest"
        );
        assert_eq!(
            child_digest, FROZEN_TEXT_DIGEST,
            "the cross-process digest must equal the frozen corpus digest"
        );
    }

    /// Supplementary: one backend presents the *same tree* `PRESENT_ROUNDS + 1`
    /// times (the guest's `run()` then `PRESENT_ROUNDS` host-driven `run()`s,
    /// reading back after each); every frame is byte-identical and equals the
    /// frozen corpus digest — the literal "same tree, repeated present".
    #[test]
    fn same_tree_presented_repeatedly_on_one_backend_is_byte_identical() {
        let backend = real_backend();
        let (stdout, exit) = exit_of(run_guest(&fixture(), &backend));
        assert!(stdout.is_empty());
        if backend.window_title().is_none() {
            assert!(
                (111..=115).contains(&exit),
                "expected a typed ui-error refusal on headless, got exit {exit}"
            );
            eprintln!("ENV_LIMIT: no interactive desktop; the repeated-present check is skipped");
            return;
        }
        assert_eq!(exit, EXIT_SUCCESS, "the guest built the tree and presented");

        let first_digest = backend
            .read_presented_frame()
            .expect("read back the guest's first present")
            .sha256();
        assert_eq!(
            first_digest, FROZEN_TEXT_DIGEST,
            "the guest-triggered frame must match the frozen corpus digest"
        );
        let first_pixels = backend
            .read_presented_frame()
            .expect("read back the guest's first present")
            .pixels()
            .to_vec();
        for round in 0..PRESENT_ROUNDS {
            backend.inner.run().expect("present");
            let frame = backend.read_presented_frame().expect("read back");
            assert_eq!(frame.sha256(), first_digest, "round {round} digest");
            assert_eq!(frame.pixels(), first_pixels, "round {round} bytes");
        }
    }
}

/// Child-process digest emitter for boundary ② (mirrors the STEP-0338
/// `frame_digest` example pattern). Only acts when spawned by
/// [`guest_frame_digest_matches_across_processes`]; a no-op otherwise.
#[cfg(windows)]
#[test]
fn emit_offscreen_text_digest() {
    if std::env::var("SICO_G5_DIGEST_CHILD").as_deref() != Ok("1") {
        return;
    }
    let frame = offscreen_text_frame();
    println!("G5_DIGEST {}", frame.sha256());
}

#[cfg(not(windows))]
#[test]
fn windowed_backend_refuses_typed_off_windows() {
    let error = WindowedUiBackend::new().expect_err("the backend must refuse off Windows");
    assert_eq!(error, UiBackendError::UnsupportedPlatform);
}
