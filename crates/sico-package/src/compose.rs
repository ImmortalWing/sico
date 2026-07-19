//! Versioned Script Adapter and Program/Adapter composition (STEP-0080).
//!
//! The adapter is the only Script component allowed to import the broad WASI
//! CLI environment/stdio interfaces (RFC-0029). It translates WASI CLI
//! arguments/stdin/stdout/stderr to and from the frozen
//! `sico:script/program@0.1.0` boundary and exports a `run` function; the
//! composed command wires a Program Component, the adapter and the WASI
//! imports and exports the `wasi:cli/run@0.2.12` instance.
//!
//! WASI 0.2 exit semantics are `result<_, _>` only, so the composed command
//! path collapses guest exit values to 0/1; exact `0..=119` guest codes are a
//! direct-runner contract (STEP-0081/0082), not an adapter claim.

#![forbid(unsafe_code)]

use wasm_encoder::{
    BlockType, CanonicalOption, CodeSection, ComponentBuilder, ComponentExportKind,
    ComponentTypeRef, ComponentValType, ConstExpr, ExportKind, Function, FunctionSection,
    GlobalSection, GlobalType, ImportSection, Instruction, MemArg, MemorySection, MemoryType,
    Module, ModuleArg, PrimitiveValType, TypeSection, ValType,
};

/// Versioned adapter identity recorded in manifest v1 and cache keys.
pub const SCRIPT_ADAPTER_ID: &str = "sico:script/adapter@0.1.0";
/// WASI interface version satisfied by the Wasmtime 46.0.1 CLI and pinned for
/// the composed command boundary.
pub const WASI_CLI_VERSION: &str = "0.2.12";

pub(crate) const TYPES_INSTANCE: &str = "sico:script/types@0.1.0";
pub(crate) const PROGRAM_RUN_IMPORT: &str = "program-run";
const STDIN_LIMIT: i32 = 8 * 1024 * 1024;
const READ_CHUNK: i64 = 65_536;
const SCRATCH: i32 = 1024;
const HEAP_BASE: i32 = 2048;
const MEMORY_MIN_PAGES: u64 = 512;
const MEMORY_MAX_PAGES: u64 = 1024;

