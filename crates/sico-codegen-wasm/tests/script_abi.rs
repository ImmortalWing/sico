use std::{fmt::Write as _, fs, path::Path};

use sico_codegen_wasm::{CodegenError, SCRIPT_MEMORY_BYTES, compile_script_program};
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

/// STEP-0497: the guest linear memory a Component declares is the floor every
/// execution path's effective memory ceiling has to clear.
///
/// The declaration is fixed — the minimum equals the maximum, so the guest
/// memory never grows — and it is the number the Host and the package limit
/// are both measured against. This guard pins the emitted bytes, so a change
/// to the declared page count fails here instead of silently moving the floor
/// under every launch path.
#[test]
fn script_and_package_components_declare_the_same_fixed_guest_memory() {
    assert_eq!(SCRIPT_MEMORY_BYTES, 33_792 * 65_536);
    assert_eq!(SCRIPT_MEMORY_BYTES, 2_214_592_512);

    let script = compile_script_program(&echo_module()).unwrap();
    let script_memories = declared_guest_memory(&script);
    assert_eq!(
        script_memories,
        vec![(33_792, Some(33_792))],
        "a Script program component declares exactly one fixed guest memory"
    );

    let package = sico_codegen_wasm::package_builder::csv_package();
    let package_memories = declared_guest_memory(&package);
    assert_eq!(
        package_memories,
        vec![(33_792, Some(33_792))],
        "a package component declares exactly one fixed guest memory"
    );
}

/// The `(minimum, maximum)` page counts of every memory a Component declares
/// or imports, walked through the nested core modules.
///
/// `parse_all` descends into the nested modules on its own, so one pass over
/// the Component bytes reaches the core memory section.
fn declared_guest_memory(bytes: &[u8]) -> Vec<(u64, Option<u64>)> {
    let mut memories = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(bytes).flatten() {
        match payload {
            wasmparser::Payload::MemorySection(reader) => {
                for memory in reader.into_iter().flatten() {
                    memories.push((memory.initial, memory.maximum));
                }
            }
            wasmparser::Payload::ImportSection(reader) => {
                for group in reader.into_iter().flatten() {
                    match group {
                        wasmparser::Imports::Single(_index, import) => {
                            if let wasmparser::TypeRef::Memory(memory) = import.ty {
                                memories.push((memory.initial, memory.maximum));
                            }
                        }
                        wasmparser::Imports::Compact1 { items, .. } => {
                            for item in items.into_iter().flatten() {
                                if let wasmparser::TypeRef::Memory(memory) = item.ty {
                                    memories.push((memory.initial, memory.maximum));
                                }
                            }
                        }
                        wasmparser::Imports::Compact2 {
                            ty,
                            names,
                            module: _,
                        } => {
                            if let wasmparser::TypeRef::Memory(memory) = ty {
                                for _ in 0..names.count() {
                                    memories.push((memory.initial, memory.maximum));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    memories
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
        locals: Vec::new(),
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
        locals: Vec::new(),
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
        locals: Vec::new(),
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

#[test]
fn script_task_scope_and_collect_programs_are_deterministic_and_valid() {
    // RFC-0036 §5.4: the sequential-v1 projection compiles explicit task IR
    // (scope table, Spawn, Await, TaskCollect) to the exact M9 behavior.
    for (fixture, snapshot_name) in [
        (
            "tests/end-to-end/script-task-pair.sico",
            "script-task-pair-component",
        ),
        (
            "tests/end-to-end/script-task-collect.sico",
            "script-task-collect-component",
        ),
    ] {
        let module = lower_end_to_end(fixture);
        assert!(
            module.task_scopes.is_some(),
            "{fixture}: task scope table must be present"
        );
        assert!(sico_ir::verify(&module).is_empty(), "{fixture}");
        let bytes = compile_script_program(&module).unwrap();
        assert_eq!(bytes, compile_script_program(&module).unwrap());
        validate(&bytes);
        snapshot(snapshot_name, &bytes);
    }
}

#[test]
fn async_function_export_stays_refused_at_the_component_boundary() {
    // RFC-0013/RFC-0036 §5.4: Task/Future/Stream types never cross a
    // Component import/export; an async function's `Future[T]` IR signature
    // is refused there even though the sequential Script profile runs it.
    let source = sico_source::SourceFile::from_text(
        sico_source::SourceId::new(0),
        "async-boundary.sico",
        "async function compute(value: I64) returns I64:\n  return value\nend function\n",
    )
    .unwrap();
    let module = sico_ir::lower_core(&source).unwrap();
    assert!(matches!(
        sico_codegen_wasm::compile_component(&module),
        Err(CodegenError::AsyncUnsupported { feature, .. }) if feature == "Future"
    ));
}

fn lower_end_to_end(relative: &str) -> Module {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = repository.join(relative);
    let source = sico_source::SourceFile::from_text(
        sico_source::SourceId::new(0),
        path.display().to_string(),
        fs::read_to_string(path).unwrap(),
    )
    .unwrap();
    let mut module = sico_ir::lower_core(&source).unwrap();
    // Same entry adaptation as the CLI: the source `main` is the Script
    // entry `run` at the Component boundary.
    for function in &mut module.functions {
        if function.name == "main" {
            "run".clone_into(&mut function.name);
        }
    }
    module
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
        locals: Vec::new(),
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
