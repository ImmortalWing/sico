//! Guest-visible `sico:script/http@0.2.0` fixtures (M12 runner integration,
//! RFC-0037). The compiler frontend does not yet emit `http@0.2.0` imports —
//! that is the M14 application-language baseline — so these tests encode
//! real guest Components directly with `wasm-encoder` and run them through
//! the runner against the deterministic TLS fixture server.
//!
//! Every scenario reads its URL from `arguments[0]` and reports through the
//! Script output: `exit = status/100 - 1` on an HTTP response, `exit =
//! 110 + <http-error discriminant>` on a typed refusal (permission=0 …
//! io=7), and `stdout` carries the response body.

use sico_http_provider::secrets::{InjectionPolicy, SecretBinding, SecretStore};
use sico_http_provider::streaming::{HttpFixtureServer, ResponsePlan};
use sico_runner::{
    CancelToken, FsGrants, HttpPolicy, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};
use wasm_encoder::{
    BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ComponentTypeRef,
    ComponentValType, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, InstanceType, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeBounds, TypeSection,
    ValType,
};

const GET_STRING_OFFSET: u32 = 1024;
const POST_STRING_OFFSET: u32 = 1088;
const UPLOAD_PAYLOAD_OFFSET: u32 = 2048;
const RESULT_AREA: u32 = 4096;
const AREA_REQUEST: u32 = 8192;
const AREA_READ: u32 = 8448;
const AREA_OPEN: u32 = 8704;
const AREA_WRITE: u32 = 8960;
const AREA_FINISH: u32 = 9216;
const SCRATCH_OFFSET: u32 = 65_536;
const SCRATCH_CAPACITY: u32 = 512 * 1024;
const EMPTY_LIST_PTR: u32 = 3072;
const MEMORY_PAGES: u32 = 64;
const ARENA_LIMIT: u32 = MEMORY_PAGES * 65_536;
const TRANSPORT_INSTANCE: &str = "fs-transport";
const HTTP2_INTERFACE: &str = "sico:script/http@0.2.0";
const TIMEOUT_MS: u64 = 8_000;

/// Which `http@0.2.0` surface the fixture guest exercises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    /// One buffered `request`; stdout = response body.
    BufferedGet,
    /// Two buffered requests to the same URL in one Store; stdout = the two
    /// bodies concatenated (connection-pooling evidence).
    BufferedGetTwice,
    /// `request-streaming` + incremental reads of a chunked body.
    StreamingGet,
    /// `open-upload` + two writes + `finish` + streaming reads of the reply.
    Upload,
}

