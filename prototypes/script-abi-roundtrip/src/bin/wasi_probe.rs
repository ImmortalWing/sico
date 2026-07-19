//! Emits a minimal WASI command component probing the exact interface
//! versions the Wasmtime CLI satisfies: imports
//! `wasi:cli/environment@<version>#get-arguments` and exports
//! `wasi:cli/run@<version>#run` returning err unless argc == expected.

use std::env;
use std::fs;

use wasm_encoder::{
    BlockType, CanonicalOption, CodeSection, ComponentBuilder, ComponentExportKind,
    ComponentTypeRef, ComponentValType, ConstExpr, ExportKind, Function, FunctionSection,
    GlobalSection, GlobalType, Instruction, MemArg, MemorySection, MemoryType, Module, ModuleArg,
    PrimitiveValType, TypeSection, ValType,
};

fn main() {
    let version = env::args().nth(1).unwrap_or_else(|| "0.2.6".into());
    let expected_argc: i32 = env::args()
        .nth(2)
        .unwrap_or_else(|| "2".into())
        .parse()
        .unwrap();
    let out = env::args().nth(3).unwrap_or_else(|| "probe.wasm".into());

    let mut builder = ComponentBuilder::default();
    let (string_list, list_type) = builder.type_defined(Some("string-list"));
    list_type.list(PrimitiveValType::String);
    let string_list = builder.export("string-list", ComponentExportKind::Type, string_list, None);
    let (_arguments_ty, mut arguments) = builder.type_function(Some("get-arguments"));
    arguments.params(std::iter::empty::<(&str, ComponentValType)>());
    arguments.result(Some(ComponentValType::Type(string_list)));
    let mut instance = wasm_encoder::InstanceType::new();
    instance.ty().defined_type().list(PrimitiveValType::String);
    {
        let mut func = instance.ty().function();
        func.params(std::iter::empty::<(&str, ComponentValType)>());
        func.result(Some(ComponentValType::Type(0)));
    }
    instance.export("get-arguments", ComponentTypeRef::Func(1));
    let instance_ty = builder.type_instance(Some("environment"), &instance);
    let environment = builder.import(
        format!("wasi:cli/environment@{version}"),
        ComponentTypeRef::Instance(instance_ty),
    );
    let get_arguments =
        builder.alias_export(environment, "get-arguments", ComponentExportKind::Func);

    let mut core = ComponentCore::new();
    let core_module = builder.core_module_raw(Some("probe-core"), &core.take_bytes());
    let core_instance = builder.core_instantiate(
        Some("probe-core"),
        core_module,
        std::iter::empty::<(&str, ModuleArg)>(),
    );
    let memory =
        builder.core_alias_export(Some("memory"), core_instance, "memory", ExportKind::Memory);
    let realloc =
        builder.core_alias_export(Some("realloc"), core_instance, "realloc", ExportKind::Func);
    let lowered = builder.lower_func(
        Some("get-arguments-lowered"),
        get_arguments,
        [
            CanonicalOption::UTF8,
            CanonicalOption::Memory(memory),
            CanonicalOption::Realloc(realloc),
        ],
    );

    let trampoline = trampoline_bytes(expected_argc);
    let trampoline_module = builder.core_module_raw(Some("probe-trampoline"), &trampoline);
    let imports = builder.core_instantiate_exports(
        Some("probe-imports"),
        [
            ("memory", ExportKind::Memory, memory),
            ("realloc", ExportKind::Func, realloc),
            ("get_arguments", ExportKind::Func, lowered),
        ],
    );
    let trampoline_instance = builder.core_instantiate(
        Some("probe-trampoline"),
        trampoline_module,
        [("probe", ModuleArg::Instance(imports))],
    );
    let run_core =
        builder.core_alias_export(Some("run"), trampoline_instance, "run", ExportKind::Func);
    let (result_ty, result) = builder.type_defined(Some("unit-result"));
    result.result(None, None);
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params(std::iter::empty::<(&str, ComponentValType)>());
    run.result(Some(ComponentValType::Type(result_ty)));
    let lifted = builder.lift_func(Some("run"), run_core, run_ty, []);
    builder.export("run", ComponentExportKind::Func, lifted, None);
    let inner = builder.finish();

    // The Wasmtime CLI looks for an exported `wasi:cli/run@<version>`
    // instance; wrap the inner component so its `{ run }` instance export
    // carries the interface name.
    let mut outer = ComponentBuilder::default();
    let mut outer_instance = wasm_encoder::InstanceType::new();
    outer_instance
        .ty()
        .defined_type()
        .list(PrimitiveValType::String);
    {
        let mut func = outer_instance.ty().function();
        func.params(std::iter::empty::<(&str, ComponentValType)>());
        func.result(Some(ComponentValType::Type(0)));
    }
    outer_instance.export("get-arguments", ComponentTypeRef::Func(1));
    let outer_instance_ty = outer.type_instance(Some("environment"), &outer_instance);
    let environment = outer.import(
        format!("wasi:cli/environment@{version}"),
        ComponentTypeRef::Instance(outer_instance_ty),
    );
    let inner_component = outer.component_raw(Some("probe-inner"), &inner);
    let inner_instance = outer.instantiate(
        Some("probe-inner"),
        inner_component,
        [(
            format!("wasi:cli/environment@{version}").as_str(),
            ComponentExportKind::Instance,
            environment,
        )],
    );
    outer.export(
        format!("wasi:cli/run@{version}"),
        ComponentExportKind::Instance,
        inner_instance,
        None,
    );
    fs::write(&out, outer.finish()).unwrap();
    println!("wrote {out}");
}