/// Builds the deterministic versioned Script Adapter Component.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn script_adapter_component() -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let types_ty = builder.type_instance(
        Some("script-types"),
        &sico_codegen_wasm::script_types_instance(),
    );
    let types_instance = builder.import(TYPES_INSTANCE, ComponentTypeRef::Instance(types_ty));
    let input_ty = builder.alias_export(types_instance, "script-input", ComponentExportKind::Type);
    let result_ty =
        builder.alias_export(types_instance, "script-result", ComponentExportKind::Type);
    let (run_ty, mut run) = builder.type_function(Some("program-run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));
    let program_run = builder.import(PROGRAM_RUN_IMPORT, ComponentTypeRef::Func(run_ty));

    let error = import_wasi(&mut builder, "wasi:io/error", &error_instance());
    let stream_error_resource = builder.alias_export(error, "error", ComponentExportKind::Type);
    let environment = import_wasi(
        &mut builder,
        "wasi:cli/environment",
        &environment_instance(),
    );
    let get_arguments =
        builder.alias_export(environment, "get-arguments", ComponentExportKind::Func);
    let streams = import_wasi(
        &mut builder,
        "wasi:io/streams",
        &streams_instance(stream_error_resource),
    );
    let input_stream = builder.alias_export(streams, "input-stream", ComponentExportKind::Type);
    let output_stream = builder.alias_export(streams, "output-stream", ComponentExportKind::Type);
    let stdin = import_wasi(
        &mut builder,
        "wasi:cli/stdin",
        &stream_handle_instance("get-stdin", input_stream),
    );
    let get_stdin = builder.alias_export(stdin, "get-stdin", ComponentExportKind::Func);
    let stdout = import_wasi(
        &mut builder,
        "wasi:cli/stdout",
        &stream_handle_instance("get-stdout", output_stream),
    );
    let get_stdout = builder.alias_export(stdout, "get-stdout", ComponentExportKind::Func);
    let stderr = import_wasi(
        &mut builder,
        "wasi:cli/stderr",
        &stream_handle_instance("get-stderr", output_stream),
    );
    let get_stderr = builder.alias_export(stderr, "get-stderr", ComponentExportKind::Func);
    let blocking_read = builder.alias_export(
        streams,
        "[method]input-stream.blocking-read",
        ComponentExportKind::Func,
    );
    let blocking_write = builder.alias_export(
        streams,
        "[method]output-stream.blocking-write-and-flush",
        ComponentExportKind::Func,
    );
    let exit_interface = import_wasi(&mut builder, "wasi:cli/exit", &exit_instance());
    let exit = builder.alias_export(exit_interface, "exit", ComponentExportKind::Func);

    let memory_module = builder.core_module_raw(Some("adapter-memory"), &memory_module());
    let memory_instance = builder.core_instantiate(
        Some("adapter-memory"),
        memory_module,
        std::iter::empty::<(&str, ModuleArg)>(),
    );
    let memory = builder.core_alias_export(
        Some("memory"),
        memory_instance,
        "memory",
        ExportKind::Memory,
    );
    let realloc = builder.core_alias_export(
        Some("realloc"),
        memory_instance,
        "realloc",
        ExportKind::Func,
    );

    let program_run_l = builder.lower_func(
        Some("program-run-lowered"),
        program_run,
        [
            CanonicalOption::UTF8,
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );
    let get_arguments_l = builder.lower_func(
        Some("get-arguments-lowered"),
        get_arguments,
        [
            CanonicalOption::UTF8,
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );
    let get_stdin_l = builder.lower_func(Some("get-stdin-lowered"), get_stdin, []);
    let get_stdout_l = builder.lower_func(Some("get-stdout-lowered"), get_stdout, []);
    let get_stderr_l = builder.lower_func(Some("get-stderr-lowered"), get_stderr, []);
    let blocking_read_l = builder.lower_func(
        Some("blocking-read-lowered"),
        blocking_read,
        [
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );
    let blocking_write_l = builder.lower_func(
        Some("blocking-write-lowered"),
        blocking_write,
        [
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );
    let exit_l = builder.lower_func(Some("exit-lowered"), exit, []);

    let adapter_imports = builder.core_instantiate_exports(
        Some("adapter-imports"),
        [
            ("memory", ExportKind::Memory, memory),
            ("realloc", ExportKind::Func, realloc),
        ],
    );
    let wasi_imports = builder.core_instantiate_exports(
        Some("wasi-imports"),
        [
            ("program_run", ExportKind::Func, program_run_l),
            ("get_arguments", ExportKind::Func, get_arguments_l),
            ("get_stdin", ExportKind::Func, get_stdin_l),
            ("get_stdout", ExportKind::Func, get_stdout_l),
            ("get_stderr", ExportKind::Func, get_stderr_l),
            ("blocking_read", ExportKind::Func, blocking_read_l),
            ("blocking_write", ExportKind::Func, blocking_write_l),
            ("exit", ExportKind::Func, exit_l),
        ],
    );
    let trampoline = builder.core_module_raw(Some("adapter-trampoline"), &trampoline_module());
    let trampoline_instance = builder.core_instantiate(
        Some("adapter-trampoline"),
        trampoline,
        [
            ("adapter", ModuleArg::Instance(adapter_imports)),
            ("wasi", ModuleArg::Instance(wasi_imports)),
        ],
    );
    let run_core =
        builder.core_alias_export(Some("run"), trampoline_instance, "run", ExportKind::Func);
    let (unit_result, result) = builder.type_defined(Some("unit-result"));
    result.result(None, None);
    let (command_ty, mut command) = builder.type_function(Some("run"));
    command.params(std::iter::empty::<(&str, ComponentValType)>());
    command.result(Some(ComponentValType::Type(unit_result)));
    let lifted = builder.lift_func(Some("run"), run_core, command_ty, []);
    builder.export("run", ComponentExportKind::Func, lifted, None);
    builder.finish()
}

/// Composes a Program Component with the versioned adapter and the WASI CLI
/// imports into one command Component exporting `wasi:cli/run@0.2.12`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn compose_script_command(program: &[u8], adapter: &[u8]) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let types_ty = builder.type_instance(
        Some("script-types"),
        &sico_codegen_wasm::script_types_instance(),
    );
    let types_instance = builder.import(TYPES_INSTANCE, ComponentTypeRef::Instance(types_ty));
    let error = import_wasi(&mut builder, "wasi:io/error", &error_instance());
    let stream_error_resource = builder.alias_export(error, "error", ComponentExportKind::Type);
    let environment = import_wasi(
        &mut builder,
        "wasi:cli/environment",
        &environment_instance(),
    );
    let streams = import_wasi(
        &mut builder,
        "wasi:io/streams",
        &streams_instance(stream_error_resource),
    );
    let input_stream = builder.alias_export(streams, "input-stream", ComponentExportKind::Type);
    let output_stream = builder.alias_export(streams, "output-stream", ComponentExportKind::Type);
    let stdin = import_wasi(
        &mut builder,
        "wasi:cli/stdin",
        &stream_handle_instance("get-stdin", input_stream),
    );
    let stdout = import_wasi(
        &mut builder,
        "wasi:cli/stdout",
        &stream_handle_instance("get-stdout", output_stream),
    );
    let stderr = import_wasi(
        &mut builder,
        "wasi:cli/stderr",
        &stream_handle_instance("get-stderr", output_stream),
    );
    let exit_interface = import_wasi(&mut builder, "wasi:cli/exit", &exit_instance());

    let program_component = builder.component_raw(Some("program"), program);
    let program_instance = builder.instantiate(
        Some("program"),
        program_component,
        [(
            TYPES_INSTANCE,
            ComponentExportKind::Instance,
            types_instance,
        )],
    );
    let program_run = builder.alias_export(program_instance, "run", ComponentExportKind::Func);

    let adapter_component = builder.component_raw(Some("adapter"), adapter);
    let adapter_instance = builder.instantiate(
        Some("adapter"),
        adapter_component,
        [
            (
                TYPES_INSTANCE.to_owned(),
                ComponentExportKind::Instance,
                types_instance,
            ),
            (
                PROGRAM_RUN_IMPORT.to_owned(),
                ComponentExportKind::Func,
                program_run,
            ),
            (
                environment_name("wasi:io/error"),
                ComponentExportKind::Instance,
                error,
            ),
            (
                environment_name("wasi:cli/environment"),
                ComponentExportKind::Instance,
                environment,
            ),
            (
                environment_name("wasi:cli/stdin"),
                ComponentExportKind::Instance,
                stdin,
            ),
            (
                environment_name("wasi:cli/stdout"),
                ComponentExportKind::Instance,
                stdout,
            ),
            (
                environment_name("wasi:cli/stderr"),
                ComponentExportKind::Instance,
                stderr,
            ),
            (
                environment_name("wasi:io/streams"),
                ComponentExportKind::Instance,
                streams,
            ),
            (
                environment_name("wasi:cli/exit"),
                ComponentExportKind::Instance,
                exit_interface,
            ),
        ],
    );
    builder.export(
        environment_name("wasi:cli/run"),
        ComponentExportKind::Instance,
        adapter_instance,
        None,
    );
    builder.finish()
}

fn environment_name(interface: &str) -> String {
    format!("{interface}@{WASI_CLI_VERSION}")
}

fn import_wasi(
    builder: &mut ComponentBuilder,
    interface: &str,
    ty: &wasm_encoder::InstanceType,
) -> u32 {
    let ty = builder.type_instance(Some(interface), ty);
    builder.import(
        environment_name(interface).as_str(),
        ComponentTypeRef::Instance(ty),
    )
}

fn environment_instance() -> wasm_encoder::InstanceType {
    let mut instance = wasm_encoder::InstanceType::new();
    instance.ty().defined_type().list(PrimitiveValType::String);
    {
        let mut func = instance.ty().function();
        func.params(std::iter::empty::<(&str, ComponentValType)>());
        func.result(Some(ComponentValType::Type(0)));
    }
    instance.export("get-arguments", ComponentTypeRef::Func(1));
    instance
}

fn stream_handle_instance(export: &str, resource: u32) -> wasm_encoder::InstanceType {
    let mut instance = wasm_encoder::InstanceType::new();
    instance.alias(wasm_encoder::Alias::Outer {
        count: 1,
        kind: wasm_encoder::ComponentOuterAliasKind::Type,
        index: resource,
    });
    instance.ty().defined_type().own(0);
    {
        let mut func = instance.ty().function();
        func.params(std::iter::empty::<(&str, ComponentValType)>());
        func.result(Some(ComponentValType::Type(1)));
    }
    instance.export(export, ComponentTypeRef::Func(2));
    instance
}

/// `wasi:io/streams@0.2.x`: `stream-error` is a variant with an
/// `own<error>` payload (wasi:io/error); all three resources are named
/// component-level type imports aliased into the instance type.
/// `wasi:io/error@0.2.x`: the error resource, exported with a `SubResource`
/// bound so the streams instance can reference it in `stream-error`.
fn error_instance() -> wasm_encoder::InstanceType {
    let mut instance = wasm_encoder::InstanceType::new();
    instance.export(
        "error",
        ComponentTypeRef::Type(wasm_encoder::TypeBounds::SubResource),
    );
    instance
}

fn streams_instance(error: u32) -> wasm_encoder::InstanceType {
    let mut instance = wasm_encoder::InstanceType::new();
    instance.alias(wasm_encoder::Alias::Outer {
        count: 1,
        kind: wasm_encoder::ComponentOuterAliasKind::Type,
        index: error,
    });
    instance.export(
        "input-stream",
        ComponentTypeRef::Type(wasm_encoder::TypeBounds::SubResource),
    );
    instance.export(
        "output-stream",
        ComponentTypeRef::Type(wasm_encoder::TypeBounds::SubResource),
    );
    instance.ty().defined_type().own(0);
    instance.ty().defined_type().variant([
        ("last-operation-failed", Some(ComponentValType::Type(3))),
        ("closed", None),
    ]);
    instance.export(
        "stream-error",
        ComponentTypeRef::Type(wasm_encoder::TypeBounds::Eq(4)),
    );
    instance.ty().defined_type().borrow(1);
    instance.ty().defined_type().list(PrimitiveValType::U8);
    instance.ty().defined_type().result(
        Some(ComponentValType::Type(7)),
        Some(ComponentValType::Type(5)),
    );
    {
        let mut func = instance.ty().function();
        func.params([
            ("self", ComponentValType::Type(6)),
            ("len", ComponentValType::Primitive(PrimitiveValType::U64)),
        ]);
        func.result(Some(ComponentValType::Type(8)));
    }
    instance.ty().defined_type().borrow(2);
    instance
        .ty()
        .defined_type()
        .result(None, Some(ComponentValType::Type(5)));
    {
        let mut func = instance.ty().function();
        func.params([
            ("self", ComponentValType::Type(10)),
            ("contents", ComponentValType::Type(7)),
        ]);
        func.result(Some(ComponentValType::Type(11)));
    }
    instance.export(
        "[method]input-stream.blocking-read",
        ComponentTypeRef::Func(9),
    );
    instance.export(
        "[method]output-stream.blocking-write-and-flush",
        ComponentTypeRef::Func(12),
    );
    instance
}

fn exit_instance() -> wasm_encoder::InstanceType {
    let mut instance = wasm_encoder::InstanceType::new();
    instance.ty().defined_type().result(None, None);
    {
        let mut func = instance.ty().function();
        func.params([("status", ComponentValType::Type(0))]);
        func.result(None);
    }
    instance.export("exit", ComponentTypeRef::Func(1));
    instance
}

/// Adapter private memory: a bounded bump arena with a canonical realloc.
fn memory_module() -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    let mut functions = FunctionSection::new();
    functions.function(0);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: MEMORY_MIN_PAGES,
        maximum: Some(MEMORY_MAX_PAGES),
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
        &ConstExpr::i32_const(HEAP_BASE),
    );
    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("realloc", ExportKind::Func, 0);
    let mut realloc = Function::new(vec![(1, ValType::I32)]);
    realloc.instruction(&Instruction::GlobalGet(0));
    realloc.instruction(&Instruction::LocalGet(2));
    realloc.instruction(&Instruction::I32Add);
    realloc.instruction(&Instruction::I32Const(-1));
    realloc.instruction(&Instruction::I32Add);
    realloc.instruction(&Instruction::I32Const(0));
    realloc.instruction(&Instruction::LocalGet(2));
    realloc.instruction(&Instruction::I32Sub);
    realloc.instruction(&Instruction::I32And);
    realloc.instruction(&Instruction::LocalTee(4));
    realloc.instruction(&Instruction::LocalGet(3));
    realloc.instruction(&Instruction::I32Add);
    realloc.instruction(&Instruction::GlobalSet(0));
    realloc.instruction(&Instruction::LocalGet(4));
    realloc.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&realloc);
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    module.finish()
}

