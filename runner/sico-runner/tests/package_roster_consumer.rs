//! M24 gate-3 **registry-path** consumer evidence for the three roster
//! packages (`sico:user/image-match@1`, `image-grid@1`, `image-contour@1`,
//! RFC-0043 §Summary, frozen by A1 result + A2 input, ADR-0019).
//!
//! This closes the **second half** of M24 gate 3 (STEP-0586 ledger I4):
//! corpora half is closed by `image_match_package.rs` / `image_grid_package.rs`
//! / `image_contour_package.rs` (each asserts the real Component against a
//! frozen, independently-generated oracle over the empty-linker direct call).
//! Those tests instantiate a package with a raw empty linker and never run a
//! Program that *imports* one through [`Runner`]. This file does exactly that:
//! every package is installed along the **M7 registry install path**
//! ([`Runner::prepare_program_with_packages`] + [`PreparedProgram::run`], the
//! same `prepare_package` → `link_packages` → `instantiate_packages` sequence
//! the real `sico run` CLI depends on, and the same path
//! `package_consumer_forward.rs` proves for `csv`).
//!
//! What is proved here, per package, through the registry path:
//!
//! 1. a hand-written `wasm-encoder` Program imports the package's real
//!    `sico:user/…` interface, the runner links it against the prepared
//!    package, and the guest receives the package's **real return bytes**
//!    (content, not just a result count) — the `ok` arm's A1 region / count-
//!    prefixed `list<region>` blob, and the `error` arm's RFC-0041 case name
//!    text, both byte-exact against the frozen corpus);
//! 2. the same install path **fails closed** on the refusal surface that
//!    `package_consumer_authority.rs` establishes for `csv`: an unprepared
//!    package import, an arity mismatch, an impure package, and (for a
//!    same-arity type mismatch) a typed failure with no partial call.
//!
//! Evidence level is `internal-fixture` (package/Component level), `contract-
//! verified` + runtime-verified at the runner bridge. It moves no M24 §3 gate
//! (the aggregate gate-3 ruling is the exit audit's); the corpora half and this
//! registry-consumer half are recorded **separately**.

use sico_codegen_wasm::package_builder::{
    image_contour_package, image_grid_package, image_match_package,
};
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

const TRANSPORT_INSTANCE: &str = "roster-transport";
const BMP_PTR: u32 = 1024;
const EMPTY_LIST_PTR: u32 = 8192;
const RESULT_AREA: u32 = 16_384;
const OUTPUT_AREA: u32 = 20_480;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;

