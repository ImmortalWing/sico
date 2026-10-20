//! Runner-boundary **happy-path** coverage for the RFC-0039 §2.3/§2.4 package
//! **consumer** bridge (STEP-0147): a Program that imports a versioned
//! `sico:user/…` package is linked, run through [`Runner`], and actually
//! receives the package's return bytes back through the dynamic `Val` bridge.
//!
//! [`package_consumer_authority`](../tests/package_consumer_authority.rs)
//! proves only the three fail-closed launch refusals of that same path. Every
//! other package test (`package_builder.rs`/`image_codec_package.rs`/
//! `vision_package.rs`) instantiates a package with a raw empty linker and
//! never runs a Program that imports one through the Runner. Nothing in the
//! repository exercised the *successful* forward + return, so this fixture
//! closes that gap end to end on the real `csv` package:
//!
//! - `parse-line("a,b")` → the runner bridge reaches the package and the
//!   returned `result<list<string>, string>` comes back as the typed `ok`
//!   branch with a **two-element** list (content, not just a result count);
//! - `parse-line("\"abc")` → the same path returns the typed `err` branch
//!   (discriminant `1`, the package's `unterminated-quote` refusal).
//!
//! The guest reports `exit = ok_disc*1_000_000 + ok_list_len*10 + err_disc`,
//! so the frozen expected value `21` pins all three facts (ok polarity, real
//! payload length `2`, and err polarity) at once. This is `internal-fixture`,
//! `contract-verified` + runtime-verified at the runner bridge — it proves the
//! consumer path forwards and returns correctly; it is **not** rendered
//! feature support and it moves no M24 §3 gate (the roster/pilot packages stay
//! gated on the E8017 record-encoding contract door and #3/#4 external doors).

use sico_codegen_wasm::package_builder::csv_package;
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PackageBinary, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use wasm_encoder::{
    BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ComponentTypeRef,
    ComponentValType, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, InstanceType, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeBounds, TypeSection,
    ValType,
};

const OK_PTR: u32 = 1024;
const OK_LEN: u32 = 5; // "a,b"
const ERR_PTR: u32 = 1080;
const ERR_LEN: u32 = 4; // "\"abc"
const RESULT_AREA: u32 = 4096;
const AREA_OK: u32 = 8192;
const AREA_ERR: u32 = 12_288;
const EMPTY_LIST_PTR: u32 = 3072;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;
const TRANSPORT_INSTANCE: &str = "csv-transport";
const CSV_INSTANCE: &str = "sico:user/csv@1.0.0";