/// Encodes one fixture Component importing `sico:script/http@0.2.0`.
fn fixture_component(scenario: Scenario) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();

    // Script world types: script-input / script-output / script-error.
    let types_ty = builder.type_instance(Some("script-types"), &script_types());
    let types_instance = builder.import("sico:script/types", ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));

    // The versioned secure-HTTP instance.
    let http_ty = builder.type_instance(Some("http2"), &http2_instance_type(scenario));
    let http_instance = builder.import(HTTP2_INTERFACE, ComponentTypeRef::Instance(http_ty));

    // Transport core module: shared memory + bump allocator for every
    // canonical lift/lower in this component.
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

    // Lower each used interface function against the transport memory.
    let functions = scenario_functions(scenario);
    let mut lowered: Vec<(&'static str, u32)> = Vec::new();
    for (name, _) in &functions {
        let function = builder.alias_export(http_instance, name, ComponentExportKind::Func);
        let lowered_function = builder.lower_func(
            Some(name),
            function,
            [
                wasm_encoder::CanonicalOption::UTF8,
                wasm_encoder::CanonicalOption::Memory(transport_memory),
                wasm_encoder::CanonicalOption::Realloc(transport_realloc),
            ],
        );
        lowered.push((name, lowered_function));
    }
    let lowered_instance = builder.core_instantiate_exports(
        Some("http2-lowered"),
        lowered
            .iter()
            .map(|(name, index)| (*name, ExportKind::Func, *index)),
    );

    // Guest core module.
    let core = builder.core_module_raw(Some("guest"), &scenario_module(scenario));
    let instance = builder.core_instantiate(
        Some("guest"),
        core,
        [
            (TRANSPORT_INSTANCE, ModuleArg::Instance(transport_exports)),
            (HTTP2_INTERFACE, ModuleArg::Instance(lowered_instance)),
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

/// The component-level instance type of the `http@0.2.0` subset one
/// scenario requires (width subtyping). Index budget (index-consuming ops
/// in declaration order): 0 list<u8>, 1 header, 2 list<header>,
/// 3 http-error, 4 options, 5 response-head, 6 response-body resource,
/// 7 streaming-response, 8 response; per-scenario suffixes below.
fn http2_instance_type(scenario: Scenario) -> InstanceType {
    // Index bookkeeping follows the frozen Script adapter's proven rules:
    // SubResource and `Eq` exports consume type indices, `ty()` consumes
    // one, `Func` exports consume none, and resource handles inside
    // signatures must reference explicit `borrow()`/`own()` wrappers.
    // Shared prefix (both scenarios):
    // 0 resource response-body; 1 ty list<u8>; 2 alias byte-list;
    // 3 ty header; 4 alias header; 5 ty list<header>; 6 alias header-list;
    // 7 ty http-error; 8 alias http-error; 9 ty options; 10 alias options;
    // 11 ty response-head; 12 alias response-head.
    let mut types = InstanceType::new();
    types.export(
        "response-body",
        ComponentTypeRef::Type(TypeBounds::SubResource),
    );
    types.ty().defined_type().list(PrimitiveValType::U8);
    types.export("byte-list", ComponentTypeRef::Type(TypeBounds::Eq(1)));
    types.ty().defined_type().record([
        (
            "name",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
        (
            "value",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
    ]);
    types.export("header", ComponentTypeRef::Type(TypeBounds::Eq(3)));
    types.ty().defined_type().list(ComponentValType::Type(4));
    types.export("header-list", ComponentTypeRef::Type(TypeBounds::Eq(5)));
    types.ty().defined_type().enum_type([
        "permission",
        "authority",
        "tls",
        "dns",
        "limit",
        "cancel",
        "protocol",
        "io",
    ]);
    types.export("http-error", ComponentTypeRef::Type(TypeBounds::Eq(7)));
    types.ty().defined_type().record([
        ("headers", ComponentValType::Type(6)),
        (
            "follow-redirects",
            ComponentValType::Primitive(PrimitiveValType::Bool),
        ),
        (
            "retry-idempotent",
            ComponentValType::Primitive(PrimitiveValType::Bool),
        ),
        (
            "timeout-ms",
            ComponentValType::Primitive(PrimitiveValType::U64),
        ),
    ]);
    types.export("options", ComponentTypeRef::Type(TypeBounds::Eq(9)));
    types.ty().defined_type().record([
        ("status", ComponentValType::Primitive(PrimitiveValType::S64)),
        ("headers", ComponentValType::Type(6)),
    ]);
    types.export("response-head", ComponentTypeRef::Type(TypeBounds::Eq(11)));
    match scenario {
        Scenario::BufferedGet | Scenario::BufferedGetTwice => {
            // 13 result<response(18), http-error(8)>; 14 func request.
            types.ty().defined_type().record([
                ("status", ComponentValType::Primitive(PrimitiveValType::S64)),
                ("headers", ComponentValType::Type(6)),
                ("body", ComponentValType::Type(2)),
            ]);
            types.export("response", ComponentTypeRef::Type(TypeBounds::Eq(13)));
            types.ty().defined_type().result(
                Some(ComponentValType::Type(14)),
                Some(ComponentValType::Type(8)),
            );
            let mut function = types.ty().function();
            function.params(vec![
                (
                    "method",
                    ComponentValType::Primitive(PrimitiveValType::String),
                ),
                ("url", ComponentValType::Primitive(PrimitiveValType::String)),
                ("options", ComponentValType::Type(10)),
                ("body", ComponentValType::Type(2)),
            ]);
            function.result(Some(ComponentValType::Type(15)));
            types.export("request", ComponentTypeRef::Func(16));
        }
        Scenario::StreamingGet => {
            // 13 borrow(0); 14 own(0); 15 ty streaming [12, 14];
            // 16 alias streaming; 17 ty response [6, 2]; 18 alias response;
            // 19 result<16, 8>; 20 func request-streaming;
            // 21 result<2, 8>; 22 func read [13, u64].
            types.ty().defined_type().borrow(0);
            types.ty().defined_type().own(0);
            types.ty().defined_type().record([
                ("head", ComponentValType::Type(12)),
                ("body", ComponentValType::Type(14)),
            ]);
            types.export(
                "streaming-response",
                ComponentTypeRef::Type(TypeBounds::Eq(15)),
            );
            types.ty().defined_type().record([
                ("status", ComponentValType::Primitive(PrimitiveValType::S64)),
                ("headers", ComponentValType::Type(6)),
                ("body", ComponentValType::Type(2)),
            ]);
            types.export("response", ComponentTypeRef::Type(TypeBounds::Eq(17)));
            types.ty().defined_type().result(
                Some(ComponentValType::Type(16)),
                Some(ComponentValType::Type(8)),
            );
            let mut function = types.ty().function();
            function.params(vec![
                (
                    "method",
                    ComponentValType::Primitive(PrimitiveValType::String),
                ),
                ("url", ComponentValType::Primitive(PrimitiveValType::String)),
                ("options", ComponentValType::Type(10)),
                ("body", ComponentValType::Type(2)),
            ]);
            function.result(Some(ComponentValType::Type(19)));
            types.export("request-streaming", ComponentTypeRef::Func(20));
            types.ty().defined_type().result(
                Some(ComponentValType::Type(2)),
                Some(ComponentValType::Type(8)),
            );
            let mut read = types.ty().function();
            read.params([
                ("self", ComponentValType::Type(13)),
                ("max", ComponentValType::Primitive(PrimitiveValType::U64)),
            ]);
            read.result(Some(ComponentValType::Type(21)));
            types.export("[method]response-body.read", ComponentTypeRef::Func(22));
        }
        Scenario::Upload => {
            // 13 resource request-body; 14 borrow(13); 15 own(13);
            // 16 own(0); 17 ty streaming [12, 16]; 18 alias streaming;
            // 19 ty response [6, 2]; 20 alias response;
            // 21 result<15, 8>; 22 func open-upload; 23 result<_, 8>;
            // 24 func write [14, 2]; 25 result<18, 8>; 26 func finish;
            // 27 result<2, 8>; 28 borrow(0); 29 func read [28, u64].
            types.export(
                "request-body",
                ComponentTypeRef::Type(TypeBounds::SubResource),
            );
            types.ty().defined_type().borrow(13);
            types.ty().defined_type().own(13);
            types.ty().defined_type().own(0);
            types.ty().defined_type().record([
                ("head", ComponentValType::Type(12)),
                ("body", ComponentValType::Type(16)),
            ]);
            types.export(
                "streaming-response",
                ComponentTypeRef::Type(TypeBounds::Eq(17)),
            );
            types.ty().defined_type().record([
                ("status", ComponentValType::Primitive(PrimitiveValType::S64)),
                ("headers", ComponentValType::Type(6)),
                ("body", ComponentValType::Type(2)),
            ]);
            types.export("response", ComponentTypeRef::Type(TypeBounds::Eq(19)));
            types.ty().defined_type().result(
                Some(ComponentValType::Type(15)),
                Some(ComponentValType::Type(8)),
            );
            let mut open = types.ty().function();
            open.params(vec![
                (
                    "method",
                    ComponentValType::Primitive(PrimitiveValType::String),
                ),
                ("url", ComponentValType::Primitive(PrimitiveValType::String)),
                ("options", ComponentValType::Type(10)),
            ]);
            open.result(Some(ComponentValType::Type(21)));
            types.export("open-upload", ComponentTypeRef::Func(22));
            types
                .ty()
                .defined_type()
                .result(None, Some(ComponentValType::Type(8)));
            let mut write = types.ty().function();
            write.params([
                ("self", ComponentValType::Type(14)),
                ("bytes", ComponentValType::Type(2)),
            ]);
            write.result(Some(ComponentValType::Type(23)));
            types.export("[method]request-body.write", ComponentTypeRef::Func(24));
            types.ty().defined_type().result(
                Some(ComponentValType::Type(18)),
                Some(ComponentValType::Type(8)),
            );
            let mut finish = types.ty().function();
            finish.params([("body", ComponentValType::Type(15))]);
            finish.result(Some(ComponentValType::Type(25)));
            types.export("finish", ComponentTypeRef::Func(26));
            types.ty().defined_type().result(
                Some(ComponentValType::Type(2)),
                Some(ComponentValType::Type(8)),
            );
            types.ty().defined_type().borrow(0);
            let mut read = types.ty().function();
            read.params([
                ("self", ComponentValType::Type(28)),
                ("max", ComponentValType::Primitive(PrimitiveValType::U64)),
            ]);
            read.result(Some(ComponentValType::Type(27)));
            types.export("[method]response-body.read", ComponentTypeRef::Func(29));
        }
    }
    types
}

/// Lowered core signatures per scenario, in the guest module's import
/// order. Every call returns aggregates containing memory types, so the
/// canonical ABI uses the return-area form: one extra trailing i32 (the
/// caller-provided result area) and no core results.
/// Lowered core signature: (params, results).
type CoreSignature = (Vec<ValType>, Vec<ValType>);

fn scenario_functions(scenario: Scenario) -> Vec<(&'static str, CoreSignature)> {
    let buffered_params: Vec<ValType> = vec![
        ValType::I32,
        ValType::I32, // method
        ValType::I32,
        ValType::I32, // url
        ValType::I32,
        ValType::I32, // options.headers
        ValType::I32, // follow-redirects
        ValType::I32, // retry-idempotent
        ValType::I64, // timeout-ms
        ValType::I32,
        ValType::I32, // body
        ValType::I32, // result area
    ];
    match scenario {
        Scenario::BufferedGet | Scenario::BufferedGetTwice => {
            vec![("request", (buffered_params, vec![]))]
        }
        Scenario::StreamingGet => {
            vec![
                ("request-streaming", (buffered_params, vec![])),
                (
                    "[method]response-body.read",
                    (vec![ValType::I32, ValType::I64, ValType::I32], vec![]),
                ),
            ]
        }
        Scenario::Upload => {
            vec![
                (
                    "open-upload",
                    (
                        vec![
                            ValType::I32,
                            ValType::I32, // method
                            ValType::I32,
                            ValType::I32, // url
                            ValType::I32,
                            ValType::I32, // options.headers
                            ValType::I32, // follow-redirects
                            ValType::I32, // retry-idempotent
                            ValType::I64, // timeout-ms
                            ValType::I32, // result area
                        ],
                        vec![],
                    ),
                ),
                (
                    "[method]request-body.write",
                    (
                        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                        vec![],
                    ),
                ),
                ("finish", (vec![ValType::I32, ValType::I32], vec![])),
                (
                    "[method]response-body.read",
                    (vec![ValType::I32, ValType::I64, ValType::I32], vec![]),
                ),
            ]
        }
    }
}

/// Shared transport: one memory + a checked bump allocator serving every
/// canonical lift/lower and the guest's static data.
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
        &ConstExpr::i32_const(i32::try_from(GET_STRING_OFFSET).unwrap_or(0)),
        b"GET".to_vec(),
    );
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(POST_STRING_OFFSET).unwrap_or(0)),
        b"POST".to_vec(),
    );
    data.active(
        0,
        &ConstExpr::i32_const(i32::try_from(UPLOAD_PAYLOAD_OFFSET).unwrap_or(0)),
        b"upload-payload-0123456789abcdef".to_vec(),
    );
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&realloc_into_code(realloc));
    module.section(&data);
    module.finish()
}

