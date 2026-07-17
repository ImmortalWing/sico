//! Deterministic Core Wasm generation from independently verified Sico IR.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sico_ir::{
    Block, BlockId, Function as IrFunction, Module as IrModule, Operation, Terminator, Type,
    ValueId, VerifyError, verify,
};
use wasm_encoder::{
    BlockType, CanonicalOption, CodeSection, ComponentBuilder, ComponentExportKind,
    ComponentValType, ExportKind, ExportSection, Function, FunctionSection, Instruction, MemArg,
    MemorySection, MemoryType, Module, ModuleArg, PrimitiveValType, TypeSection, ValType,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodegenError {
    InvalidIr(Vec<VerifyError>),
    ModuleTooLarge {
        functions: usize,
    },
    AsyncUnsupported {
        function: String,
        feature: String,
        contract: &'static str,
    },
    Unsupported {
        function: String,
        feature: String,
    },
    IntegerOutsideProvenI64 {
        function: String,
        bytes: usize,
    },
}

/// Compiles the proven scalar/control subset to deterministic Core Wasm.
///
/// `Int` parameters and non-constant `Int` results are refused: RFC-0003 has
/// not accepted a fixed-width representation, so this backend only emits an
/// i64 when the mathematical value is compile-time proven to fit.
///
/// # Errors
///
/// Returns verifier errors or a typed unsupported/representation refusal.
pub fn compile(module: &IrModule) -> Result<Vec<u8>, CodegenError> {
    compile_core(module, CoreAbi::Direct)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CoreAbi {
    Direct,
    ComponentLift,
}

fn compile_core(module: &IrModule, abi: CoreAbi) -> Result<Vec<u8>, CodegenError> {
    let errors = verify(module);
    if !errors.is_empty() {
        return Err(CodegenError::InvalidIr(errors));
    }
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    for (index, function) in module.functions.iter().enumerate() {
        if !function.effects.is_empty() {
            return Err(unsupported(
                &function.name,
                "effectful function without a Component host adapter",
            ));
        }
        let function_index = u32::try_from(index).map_err(|_| CodegenError::ModuleTooLarge {
            functions: module.functions.len(),
        })?;
        let params = function
            .parameters
            .iter()
            .map(|parameter| lower_parameter_type(&function.name, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        let results = lower_result_type(&function.name, &function.return_type, abi)?;
        types.ty().function(params, results);
        functions.function(function_index);
        exports.export(&function.name, ExportKind::Func, function_index);
        code.function(&compile_function(function, abi)?);
    }
    let mut output = Module::new();
    output.section(&types);
    output.section(&functions);
    let needs_memory = abi == CoreAbi::ComponentLift
        && module
            .functions
            .iter()
            .any(|function| is_checked_fixed_result(&function.return_type));
    let mut memories = MemorySection::new();
    if needs_memory {
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        output.section(&memories);
        exports.export("memory", ExportKind::Memory, 0);
    }
    output.section(&exports);
    output.section(&code);
    Ok(output.finish())
}

/// Compiles the proven scalar/control subset to a deterministic WebAssembly Component.
///
/// Each Core Wasm export is canonically lifted to a root Component function.
/// The fixed-width numeric `Result` subset uses a private Canonical ABI return
/// area. Other aggregates, resources, strings, and async boundaries remain
/// refused until their memory and ownership adapters are implemented.
///
/// # Errors
///
/// Returns verifier errors or a typed unsupported/representation refusal.
pub fn compile_component(module: &IrModule) -> Result<Vec<u8>, CodegenError> {
    let core = compile_core(module, CoreAbi::ComponentLift)?;
    let mut builder = ComponentBuilder::default();
    let core_module = builder.core_module_raw(Some("sico-core"), &core);
    let core_instance = builder.core_instantiate(
        Some("sico-core"),
        core_module,
        std::iter::empty::<(&str, ModuleArg)>(),
    );
    let core_memory = module
        .functions
        .iter()
        .any(|function| is_checked_fixed_result(&function.return_type))
        .then(|| {
            builder.core_alias_export(Some("memory"), core_instance, "memory", ExportKind::Memory)
        });

    let mut numeric_types = None;
    for function in &module.functions {
        let core_function = builder.core_alias_export(
            Some(&function.name),
            core_instance,
            &function.name,
            ExportKind::Func,
        );
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| {
                component_type(
                    &mut builder,
                    &mut numeric_types,
                    &function.name,
                    &parameter.ty,
                )
                .map(|ty| (component_extern_name(&parameter.name), ty))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let result = match function.return_type {
            Type::Unit => None,
            _ => Some(component_type(
                &mut builder,
                &mut numeric_types,
                &function.name,
                &function.return_type,
            )?),
        };
        let (type_index, mut function_type) = builder.type_function(Some(&function.name));
        function_type.params(parameters.iter().map(|(name, ty)| (name.as_str(), *ty)));
        function_type.result(result);
        let options = core_memory
            .filter(|_| is_checked_fixed_result(&function.return_type))
            .map(CanonicalOption::Memory);
        let lifted = builder.lift_func(Some(&function.name), core_function, type_index, options);
        builder.export(
            component_extern_name(&function.name),
            ComponentExportKind::Func,
            lifted,
            None,
        );
    }

    Ok(builder.finish())
}

fn component_extern_name(name: &str) -> String {
    name.replace('_', "-")
}

#[derive(Clone, Copy)]
struct NumericComponentTypes {
    i64_result: u32,
    u64_result: u32,
}

fn component_type(
    builder: &mut ComponentBuilder,
    numeric_types: &mut Option<NumericComponentTypes>,
    function: &str,
    ty: &Type,
) -> Result<ComponentValType, CodegenError> {
    match ty {
        Type::Bool => Ok(ComponentValType::Primitive(PrimitiveValType::Bool)),
        Type::Int | Type::I64 => Ok(ComponentValType::Primitive(PrimitiveValType::S64)),
        Type::U64 => Ok(ComponentValType::Primitive(PrimitiveValType::U64)),
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE) =>
        {
            let types = *numeric_types.get_or_insert_with(|| define_numeric_types(builder));
            Ok(ComponentValType::Type(match ok.as_ref() {
                Type::I64 => types.i64_result,
                Type::U64 => types.u64_result,
                _ => unreachable!("guarded fixed result"),
            }))
        }
        _ => Err(unsupported(function, "non-scalar Component value")),
    }
}

fn define_numeric_types(builder: &mut ComponentBuilder) -> NumericComponentTypes {
    let (error_type, error) = builder.type_defined(Some("numeric-error"));
    error.enum_type(["overflow", "underflow"]);
    let error_type = builder.export("numeric-error", ComponentExportKind::Type, error_type, None);

    let (i64_result, result) = builder.type_defined(Some("checked-i64"));
    result.result(
        Some(ComponentValType::Primitive(PrimitiveValType::S64)),
        Some(ComponentValType::Type(error_type)),
    );
    let i64_result = builder.export("checked-i64", ComponentExportKind::Type, i64_result, None);

    let (u64_result, result) = builder.type_defined(Some("checked-u64"));
    result.result(
        Some(ComponentValType::Primitive(PrimitiveValType::U64)),
        Some(ComponentValType::Type(error_type)),
    );
    let u64_result = builder.export("checked-u64", ComponentExportKind::Type, u64_result, None);
    NumericComponentTypes {
        i64_result,
        u64_result,
    }
}

fn lower_parameter_type(function: &str, ty: &Type) -> Result<ValType, CodegenError> {
    match ty {
        Type::Bool => Ok(ValType::I32),
        Type::I64 | Type::U64 => Ok(ValType::I64),
        Type::Int => Err(unsupported(function, "unbounded Int parameter")),
        Type::Task(_) => Err(async_unsupported(function, "Task")),
        Type::Future(_) => Err(async_unsupported(function, "Future")),
        Type::Stream(_) => Err(async_unsupported(function, "Stream")),
        _ => Err(unsupported(function, "non-scalar parameter")),
    }
}

fn lower_result_type(
    function: &str,
    ty: &Type,
    abi: CoreAbi,
) -> Result<Vec<ValType>, CodegenError> {
    match ty {
        Type::Unit => Ok(Vec::new()),
        Type::Bool => Ok(vec![ValType::I32]),
        Type::Int | Type::I64 | Type::U64 => Ok(vec![ValType::I64]),
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE) =>
        {
            if abi == CoreAbi::ComponentLift {
                Ok(vec![ValType::I32])
            } else {
                Ok(vec![ValType::I32, ValType::I64])
            }
        }
        Type::Task(_) => Err(async_unsupported(function, "Task")),
        Type::Future(_) => Err(async_unsupported(function, "Future")),
        Type::Stream(_) => Err(async_unsupported(function, "Stream")),
        _ => Err(unsupported(function, "non-scalar result")),
    }
}

fn compile_function(function: &IrFunction, abi: CoreAbi) -> Result<Function, CodegenError> {
    let (locals, layout) = LocalLayout::new(function)?;
    let mut body = Function::new(locals);
    let mut constants = BTreeMap::new();
    let entry = block(function, function.entry)?;
    compile_instructions(function, entry, &layout, &mut body, &mut constants)?;
    match &entry.terminator {
        Terminator::Return(value) => emit_return(function, &layout, &mut body, *value, abi)?,
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            body.instruction(&Instruction::LocalGet(
                layout.scalar(&function.name, *condition)?,
            ));
            body.instruction(&Instruction::If(BlockType::Empty));
            compile_return_block(
                function,
                *then_block,
                &layout,
                &mut body,
                &mut constants,
                abi,
            )?;
            body.instruction(&Instruction::Else);
            compile_return_block(
                function,
                *else_block,
                &layout,
                &mut body,
                &mut constants,
                abi,
            )?;
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::Unreachable);
        }
        Terminator::Jump(_) | Terminator::Match { .. } | Terminator::Unreachable => {
            return Err(unsupported(&function.name, "entry terminator"));
        }
    }
    body.instruction(&Instruction::End);
    Ok(body)
}

struct LocalLayout {
    slots: BTreeMap<ValueId, Vec<u32>>,
}

impl LocalLayout {
    fn new(function: &IrFunction) -> Result<(Vec<(u32, ValType)>, Self), CodegenError> {
        let mut slots = BTreeMap::new();
        for (index, parameter) in function.parameters.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| CodegenError::ModuleTooLarge {
                functions: function.parameters.len(),
            })?;
            slots.insert(parameter.id, vec![index]);
        }
        let mut next =
            u32::try_from(function.parameters.len()).map_err(|_| CodegenError::ModuleTooLarge {
                functions: function.parameters.len(),
            })?;
        let mut locals = Vec::new();
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            let types = lower_local_types(&function.name, &instruction.ty)?;
            let mut value_slots = Vec::with_capacity(types.len());
            for ty in types {
                value_slots.push(next);
                next = next
                    .checked_add(1)
                    .ok_or_else(|| unsupported(&function.name, "too many Wasm locals"))?;
                locals.push((1, ty));
            }
            slots.insert(instruction.result, value_slots);
        }
        Ok((locals, Self { slots }))
    }

    fn get(&self, function: &str, value: ValueId) -> Result<&[u32], CodegenError> {
        self.slots
            .get(&value)
            .map(Vec::as_slice)
            .ok_or_else(|| unsupported(function, "missing local after verification"))
    }

    fn scalar(&self, function: &str, value: ValueId) -> Result<u32, CodegenError> {
        let [slot] = self.get(function, value)? else {
            return Err(unsupported(function, "aggregate used as scalar"));
        };
        Ok(*slot)
    }
}

