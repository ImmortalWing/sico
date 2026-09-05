use std::{fmt::Write as _, fs, path::Path};

use sico_codegen_wasm::{
    CodegenError, DebugBuildInput, compile, compile_component, compile_component_with_debug,
};
use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, MatchArm, Module, Operation,
    Parameter, Pattern, SourceRange, Terminator, Type, ValueId,
};
use sico_observability::{ContractError, sha256_hex, verify_debug_artifacts};
use sico_source::{SourceFile, SourceId};

#[test]
fn numeric_source_builds_byte_identical_valid_wasm() {
    let source = SourceFile::from_text(
        SourceId::new(0),
        "answer.sico",
        "function main() returns Int:\n  return 40 + 2\nend function\n",
    )
    .unwrap();
    let ir = sico_ir::lower_core(&source).unwrap();
    let bytes = compile(&ir).unwrap();
    assert_eq!(bytes, compile(&ir).unwrap());
    validate(&bytes);
    snapshot("numeric", &bytes);
}

#[test]
fn boolean_control_flow_builds_byte_identical_valid_wasm() {
    let ir = select_module();
    let bytes = compile(&ir).unwrap();
    assert_eq!(bytes, compile(&ir).unwrap());
    validate(&bytes);
    snapshot("control", &bytes);
}

#[test]
fn scalar_components_are_deterministic_and_validate() {
    for (name, ir) in [
        ("numeric-component", numeric_module()),
        ("control-component", select_module()),
    ] {
        let bytes = compile_component(&ir).unwrap();
        assert_eq!(bytes, compile_component(&ir).unwrap());
        validate(&bytes);
        snapshot(name, &bytes);
    }
}

#[test]
fn debug_component_map_and_identity_are_deterministic_and_fail_closed() {
    let text = b"function main() returns Int:\n  return 40 + 2\nend function\n";
    let source = SourceFile::from_text(
        SourceId::new(0),
        "answer.sico",
        std::str::from_utf8(text).unwrap(),
    )
    .unwrap();
    let ir = sico_ir::lower_core(&source).unwrap();
    let compiler_digest = sha256_hex(b"sico-test-compiler");
    let input = DebugBuildInput {
        document_id: "doc.answer",
        source_bytes: text,
        display_uri: Some("workspace://answer.sico"),
        compiler_package: "sico-compiler",
        compiler_version: "0.0.2-dev",
        compiler_executable_sha256: &compiler_digest,
        adapter_identities: Vec::new(),
        wit_identities: Vec::new(),
    };
    let first = compile_component_with_debug(&ir, &input).unwrap();
    let second = compile_component_with_debug(&ir, &input).unwrap();
    assert_eq!(first.component, second.component);
    assert_eq!(first.debug_map, second.debug_map);
    assert_eq!(first.identity, second.identity);
    validate(&first.component);
    let (map, identity) =
        verify_debug_artifacts(&first.component, &first.debug_map, &first.identity).unwrap();
    assert_eq!(map.functions.len(), 1);
    assert!(!map.mappings.is_empty());
    assert!(map.mappings.iter().all(|mapping| {
        mapping.core_module == "sico-core"
            && mapping.instruction_start < mapping.instruction_end
            && mapping
                .source
                .as_ref()
                .is_some_and(|span| span.end <= text.len() as u64)
    }));
    assert_eq!(identity.source.sha256, sha256_hex(text));

    let mut stale_map = first.debug_map.clone();
    stale_map.push(b' ');
    assert!(matches!(
        verify_debug_artifacts(&first.component, &stale_map, &first.identity),
        Err(ContractError::NonCanonical("debug-map-json"))
    ));
}

#[test]
fn dynamic_fixed_width_core_and_component_are_deterministic_and_validate() {
    let ir = fixed_width_module();
    let core = compile(&ir).unwrap();
    assert_eq!(core, compile(&ir).unwrap());
    validate(&core);
    snapshot("fixed-width-core", &core);

    let component = compile_component(&ir).unwrap();
    assert_eq!(component, compile_component(&ir).unwrap());
    validate(&component);
    snapshot("fixed-width-component", &component);
}

