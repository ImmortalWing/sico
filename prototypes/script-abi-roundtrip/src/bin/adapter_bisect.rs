//! Bisects the STEP-0080 adapter flow: v1 = get-arguments only,
//! v2 = + one blocking-read, v3 = + full read loop.

use std::env;
use std::fs;

use wasm_encoder::{
    BlockType, CanonicalOption, CodeSection, ComponentBuilder, ComponentExportKind,
    ComponentTypeRef, ComponentValType, ConstExpr, ExportKind, Function, FunctionSection,
    GlobalSection, GlobalType, ImportSection, Instruction, MemArg, MemorySection, MemoryType,
    Module, ModuleArg, PrimitiveValType, TypeSection, ValType,
};

const HEAP_BASE: i32 = 2048;
const SCRATCH: i32 = 1024;

fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "v1".into());
    let out = env::args().nth(2).unwrap_or_else(|| "bisect.wasm".into());
    let component = match mode.as_str() {
        "v1" => build(false, false),
        "v2" => build(true, false),
        _ => build(true, true),
    };
    fs::write(&out, component).unwrap();
    println!("wrote {out}");
}

fn build(with_read: bool, with_loop: bool) -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let environment = import_instance(
        &mut builder,
        "wasi:cli/environment@0.2.12",
        &environment_instance(),
    );
    let get_arguments =
        builder.alias_export(environment, "get-arguments", ComponentExportKind::Func);
    let error = import_instance(&mut builder, "wasi:io/error@0.2.12", &error_instance());
    let error_ty = builder.alias_export(error, "error", ComponentExportKind::Type);
    let streams = import_instance(
        &mut builder,
        "wasi:io/streams@0.2.12",
        &streams_instance(error_ty),
    );
    let input_stream = builder.alias_export(streams, "input-stream", ComponentExportKind::Type);
    let blocking_read = builder.alias_export(
        streams,
        "[method]input-stream.blocking-read",
        ComponentExportKind::Func,
    );
    let stdin = import_instance(
        &mut builder,
        "wasi:cli/stdin@0.2.12",
        &stream_handle_instance("get-stdin", input_stream),
    );
    let get_stdin = builder.alias_export(stdin, "get-stdin", ComponentExportKind::Func);

    let memory_module = builder.core_module_raw(Some("memory"), &memory_module());
    let memory_instance = builder.core_instantiate(
        Some("memory"),
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
    let get_arguments_l = builder.lower_func(
        Some("get-arguments-l"),
        get_arguments,
        [
            CanonicalOption::UTF8,
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );
    let get_stdin_l = builder.lower_func(Some("get-stdin-l"), get_stdin, []);
    let blocking_read_l = builder.lower_func(
        Some("blocking-read-l"),
        blocking_read,
        [
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );

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
            ("get_arguments", ExportKind::Func, get_arguments_l),
            ("get_stdin", ExportKind::Func, get_stdin_l),
            ("blocking_read", ExportKind::Func, blocking_read_l),
        ],
    );
    let trampoline =
        builder.core_module_raw(Some("trampoline"), &trampoline_module(with_read, with_loop));
    let trampoline_instance = builder.core_instantiate(
        Some("trampoline"),
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
    let inner_bytes = builder.finish();
    let mut outer = ComponentBuilder::default();
    let environment = import_instance(
        &mut outer,
        "wasi:cli/environment@0.2.12",
        &environment_instance(),
    );
    let error = import_instance(&mut outer, "wasi:io/error@0.2.12", &error_instance());
    let error_ty = outer.alias_export(error, "error", ComponentExportKind::Type);
    let streams = import_instance(
        &mut outer,
        "wasi:io/streams@0.2.12",
        &streams_instance(error_ty),
    );
    let input_stream = outer.alias_export(streams, "input-stream", ComponentExportKind::Type);
    let stdin = import_instance(
        &mut outer,
        "wasi:cli/stdin@0.2.12",
        &stream_handle_instance("get-stdin", input_stream),
    );
    let inner_component = outer.component_raw(Some("inner"), &inner_bytes);
    let inner_instance = outer.instantiate(
        Some("inner"),
        inner_component,
        [
            (
                "wasi:cli/environment@0.2.12",
                ComponentExportKind::Instance,
                environment,
            ),
            ("wasi:io/error@0.2.12", ComponentExportKind::Instance, error),
            (
                "wasi:io/streams@0.2.12",
                ComponentExportKind::Instance,
                streams,
            ),
            (
                "wasi:cli/stdin@0.2.12",
                ComponentExportKind::Instance,
                stdin,
            ),
        ],
    );
    outer.export(
        "wasi:cli/run@0.2.12",
        ComponentExportKind::Instance,
        inner_instance,
        None,
    );
    outer.finish()
}

fn import_instance(
    builder: &mut ComponentBuilder,
    name: &str,
    instance: &wasm_encoder::InstanceType,
) -> u32 {
    let ty = builder.type_instance(Some(name), instance);
    builder.import(name, ComponentTypeRef::Instance(ty))
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
    instance.ty().defined_type().own(0);
    instance.ty().defined_type().variant([
        ("last-operation-failed", Some(ComponentValType::Type(2))),
        ("closed", None),
    ]);
    instance.export(
        "stream-error",
        ComponentTypeRef::Type(wasm_encoder::TypeBounds::Eq(3)),
    );
    instance.ty().defined_type().borrow(1);
    instance.ty().defined_type().list(PrimitiveValType::U8);
    instance.ty().defined_type().result(
        Some(ComponentValType::Type(6)),
        Some(ComponentValType::Type(4)),
    );
    {
        let mut func = instance.ty().function();
        func.params([
            ("self", ComponentValType::Type(5)),
            ("len", ComponentValType::Primitive(PrimitiveValType::U64)),
        ]);
        func.result(Some(ComponentValType::Type(7)));
    }
    instance.export(
        "[method]input-stream.blocking-read",
        ComponentTypeRef::Func(8),
    );
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
        minimum: 64,
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

/// Returns 0 on success; 3 if get-arguments fails, 4 if read errors,
/// 5 when EOF, 6 when a chunk was read.
fn trampoline_module(with_read: bool, with_loop: bool) -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function([], [ValType::I32]);
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function([ValType::I32], []);
    types.ty().function([], [ValType::I32]);
    types
        .ty()
        .function([ValType::I32, ValType::I64, ValType::I32], []);

    let mut imports = ImportSection::new();
    imports.import(
        "adapter",
        "memory",
        wasm_encoder::EntityType::Memory(MemoryType {
            minimum: 64,
            maximum: Some(1024),
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    imports.import("adapter", "realloc", wasm_encoder::EntityType::Function(1));
    imports.import(
        "wasi",
        "get_arguments",
        wasm_encoder::EntityType::Function(2),
    );
    imports.import("wasi", "get_stdin", wasm_encoder::EntityType::Function(3));
    imports.import(
        "wasi",
        "blocking_read",
        wasm_encoder::EntityType::Function(4),
    );
    let mut functions = FunctionSection::new();
    functions.function(0);
    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("run", ExportKind::Func, 4);

    let mut run = Function::new(vec![(3, ValType::I32)]);
    // ret = realloc(0,0,4,8); get_arguments(ret)
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(4));
    run.instruction(&Instruction::I32Const(8));
    run.instruction(&Instruction::Call(0));
    run.instruction(&Instruction::LocalSet(0));
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::Call(1));
    if !with_read {
        run.instruction(&Instruction::I32Const(0));
        run.instruction(&Instruction::End);
    } else {
        // h = get_stdin()
        run.instruction(&Instruction::Call(2));
        run.instruction(&Instruction::LocalSet(1));
        if with_loop {
            run.instruction(&Instruction::Block(BlockType::Empty));
            run.instruction(&Instruction::Loop(BlockType::Empty));
        }
        // read(h, 65536, SCRATCH)
        run.instruction(&Instruction::LocalGet(1));
        run.instruction(&Instruction::I64Const(65_536));
        run.instruction(&Instruction::I32Const(SCRATCH));
        run.instruction(&Instruction::Call(3));
        // if tag != 0: return 1 (read error)
        run.instruction(&Instruction::I32Const(SCRATCH));
        run.instruction(&Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        run.instruction(&Instruction::If(BlockType::Empty));
        run.instruction(&Instruction::I32Const(1));
        run.instruction(&Instruction::Return);
        run.instruction(&Instruction::End);
        // len = load(SCRATCH+8)
        run.instruction(&Instruction::I32Const(SCRATCH));
        run.instruction(&Instruction::I32Load(MemArg {
            offset: 8,
            align: 2,
            memory_index: 0,
        }));
        run.instruction(&Instruction::LocalSet(2));
        if with_loop {
            // len == 0 (EOF): exit loop and return 0
            run.instruction(&Instruction::LocalGet(2));
            run.instruction(&Instruction::I32Eqz);
            run.instruction(&Instruction::BrIf(1));
            run.instruction(&Instruction::Br(0));
            run.instruction(&Instruction::End);
            run.instruction(&Instruction::End);
            run.instruction(&Instruction::I32Const(0));
            run.instruction(&Instruction::End);
        } else {
            // any successful read returns 0
            run.instruction(&Instruction::I32Const(0));
            run.instruction(&Instruction::End);
        }
    }
    let mut code = CodeSection::new();
    code.function(&run);
    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    module.section(&exports);
    module.section(&code);
    module.finish()
}