fn compile_return_block(
    function: &IrFunction,
    id: BlockId,
    layout: &LocalLayout,
    body: &mut Function,
    constants: &mut BTreeMap<ValueId, i64>,
    abi: CoreAbi,
) -> Result<(), CodegenError> {
    let block = block(function, id)?;
    compile_instructions(function, block, layout, body, constants)?;
    let Terminator::Return(value) = block.terminator else {
        return Err(unsupported(&function.name, "non-return branch block"));
    };
    emit_return(function, layout, body, value, abi)?;
    Ok(())
}

fn compile_instructions(
    function: &IrFunction,
    block: &Block,
    layout: &LocalLayout,
    body: &mut Function,
    constants: &mut BTreeMap<ValueId, i64>,
) -> Result<(), CodegenError> {
    for instruction in &block.instructions {
        match &instruction.operation {
            Operation::ConstInt(value) => {
                let parsed =
                    value
                        .parse::<i64>()
                        .map_err(|_| CodegenError::IntegerOutsideProvenI64 {
                            function: function.name.clone(),
                            bytes: value.len(),
                        })?;
                constants.insert(instruction.result, parsed);
                body.instruction(&Instruction::I64Const(parsed));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstI64(value) => {
                body.instruction(&Instruction::I64Const(*value));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstU64(value) => {
                body.instruction(&Instruction::I64Const(i64::from_le_bytes(
                    value.to_le_bytes(),
                )));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::ConstBool(value) => {
                body.instruction(&Instruction::I32Const(i32::from(*value)));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::Copy(source)
                if matches!(instruction.ty, Type::Bool | Type::I64 | Type::U64) =>
            {
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *source)?,
                ));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::Copy(source) if instruction.ty == Type::Int => {
                let Some(value) = constants.get(source).copied() else {
                    return Err(unsupported(&function.name, "non-constant Int copy"));
                };
                constants.insert(instruction.result, value);
                body.instruction(&Instruction::I64Const(value));
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::AddInt { left, right } => {
                let (Some(left_value), Some(right_value)) =
                    (constants.get(left), constants.get(right))
                else {
                    return Err(unsupported(
                        &function.name,
                        "non-constant arbitrary Int add",
                    ));
                };
                let value = left_value.checked_add(*right_value).ok_or_else(|| {
                    CodegenError::IntegerOutsideProvenI64 {
                        function: function.name.clone(),
                        bytes: 8,
                    }
                })?;
                constants.insert(instruction.result, value);
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *left)?,
                ));
                body.instruction(&Instruction::LocalGet(
                    layout.scalar(&function.name, *right)?,
                ));
                body.instruction(&Instruction::I64Add);
                body.instruction(&Instruction::LocalSet(
                    layout.scalar(&function.name, instruction.result)?,
                ));
            }
            Operation::CheckedAdd { left, right } => {
                emit_checked_fixed(function, layout, instruction, *left, *right, true, body)?;
            }
            Operation::CheckedSub { left, right } => {
                emit_checked_fixed(function, layout, instruction, *left, *right, false, body)?;
            }
            Operation::EqualFixed { left, right } => {
                emit_fixed_comparison(function, layout, instruction, *left, *right, true, body)?;
            }
            Operation::LessFixed { left, right } => {
                emit_fixed_comparison(function, layout, instruction, *left, *right, false, body)?;
            }
            _ => return Err(unsupported(&function.name, "operation")),
        }
    }
    Ok(())
}