#[test]
fn general_calls_cfg_and_internal_data_are_deterministic_and_validate() {
    let ir = general_module();
    let core = compile(&ir).unwrap();
    assert_eq!(core, compile(&ir).unwrap());
    validate(&core);
    snapshot("general-core", &core);

    let component = compile_component(&ir).unwrap();
    assert_eq!(component, compile_component(&ir).unwrap());
    validate(&component);
    snapshot("general-component", &component);

    let mut binding = ir;
    let Terminator::Match { arms, .. } = &mut binding.functions[3].blocks[0].terminator else {
        panic!("expected match")
    };
    arms[0].patterns[0] = Pattern::Binding("value".into());
    assert!(matches!(
        compile(&binding),
        Err(CodegenError::Unsupported { feature, .. }) if feature == "match binding transport"
    ));

    let mut bad_projection = general_module();
    bad_projection.functions[4].return_type = Type::Bool;
    bad_projection.functions[4].blocks[0].instructions[3].ty = Type::Bool;
    assert!(matches!(
        compile(&bad_projection),
        Err(CodegenError::Unsupported { feature, .. })
            if feature == "projected field does not match its declared Wasm layout"
    ));
}

#[test]
fn debug_map_covers_calls_matches_back_edges_and_block_terminators() {
    let mut ir = general_module();
    let mut next = 1_u32;
    let mut expected = std::collections::BTreeSet::new();
    for function in &mut ir.functions {
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                instruction.range = SourceRange {
                    start: next,
                    end: next + 1,
                };
                expected.insert((next, next + 1));
                next += 2;
            }
            block.range = SourceRange {
                start: next,
                end: next + 1,
            };
            expected.insert((next, next + 1));
            next += 2;
        }
    }
    ir.source_len = next + 1;
    let source = vec![b' '; next as usize + 1];
    let compiler_digest = sha256_hex(b"sico-general-debug-compiler");
    let artifact = compile_component_with_debug(
        &ir,
        &DebugBuildInput {
            document_id: "doc.general",
            source_bytes: &source,
            display_uri: Some("workspace://general.sico"),
            compiler_package: "sico-compiler",
            compiler_version: "0.0.2-dev",
            compiler_executable_sha256: &compiler_digest,
            adapter_identities: Vec::new(),
            wit_identities: Vec::new(),
        },
    )
    .unwrap();
    let (map, _) =
        verify_debug_artifacts(&artifact.component, &artifact.debug_map, &artifact.identity)
            .unwrap();
    assert_eq!(map.functions.len(), 7);
    let actual = map
        .mappings
        .iter()
        .filter_map(|mapping| mapping.source.as_ref())
        .map(|span| {
            (
                u32::try_from(span.start).unwrap(),
                u32::try_from(span.end).unwrap(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, expected);
    for window in map.mappings.windows(2) {
        if window[0].core_module == window[1].core_module
            && window[0].component_function == window[1].component_function
        {
            assert!(window[0].instruction_end <= window[1].instruction_start);
        }
    }
}

#[test]
fn fixed_width_source_call_reaches_the_general_backend() {
    let source = SourceFile::from_text(
        SourceId::new(0),
        "call.sico",
        "function identity(value: I64) returns I64:\n  return value\nend function\n\nfunction through_call(value: I64) returns I64:\n  return identity(value)\nend function\n",
    )
    .unwrap();
    let ir = sico_ir::lower_core(&source).unwrap();
    assert!(matches!(
        ir.functions[1].blocks[0].instructions[0].operation,
        Operation::Call {
            function: FunctionId(1),
            ..
        }
    ));
    let bytes = compile_component(&ir).unwrap();
    assert_eq!(bytes, compile_component(&ir).unwrap());
    validate(&bytes);
}

#[test]
#[ignore = "maintainer-only deterministic snapshot refresh"]
fn update_wasm_snapshots() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries = [
        ("control", compile(&select_module()).unwrap()),
        (
            "control-component",
            compile_component(&select_module()).unwrap(),
        ),
        ("numeric", compile(&numeric_module()).unwrap()),
        (
            "numeric-component",
            compile_component(&numeric_module()).unwrap(),
        ),
        ("fixed-width-core", compile(&fixed_width_module()).unwrap()),
        (
            "fixed-width-component",
            compile_component(&fixed_width_module()).unwrap(),
        ),
        ("general-core", compile(&general_module()).unwrap()),
        (
            "general-component",
            compile_component(&general_module()).unwrap(),
        ),
    ];
    let contents = entries
        .iter()
        .map(|(name, bytes)| format!("{name}={}", hex(bytes)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        repository.join("tests/wasm/artifacts.hex"),
        format!("{contents}\n"),
    )
    .unwrap();
}

#[test]
fn deterministic_scalar_property_covers_2048_sources() {
    for seed in 0..2_048_u32 {
        let left = i64::from(seed);
        let right = i64::from(seed.rotate_left(7) % 10_000);
        let text =
            format!("function main() returns Int:\n  return {left} + {right}\nend function\n");
        let source = SourceFile::from_text(SourceId::new(seed), "property.sico", text).unwrap();
        let ir = sico_ir::lower_core(&source).unwrap();
        let first = compile_component(&ir).unwrap();
        assert_eq!(first, compile_component(&ir).unwrap());
        validate(&first);
    }
}

#[test]
fn thousand_function_component_is_valid_and_deterministic() {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("large.sico", 0);
    for index in 0..1_000_u32 {
        module.functions.push(Function {
            id: FunctionId(index + 1),
            name: format!("f{index:04}"),
            parameters: Vec::new(),
            return_type: Type::Int,
            effects: Vec::new(),
            locals: Vec::new(),
            entry: BlockId(0),
            blocks: vec![Block {
                id: BlockId(0),
                instructions: vec![Instruction {
                    result: ValueId(0),
                    ty: Type::Int,
                    operation: Operation::ConstInt(index.to_string()),
                    range,
                }],
                terminator: Terminator::Return(Some(ValueId(0))),
                range,
            }],
            range,
        });
    }
    let first = compile_component(&module).unwrap();
    assert_eq!(first, compile_component(&module).unwrap());
    validate(&first);
}

#[test]
fn repository_wit_contracts_parse_with_expected_namespace() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["boundary-probe-v0", "async-flow-v0", "script-profile-v0"] {
        let wit = repository.join("wit").join(name);
        let mut resolve = wit_parser::Resolve::default();
        let (package, sources) = resolve.push_dir(&wit).unwrap();
        assert_eq!(resolve.packages[package].name.namespace, "sico");
        assert!(sources.paths().next().is_some());
    }
}

#[test]
fn task_future_and_stream_have_explicit_backend_refusals() {
    for (feature, ty) in [
        ("Task", Type::Task(Box::new(Type::Int))),
        ("Future", Type::Future(Box::new(Type::Int))),
        ("Stream", Type::Stream(Box::new(Type::Int))),
    ] {
        let ir = identity_module(ty);
        assert!(matches!(
            compile_component(&ir),
            Err(CodegenError::AsyncUnsupported { feature: actual, contract, .. })
                if actual == feature && !contract.is_empty()
        ));
    }
}

#[test]
fn invalid_ir_and_unproven_arbitrary_int_never_emit_artifacts() {
    let mut invalid = select_module();
    invalid.schema = "invalid".into();
    assert!(matches!(compile(&invalid), Err(CodegenError::InvalidIr(_))));

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path =
        repository.join("syntax-candidates/b/numbers-units/valid/int-arbitrary-precision.sico");
    let source = SourceFile::from_text(
        SourceId::new(0),
        path.display().to_string(),
        fs::read_to_string(path).unwrap(),
    )
    .unwrap();
    let ir = sico_ir::lower_core(&source).unwrap();
    assert!(matches!(
        compile(&ir),
        Err(CodegenError::IntegerOutsideProvenI64 { .. })
    ));

    let mut non_constant = select_module();
    non_constant.functions[0].parameters[0].ty = Type::Int;
    non_constant.functions[0].blocks.truncate(1);
    non_constant.functions[0].blocks[0].terminator = Terminator::Return(Some(ValueId(0)));
    assert!(matches!(
        compile(&non_constant),
        Err(CodegenError::Unsupported { feature, .. }) if feature == "unbounded Int parameter"
    ));

    let aggregate_path =
        repository.join("syntax-candidates/b/nominal-invariants/valid/complete-record.sico");
    let aggregate_source = SourceFile::from_text(
        SourceId::new(0),
        aggregate_path.display().to_string(),
        fs::read_to_string(aggregate_path).unwrap(),
    )
    .unwrap();
    let aggregate_ir = sico_ir::lower_core(&aggregate_source).unwrap();
    assert!(matches!(
        compile_component(&aggregate_ir),
        Err(CodegenError::Unsupported { .. })
    ));

    let mut effectful = numeric_module();
    effectful.functions[0].effects.push("Console".into());
    assert!(matches!(
        compile_component(&effectful),
        Err(CodegenError::Unsupported { feature, .. })
            if feature == "effectful function without a Component host adapter"
    ));
}

fn select_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("select.sico", 0);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "select".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "condition".into(),
            ty: Type::Bool,
            range,
        }],
        return_type: Type::Int,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![
            Block {
                id: BlockId(0),
                instructions: Vec::new(),
                terminator: Terminator::Branch {
                    condition: ValueId(0),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
                range,
            },
            Block {
                id: BlockId(1),
                instructions: vec![Instruction {
                    result: ValueId(1),
                    ty: Type::Int,
                    operation: Operation::ConstInt("7".into()),
                    range,
                }],
                terminator: Terminator::Return(Some(ValueId(1))),
                range,
            },
            Block {
                id: BlockId(2),
                instructions: vec![Instruction {
                    result: ValueId(2),
                    ty: Type::Int,
                    operation: Operation::ConstInt("9".into()),
                    range,
                }],
                terminator: Terminator::Return(Some(ValueId(2))),
                range,
            },
        ],
        range,
    });
    module
}

