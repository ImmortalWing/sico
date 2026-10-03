//! Guest-visible `sico:script/ui@1.0.0` authority fixtures (M24 #5, RFC-0052
//! §2/§3 as re-homed by Amendment A1, paired with ADR-0018).
//!
//! The compiler frontend does not emit `ui@1` imports (M14 baseline), so these
//! tests encode a real guest Component with `wasm-encoder` and run it through
//! the runner against a deterministic [`UiRenderer`] test double. The double is
//! **not** a renderer and claims no frame behaviour (the granted render/frame
//! path stays gated on #4, the isolated Direct2D adapter); its only job is to
//! prove the binding reaches the backend exactly when the UI capability is
//! granted, and never when it is denied.
//!
//! Two authority-gated paths are exercised, each a distinct closure in `ui.rs`:
//! the shared `guarded` funcs (via `window`) and the one func that does NOT
//! route through it (`poll-event`). The guest reports through the Script
//! output. For `window`: `exit = 110 + <ui-error discriminant>` on a typed
//! refusal (authority-denied = 5 → exit 115), and `exit = <handle id>` on the
//! granted positive control. For `poll-event`: `exit = 100 + <option
//! discriminant>`, so a denied poll yields the typed `none` (exit 100) with the
//! renderer queue untouched, and a granted poll yields a delivered `Some` event
//! (exit 101). `stdout` stays empty in both.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
    ui::{UiGrants, UiRenderer},
};
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
const TITLE_LEN: u32 = 10;
const RESULT_AREA: u32 = 4096;
const AREA_WINDOW: u32 = 8192;
const AREA_POLL: u32 = 12_288;
const EMPTY_LIST_PTR: u32 = 3072;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;
const TRANSPORT_INSTANCE: &str = "ui-transport";
const UI_INTERFACE: &str = "sico:script/ui@1.0.0";

/// A deterministic backend double: it counts every call that reaches it and
/// returns fixed handles, proving the binding gates on authority before any
/// renderer effect. It exercises the binding/authority wiring only and claims
/// no real window, frame or event support.
struct FixtureRenderer {
    calls: AtomicUsize,
}

impl FixtureRenderer {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl UiRenderer for FixtureRenderer {
    fn window(
        &self,
        _title: &str,
        _width: i64,
        _height: i64,
    ) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // A fixed handle id of 1 so the granted path yields `exit = 1`.
        Ok(1)
    }
    fn column(&self, _parent: u64) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(2)
    }
    fn row(&self, _parent: u64) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(3)
    }
    fn text(&self, _parent: u64, _content: &str) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(4)
    }
    fn button(&self, _parent: u64, _label: &str) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(5)
    }
    fn dropdown(&self, _parent: u64, _options: &[String]) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(6)
    }
    fn image_preview(
        &self,
        _parent: u64,
        _w: u64,
        _h: u64,
    ) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(7)
    }
    fn progress(&self, _parent: u64) -> Result<u64, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(8)
    }
    fn set_text(&self, _handle: u64, _content: &str) -> Result<bool, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
    fn set_progress(
        &self,
        _handle: u64,
        _done: u64,
        _of: u64,
    ) -> Result<bool, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
    fn draw_image(
        &self,
        _handle: u64,
        _w: u64,
        _h: u64,
        _pixels: &[u8],
    ) -> Result<bool, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
    fn poll_event(&self) -> Option<sico_runner::ui::UiEvent> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Some(sico_runner::ui::UiEvent::Closed)
    }
    fn run(&self) -> Result<bool, sico_runner::ui::UiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}

/// Encodes one fixture Component importing the `sico:script/ui@1.0.0` subset
/// (`window` and `poll-event`), lowering both against the shared transport and
/// wiring `guest` as the guest core module (which imports the subset it uses).
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

    // The versioned UI host-provider instance (subset: only `window`).
    let ui_ty = builder.type_instance(Some("ui1"), &ui_instance_type());
    let ui_instance = builder.import(UI_INTERFACE, ComponentTypeRef::Instance(ui_ty));

    // Transport core module: shared memory + a checked bump allocator.
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

    // Lower `window` and `poll-event` against the transport memory.
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