fn lower_local_types(function: &str, ty: &Type) -> Result<Vec<ValType>, CodegenError> {
    match ty {
        Type::Bool => Ok(vec![ValType::I32]),
        Type::Int | Type::I64 | Type::U64 => Ok(vec![ValType::I64]),
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE) =>
        {
            Ok(vec![ValType::I32, ValType::I64])
        }
        Type::Task(_) => Err(async_unsupported(function, "Task")),
        Type::Future(_) => Err(async_unsupported(function, "Future")),
        Type::Stream(_) => Err(async_unsupported(function, "Stream")),
        _ => Err(unsupported(function, "non-scalar local")),
    }
}

fn emit_checked_fixed(
    function: &IrFunction,
    layout: &LocalLayout,
    instruction: &sico_ir::Instruction,
    left: ValueId,
    right: ValueId,
    add: bool,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let Type::Result { ok, .. } = &instruction.ty else {
        return Err(unsupported(&function.name, "checked result type"));
    };
    let [tag, payload] = layout.get(&function.name, instruction.result)? else {
        return Err(unsupported(&function.name, "checked result layout"));
    };
    let left = layout.scalar(&function.name, left)?;
    let right = layout.scalar(&function.name, right)?;

    body.instruction(&Instruction::LocalGet(left));
    body.instruction(&Instruction::LocalGet(right));
    body.instruction(if add {
        &Instruction::I64Add
    } else {
        &Instruction::I64Sub
    });
    body.instruction(&Instruction::LocalSet(*payload));

    match (ok.as_ref(), add) {
        (Type::I64, true) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::I64And);
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
        }
        (Type::I64, false) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Xor);
            body.instruction(&Instruction::I64And);
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
        }
        (Type::U64, true) => {
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64LtU);
        }
        (Type::U64, false) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::LocalGet(right));
            body.instruction(&Instruction::I64LtU);
        }
        _ => return Err(unsupported(&function.name, "checked fixed operand")),
    }
    body.instruction(&Instruction::If(BlockType::Empty));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::LocalSet(*tag));
    match (ok.as_ref(), add) {
        (Type::I64, _) => {
            body.instruction(&Instruction::LocalGet(left));
            body.instruction(&Instruction::I64Const(0));
            body.instruction(&Instruction::I64LtS);
            body.instruction(&Instruction::I64ExtendI32U);
        }
        (Type::U64, true) => {
            body.instruction(&Instruction::I64Const(0));
        }
        (Type::U64, false) => {
            body.instruction(&Instruction::I64Const(1));
        }
        _ => return Err(unsupported(&function.name, "checked fixed error")),
    }
    body.instruction(&Instruction::LocalSet(*payload));
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(*tag));
    body.instruction(&Instruction::End);
    Ok(())
}

