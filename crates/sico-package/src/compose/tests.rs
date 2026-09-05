use super::*;

fn validate(bytes: &[u8]) {
    wasmparser::Validator::new()
        .validate_all(bytes)
        .expect("adapter composition must pass the real Wasm validator");
}

#[test]
fn adapter_component_is_deterministic_and_valid() {
    let adapter = script_adapter_component();
    assert_eq!(adapter, script_adapter_component());
    validate(&adapter);
}

#[test]
fn composed_command_is_deterministic_valid_and_has_a_closed_import_set() {
    let adapter = script_adapter_component();
    let program = program_fixture();
    let composed = compose_script_command(&program, &adapter);
    assert_eq!(composed, compose_script_command(&program, &adapter));
    validate(&composed);

    let imports = component_imports(&composed);
    let expected = [
        "sico:script/types@0.1.0".to_owned(),
        environment_name("wasi:cli/environment"),
        environment_name("wasi:cli/exit"),
        environment_name("wasi:cli/stderr"),
        environment_name("wasi:cli/stdin"),
        environment_name("wasi:cli/stdout"),
        environment_name("wasi:io/error"),
        environment_name("wasi:io/streams"),
    ];
    assert_eq!(imports, expected);
}

fn component_imports(bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.unwrap() {
            wasmparser::Payload::ComponentImportSection(section) => {
                for import in section {
                    imports.push(import.unwrap().name.name.to_owned());
                }
            }
            // Nested modules/components begin a new Version payload; only the
            // outermost import section is the boundary.
            wasmparser::Payload::Version { .. } if !imports.is_empty() => break,
            _ => {}
        }
    }
    imports.sort();
    imports
}

/// A minimal STEP-0079 echo Program Component.
fn program_fixture() -> Vec<u8> {
    use sico_ir::{
        Block, BlockId, ConstructField, Function as IrFunction, FunctionId, Instruction,
        Module as IrModule, Operation, Parameter, SourceRange, Terminator, Type, ValueId,
    };
    let range = SourceRange { start: 0, end: 0 };
    let script_result = || Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    };
    let mut module = IrModule::new("echo.sico", 0);
    module.functions.push(IrFunction {
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
            instructions: vec![
                Instruction {
                    result: ValueId(1),
                    ty: Type::Bytes,
                    operation: Operation::Project {
                        base: ValueId(0),
                        field: "stdin".into(),
                    },
                    range,
                },
                Instruction {
                    result: ValueId(2),
                    ty: Type::Bytes,
                    operation: Operation::ConstBytes(Vec::new()),
                    range,
                },
                Instruction {
                    result: ValueId(3),
                    ty: Type::I64,
                    operation: Operation::ConstI64(0),
                    range,
                },
                Instruction {
                    result: ValueId(4),
                    ty: Type::Named("ScriptOutput".into()),
                    operation: Operation::Construct {
                        name: "ScriptOutput".into(),
                        fields: vec![
                            ConstructField {
                                name: "stdout".into(),
                                value: ValueId(1),
                            },
                            ConstructField {
                                name: "stderr".into(),
                                value: ValueId(2),
                            },
                            ConstructField {
                                name: "exit_code".into(),
                                value: ValueId(3),
                            },
                        ],
                    },
                    range,
                },
                Instruction {
                    result: ValueId(5),
                    ty: script_result(),
                    operation: Operation::Variant {
                        name: "ok".into(),
                        payload: vec![ValueId(4)],
                    },
                    range,
                },
            ],
            terminator: Terminator::Return(Some(ValueId(5))),
            range,
        }],
        range,
    });
    sico_codegen_wasm::compile_script_program(&module).unwrap()
}