/// One registry-path consumer scenario for a roster package.
struct RosterCall {
    /// The package's exported interface identity, e.g.
    /// `sico:user/image-match@1.0.0`.
    interface: &'static str,
    /// The package export the guest imports, e.g. `template`.
    function: &'static str,
    /// Static input byte arrays the guest forwards (bmp, and `tpl` for match).
    inputs: Vec<Vec<u8>>,
    /// The `U64` second parameter for grid/contour; `None` for match.
    min_value: Option<u64>,
    /// The `result` discriminant the guest expects: 0 = ok, 1 = err. The guest
    /// returns the payload bytes (region/list blob, or error case name) as
    /// stdout only when the discriminant matches, and a non-zero exit
    /// otherwise.
    expected_disc: u8,
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// Static input offset for `inputs[i]` (bmp at 1024, tpl at 1024 + bmp row).
fn input_offset(call: &RosterCall, index: usize) -> u32 {
    if index == 0 {
        BMP_PTR
    } else {
        BMP_PTR + round_up(call.inputs[0].len() as u32, 64)
    }
}

fn round_up(value: u32, align: u32) -> u32 {
    value.div_ceil(align) * align
}

/// The component-level instance type of a roster package export as the guest
/// declares it: a single function with `list<u8>`/`u64` params returning
/// `result<list<u8>, string>`. Type index bookkeeping mirrors
/// `package_consumer_forward.rs` (each `ty()` consumes one index).
fn roster_instance_type(call: &RosterCall) -> InstanceType {
    let mut types = InstanceType::new();
    // 0: list<u8>; 1: result<list<u8>, string>; 2: func.
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.ty().defined_type().result(
        Some(ComponentValType::Type(0)),
        Some(ComponentValType::Primitive(PrimitiveValType::String)),
    );
    let mut function = types.ty().function();
    if call.min_value.is_some() {
        function.params([
            ("bmp", ComponentValType::Type(0)),
            ("min", ComponentValType::Primitive(PrimitiveValType::U64)),
        ]);
    } else {
        function.params([
            ("bmp", ComponentValType::Type(0)),
            ("tpl", ComponentValType::Type(0)),
        ]);
    }
    function.result(Some(ComponentValType::Type(1)));
    types.export(call.function, ComponentTypeRef::Func(2));
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

/// Encodes the consumer Program: imports `sico:script/types` (type-only) and
/// the roster package interface, lowers the package export against a shared
/// transport (memory + checked bump allocator), and wires the guest core
/// module that forwards the static inputs and returns the payload bytes.
fn consumer_component(call: &RosterCall) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();

    let types_ty = builder.type_instance(Some("script-types"), &script_types());
    let types_instance = builder.import("sico:script/types", ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    // The roster package — the runner advertises it through `link_packages`.
    let pkg_ty = builder.type_instance(Some("pkg"), &roster_instance_type(call));
    let pkg_instance = builder.import(call.interface, ComponentTypeRef::Instance(pkg_ty));

    // Transport core module: shared memory + checked bump allocator + the
    // static inputs the guest forwards.
    let transport_module_index =
        builder.core_module_raw(Some(TRANSPORT_INSTANCE), &transport_module(call));
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

    // Lower the package export against the transport memory. Its
    // `result<list<u8>, string>` return flattens to more than one core type,
    // so the canonical ABI uses the return-area (trailing i32 retptr) form.
    let pkg_function = builder.alias_export(pkg_instance, call.function, ComponentExportKind::Func);
    let lowered = builder.lower_func(
        Some("lowered"),
        pkg_function,
        [
            wasm_encoder::CanonicalOption::UTF8,
            wasm_encoder::CanonicalOption::Memory(transport_memory),
            wasm_encoder::CanonicalOption::Realloc(transport_realloc),
        ],
    );
    let lowered_instance = builder.core_instantiate_exports(
        Some("pkg-lowered"),
        [(call.function, ExportKind::Func, lowered)],
    );

    // Guest core module.
    let core = builder.core_module_raw(Some("guest"), &guest_module(call));
    let instance = builder.core_instantiate(
        Some("guest"),
        core,
        [
            (TRANSPORT_INSTANCE, ModuleArg::Instance(transport_exports)),
            (call.interface, ModuleArg::Instance(lowered_instance)),
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

/// Shared transport: one memory + a checked bump allocator and the static
/// input byte arrays the guest forwards.
fn transport_module(call: &RosterCall) -> Vec<u8> {
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
    for (index, input) in call.inputs.iter().enumerate() {
        data.active(
            0,
            &ConstExpr::i32_const(input_offset(call, index) as i32),
            input.clone(),
        );
    }
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

fn load32(offset: u32) -> Instruction<'static> {
    Instruction::I32Load(MemArg {
        offset: u64::from(offset),
        align: 2,
        memory_index: 0,
    })
}

fn store32(offset: u32) -> Instruction<'static> {
    Instruction::I32Store(MemArg {
        offset: u64::from(offset),
        align: 2,
        memory_index: 0,
    })
}

/// Emits a zero-tag (ok) script-output whose stdout is `(ptr_local, len_local)`
/// and whose exit-code is `exit_local`, then leaves the area pointer for the
/// function return. Script-result area layout (a `result<record, error>`
/// variant whose record payload has i64 alignment): tag i32 @0; stdout ptr/len
/// @8/12; stderr ptr/len @16/20; exit-code i64 @24.
fn push_ok_return(
    instructions: &mut Vec<Instruction<'static>>,
    exit_local: u32,
    stdout_ptr_local: u32,
    stdout_len_local: u32,
) {
    for instruction in [
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::I32Const(0),
        Instruction::I32Store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::LocalGet(stdout_ptr_local),
        store32(8),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::LocalGet(stdout_len_local),
        store32(12),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        store32(16),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::I32Const(0),
        store32(20),
        Instruction::I32Const(OUTPUT_AREA as i32),
        Instruction::LocalGet(exit_local),
        Instruction::I64Store(MemArg {
            offset: 24,
            align: 3,
            memory_index: 0,
        }),
        Instruction::I32Const(OUTPUT_AREA as i32),
    ] {
        instructions.push(instruction);
    }
}

/// Guest core module. Function index space: 0 = imported realloc, 1 = imported
/// lowered package export, then `run`, then the no-op post-return.
fn guest_module(call: &RosterCall) -> Vec<u8> {
    let mut types = TypeSection::new();
    // 0: realloc
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    // 1: lowered package export. `result<list<u8>, string>` flattens to more
    //    than one core type, so the canonical ABI uses the return-area form:
    //    one trailing i32 retptr. For match the params are two (ptr,len)
    //    pairs; for grid/contour one (ptr,len) pair + an i64 min.
    if call.min_value.is_some() {
        types
            .ty()
            .function([ValType::I32, ValType::I32, ValType::I64, ValType::I32], []);
    } else {
        types.ty().function(
            [
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
            ],
            [],
        );
    }
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
    imports.import(call.interface, call.function, EntityType::Function(1));

    let mut function_section = FunctionSection::new();
    function_section.function(2);
    function_section.function(3);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, 2);
    exports.export("cabi_post_run", ExportKind::Func, 3);

    let mut code = CodeSection::new();
    code.function(&run_body(call));
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

/// Guest `run`: forward one package call and report the payload bytes as
/// stdout when the result discriminant matches the expected polarity, else a
/// non-zero exit. Locals: params 0..3 (input, ignored), 4 = exit i64, 5 = disc
/// i32, 6 = payload ptr i32, 7 = payload len i32.
fn run_body(call: &RosterCall) -> Function {
    let mut body = Function::new(vec![(1, ValType::I64), (3, ValType::I32)]);
    let mut instructions: Vec<Instruction<'static>> = vec![
        // Lowered package call: push the static inputs, then the retptr.
        Instruction::I32Const(input_offset(call, 0) as i32),
        Instruction::I32Const(call.inputs[0].len() as i32),
    ];
    if let Some(min) = call.min_value {
        instructions.push(Instruction::I64Const(min as i64));
    } else {
        instructions.push(Instruction::I32Const(input_offset(call, 1) as i32));
        instructions.push(Instruction::I32Const(call.inputs[1].len() as i32));
    }
    instructions.extend([
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::Call(1),
        // disc = load8u(RESULT_AREA + 0)
        Instruction::I32Const(RESULT_AREA as i32),
        load8(0),
        Instruction::LocalSet(5),
        // if disc == expected: stdout = payload (ptr@+4, len@+8), exit = 0;
        // else: stdout = empty, exit = 1000 + disc (non-zero, never partial).
        Instruction::LocalGet(5),
        Instruction::I32Const(i32::from(call.expected_disc)),
        Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(RESULT_AREA as i32),
        load32(4),
        Instruction::LocalSet(6),
        Instruction::I32Const(RESULT_AREA as i32),
        load32(8),
        Instruction::LocalSet(7),
        Instruction::I64Const(0),
        Instruction::LocalSet(4),
        Instruction::Else,
        Instruction::I32Const(EMPTY_LIST_PTR as i32),
        Instruction::LocalSet(6),
        Instruction::I32Const(0),
        Instruction::LocalSet(7),
        Instruction::LocalGet(5),
        Instruction::I64ExtendI32U,
        Instruction::I64Const(1000),
        Instruction::I64Add,
        Instruction::LocalSet(4),
        Instruction::End,
    ]);
    push_ok_return(&mut instructions, 4, 6, 7);
    instructions.push(Instruction::End);
    for instruction in instructions {
        body.instruction(&instruction);
    }
    body
}

/// Installs `package` along the registry path and runs the consumer for
/// `call`, returning the guest's `Output`.
fn run_consumer(call: &RosterCall, package: Vec<u8>) -> sico_runner::ScriptOutput {
    let component = consumer_component(call);
    let packages = [PackageBinary { bytes: package }];
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
    output
}

// ---------------------------------------------------------------------------
// Happy path: each roster package installed along the registry path returns
// the frozen corpus payload bytes through the real consumer.
// ---------------------------------------------------------------------------

#[test]
fn image_match_template_reaches_the_consumer_through_the_registry() {
    // Frozen `grey-exact-offset` case: a 16-byte A1 region.
    let call = RosterCall {
        interface: "sico:user/image-match@1.0.0",
        function: "template",
        inputs: vec![
            unhex(
                "080000000600000001000000001f3e5d7c9bbad935547392b1d0ef136a89a8c7e60a29489fbedd01203f5e7dd4f31736557493b20e2d4c6b8aa9c8e7",
            ),
            unhex("030000000200000001000000c7e60a01203f"),
        ],
        min_value: None,
        expected_disc: 0,
    };
    let output = run_consumer(&call, image_match_package());
    assert!(output.stderr.is_empty(), "the guest writes no stderr");
    assert_eq!(output.exit_code, 0, "ok polarity reached the guest");
    assert_eq!(
        output.stdout,
        unhex("03000000020000000300000002000000"),
        "the 16-byte A1 region must be byte-exact through the registry path"
    );
}

#[test]
fn image_match_error_case_name_reaches_the_consumer_through_the_registry() {
    // Frozen `bad-format-tag` case: the RFC-0041 `unsupported-format` refusal.
    let call = RosterCall {
        interface: "sico:user/image-match@1.0.0",
        function: "template",
        inputs: vec![
            unhex("040000000400000007000000"),
            unhex("030000000200000001000000c7e60a01203f"),
        ],
        min_value: None,
        expected_disc: 1,
    };
    let output = run_consumer(&call, image_match_package());
    assert_eq!(output.exit_code, 0, "err polarity reached the guest");
    assert_eq!(
        output.stdout,
        b"unsupported-format".to_vec(),
        "the error arm must carry the typed case-name text through the registry path"
    );
}

#[test]
fn image_grid_detect_lines_reaches_the_consumer_through_the_registry() {
    // Frozen `single-row-min3` case: count(1) + one 16-byte region.
    let call = RosterCall {
        interface: "sico:user/image-grid@1.0.0",
        function: "detect-lines",
        inputs: vec![unhex("060000000100000001000000ffffff00ffff")],
        min_value: Some(3),
        expected_disc: 0,
    };
    let output = run_consumer(&call, image_grid_package());
    assert_eq!(output.exit_code, 0, "ok polarity reached the guest");
    assert_eq!(
        output.stdout,
        unhex("0100000000000000000000000300000001000000"),
        "count-prefixed list<region> must be byte-exact through the registry path"
    );
}

#[test]
fn image_grid_error_case_name_reaches_the_consumer_through_the_registry() {
    let call = RosterCall {
        interface: "sico:user/image-grid@1.0.0",
        function: "detect-lines",
        inputs: vec![unhex("040000000400000007000000")],
        min_value: Some(2),
        expected_disc: 1,
    };
    let output = run_consumer(&call, image_grid_package());
    assert_eq!(output.exit_code, 0, "err polarity reached the guest");
    assert_eq!(output.stdout, b"unsupported-format".to_vec());
}

#[test]
fn image_contour_find_reaches_the_consumer_through_the_registry() {
    // Frozen `single-blob-3x3` case: count(1) + one 16-byte region.
    let call = RosterCall {
        interface: "sico:user/image-contour@1.0.0",
        function: "find",
        inputs: vec![unhex(
            "050000000500000001000000000000000000ffffff0000ffffff0000ffffff000000000000",
        )],
        min_value: Some(1),
        expected_disc: 0,
    };
    let output = run_consumer(&call, image_contour_package());
    assert_eq!(output.exit_code, 0, "ok polarity reached the guest");
    assert_eq!(
        output.stdout,
        unhex("0100000001000000010000000300000003000000"),
        "count-prefixed list<region> must be byte-exact through the registry path"
    );
}

#[test]
fn image_contour_error_case_name_reaches_the_consumer_through_the_registry() {
    let call = RosterCall {
        interface: "sico:user/image-contour@1.0.0",
        function: "find",
        inputs: vec![unhex("040000000400000007000000")],
        min_value: Some(2),
        expected_disc: 1,
    };
    let output = run_consumer(&call, image_contour_package());
    assert_eq!(output.exit_code, 0, "err polarity reached the guest");
    assert_eq!(output.stdout, b"unsupported-format".to_vec());
}

/// The consumer run instantiates the already-compiled package Component into
/// each fresh Store (the `instantiate_packages` per-Store path, never a cached
/// global), so the same Program + package must return the same bytes again.
#[test]
fn roster_consumer_run_is_repeatable() {
    let call = RosterCall {
        interface: "sico:user/image-match@1.0.0",
        function: "template",
        inputs: vec![
            unhex(
                "080000000600000001000000001f3e5d7c9bbad935547392b1d0ef136a89a8c7e60a29489fbedd01203f5e7dd4f31736557493b20e2d4c6b8aa9c8e7",
            ),
            unhex("030000000200000001000000c7e60a01203f"),
        ],
        min_value: None,
        expected_disc: 0,
    };
    let first = run_consumer(&call, image_match_package());
    let second = run_consumer(&call, image_match_package());
    assert_eq!(
        first.stdout, second.stdout,
        "registry consumer is byte-stable"
    );
    assert_eq!(first.exit_code, second.exit_code);
}

// ---------------------------------------------------------------------------
// Fail-closed: the refusal surface `package_consumer_authority.rs` establishes
// for `csv`, asserted per roster package along the same registry path.
// ---------------------------------------------------------------------------

/// A Program component that imports `interface` and declares `function` with
/// `params` core-visible parameters and exactly one `string` result. The
/// runner matches the declaration against the prepared package's export by
/// name + arity, so `params` is the knob under test. Type index 0 is a
/// `list<u8>`; `params` refer to it via `bytes_type()`.
fn guest_importing(
    interface: &str,
    function: &str,
    params: Vec<(&str, ComponentValType)>,
) -> Vec<u8> {
    let mut instance = InstanceType::new();
    instance.ty().defined_type().list(PrimitiveValType::U8);
    let mut fn_ty = instance.ty().function();
    fn_ty.params(params);
    fn_ty.result(Some(ComponentValType::Primitive(PrimitiveValType::String)));
    instance.export(function, ComponentTypeRef::Func(1));

    let mut builder = ComponentBuilder::default();
    let instance_ty = builder.type_instance(Some("pkg"), &instance);
    builder.import(interface, ComponentTypeRef::Instance(instance_ty));
    builder.finish()
}

fn launch_message(outcome: Result<sico_runner::PreparedProgram, RunOutcome>) -> String {
    match outcome {
        Ok(_) => panic!("expected a fail-closed launch refusal, got a prepared program"),
        Err(RunOutcome::Launch(message)) => message,
        Err(other) => panic!("expected a Launch refusal, got {other:?}"),
    }
}

fn bytes_type() -> ComponentValType {
    ComponentValType::Type(0)
}

#[test]
fn guest_importing_an_unprepared_roster_package_fails_closed() {
    for (interface, function) in [
        ("sico:user/image-match@1.0.0", "template"),
        ("sico:user/image-grid@1.0.0", "detect-lines"),
        ("sico:user/image-contour@1.0.0", "find"),
    ] {
        let runner = Runner::new().expect("runner builds");
        let message = launch_message(runner.prepare_program_with_packages(
            &guest_importing(interface, function, vec![("p0", bytes_type())]),
            &FsGrants::default(),
            &NetGrants::default(),
            &[],
        ));
        assert!(
            message.contains("no prepared package exports it"),
            "{interface}: an import with no matching package must be a typed launch refusal, got: {message}"
        );
    }
}

#[test]
fn roster_package_import_arity_mismatch_fails_closed() {
    // The bridge forwards dynamically typed `Val`s straight to the package
    // export, so an arity mismatch is unsafe and must be refused at link time.
    // Each roster export takes exactly 2 params (two lists, or a list + u64);
    // a 1-param declaration must be refused.
    let packages_by_interface = [
        (
            "sico:user/image-match@1.0.0",
            "template",
            image_match_package(),
        ),
        (
            "sico:user/image-grid@1.0.0",
            "detect-lines",
            image_grid_package(),
        ),
        (
            "sico:user/image-contour@1.0.0",
            "find",
            image_contour_package(),
        ),
    ];
    for (interface, function, package) in packages_by_interface {
        let runner = Runner::new().expect("runner builds");
        let packages = [PackageBinary { bytes: package }];
        let message = launch_message(runner.prepare_program_with_packages(
            &guest_importing(interface, function, vec![("p0", bytes_type())]),
            &FsGrants::default(),
            &NetGrants::default(),
            &packages,
        ));
        assert!(
            message.contains("arity mismatch"),
            "{interface}: a guest import whose arity differs from the package export must fail closed, got: {message}"
        );
    }
}

#[test]
fn roster_package_same_arity_type_mismatch_fails_closed_without_a_partial_call() {
    // The bridge checks arity, not the full type. A same-arity type mismatch
    // must not produce a partial/incorrect call: the consumer declares
    // `template` as `func(list<u8>, u64) -> result<list<u8>, string>` (arity 2
    // like the real `func(list<u8>, list<u8>)`, but the second param type
    // differs). Link time accepts it (arity matches); the mismatch surfaces
    // when the guest actually calls the export, as a typed failure — never a
    // successful Output carrying bytes the guest could mistake for a result.
    let call = RosterCall {
        interface: "sico:user/image-match@1.0.0",
        function: "template",
        inputs: vec![
            unhex(
                "080000000600000001000000001f3e5d7c9bbad935547392b1d0ef136a89a8c7e60a29489fbedd01203f5e7dd4f31736557493b20e2d4c6b8aa9c8e7",
            ),
            unhex("030000000200000001000000c7e60a01203f"),
        ],
        // The real package export's second param is a `list<u8>`, but the
        // guest declares it as a `u64` (the grid/contour shape): same arity,
        // wrong type.
        min_value: Some(1),
        expected_disc: 0,
    };
    let runner = Runner::new().expect("runner builds");
    let packages = [PackageBinary {
        bytes: image_match_package(),
    }];
    let prepared = runner.prepare_program_with_packages(
        &consumer_component(&call),
        &FsGrants::default(),
        &NetGrants::default(),
        &packages,
    );
    match prepared {
        Ok(prepared) => {
            // Link time accepted it; the mismatch must still fail when the
            // guest calls, never yielding a partial/incorrect Output.
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
            assert!(
                !matches!(outcome, RunOutcome::Output(_)),
                "a same-arity type mismatch must not yield a successful Output, got {outcome:?}"
            );
        }
        Err(RunOutcome::Launch(message)) => {
            assert!(
                message.contains("arity mismatch"),
                "a same-arity type mismatch must fail closed, got: {message}"
            );
        }
        Err(other) => panic!("expected a typed launch refusal, got {other:?}"),
    }
}

#[test]
fn impure_roster_package_is_refused_at_preparation() {
    // A "package" that itself imports something is not a pure v0 package;
    // preparation must refuse it before it is ever linked into a run, so it
    // can never smuggle ambient authority into the consumer's Store.
    let runner = Runner::new().expect("runner builds");
    let packages = [PackageBinary {
        bytes: guest_importing(
            "sico:user/image-match@1.0.0",
            "template",
            vec![("p0", bytes_type())],
        ),
    }];
    let message = launch_message(runner.prepare_program_with_packages(
        &ComponentBuilder::default().finish(),
        &FsGrants::default(),
        &NetGrants::default(),
        &packages,
    ));
    assert!(
        message.contains("must not import"),
        "a package with its own imports must be refused as impure, got: {message}"
    );
}
