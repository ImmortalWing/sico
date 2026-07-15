//! Deterministic Core Wasm generation from independently verified Sico IR.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sico_ir::{
    Block, BlockId, Function as IrFunction, Module as IrModule, Operation, Terminator, Type,
    ValueId, VerifyError, verify,
};
use wasm_encoder::{
    BlockType, CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction,
    Module, TypeSection, ValType,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodegenError {
    InvalidIr(Vec<VerifyError>),
    ModuleTooLarge { functions: usize },
    Unsupported { function: String, feature: String },
    IntegerOutsideProvenI64 { function: String, bytes: usize },
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
    let errors = verify(module);
    if !errors.is_empty() {
        return Err(CodegenError::InvalidIr(errors));
    }
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    for (index, function) in module.functions.iter().enumerate() {
        let function_index = u32::try_from(index).map_err(|_| CodegenError::ModuleTooLarge {
            functions: module.functions.len(),
        })?;
        let params = function
            .parameters
            .iter()
            .map(|parameter| lower_parameter_type(&function.name, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        let results = lower_result_type(&function.name, &function.return_type)?;
        types.ty().function(params, results);
        functions.function(function_index);
        exports.export(&function.name, ExportKind::Func, function_index);
        code.function(&compile_function(function)?);
    }
    let mut output = Module::new();
    output.section(&types);
    output.section(&functions);
    output.section(&exports);
    output.section(&code);
    Ok(output.finish())
}

fn lower_parameter_type(function: &str, ty: &Type) -> Result<ValType, CodegenError> {
    match ty {
        Type::Bool => Ok(ValType::I32),
        Type::Int => Err(unsupported(function, "unbounded Int parameter")),
        _ => Err(unsupported(function, "non-scalar parameter")),
    }
}

fn lower_result_type(function: &str, ty: &Type) -> Result<Vec<ValType>, CodegenError> {
    match ty {
        Type::Unit => Ok(Vec::new()),
        Type::Bool => Ok(vec![ValType::I32]),
        Type::Int => Ok(vec![ValType::I64]),
        _ => Err(unsupported(function, "non-scalar result")),
    }
}

fn compile_function(function: &IrFunction) -> Result<Function, CodegenError> {
    let locals = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .map(|instruction| lower_local_type(&function.name, &instruction.ty).map(|ty| (1_u32, ty)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut body = Function::new(locals);
    let mut constants = BTreeMap::new();
    let entry = block(function, function.entry)?;
    compile_instructions(function, entry, &mut body, &mut constants)?;
    match &entry.terminator {
        Terminator::Return(value) => emit_return(&mut body, *value),
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            body.instruction(&Instruction::LocalGet(condition.0));
            body.instruction(&Instruction::If(BlockType::Empty));
            compile_return_block(function, *then_block, &mut body, &mut constants)?;
            body.instruction(&Instruction::Else);
            compile_return_block(function, *else_block, &mut body, &mut constants)?;
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

fn compile_return_block(
    function: &IrFunction,
    id: BlockId,
    body: &mut Function,
    constants: &mut BTreeMap<ValueId, i64>,
) -> Result<(), CodegenError> {
    let block = block(function, id)?;
    compile_instructions(function, block, body, constants)?;
    let Terminator::Return(value) = block.terminator else {
        return Err(unsupported(&function.name, "non-return branch block"));
    };
    emit_return(body, value);
    Ok(())
}

fn compile_instructions(
    function: &IrFunction,
    block: &Block,
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
                body.instruction(&Instruction::LocalSet(instruction.result.0));
            }
            Operation::ConstBool(value) => {
                body.instruction(&Instruction::I32Const(i32::from(*value)));
                body.instruction(&Instruction::LocalSet(instruction.result.0));
            }
            Operation::Copy(source) if instruction.ty == Type::Bool => {
                body.instruction(&Instruction::LocalGet(source.0));
                body.instruction(&Instruction::LocalSet(instruction.result.0));
            }
            Operation::Copy(source) if instruction.ty == Type::Int => {
                let Some(value) = constants.get(source).copied() else {
                    return Err(unsupported(&function.name, "non-constant Int copy"));
                };
                constants.insert(instruction.result, value);
                body.instruction(&Instruction::I64Const(value));
                body.instruction(&Instruction::LocalSet(instruction.result.0));
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
                body.instruction(&Instruction::LocalGet(left.0));
                body.instruction(&Instruction::LocalGet(right.0));
                body.instruction(&Instruction::I64Add);
                body.instruction(&Instruction::LocalSet(instruction.result.0));
            }
            _ => return Err(unsupported(&function.name, "operation")),
        }
    }
    Ok(())
}

fn lower_local_type(function: &str, ty: &Type) -> Result<ValType, CodegenError> {
    match ty {
        Type::Bool => Ok(ValType::I32),
        Type::Int => Ok(ValType::I64),
        _ => Err(unsupported(function, "non-scalar local")),
    }
}

fn block(function: &IrFunction, id: BlockId) -> Result<&Block, CodegenError> {
    function
        .blocks
        .iter()
        .find(|block| block.id == id)
        .ok_or_else(|| unsupported(&function.name, "missing block after verification"))
}

fn emit_return(body: &mut Function, value: Option<ValueId>) {
    if let Some(value) = value {
        body.instruction(&Instruction::LocalGet(value.0));
    }
    body.instruction(&Instruction::Return);
}

fn unsupported(function: &str, feature: &str) -> CodegenError {
    CodegenError::Unsupported {
        function: function.to_owned(),
        feature: feature.to_owned(),
    }
}