/// Encodes the fixture Program: it imports `sico:user/csv@1.0.0` (component
/// level, filled by the runner's package bridge), lowers `parse-line` against
/// the shared transport, and wires the guest core module that calls it twice.
fn consumer_component() -> Vec<u8> {
    let mut builder = ComponentBuilder::default();

    let types_ty = builder.type_instance(Some("script-types"), &script_types());
    let types_instance = builder.import("sico:script/types", ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    // The package the guest imports — the runner advertises it through
    // `link_packages`, and its real `csv` export satisfies this declaration.
    let csv_ty = builder.type_instance(Some("csv1"), &csv_instance_type());
    let csv_instance = builder.import(CSV_INSTANCE, ComponentTypeRef::Instance(csv_ty));

    // Transport core module: shared memory + a checked bump allocator and the
    // two static parse-line inputs the guest reads.
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

    // Lower `parse-line` against the transport memory. Its
    // `result<list<string>, string>` return flattens to more than one core
    // type, so the canonical ABI uses the return-area (trailing i32 retptr)
    // form; the returned list/string content is copied into guest memory via
    // `realloc`.
    let parse_function =
        builder.alias_export(csv_instance, "parse-line", ComponentExportKind::Func);
    let parse_lowered = builder.lower_func(
        Some("parse-line"),
        parse_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let lowered_instance = builder.core_instantiate_exports(
        Some("csv1-lowered"),
        [("parse-line", ExportKind::Func, parse_lowered)],
    );

    // Guest core module.
    let core = builder.core_module_raw(Some("guest"), &guest_module());
    let instance = builder.core_instantiate(
        Some("guest"),
        core,
        [
            (TRANSPORT_INSTANCE, ModuleArg::Instance(transport_exports)),
            (CSV_INSTANCE, ModuleArg::Instance(lowered_instance)),
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

/// The component-level instance type of `sico:user/csv@1.0.0` as the real
/// package exports it: a single function
/// `parse-line: func(string) -> result<list<string>, string>`. The list/result
/// types are referenced by index but not exported as named items, because the
/// runner's `link_packages` requires every instance export to be a function.
/// Index bookkeeping: each `ty()` consumes one index, `Func` exports consume
/// none.
fn csv_instance_type() -> InstanceType {
    let mut types = InstanceType::new();
    // 0 list<string>; 1 result<list<string>, string>; 2 func parse-line.
    types.ty().defined_type().list(PrimitiveValType::String);
    types.ty().defined_type().result(
        Some(ComponentValType::Type(0)),
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    let mut function = types.ty().function();
    function.params([(
        "line",
        ComponentValType::Primitive(PrimitiveValType::String),
    )]);
    function.result(Some(ComponentValType::Type(1)));
    types.export("parse-line", ComponentTypeRef::Func(2));
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

/// Shared transport: one memory + a checked bump allocator and the two static
/// parse-line inputs (`"a,b"` and `"\"abc"`) the guest forwards to the package.
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
    data.active(0, &ConstExpr::i32_const(OK_PTR as i32), b"a,b".to_vec());
    data.active(0, &ConstExpr::i32_const(ERR_PTR as i32), b"\"abc".to_vec());
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
/// `exit = exit_local`, then leaves the area pointer for the function return.
/// Area layout: tag byte @0; stdout @8/12; stderr @16/20; exit-code i64 @24.
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

/// Guest core module. Function index space: 0 = imported realloc, 1 = imported
/// lowered `parse-line`, then `run`, then the no-op post-return.
fn guest_module() -> Vec<u8> {
    let mut types = TypeSection::new();
    // 0: realloc
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 1: parse-line — (input ptr, input len, result area) -> (); the
    //    `result<list<string>, string>` return flattens to more than one core
    //    type, so the canonical ABI uses the return-area form.
    types
        .ty()
        .function([ValType::I32, ValType::I32, ValType::I32], []);
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
    imports.import(CSV_INSTANCE, "parse-line", EntityType::Function(1));

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

/// Guest `run`: forward two `parse-line` calls through the package bridge and
/// report `exit = ok_disc*1_000_000 + ok_list_len*10 + err_disc`. The
/// discriminant is read at `AREA + 0` and the ok branch's list length at
/// `AREA + 8` (the `result<list<string>, string>` return-area union carries
/// `(ptr, len)` for both the list and the error string, aligned at 4).
/// Locals: params 0..3 (input), 4 = exit i64.
fn run_body() -> Function {
    let mut body = Function::new(vec![(1, ValType::I64)]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // parse-line("a,b", AREA_OK)
        Instruction::I32Const(OK_PTR as i32),
        Instruction::I32Const(OK_LEN as i32),
        Instruction::I32Const(AREA_OK as i32),
        Instruction::Call(1),
        // exit = ok_disc * 1_000_000
        Instruction::I32Const(AREA_OK as i32),
        load8(0),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(1_000_000),
        Instruction::I64Mul,
        Instruction::LocalSet(4),
        // exit += ok_list_len * 10   (len at AREA_OK + 8)
        Instruction::LocalGet(4),
        Instruction::I32Const(AREA_OK as i32),
        Instruction::I32Load(MemArg {
            offset: 8,
            align: 2,
            memory_index: 0,
        }),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(10),
        Instruction::I64Mul,
        Instruction::I64Add,
        Instruction::LocalSet(4),
        // parse-line("\"abc", AREA_ERR)
        Instruction::I32Const(ERR_PTR as i32),
        Instruction::I32Const(ERR_LEN as i32),
        Instruction::I32Const(AREA_ERR as i32),
        Instruction::Call(1),
        // exit += err_disc            (disc at AREA_ERR + 0)
        Instruction::LocalGet(4),
        Instruction::I32Const(AREA_ERR as i32),
        load8(0),
        Instruction::I64ExtendI32U,
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

#[test]
fn program_receives_package_return_bytes_through_the_runner_bridge() {
    let component = consumer_component();
    let packages = [PackageBinary {
        bytes: csv_package(),
    }];
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_packages(
            &component,
            &FsGrants::default(),
            &NetGrants::default(),
            &packages,
        )
        .expect("consumer component links against the prepared package");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    let RunOutcome::Output(output) = outcome else {
        panic!("expected guest output, got {outcome:?}");
    };
    assert!(output.stdout.is_empty(), "the guest writes no stdout");
    assert_eq!(
        output.exit_code, 21,
        "ok_disc*1e6 + ok_list_len*10 + err_disc = 0 + 20 + 1; a value other than 21 means the bridge did not forward or return the package's real result"
    );
}

#[test]
fn consumer_run_is_repeatable() {
    // The bridge instantiates the package into each run's fresh Store, so the
    // same Program + package must return the same result on a second run
    // (the `instantiate_packages` per-Store path, not a cached global).
    for _ in 0..2 {
        program_receives_package_return_bytes_through_the_runner_bridge();
    }
}