/// The `window` fixture Component (imports and calls only `window`).
fn fixture_component() -> Vec<u8> {
    component_with_guest(&guest_module())
}

/// The `poll-event` fixture Component (imports and calls only `poll-event`).
fn poll_event_component() -> Vec<u8> {
    component_with_guest(&poll_guest_module())
}

/// The component-level instance type of the `ui@1.0.0` subset (`window` +
/// `poll-event`, width-subtyped over the frozen 13-func surface). Index
/// bookkeeping follows the http2 adapter's proven rules: `ty()` consumes one
/// index, each `Eq` alias export consumes one, `Func` exports consume none.
/// `window` is `Func(5)`, `poll-event` is `Func(11)`.
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
    let mut function = types.ty().function();
    function.params(vec![
        (
            "title",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
        ("width", ComponentValType::Primitive(PrimitiveValType::S64)),
        ("height", ComponentValType::Primitive(PrimitiveValType::S64)),
    ]);
    function.result(Some(ComponentValType::Type(4)));
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
/// title string every canonical lift/lower reads.
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
/// `exit = 110 + discriminant` read from `disc_area_offset`, then returns the
/// area pointer. Area layout: tag byte @0; stdout @8/12; stderr @16/20;
/// exit-code i64 @24; 32 bytes total.
fn push_error_exit(
    instructions: &mut Vec<Instruction<'static>>,
    exit_local: u32,
    disc_area_offset: u32,
) {
    for instruction in [
        Instruction::I32Const(disc_area_offset as i32),
        load8(0),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(110),
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

/// Builds the guest core module. Function index space: 0 = imported realloc,
/// 1 = imported lowered `window`, then `run`, then the no-op post-return.
fn guest_module() -> Vec<u8> {
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
    // 2: run
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 3: post-run
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

    let mut function_section = FunctionSection::new();
    function_section.function(2);
    function_section.function(3);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, 2);
    exports.export("cabi_post_run", ExportKind::Func, 3);

    let mut code = CodeSection::new();
    code.function(&run_body());
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

/// Guest `run` body: call `window`, branch on the result tag, and emit either
/// `exit = 110 + discriminant` (denied) or `exit = handle id` (granted).
/// Locals: params 0..3 (input), 4 = tag i32, 5 = exit i64.
fn run_body() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32), (1, ValType::I64)]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // window(title_ptr, title_len, 640, 480, AREA_WINDOW)
        Instruction::I32Const(TITLE_OFFSET as i32),
        Instruction::I32Const(TITLE_LEN as i32),
        Instruction::I64Const(640),
        Instruction::I64Const(480),
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
    push_error_exit(&mut instructions, 5, AREA_WINDOW + 8);
    for instruction in [
        Instruction::End,
        // ok path: exit = handle id (i64 @ AREA_WINDOW + 8)
        Instruction::I32Const(AREA_WINDOW as i32),
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

/// Builds the `poll-event` guest core module. Function index space:
/// 0 = imported realloc, 1 = imported lowered `poll-event`, then `run`, then
/// the no-op post-return. The lowered `poll-event` takes a single trailing i32
/// return-area pointer and returns nothing (its `option<ui-event>` result
/// flattens to more than one core type).
fn poll_guest_module() -> Vec<u8> {
    let mut types = TypeSection::new();
    // 0: realloc
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 1: poll-event — (result area) -> ()
    types.ty().function([ValType::I32], []);
    // 2: run
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 3: post-run
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
    imports.import(UI_INTERFACE, "poll-event", EntityType::Function(1));

    let mut function_section = FunctionSection::new();
    function_section.function(2);
    function_section.function(3);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, 2);
    exports.export("cabi_post_run", ExportKind::Func, 3);

    let mut code = CodeSection::new();
    code.function(&poll_run_body());
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

/// Guest `run` body for the event fixture: call `poll-event` and emit
/// `exit = 100 + <option discriminant>`, so a denied run (typed `none`) yields
/// 100 and a granted run (a delivered `Some` event) yields 101. The guest
/// reads only the discriminant byte at `AREA_POLL + 0` and never decodes the
/// event payload — the point is the authority gate, not the frame.
/// Locals: params 0..3 (input), 4 = exit i64.
fn poll_run_body() -> Function {
    let mut body = Function::new(vec![(1, ValType::I64)]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // poll_event(AREA_POLL)
        Instruction::I32Const(AREA_POLL as i32),
        Instruction::Call(1),
        // exit = 100 + load8(AREA_POLL + 0)
        Instruction::I32Const(AREA_POLL as i32),
        load8(0),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(100),
        Instruction::I64Add,
        Instruction::LocalSet(4),
    ];
    push_ok_return(&mut instructions, 4);
    instructions.push(Instruction::End);
    for instruction in instructions {
        body.instruction(&instruction);
    }
    body
}

fn run_ui(deny: bool) -> (RunOutcome, Arc<FixtureRenderer>) {
    let component = fixture_component();
    let renderer = Arc::new(FixtureRenderer::new());
    let grants = if deny {
        UiGrants::default()
    } else {
        UiGrants { window: true }
    };
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(grants, renderer.clone());
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("fixture component links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    (outcome, renderer)
}

fn run_poll(deny: bool) -> (RunOutcome, Arc<FixtureRenderer>) {
    let component = poll_event_component();
    let renderer = Arc::new(FixtureRenderer::new());
    let grants = if deny {
        UiGrants::default()
    } else {
        UiGrants { window: true }
    };
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(grants, renderer.clone());
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("poll-event fixture component links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    (outcome, renderer)
}

fn exit_of(outcome: RunOutcome) -> (Vec<u8>, i64) {
    match outcome {
        RunOutcome::Output(output) => (output.stdout, output.exit_code),
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn denied_window_is_typed_authority_refusal_without_touching_renderer() {
    let (outcome, renderer) = run_ui(true);
    let (stdout, exit) = exit_of(outcome);
    assert!(stdout.is_empty());
    assert_eq!(
        exit, 115,
        "authority-denied is discriminant 5: exit = 110 + 5 → {exit}"
    );
    assert_eq!(
        renderer.calls.load(Ordering::SeqCst),
        0,
        "a denied call must never reach the renderer backend"
    );
}

#[test]
fn granted_window_reaches_the_renderer_once() {
    let (outcome, renderer) = run_ui(false);
    let (stdout, exit) = exit_of(outcome);
    assert!(stdout.is_empty());
    assert_eq!(exit, 1, "granted path returns the fixture handle id 1");
    assert_eq!(
        renderer.calls.load(Ordering::SeqCst),
        1,
        "exactly one renderer call crosses the granted authority gate"
    );
}

#[test]
fn denied_poll_event_is_none_without_touching_renderer() {
    // `poll-event` is the one surface func that does NOT route through the
    // shared `guarded` error path: with no UI capability it returns the typed
    // `none` option and must not pump the renderer's OS queue at all.
    let (outcome, renderer) = run_poll(true);
    let (stdout, exit) = exit_of(outcome);
    assert!(stdout.is_empty());
    assert_eq!(
        exit, 100,
        "a denied poll is the `none` option (discriminant 0): exit = 100 + 0 → {exit}"
    );
    assert_eq!(
        renderer.calls.load(Ordering::SeqCst),
        0,
        "a denied poll must never reach the renderer's event queue"
    );
}

#[test]
fn granted_poll_event_delivers_an_event_once() {
    let (outcome, renderer) = run_poll(false);
    let (stdout, exit) = exit_of(outcome);
    assert!(stdout.is_empty());
    assert_eq!(
        exit, 101,
        "the granted poll delivers a `Some` event (discriminant 1): exit = 100 + 1 → {exit}"
    );
    assert_eq!(
        renderer.calls.load(Ordering::SeqCst),
        1,
        "exactly one renderer poll crosses the granted authority gate"
    );
}