fn numeric_module() -> Module {
    let source = SourceFile::from_text(
        SourceId::new(0),
        "answer.sico",
        "function main() returns Int:\n  return 40 + 2\nend function\n",
    )
    .unwrap();
    sico_ir::lower_core(&source).unwrap()
}

fn identity_module(ty: Type) -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("async-boundary.sico", 0);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "identity".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "value".into(),
            ty: ty.clone(),
            range,
        }],
        return_type: ty,
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
    module
}

fn fixed_width_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("fixed-width.sico", 0);
    for (index, (name, ty, operation, return_type)) in [
        (
            "i64_checked_add",
            Type::I64,
            Operation::CheckedAdd {
                left: ValueId(0),
                right: ValueId(1),
            },
            fixed_result(Type::I64),
        ),
        (
            "i64_checked_sub",
            Type::I64,
            Operation::CheckedSub {
                left: ValueId(0),
                right: ValueId(1),
            },
            fixed_result(Type::I64),
        ),
        (
            "u64_checked_add",
            Type::U64,
            Operation::CheckedAdd {
                left: ValueId(0),
                right: ValueId(1),
            },
            fixed_result(Type::U64),
        ),
        (
            "u64_checked_sub",
            Type::U64,
            Operation::CheckedSub {
                left: ValueId(0),
                right: ValueId(1),
            },
            fixed_result(Type::U64),
        ),
        (
            "i64_equal",
            Type::I64,
            Operation::EqualFixed {
                left: ValueId(0),
                right: ValueId(1),
            },
            Type::Bool,
        ),
        (
            "i64_less_than",
            Type::I64,
            Operation::LessFixed {
                left: ValueId(0),
                right: ValueId(1),
            },
            Type::Bool,
        ),
        (
            "u64_equal",
            Type::U64,
            Operation::EqualFixed {
                left: ValueId(0),
                right: ValueId(1),
            },
            Type::Bool,
        ),
        (
            "u64_less_than",
            Type::U64,
            Operation::LessFixed {
                left: ValueId(0),
                right: ValueId(1),
            },
            Type::Bool,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        module.functions.push(fixed_function(
            index,
            name,
            ty,
            operation,
            return_type,
            range,
        ));
    }
    module
}