/// The adapter orchestration: read WASI arguments/stdin, invoke the lowered
/// Program `run`, write the channels and map the Script result to WASI exit
/// semantics. Scratch result areas live below the bump-arena base.
#[allow(clippy::too_many_lines)]
fn trampoline_module() -> Vec<u8> {
    const REALLOC: u32 = 0;
    const PROGRAM_RUN: u32 = 1;
    const GET_ARGUMENTS: u32 = 2;
    const GET_STDIN: u32 = 3;
    const GET_STDOUT: u32 = 4;
    const GET_STDERR: u32 = 5;
    const BLOCKING_READ: u32 = 6;
    const BLOCKING_WRITE: u32 = 7;
    const EXIT: u32 = 8;
    const WRITE_ALL: u32 = 9;

    let mut types = TypeSection::new();
    types.ty().function([], [ValType::I32]);
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
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
    types.ty().function([ValType::I32], []);
    types.ty().function([], [ValType::I32]);
    types
        .ty()
        .function([ValType::I32, ValType::I64, ValType::I32], []);
    types
        .ty()
        .function([ValType::I32, ValType::I32, ValType::I32, ValType::I32], []);
    types.ty().function([ValType::I32], []);
    types
        .ty()
        .function([ValType::I32, ValType::I32, ValType::I32], []);

    let mut imports = ImportSection::new();
    imports.import(
        "adapter",
        "memory",
        wasm_encoder::EntityType::Memory(MemoryType {
            minimum: MEMORY_MIN_PAGES,
            maximum: Some(MEMORY_MAX_PAGES),
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    imports.import("adapter", "realloc", wasm_encoder::EntityType::Function(1));
    imports.import("wasi", "program_run", wasm_encoder::EntityType::Function(2));
    imports.import(
        "wasi",
        "get_arguments",
        wasm_encoder::EntityType::Function(3),
    );
    imports.import("wasi", "get_stdin", wasm_encoder::EntityType::Function(4));
    imports.import("wasi", "get_stdout", wasm_encoder::EntityType::Function(4));
    imports.import("wasi", "get_stderr", wasm_encoder::EntityType::Function(4));
    imports.import(
        "wasi",
        "blocking_read",
        wasm_encoder::EntityType::Function(5),
    );
    imports.import(
        "wasi",
        "blocking_write",
        wasm_encoder::EntityType::Function(6),
    );
    imports.import("wasi", "exit", wasm_encoder::EntityType::Function(7));
    let mut functions = FunctionSection::new();
    functions.function(8);
    functions.function(0);
    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("run", ExportKind::Func, 10);

    // write_all(handle, ptr, len): one blocking write-and-flush, stream
    // errors collapse to exit(err).
    let mut write_all = Function::new(Vec::new());
    write_all.instruction(&Instruction::LocalGet(2));
    write_all.instruction(&Instruction::I32Eqz);
    write_all.instruction(&Instruction::If(BlockType::Empty));
    write_all.instruction(&Instruction::Return);
    write_all.instruction(&Instruction::End);
    write_all.instruction(&Instruction::LocalGet(0));
    write_all.instruction(&Instruction::LocalGet(1));
    write_all.instruction(&Instruction::LocalGet(2));
    write_all.instruction(&Instruction::I32Const(SCRATCH));
    write_all.instruction(&Instruction::Call(BLOCKING_WRITE));
    write_all.instruction(&Instruction::I32Const(SCRATCH));
    write_all.instruction(&Instruction::I32Load8U(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    write_all.instruction(&Instruction::If(BlockType::Empty));
    write_all.instruction(&Instruction::I32Const(1));
    write_all.instruction(&Instruction::Call(EXIT));
    write_all.instruction(&Instruction::End);
    write_all.instruction(&Instruction::End);

    let mut run = Function::new(vec![(11, ValType::I32), (1, ValType::I64)]);
    // ret = realloc(0, 0, 4, 8); get_arguments(ret)
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(4));
    run.instruction(&Instruction::I32Const(8));
    run.instruction(&Instruction::Call(REALLOC));
    run.instruction(&Instruction::LocalSet(0));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::Call(GET_ARGUMENTS));
    // args_ptr/args_len; skip argv[0]
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalSet(1));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalSet(2));
    run.instruction(&Instruction::LocalGet(2));
    run.instruction(&Instruction::If(BlockType::Empty));
    run.instruction(&Instruction::LocalGet(1));
    run.instruction(&Instruction::I32Const(8));
    run.instruction(&Instruction::I32Add);
    run.instruction(&Instruction::LocalSet(1));
    run.instruction(&Instruction::LocalGet(2));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::I32Sub);
    run.instruction(&Instruction::LocalSet(2));
    run.instruction(&Instruction::End);
    // buf = realloc(0, 0, 1, 8 MiB); h_in = get_stdin()
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::I32Const(STDIN_LIMIT));
    run.instruction(&Instruction::Call(REALLOC));
    run.instruction(&Instruction::LocalSet(3));
    run.instruction(&Instruction::Call(GET_STDIN));
    run.instruction(&Instruction::LocalSet(5));
    // read loop into buf with a hard 8 MiB bound
    run.instruction(&Instruction::Block(BlockType::Empty));
    run.instruction(&Instruction::Loop(BlockType::Empty));
    run.instruction(&Instruction::LocalGet(5));
    run.instruction(&Instruction::I64Const(READ_CHUNK));
    run.instruction(&Instruction::I32Const(SCRATCH));
    run.instruction(&Instruction::Call(BLOCKING_READ));
    // result tag: 0 = ok; 1 = stream-error, where the payload discriminant
    // distinguishes `closed` (clean EOF, case 1) from
    // `last-operation-failed` (case 0, fatal)
    run.instruction(&Instruction::I32Const(SCRATCH));
    run.instruction(&Instruction::I32Load8U(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    run.instruction(&Instruction::If(BlockType::Empty));
    run.instruction(&Instruction::I32Const(SCRATCH));
    run.instruction(&Instruction::I32Load8U(MemArg {
        offset: 4,
        align: 0,
        memory_index: 0,
    }));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::I32Eq);
    run.instruction(&Instruction::BrIf(2));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::Call(EXIT));
    run.instruction(&Instruction::End);
    run.instruction(&Instruction::I32Const(SCRATCH));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalSet(8));
    run.instruction(&Instruction::I32Const(SCRATCH));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 8,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalSet(9));
    run.instruction(&Instruction::LocalGet(9));
    run.instruction(&Instruction::I32Eqz);
    run.instruction(&Instruction::BrIf(1));
    run.instruction(&Instruction::LocalGet(4));
    run.instruction(&Instruction::LocalGet(9));
    run.instruction(&Instruction::I32Add);
    run.instruction(&Instruction::LocalTee(10));
    run.instruction(&Instruction::I32Const(STDIN_LIMIT));
    run.instruction(&Instruction::I32GtU);
    run.instruction(&Instruction::If(BlockType::Empty));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::Call(EXIT));
    run.instruction(&Instruction::End);
    run.instruction(&Instruction::LocalGet(3));
    run.instruction(&Instruction::LocalGet(4));
    run.instruction(&Instruction::I32Add);
    run.instruction(&Instruction::LocalGet(8));
    run.instruction(&Instruction::LocalGet(9));
    run.instruction(&Instruction::MemoryCopy {
        dst_mem: 0,
        src_mem: 0,
    });
    run.instruction(&Instruction::LocalGet(4));
    run.instruction(&Instruction::LocalGet(9));
    run.instruction(&Instruction::I32Add);
    run.instruction(&Instruction::LocalSet(4));
    run.instruction(&Instruction::Br(0));
    run.instruction(&Instruction::End);
    run.instruction(&Instruction::End);
    // ret32 = realloc(0, 0, 8, 32); program_run(args, buf, total, ret32)
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(8));
    run.instruction(&Instruction::I32Const(32));
    run.instruction(&Instruction::Call(REALLOC));
    run.instruction(&Instruction::LocalSet(0));
    run.instruction(&Instruction::LocalGet(1));
    run.instruction(&Instruction::LocalGet(2));
    run.instruction(&Instruction::LocalGet(3));
    run.instruction(&Instruction::LocalGet(4));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::Call(PROGRAM_RUN));
    // channels
    run.instruction(&Instruction::Call(GET_STDOUT));
    run.instruction(&Instruction::LocalSet(6));
    run.instruction(&Instruction::Call(GET_STDERR));
    run.instruction(&Instruction::LocalSet(7));
    // result mapping
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load8U(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    run.instruction(&Instruction::I32Eqz);
    run.instruction(&Instruction::If(BlockType::Empty));
    run.instruction(&Instruction::LocalGet(6));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 8,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::Call(WRITE_ALL));
    run.instruction(&Instruction::LocalGet(7));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 16,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 20,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::Call(WRITE_ALL));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I64Load(MemArg {
        offset: 24,
        align: 3,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalSet(11));
    run.instruction(&Instruction::LocalGet(11));
    run.instruction(&Instruction::I64Const(0));
    run.instruction(&Instruction::I64Ne);
    run.instruction(&Instruction::If(BlockType::Empty));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::Call(EXIT));
    run.instruction(&Instruction::End);
    run.instruction(&Instruction::Else);
    run.instruction(&Instruction::LocalGet(7));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 16,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::Call(WRITE_ALL));
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::Call(EXIT));
    run.instruction(&Instruction::End);
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::End);

    let mut code = CodeSection::new();
    code.function(&write_all);
    code.function(&run);
    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    module.section(&exports);
    module.section(&code);
    module.finish()
}

#[cfg(test)]
mod tests;