fn emit_fixed_comparison(
    function: &IrFunction,
    layout: &LocalLayout,
    instruction: &sico_ir::Instruction,
    left: ValueId,
    right: ValueId,
    equal: bool,
    body: &mut Function,
) -> Result<(), CodegenError> {
    let left_slot = layout.scalar(&function.name, left)?;
    let right_slot = layout.scalar(&function.name, right)?;
    body.instruction(&Instruction::LocalGet(left_slot));
    body.instruction(&Instruction::LocalGet(right_slot));
    if equal {
        body.instruction(&Instruction::I64Eq);
    } else {
        let ty = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find(|candidate| candidate.result == left)
            .map(|candidate| &candidate.ty)
            .or_else(|| {
                function
                    .parameters
                    .iter()
                    .find(|parameter| parameter.id == left)
                    .map(|parameter| &parameter.ty)
            })
            .ok_or_else(|| unsupported(&function.name, "comparison operand type"))?;
        body.instruction(if *ty == Type::I64 {
            &Instruction::I64LtS
        } else {
            &Instruction::I64LtU
        });
    }
    body.instruction(&Instruction::LocalSet(
        layout.scalar(&function.name, instruction.result)?,
    ));
    Ok(())
}

fn block(function: &IrFunction, id: BlockId) -> Result<&Block, CodegenError> {
    function
        .blocks
        .iter()
        .find(|block| block.id == id)
        .ok_or_else(|| unsupported(&function.name, "missing block after verification"))
}