fn realloc_into_code(realloc: Function) -> CodeSection {
    let mut code = CodeSection::new();
    code.function(&realloc);
    code
}

fn load(offset: u32) -> Instruction<'static> {
    Instruction::I32Load(MemArg {
        offset: u64::from(offset),
        align: 2,
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

/// Emits the ok return: tag 0, stdout from `(ptr_expr, len_local)`, empty
/// stderr, exit from `exit_local`, then the area pointer.
fn push_ok_return(
    instructions: &mut Vec<Instruction<'static>>,
    stdout_ptr: Instruction<'static>,
    stdout_len: Instruction<'static>,
    exit_local: u32,
) {
    for instruction in [
        Instruction::I32Const(RESULT_AREA as i32),
        Instruction::I32Const(0),
        Instruction::I32Store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }),
        Instruction::I32Const(RESULT_AREA as i32),
        stdout_ptr,
        store(8),
        Instruction::I32Const(RESULT_AREA as i32),
        stdout_len,
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

/// Builds the guest core module for one scenario. Function index space:
/// 0 = imported realloc, 1.. = the scenario's lowered `http@0.2.0` calls,
/// then `run`, then the no-op post-return.
fn scenario_module(scenario: Scenario) -> Vec<u8> {
    let functions = scenario_functions(scenario);
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    for (_, (params, results)) in &functions {
        types.ty().function(params.clone(), results.clone());
    }
    let run_type_index = (1 + functions.len()) as u32;
    // The lifted `run` result contains memory types, so the core function
    // returns one i32: the pointer to its 32-byte result area.
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    let nop_type_index = run_type_index + 1;
    // cabi_post_run receives the core result of `run` (the area pointer).
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
    for (type_index, (name, _)) in (1_u32..).zip(functions.iter()) {
        imports.import(HTTP2_INTERFACE, name, EntityType::Function(type_index));
    }
    let run_index = (1 + functions.len()) as u32;
    let nop_index = run_index + 1;

    let mut function_section = FunctionSection::new();
    function_section.function(run_type_index);
    function_section.function(nop_type_index);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    exports.export("run", ExportKind::Func, run_index);
    exports.export("cabi_post_run", ExportKind::Func, nop_index);
    let mut code = CodeSection::new();
    code.function(&run_body(scenario, run_index));
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

fn push_url_prologue(instructions: &mut Vec<Instruction<'static>>) {
    // Flattened script-input: param 0 = arguments array pointer, whose
    // first element is the single url string: (ptr @+0, len @+4).
    for instruction in [
        Instruction::LocalGet(0),
        load(0),
        Instruction::LocalSet(11),
        Instruction::LocalGet(0),
        load(4),
        Instruction::LocalSet(12),
    ] {
        instructions.push(instruction);
    }
}

/// Reads `exit = 110 + discriminant` into `exit_local` from an i32 area
/// slot and emits the six-result ok(script-output) return with empty
/// stdout/stderr.
/// Emits the return sequence: a zero-tag (ok) script-output with empty
/// stdout/stderr and `exit = 110 + discriminant`, then returns the area
/// pointer. Area layout: tag byte @0; stdout @8/12; stderr @16/20;
/// exit-code i64 @24; 32 bytes total.
fn push_error_exit(
    instructions: &mut Vec<Instruction<'static>>,
    exit_local: u32,
    disc_area_offset: u32,
) {
    for instruction in [
        Instruction::I32Const(disc_area_offset as i32),
        load(0),
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

fn push_status_exit(
    instructions: &mut Vec<Instruction<'static>>,
    status_area_offset: u32,
    exit_local: u32,
) {
    for instruction in [
        Instruction::I32Const(status_area_offset as i32),
        Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }),
        Instruction::I64Const(200),
        Instruction::I64Sub,
        Instruction::I64Const(100),
        Instruction::I64DivU,
        Instruction::LocalSet(exit_local),
    ] {
        instructions.push(instruction);
    }
}

fn run_body(scenario: Scenario, _run_index: u32) -> Function {
    let mut body = Function::new(match scenario {
        Scenario::BufferedGet => vec![
            (1, ValType::I32),
            (1, ValType::I64),
            (7, ValType::I32),
            (1, ValType::I64),
        ],
        Scenario::BufferedGetTwice => vec![
            (1, ValType::I32),
            (1, ValType::I64),
            (7, ValType::I32),
            (1, ValType::I64),
            (1, ValType::I32),
        ],
        Scenario::StreamingGet | Scenario::Upload => vec![
            (1, ValType::I32),
            (1, ValType::I64),
            (4, ValType::I32),
            (3, ValType::I32),
            (3, ValType::I32),
            (1, ValType::I64),
        ],
    });
    let mut instructions: Vec<Instruction<'static>> = Vec::new();
    match scenario {
        Scenario::BufferedGet => buffered_get_body(&mut instructions, false),
        Scenario::BufferedGetTwice => buffered_get_body(&mut instructions, true),
        Scenario::StreamingGet => streaming_body(&mut instructions, false),
        Scenario::Upload => streaming_body(&mut instructions, true),
    }
    for instruction in instructions {
        body.instruction(&instruction);
    }
    body.instruction(&Instruction::End);
    body
}

/// Local map (BufferedGet/…Twice): 4 tag, 5 status:i64, 6 bptr, 7 blen,
/// 8 scratch, 9 urllen2, 10 argsptr, 11 urlptr, 12 urllen, 13 exit:i64
/// (…Twice adds 14 total).
fn buffered_get_body(instructions: &mut Vec<Instruction<'static>>, twice: bool) {
    push_url_prologue(instructions);
    let request_index = 1_u32;
    // Call: (GET, url, empty headers, follow=0, retry=1, timeout, empty
    // body, result area). Area layout: tag@0, status@8, hptr@16, hlen@20,
    // body ptr@24, len@28.
    let call = |instructions: &mut Vec<Instruction<'static>>| {
        for instruction in [
            Instruction::I32Const(GET_STRING_OFFSET as i32),
            Instruction::I32Const(3),
            Instruction::LocalGet(11),
            Instruction::LocalGet(12),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(1),
            Instruction::I64Const(TIMEOUT_MS as i64),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(AREA_REQUEST as i32),
            Instruction::Call(request_index),
            Instruction::I32Const(AREA_REQUEST as i32),
            load(0),
            Instruction::LocalSet(4),
            Instruction::LocalGet(4),
            Instruction::If(BlockType::Empty),
        ] {
            instructions.push(instruction);
        }
        push_error_exit(instructions, 13, AREA_REQUEST + 8);
        instructions.push(Instruction::End);
        for instruction in [
            Instruction::I32Const(AREA_REQUEST as i32),
            Instruction::I64Load(MemArg {
                offset: 8,
                align: 3,
                memory_index: 0,
            }),
            Instruction::LocalSet(5),
            Instruction::I32Const(AREA_REQUEST as i32),
            load(24),
            Instruction::LocalSet(6),
            Instruction::I32Const(AREA_REQUEST as i32),
            load(28),
            Instruction::LocalSet(7),
        ] {
            instructions.push(instruction);
        }
    };
    let copy_body = |instructions: &mut Vec<Instruction<'static>>, total_local: u32| {
        for instruction in [
            Instruction::I32Const(SCRATCH_OFFSET as i32),
            Instruction::LocalGet(total_local),
            Instruction::I32Add,
            Instruction::LocalGet(6),
            Instruction::LocalGet(7),
            Instruction::MemoryCopy {
                src_mem: 0,
                dst_mem: 0,
            },
            Instruction::LocalGet(total_local),
            Instruction::LocalGet(7),
            Instruction::I32Add,
            Instruction::LocalSet(total_local),
        ] {
            instructions.push(instruction);
        }
    };
    call(instructions);
    if twice {
        copy_body(instructions, 14);
        call(instructions);
        copy_body(instructions, 14);
        // exit = status/100 - 1 (from the second response).
        push_status_exit(instructions, AREA_REQUEST + 8, 13);
        push_ok_return(
            instructions,
            Instruction::I32Const(SCRATCH_OFFSET as i32),
            Instruction::LocalGet(14),
            13,
        );
    } else {
        push_status_exit(instructions, AREA_REQUEST + 8, 13);
        push_ok_return(
            instructions,
            Instruction::LocalGet(6),
            Instruction::LocalGet(7),
            13,
        );
    }
}

/// Local map (StreamingGet/Upload): 4 tag, 5 status:i64, 6 scratch,
/// 7 blen, 8 handle, 9 total, 10 argsptr, 11 urlptr, 12 urllen, 13 rtag,
/// 14 rptr/disc, 15 rlen, 16 exit:i64.
fn streaming_body(instructions: &mut Vec<Instruction<'static>>, upload: bool) {
    push_url_prologue(instructions);
    if upload {
        // open-upload(method, url, options, area): tag@0, handle@4, err@4.
        for instruction in [
            Instruction::I32Const(POST_STRING_OFFSET as i32),
            Instruction::I32Const(4),
            Instruction::LocalGet(11),
            Instruction::LocalGet(12),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I64Const(TIMEOUT_MS as i64),
            Instruction::I32Const(AREA_OPEN as i32),
            Instruction::Call(1),
        ] {
            instructions.push(instruction);
        }
        instructions.push(Instruction::I32Const(AREA_OPEN as i32));
        instructions.push(load(0));
        instructions.push(Instruction::LocalSet(4));
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::If(BlockType::Empty));
        push_error_exit(instructions, 16, AREA_OPEN + 4);
        instructions.push(Instruction::End);
        instructions.push(Instruction::I32Const(AREA_OPEN as i32));
        instructions.push(load(4));
        instructions.push(Instruction::LocalSet(8));
        // write(handle, payload, len, area): tag@0, err disc@4.
        for instruction in [
            Instruction::LocalGet(8),
            Instruction::I32Const(UPLOAD_PAYLOAD_OFFSET as i32),
            Instruction::I32Const(32),
            Instruction::I32Const(AREA_WRITE as i32),
            Instruction::Call(2),
            Instruction::I32Const(AREA_WRITE as i32),
            load(0),
            Instruction::LocalSet(13),
            Instruction::LocalGet(13),
            Instruction::If(BlockType::Empty),
        ] {
            instructions.push(instruction);
        }
        push_error_exit(instructions, 16, AREA_WRITE + 4);
        instructions.push(Instruction::End);
        // finish(handle, area): tag@0, status@8, hptr@16, hlen@20, handle@24.
        for instruction in [
            Instruction::LocalGet(8),
            Instruction::I32Const(AREA_FINISH as i32),
            Instruction::Call(3),
            Instruction::I32Const(AREA_FINISH as i32),
            load(0),
            Instruction::LocalSet(4),
            Instruction::LocalGet(4),
            Instruction::If(BlockType::Empty),
        ] {
            instructions.push(instruction);
        }
        push_error_exit(instructions, 16, AREA_FINISH + 8);
        instructions.push(Instruction::End);
        for instruction in [
            Instruction::I32Const(AREA_FINISH as i32),
            load(24),
            Instruction::LocalSet(8),
            Instruction::I32Const(0),
            Instruction::LocalSet(9),
        ] {
            instructions.push(instruction);
        }
    } else {
        // request-streaming(area): tag@0, status@8, hptr@16, hlen@20,
        // handle@24.
        for instruction in [
            Instruction::I32Const(GET_STRING_OFFSET as i32),
            Instruction::I32Const(3),
            Instruction::LocalGet(11),
            Instruction::LocalGet(12),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I64Const(TIMEOUT_MS as i64),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::I32Const(AREA_REQUEST as i32),
            Instruction::Call(1),
            Instruction::I32Const(AREA_REQUEST as i32),
            load(0),
            Instruction::LocalSet(4),
            Instruction::LocalGet(4),
            Instruction::If(BlockType::Empty),
        ] {
            instructions.push(instruction);
        }
        push_error_exit(instructions, 16, AREA_REQUEST + 8);
        instructions.push(Instruction::End);
        for instruction in [
            Instruction::I32Const(AREA_REQUEST as i32),
            load(24),
            Instruction::LocalSet(8),
            Instruction::I32Const(0),
            Instruction::LocalSet(9),
        ] {
            instructions.push(instruction);
        }
    }
    // Incremental read loop: read(handle, max, area) -> tag@0, ptr@4,
    // len@8, err disc@4.
    instructions.push(Instruction::Block(BlockType::Empty));
    instructions.push(Instruction::Loop(BlockType::Empty));
    for instruction in [
        Instruction::LocalGet(8),
        Instruction::I64Const(i64::from(SCRATCH_CAPACITY)),
        Instruction::I32Const(AREA_READ as i32),
        Instruction::Call(if upload { 4 } else { 2 }),
        Instruction::I32Const(AREA_READ as i32),
        load(0),
        Instruction::LocalSet(13),
        Instruction::LocalGet(13),
        Instruction::If(BlockType::Empty),
    ] {
        instructions.push(instruction);
    }
    push_error_exit(instructions, 16, AREA_READ + 4);
    instructions.push(Instruction::End);
    for instruction in [
        Instruction::I32Const(AREA_READ as i32),
        load(4),
        Instruction::LocalSet(14),
        Instruction::I32Const(AREA_READ as i32),
        load(8),
        Instruction::LocalSet(15),
        Instruction::LocalGet(15),
        Instruction::I32Eqz,
        Instruction::BrIf(1),
        Instruction::LocalGet(9),
        Instruction::LocalGet(15),
        Instruction::I32Add,
        Instruction::I32Const(SCRATCH_CAPACITY as i32),
        Instruction::I32GtU,
        Instruction::If(BlockType::Empty),
        Instruction::Unreachable,
        Instruction::End,
        Instruction::I32Const(SCRATCH_OFFSET as i32),
        Instruction::LocalGet(9),
        Instruction::I32Add,
        Instruction::LocalGet(14),
        Instruction::LocalGet(15),
        Instruction::MemoryCopy {
            src_mem: 0,
            dst_mem: 0,
        },
        Instruction::LocalGet(9),
        Instruction::LocalGet(15),
        Instruction::I32Add,
        Instruction::LocalSet(9),
        Instruction::Br(0),
    ] {
        instructions.push(instruction);
    }
    instructions.push(Instruction::End);
    instructions.push(Instruction::End);
    // exit = status/100 - 1 from the response head area; stdout = scratch.
    let status_area = if upload {
        AREA_FINISH + 8
    } else {
        AREA_REQUEST + 8
    };
    push_status_exit(instructions, status_area, 16);
    push_ok_return(
        instructions,
        Instruction::I32Const(SCRATCH_OFFSET as i32),
        Instruction::LocalGet(9),
        16,
    );
}

