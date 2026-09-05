//! Temporary scratch probe: validate a program using every intrinsic.

use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
    Parameter, SourceRange, Terminator, Type, ValueId,
};

#[test]
#[allow(clippy::too_many_lines)]
fn intrinsic_fixture_validates() {
    let range = SourceRange { start: 0, end: 0 };
    let script_result = || Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    };
    let mut module = Module::new("intrinsics.sico", 0);
    let mut instructions = Vec::new();
    let mut next = 1_u32;
    let mut emit = |ty: Type, operation: Operation| {
        let id = ValueId(next);
        next += 1;
        instructions.push(Instruction {
            result: id,
            ty,
            operation,
            range,
        });
        id
    };
    let stdin = emit(
        Type::Bytes,
        Operation::Project {
            base: ValueId(0),
            field: "stdin".into(),
        },
    );
    let decoded = emit(
        Type::String,
        Operation::Intrinsic {
            name: "sico.bytes.utf8_decode".into(),
            arguments: vec![stdin],
        },
    );
    let valid = emit(
        Type::Bool,
        Operation::Intrinsic {
            name: "sico.bytes.is_utf8".into(),
            arguments: vec![stdin],
        },
    );
    let trimmed = emit(
        Type::String,
        Operation::Intrinsic {
            name: "sico.text.trim".into(),
            arguments: vec![decoded],
        },
    );
    let words = emit(
        Type::List(Box::new(Type::String)),
        Operation::Intrinsic {
            name: "sico.text.split_words".into(),
            arguments: vec![trimmed],
        },
    );
    let count = emit(
        Type::U64,
        Operation::Intrinsic {
            name: "sico.list.length".into(),
            arguments: vec![words],
        },
    );
    let text = emit(
        Type::String,
        Operation::Intrinsic {
            name: "sico.u64.to_text".into(),
            arguments: vec![count],
        },
    );
    let contains = emit(
        Type::Bool,
        Operation::Intrinsic {
            name: "sico.text.contains".into(),
            arguments: vec![text, text],
        },
    );
    let _ = (valid, contains);
    let encoded = emit(
        Type::Bytes,
        Operation::Intrinsic {
            name: "sico.text.encode".into(),
            arguments: vec![text],
        },
    );
    let empty = emit(Type::Bytes, Operation::ConstBytes(Vec::new()));
    let zero = emit(Type::I64, Operation::ConstI64(0));
    let output = emit(
        Type::Named("ScriptOutput".into()),
        Operation::Construct {
            name: "ScriptOutput".into(),
            fields: vec![
                ConstructField {
                    name: "stdout".into(),
                    value: encoded,
                },
                ConstructField {
                    name: "stderr".into(),
                    value: empty,
                },
                ConstructField {
                    name: "exit_code".into(),
                    value: zero,
                },
            ],
        },
    );
    let result = emit(
        script_result(),
        Operation::Variant {
            name: "ok".into(),
            payload: vec![output],
        },
    );
    module.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "input".into(),
            ty: Type::Named("ScriptInput".into()),
            range,
        }],
        return_type: script_result(),
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions,
            terminator: Terminator::Return(Some(result)),
            range,
        }],
        range,
    });
    let bytes = sico_codegen_wasm::compile_script_program(&module).unwrap();
    assert_eq!(
        bytes,
        sico_codegen_wasm::compile_script_program(&module).unwrap()
    );
    wasmparser::Validator::new()
        .validate_all(&bytes)
        .expect("intrinsic program must pass the real Wasm validator");
}