fn emit_return(
    function: &IrFunction,
    layout: &LocalLayout,
    body: &mut Function,
    value: Option<ValueId>,
    abi: CoreAbi,
) -> Result<(), CodegenError> {
    if let Some(value) = value {
        if abi == CoreAbi::ComponentLift && is_checked_fixed_result(&function.return_type) {
            let [tag, payload] = layout.get(&function.name, value)? else {
                return Err(unsupported(&function.name, "checked result layout"));
            };
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*tag));
            body.instruction(&Instruction::I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(*payload));
            body.instruction(&Instruction::I64Store(MemArg {
                offset: 8,
                align: 3,
                memory_index: 0,
            }));
            body.instruction(&Instruction::I32Const(0));
        } else {
            for slot in layout.get(&function.name, value)? {
                body.instruction(&Instruction::LocalGet(*slot));
            }
        }
    }
    body.instruction(&Instruction::Return);
    Ok(())
}

fn is_checked_fixed_result(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Result { ok, error }
            if matches!(ok.as_ref(), Type::I64 | Type::U64)
                && matches!(error.as_ref(), Type::Named(name) if name == sico_ir::NUMERIC_ERROR_TYPE)
    )
}

fn unsupported(function: &str, feature: &str) -> CodegenError {
    CodegenError::Unsupported {
        function: function.to_owned(),
        feature: feature.to_owned(),
    }
}

fn async_unsupported(function: &str, feature: &str) -> CodegenError {
    let contract = match feature {
        "Task" => "structured Task remains language-local; no WIT Task handle",
        "Future" => "Future lowering requires an async Component function and cancellation edge",
        "Stream" => "Stream lowering requires an explicit bound and close/cancel edge",
        _ => "unsupported asynchronous boundary",
    };
    CodegenError::AsyncUnsupported {
        function: function.to_owned(),
        feature: feature.to_owned(),
        contract,
    }
}