// ---------------------------------------------------------------------------
// Runner-level tests.
// ---------------------------------------------------------------------------

fn secure_grants(endpoints: &[String]) -> NetGrants {
    let mut net = NetGrants::default();
    for endpoint in endpoints {
        net.grant_secure(endpoint).expect("fixture grant parses");
    }
    net
}

fn run_scenario(scenario: Scenario, url: &str, net: &NetGrants, policy: &HttpPolicy) -> RunOutcome {
    let component = fixture_component(scenario);
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_policy(&component, &FsGrants::default(), net, policy)
        .expect("fixture component links");
    prepared
        .run(
            &ScriptInput {
                arguments: vec![url.to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

fn output_of(outcome: RunOutcome) -> (Vec<u8>, i64) {
    match outcome {
        RunOutcome::Output(output) => (output.stdout, output.exit_code),
        other => panic!("expected guest output, got {other:?}"),
    }
}

#[test]
fn buffered_requests_roundtrip_and_pool_within_one_store() {
    let mut server = HttpFixtureServer::start(vec![
        ResponsePlan::Length {
            status: 200,
            body: b"alpha-".to_vec(),
        },
        ResponsePlan::Length {
            status: 200,
            body: b"beta".to_vec(),
        },
    ]);
    let url = format!("https+private://localhost:{}", server.port);
    let net = secure_grants(std::slice::from_ref(&url));
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let outcome = run_scenario(Scenario::BufferedGetTwice, &url, &net, &policy);
    let (stdout, exit) = output_of(outcome);
    assert_eq!(stdout, b"alpha-beta".to_vec(), "both bodies concatenated");
    assert_eq!(exit, 0);
    assert_eq!(
        server.connection_count(),
        1,
        "the second request must reuse the pooled connection inside one Store"
    );
    assert_eq!(server.request_count(), 2);
    server.stop();
}

#[test]
fn streaming_chunked_download_is_byte_exact() {
    let mut server = HttpFixtureServer::start(vec![ResponsePlan::Chunked {
        status: 200,
        chunks: vec![
            b"stream-alpha-".to_vec(),
            b"stream-beta-".to_vec(),
            b"stream-gamma".to_vec(),
        ],
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let net = secure_grants(std::slice::from_ref(&url));
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let outcome = run_scenario(Scenario::StreamingGet, &url, &net, &policy);
    let (stdout, exit) = output_of(outcome);
    assert_eq!(stdout, b"stream-alpha-stream-beta-stream-gamma".to_vec());
    assert_eq!(exit, 0);
    server.stop();
}

#[test]
fn ungranted_endpoint_is_typed_permission_refusal() {
    let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
        status: 200,
        body: b"must-not-arrive".to_vec(),
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let outcome = run_scenario(Scenario::BufferedGet, &url, &NetGrants::default(), &policy);
    let (stdout, exit) = output_of(outcome);
    assert!(stdout.is_empty());
    assert_eq!(exit, 110, "permission is discriminant 0: {exit}");
    assert_eq!(server.connection_count(), 0);
    server.stop();
}

#[test]
fn upload_roundtrips_with_secret_injection() {
    let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
        status: 200,
        body: b"accepted".to_vec(),
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let net = secure_grants(std::slice::from_ref(&url));
    let mut secrets = SecretStore::new();
    secrets.insert("token", "canary-token-value-9f2b");
    secrets.authorize(SecretBinding {
        name: "token".to_owned(),
        endpoint: format!("https+private|localhost|{}", server.port),
        policy: InjectionPolicy::Header {
            name: "x-api-key".to_owned(),
        },
    });
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone())
        .with_secrets(std::sync::Arc::new(secrets));
    let outcome = run_scenario(Scenario::Upload, &url, &net, &policy);
    let (stdout, exit) = output_of(outcome);
    assert_eq!(
        stdout,
        b"accepted".to_vec(),
        "the finished upload's reply streams back"
    );
    assert_eq!(exit, 0);
    let observed = server.observed();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].method, "POST");
    assert!(
        observed[0]
            .headers
            .iter()
            .any(|(name, value)| name == "transfer-encoding" && value == "chunked"),
        "the upload body must arrive chunked: {:?}",
        observed[0].headers
    );
    assert!(
        observed[0]
            .headers
            .iter()
            .any(|(name, value)| name == "x-api-key" && value == "canary-token-value-9f2b"),
        "the Host injects the authorized secret; the guest never sees it"
    );
    server.stop();
}

#[test]
fn cancellation_mid_stream_is_typed_cancel() {
    let mut server = HttpFixtureServer::start(vec![ResponsePlan::SlowBody {
        status: 200,
        body: vec![0xab; 8 * 256],
        chunk: 256,
        delay_ms: 300,
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let net = secure_grants(std::slice::from_ref(&url));
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let component = fixture_component(Scenario::StreamingGet);
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_policy(&component, &FsGrants::default(), &net, &policy)
        .expect("fixture links");
    let cancel = CancelToken::new();
    let canceller = cancel.clone();
    let started = std::time::Instant::now();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(700));
        canceller.request(sico_runner::CancellationSource::Client);
    });
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec![url],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &cancel,
        )
        .expect("input bounds hold");
    // M10/M11 semantics: once the run token fires, the arbiter commits the
    // run-level Cancelled outcome (the guest's typed read error is
    // overridden by design).
    assert!(matches!(outcome, RunOutcome::Cancelled), "{outcome:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(3),
        "cancellation must not wait out the server's full body: {:?}",
        started.elapsed()
    );
    server.stop();
}

#[test]
fn connection_closed_before_response_is_typed_protocol() {
    let mut server = HttpFixtureServer::start(vec![
        ResponsePlan::CloseBeforeResponse,
        ResponsePlan::Length {
            status: 200,
            body: b"unused".to_vec(),
        },
    ]);
    let url = format!("https+private://localhost:{}", server.port);
    let net = secure_grants(std::slice::from_ref(&url));
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let outcome = run_scenario(Scenario::StreamingGet, &url, &net, &policy);
    let (_stdout, exit) = output_of(outcome);
    assert_eq!(
        exit, 116,
        "protocol is discriminant 6 (no retry: streaming POST-less GET keeps the flag off): {exit}"
    );
    server.stop();
}