struct ComponentCore {
    bytes: Vec<u8>,
}

impl ComponentCore {
    fn new() -> Self {
        let mut types = TypeSection::new();
        types.ty().function(
            [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            [ValType::I32],
        );
        let mut functions = FunctionSection::new();
        functions.function(0);
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 4,
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
            &ConstExpr::i32_const(2048),
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
        Self {
            bytes: module.finish(),
        }
    }

    fn take_bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.bytes)
    }
}

/// Core trampoline: imports memory/realloc/get-arguments, exports
/// `run: () -> i32` returning 0 when argc matches, 1 otherwise.
fn trampoline_bytes(expected_argc: i32) -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function([], [ValType::I32]);
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function([ValType::I32], []);
    let mut imports = wasm_encoder::ImportSection::new();
    imports.import(
        "probe",
        "memory",
        wasm_encoder::EntityType::Memory(MemoryType {
            minimum: 4,
            maximum: Some(1024),
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );
    imports.import("probe", "realloc", wasm_encoder::EntityType::Function(1));
    imports.import(
        "probe",
        "get_arguments",
        wasm_encoder::EntityType::Function(2),
    );
    let mut functions = FunctionSection::new();
    functions.function(0);
    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("run", ExportKind::Func, 2);
    let mut run = Function::new(vec![(1, ValType::I32)]);
    // ret = realloc(0, 0, 4, 8)
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::I32Const(4));
    run.instruction(&Instruction::I32Const(8));
    run.instruction(&Instruction::Call(0));
    run.instruction(&Instruction::LocalTee(0));
    // get_arguments(ret)
    run.instruction(&Instruction::Call(1));
    // load argc from ret+4, compare
    run.instruction(&Instruction::LocalGet(0));
    run.instruction(&Instruction::I32Load(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    run.instruction(&Instruction::I32Const(expected_argc));
    run.instruction(&Instruction::I32Eq);
    run.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
    run.instruction(&Instruction::I32Const(0));
    run.instruction(&Instruction::Else);
    run.instruction(&Instruction::I32Const(1));
    run.instruction(&Instruction::End);
    run.instruction(&Instruction::End);
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