#[allow(clippy::too_many_lines)]
fn general_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let parameter = |name: &str, ty: Type| Parameter {
        id: ValueId(0),
        name: name.into(),
        ty,
        range,
    };
    let mut module = Module::new("general.sico", 0);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "identity_i64".into(),
        parameters: vec![parameter("value", Type::I64)],
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
    module.functions.push(Function {
        id: FunctionId(2),
        name: "call_identity".into(),
        parameters: vec![parameter("value", Type::I64)],
        return_type: Type::I64,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(1),
                ty: Type::I64,
                operation: Operation::Call {
                    function: FunctionId(1),
                    arguments: vec![ValueId(0)],
                },
                range,
            }],
            terminator: Terminator::Return(Some(ValueId(1))),
            range,
        }],
        range,
    });
    module.functions.push(Function {
        id: FunctionId(3),
        name: "jump_chain".into(),
        parameters: vec![parameter("value", Type::I64)],
        return_type: Type::I64,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![
            Block {
                id: BlockId(0),
                instructions: Vec::new(),
                terminator: Terminator::Jump(BlockId(1)),
                range,
            },
            Block {
                id: BlockId(1),
                instructions: Vec::new(),
                terminator: Terminator::Jump(BlockId(2)),
                range,
            },
            Block {
                id: BlockId(2),
                instructions: Vec::new(),
                terminator: Terminator::Return(Some(ValueId(0))),
                range,
            },
        ],
        range,
    });
    module.functions.push(bool_match_function(4, range));
    module.functions.push(record_project_function(5, range));
    module.functions.push(variant_match_function(6, range));
    module.functions.push(cycle_gate_function(7, range));
    module
}

