use std::{fmt::Write as _, fs, path::Path};

use sico_codegen_wasm::{CodegenError, compile_script_program};
use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
    Parameter, SourceRange, Terminator, Type, ValueId,
};

#[test]
fn script_echo_program_is_deterministic_valid_and_snapshot() {
    let bytes = compile_script_program(&echo_module()).unwrap();
    assert_eq!(bytes, compile_script_program(&echo_module()).unwrap());
    validate(&bytes);
    snapshot("script-echo-component", &bytes);
}

#[test]
fn script_error_program_is_deterministic_valid_and_snapshot() {
    let bytes = compile_script_program(&error_module()).unwrap();
    assert_eq!(bytes, compile_script_program(&error_module()).unwrap());
    validate(&bytes);
    snapshot("script-error-component", &bytes);
}

#[test]
fn script_boundary_requires_the_exact_entry_signature() {
    let mut missing_run = Module::new("empty.sico", 0);
    assert!(matches!(
        compile_script_program(&missing_run),
        Err(CodegenError::Unsupported { feature, .. }) if feature == "missing Script entry run"
    ));

    let mut wrong_param = echo_module();
    wrong_param.functions[0].parameters[0].ty = Type::I64;
    assert!(matches!(
        compile_script_program(&wrong_param),
        Err(CodegenError::Unsupported { .. } | CodegenError::InvalidIr(_))
    ));

    let range = SourceRange { start: 0, end: 0 };
    let mut scalar_run = Module::new("scalar.sico", 0);
    scalar_run.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "value".into(),
            ty: Type::I64,
            range,
        }],
        return_type: Type::I64,
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Return(Some(ValueId(0))),
            range,
        }],
        range,
    });
    assert!(matches!(
        compile_script_program(&scalar_run),
        Err(CodegenError::Unsupported { feature, .. })
            if feature.contains("Script entry must be run(input: ScriptInput)")
    ));

    missing_run.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: Vec::new(),
        return_type: Type::Unit,
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Return(None),
            range: SourceRange { start: 0, end: 0 },
        }],
        range: SourceRange { start: 0, end: 0 },
    });
    assert!(matches!(
        compile_script_program(&missing_run),
        Err(CodegenError::Unsupported { .. })
    ));
}

#[test]
fn script_helpers_may_use_aggregates_effects_stay_refused() {
    let mut aggregate_helper = echo_module();
    aggregate_helper.functions.push(Function {
        id: FunctionId(2),
        name: "helper".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "value".into(),
            ty: Type::Bytes,
            range: SourceRange { start: 0, end: 0 },
        }],
        return_type: Type::Bytes,
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Return(Some(ValueId(0))),
            range: SourceRange { start: 0, end: 0 },
        }],
        range: SourceRange { start: 0, end: 0 },
    });
    // STEP-0083: Script helper functions use the flattened Canonical ABI for
    // aggregate parameters and results, so this module must compile.
    assert!(compile_script_program(&aggregate_helper).is_ok());

    let mut effectful = echo_module();
    effectful.functions[0].effects.push("Console".into());
    assert!(matches!(
        compile_script_program(&effectful),
        Err(CodegenError::Unsupported { feature, .. })
            if feature == "effectful function without a Component host adapter"
    ));
}

#[test]
fn unsupported_aggregate_shapes_fail_with_typed_refusals() {
    let mut list_of_bytes = echo_module();
    list_of_bytes.functions[0].blocks[0].instructions[1].ty = Type::List(Box::new(Type::Bytes));
    assert!(matches!(
        compile_script_program(&list_of_bytes),
        Err(CodegenError::Unsupported { .. } | CodegenError::InvalidIr(_))
    ));

    let mut invalid = echo_module();
    invalid.schema = "invalid".into();
    assert!(matches!(
        compile_script_program(&invalid),
        Err(CodegenError::InvalidIr(_))
    ));
}

/// `run(input: ScriptInput) -> Result[ScriptOutput, ScriptError]` echoing
/// stdin to stdout with empty stderr and exit code zero.
fn echo_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("echo.sico", 0);
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
    module
}

/// `run` returning `ScriptError { code: domain-error, message: "rejected" }`.
fn error_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("error.sico", 0);
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
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(1),
                    ty: Type::Named("ScriptErrorCode".into()),
                    operation: Operation::Variant {
                        name: "ScriptErrorCode.DomainError".into(),
                        payload: Vec::new(),
                    },
                    range,
                },
                Instruction {
                    result: ValueId(2),
                    ty: Type::String,
                    operation: Operation::ConstString("rejected".into()),
                    range,
                },
                Instruction {
                    result: ValueId(3),
                    ty: Type::Named("ScriptError".into()),
                    operation: Operation::Construct {
                        name: "ScriptError".into(),
                        fields: vec![
                            ConstructField {
                                name: "code".into(),
                                value: ValueId(1),
                            },
                            ConstructField {
                                name: "message".into(),
                                value: ValueId(2),
                            },
                        ],
                    },
                    range,
                },
                Instruction {
                    result: ValueId(4),
                    ty: script_result(),
                    operation: Operation::Variant {
                        name: "error".into(),
                        payload: vec![ValueId(3)],
                    },
                    range,
                },
            ],
            terminator: Terminator::Return(Some(ValueId(4))),
            range,
        }],
        range,
    });
    module
}

fn script_result() -> Type {
    Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    }
}

fn validate(bytes: &[u8]) {
    wasmparser::Validator::new()
        .validate_all(bytes)
        .expect("script backend output must pass the real Wasm validator");
}

fn snapshot(name: &str, bytes: &[u8]) {
    let actual = hex(bytes);
    if std::env::var_os("SICO_DUMP_WASM").is_some() {
        println!("{name}={actual}");
    } else {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let expected = fs::read_to_string(repository.join("tests/wasm/artifacts.hex")).unwrap();
        assert!(
            expected
                .lines()
                .any(|line| line == format!("{name}={actual}")),
            "missing artifact snapshot for {name}"
        );
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").unwrap();
            output
        },
    )
}