fn bool_match_function(id: u32, range: SourceRange) -> Function {
    Function {
        id: FunctionId(id),
        name: "bool_match".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "value".into(),
            ty: Type::Bool,
            range,
        }],
        return_type: Type::I64,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![
            Block {
                id: BlockId(0),
                instructions: Vec::new(),
                terminator: Terminator::Match {
                    values: vec![ValueId(0)],
                    arms: vec![
                        MatchArm {
                            patterns: vec![Pattern::Bool(true)],
                            target: BlockId(1),
                            range,
                        },
                        MatchArm {
                            patterns: vec![Pattern::Bool(false)],
                            target: BlockId(2),
                            range,
                        },
                    ],
                },
                range,
            },
            return_i64_block(1, 1, 41, range),
            return_i64_block(2, 2, 42, range),
        ],
        range,
    }
}

fn record_project_function(id: u32, range: SourceRange) -> Function {
    Function {
        id: FunctionId(id),
        name: "record_project".into(),
        parameters: Vec::new(),
        return_type: Type::I64,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(0),
                    ty: Type::I64,
                    operation: Operation::ConstI64(11),
                    range,
                },
                Instruction {
                    result: ValueId(1),
                    ty: Type::I64,
                    operation: Operation::ConstI64(22),
                    range,
                },
                Instruction {
                    result: ValueId(2),
                    ty: Type::Named("Pair".into()),
                    operation: Operation::Construct {
                        name: "Pair".into(),
                        fields: vec![
                            ConstructField {
                                name: "left".into(),
                                value: ValueId(0),
                            },
                            ConstructField {
                                name: "right".into(),
                                value: ValueId(1),
                            },
                        ],
                    },
                    range,
                },
                Instruction {
                    result: ValueId(3),
                    ty: Type::I64,
                    operation: Operation::Project {
                        base: ValueId(2),
                        field: "right".into(),
                    },
                    range,
                },
            ],
            terminator: Terminator::Return(Some(ValueId(3))),
            range,
        }],
        range,
    }
}

fn variant_match_function(id: u32, range: SourceRange) -> Function {
    Function {
        id: FunctionId(id),
        name: "variant_match".into(),
        parameters: Vec::new(),
        return_type: Type::I64,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![
            Block {
                id: BlockId(0),
                instructions: vec![Instruction {
                    result: ValueId(0),
                    ty: Type::Named("Flag".into()),
                    operation: Operation::Variant {
                        name: "Flag.On".into(),
                        payload: Vec::new(),
                    },
                    range,
                }],
                terminator: Terminator::Match {
                    values: vec![ValueId(0)],
                    arms: vec![
                        MatchArm {
                            patterns: vec![Pattern::Variant {
                                name: "Flag.Off".into(),
                                payload: Vec::new(),
                            }],
                            target: BlockId(1),
                            range,
                        },
                        MatchArm {
                            patterns: vec![Pattern::Variant {
                                name: "Flag.On".into(),
                                payload: Vec::new(),
                            }],
                            target: BlockId(2),
                            range,
                        },
                    ],
                },
                range,
            },
            return_i64_block(1, 1, 0, range),
            return_i64_block(2, 2, 1, range),
        ],
        range,
    }
}

fn cycle_gate_function(id: u32, range: SourceRange) -> Function {
    Function {
        id: FunctionId(id),
        name: "cycle_gate".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "exit".into(),
            ty: Type::Bool,
            range,
        }],
        return_type: Type::I64,
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![
            Block {
                id: BlockId(0),
                instructions: Vec::new(),
                terminator: Terminator::Branch {
                    condition: ValueId(0),
                    then_block: BlockId(1),
                    else_block: BlockId(0),
                },
                range,
            },
            return_i64_block(1, 1, 7, range),
        ],
        range,
    }
}

fn return_i64_block(id: u32, result: u32, value: i64, range: SourceRange) -> Block {
    Block {
        id: BlockId(id),
        instructions: vec![Instruction {
            result: ValueId(result),
            ty: Type::I64,
            operation: Operation::ConstI64(value),
            range,
        }],
        terminator: Terminator::Return(Some(ValueId(result))),
        range,
    }
}

fn fixed_function(
    index: usize,
    name: &str,
    ty: Type,
    operation: Operation,
    return_type: Type,
    range: SourceRange,
) -> Function {
    Function {
        id: FunctionId(u32::try_from(index + 1).unwrap()),
        name: name.into(),
        parameters: vec![
            Parameter {
                id: ValueId(0),
                name: "left".into(),
                ty: ty.clone(),
                range,
            },
            Parameter {
                id: ValueId(1),
                name: "right".into(),
                ty,
                range,
            },
        ],
        return_type: return_type.clone(),
        effects: Vec::new(),
        locals: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(2),
                ty: return_type,
                operation,
                range,
            }],
            terminator: Terminator::Return(Some(ValueId(2))),
            range,
        }],
        range,
    }
}

fn fixed_result(ok: Type) -> Type {
    Type::Result {
        ok: Box::new(ok),
        error: Box::new(Type::Named(sico_ir::NUMERIC_ERROR_TYPE.into())),
    }
}

fn validate(bytes: &[u8]) {
    wasmparser::Validator::new()
        .validate_all(bytes)
        .expect("backend output must pass the real Wasm validator");
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
