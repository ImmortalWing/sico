use std::collections::BTreeMap;

use sico_ir::{
    Block, BlockId, ConstructField, EntryError, Function, FunctionId, Instruction,
    MAX_TASK_SPAWNS_PER_SCOPE, Module, Operation, Parameter, SourceRange, TaskScope, Terminator,
    Type, ValueId, VerifyErrorKind, canonical_json, require_semantic_success, verify,
};
use sico_source::{SourceFile, SourceId};

type Mutation = (Box<dyn Fn(&mut Module)>, VerifyErrorKind);

fn range(start: u32, end: u32) -> SourceRange {
    SourceRange { start, end }
}

fn integer_identity() -> Module {
    let mut module = Module::new("identity.sico", 64);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "identity".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "value".into(),
            ty: Type::Int,
            range: range(18, 23),
        }],
        return_type: Type::Int,
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(1),
                ty: Type::Int,
                operation: Operation::Copy(ValueId(0)),
                range: range(42, 47),
            }],
            terminator: Terminator::Return(Some(ValueId(1))),
            range: range(32, 51),
        }],
        range: range(0, 64),
    });
    module
}

#[test]
fn valid_contract_is_deterministic_and_roundtrips() {
    let module = integer_identity();
    assert!(verify(&module).is_empty());
    let first = canonical_json(&module).unwrap();
    assert_eq!(first, canonical_json(&module).unwrap());
    assert_eq!(serde_json::from_str::<Module>(&first).unwrap(), module);
    assert!(first.starts_with("{\"schema\":\"sico.ir.v0\""));
}

#[test]
fn verifier_rejects_malformed_ids_ranges_values_types_targets_and_constants() {
    let mutations: Vec<Mutation> = vec![
        (
            Box::new(|module| module.schema = "wrong".into()),
            VerifyErrorKind::Schema,
        ),
        (
            Box::new(|module| module.functions[0].id = FunctionId(9)),
            VerifyErrorKind::NonCanonicalId,
        ),
        (
            Box::new(|module| module.functions[0].range.end = 65),
            VerifyErrorKind::InvalidRange,
        ),
        (
            Box::new(|module| module.functions[0].entry = BlockId(7)),
            VerifyErrorKind::MissingEntry,
        ),
        (
            Box::new(|module| module.functions[0].blocks[0].instructions[0].result = ValueId(8)),
            VerifyErrorKind::NonCanonicalId,
        ),
        (
            Box::new(|module| {
                module.functions[0].blocks[0].instructions[0].operation =
                    Operation::Copy(ValueId(99));
            }),
            VerifyErrorKind::UndefinedValue,
        ),
        (
            Box::new(|module| module.functions[0].blocks[0].terminator = Terminator::Return(None)),
            VerifyErrorKind::TypeMismatch,
        ),
        (
            Box::new(|module| {
                module.functions[0].blocks[0].instructions[0].operation =
                    Operation::ConstInt("-0".into());
            }),
            VerifyErrorKind::InvalidConstant,
        ),
        (
            Box::new(|module| {
                module.functions[0].blocks[0].terminator = Terminator::Jump(BlockId(9));
            }),
            VerifyErrorKind::UnknownTarget,
        ),
    ];
    for (mutate, expected) in mutations {
        let mut module = integer_identity();
        mutate(&mut module);
        let errors = verify(&module);
        assert!(
            errors.iter().any(|error| error.kind == expected),
            "{errors:?}"
        );
        assert!(canonical_json(&module).is_err());
    }
}

#[test]
fn branch_and_declared_effect_shapes_verify() {
    let mut module = integer_identity();
    let function = &mut module.functions[0];
    function.effects = vec!["io".into()];
    function.blocks = vec![
        Block {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(1),
                ty: Type::Bool,
                operation: Operation::ConstBool(true),
                range: range(30, 34),
            }],
            terminator: Terminator::Branch {
                condition: ValueId(1),
                then_block: BlockId(1),
                else_block: BlockId(2),
            },
            range: range(28, 40),
        },
        Block {
            id: BlockId(1),
            instructions: vec![Instruction {
                result: ValueId(2),
                ty: Type::Int,
                operation: Operation::EffectCall {
                    effect: "io".into(),
                    arguments: vec![],
                },
                range: range(41, 45),
            }],
            terminator: Terminator::Return(Some(ValueId(2))),
            range: range(41, 50),
        },
        Block {
            id: BlockId(2),
            instructions: vec![Instruction {
                result: ValueId(3),
                ty: Type::Int,
                operation: Operation::ConstInt("0".into()),
                range: range(51, 52),
            }],
            terminator: Terminator::Return(Some(ValueId(3))),
            range: range(51, 55),
        },
    ];
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
}

#[test]
fn semantic_gate_blocks_invalid_and_frontend_error_sources() {
    let invalid = SourceFile::from_text(
        SourceId::new(0),
        "invalid.sico",
        "function bad() returns Int:\n  return \"wrong\"\nend function\n",
    )
    .unwrap();
    let error = require_semantic_success(&invalid).unwrap_err();
    let EntryError::Semantic(diagnostics) = error else {
        panic!("expected semantic error")
    };
    assert_eq!(diagnostics[0].code, "E2001");

    let syntax = SourceFile::from_text(
        SourceId::new(1),
        "syntax.sico",
        "function broken( returns Int:\nend function\n",
    )
    .unwrap();
    assert!(matches!(
        require_semantic_success(&syntax),
        Err(EntryError::Frontend(_))
    ));
}

#[test]
fn all_phase_types_have_stable_serialization() {
    let types = vec![
        Type::Unit,
        Type::Bool,
        Type::Int,
        Type::Float64,
        Type::String,
        Type::Bytes,
        Type::List(Box::new(Type::String)),
        Type::Named("UserId".into()),
        Type::Option(Box::new(Type::Int)),
        Type::Result {
            ok: Box::new(Type::Int),
            error: Box::new(Type::Named("Failure".into())),
        },
        Type::Capability("Console".into()),
        Type::OwnedResource("File".into()),
        Type::BorrowedResource("File".into()),
        Type::Task(Box::new(Type::Int)),
        Type::Future(Box::new(Type::Int)),
        Type::Stream(Box::new(Type::Int)),
    ];
    let first = serde_json::to_string(&types).unwrap();
    assert_eq!(first, serde_json::to_string(&types).unwrap());
}

#[test]
fn verifier_diagnostic_cap_is_exact() {
    let mut module = Module::new("many-errors.sico", 0);
    for _ in 0..101 {
        module.functions.push(Function {
            id: FunctionId(0),
            name: "bad".into(),
            parameters: Vec::new(),
            return_type: Type::Unit,
            effects: Vec::new(),
            entry: BlockId(0),
            blocks: Vec::new(),
            range: range(0, 0),
        });
    }
    assert_eq!(verify(&module).len(), 100);
}

/// RFC-0036 §5 shape: `run` opens one task scope, spawns `compute` twice,
/// awaits the first handle, collects the second through a list literal,
/// and closes the scope. `compute` is the spawned Future-returning callee.
#[allow(clippy::too_many_lines)]
fn task_module() -> Module {
    let mut module = Module::new("tasks.sico", 512);
    module.task_scopes = Some(BTreeMap::from([(
        FunctionId(1),
        vec![TaskScope {
            scope: 0,
            parent: None,
        }],
    )]));
    module.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "pending".into(),
            ty: Type::Future(Box::new(Type::Int)),
            range: range(14, 21),
        }],
        return_type: Type::List(Box::new(Type::Int)),
        effects: vec!["io".into()],
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(1),
                    ty: Type::Unit,
                    operation: Operation::TaskScopeOpen { scope: 0 },
                    range: range(40, 54),
                },
                Instruction {
                    result: ValueId(2),
                    ty: Type::Task(Box::new(Type::Int)),
                    operation: Operation::Spawn {
                        scope: 0,
                        callee: FunctionId(2),
                        arguments: vec![ValueId(0)],
                    },
                    range: range(55, 72),
                },
                Instruction {
                    result: ValueId(3),
                    ty: Type::Task(Box::new(Type::Int)),
                    operation: Operation::Spawn {
                        scope: 0,
                        callee: FunctionId(2),
                        arguments: vec![ValueId(0)],
                    },
                    range: range(73, 90),
                },
                Instruction {
                    result: ValueId(4),
                    ty: Type::Int,
                    operation: Operation::Await(ValueId(2)),
                    range: range(91, 100),
                },
                Instruction {
                    result: ValueId(5),
                    ty: Type::List(Box::new(Type::Task(Box::new(Type::Int)))),
                    operation: Operation::Construct {
                        name: String::new(),
                        fields: vec![ConstructField {
                            name: "0".into(),
                            value: ValueId(3),
                        }],
                    },
                    range: range(101, 110),
                },
                Instruction {
                    result: ValueId(6),
                    ty: Type::List(Box::new(Type::Int)),
                    operation: Operation::TaskCollect {
                        scope: 0,
                        tasks: ValueId(5),
                    },
                    range: range(111, 130),
                },
                Instruction {
                    result: ValueId(7),
                    ty: Type::Unit,
                    operation: Operation::TaskScopeClose { scope: 0 },
                    range: range(131, 140),
                },
            ],
            terminator: Terminator::Return(Some(ValueId(6))),
            range: range(40, 145),
        }],
        range: range(0, 146),
    });
    module.functions.push(Function {
        id: FunctionId(2),
        name: "compute".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "input".into(),
            ty: Type::Future(Box::new(Type::Int)),
            range: range(160, 165),
        }],
        return_type: Type::Future(Box::new(Type::Int)),
        effects: vec!["io".into()],
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(1),
                ty: Type::Future(Box::new(Type::Int)),
                operation: Operation::Copy(ValueId(0)),
                range: range(180, 185),
            }],
            terminator: Terminator::Return(Some(ValueId(1))),
            range: range(180, 190),
        }],
        range: range(150, 191),
    });
    module
}

#[test]
fn task_scope_spawn_await_and_collect_verify() {
    let module = task_module();
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
    assert!(canonical_json(&module).is_ok());
}

#[test]
fn task_scope_table_and_operations_have_stable_serialization() {
    let module = task_module();
    let first = canonical_json(&module).unwrap();
    assert_eq!(first, canonical_json(&module).unwrap());
    assert_eq!(serde_json::from_str::<Module>(&first).unwrap(), module);
    assert!(first.contains("\"task_scopes\":{\"1\":[{\"scope\":0,\"parent\":null}]}"));
    assert!(first.contains("{\"op\":\"task_scope_open\",\"data\":{\"scope\":0}}"));
    assert!(
        first.contains("{\"op\":\"spawn\",\"data\":{\"scope\":0,\"callee\":2,\"arguments\":[0]}}")
    );
    assert!(first.contains("{\"op\":\"task_collect\",\"data\":{\"scope\":0,\"tasks\":5}}"));
    assert!(first.contains("{\"op\":\"task_scope_close\",\"data\":{\"scope\":0}}"));
    // Modules without task structure keep their exact previous shape.
    let identity = canonical_json(&integer_identity()).unwrap();
    assert!(!identity.contains("task_scopes"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn task_verifier_rejects_scope_spawn_collect_and_affine_mutations() {
    let mutations: Vec<Mutation> = vec![
        // A scope referenced by an operation must be declared.
        (
            Box::new(|module| {
                module.functions[0].blocks[0].instructions[0].operation =
                    Operation::TaskScopeOpen { scope: 1 };
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // Open/close must be LIFO-balanced within the block.
        (
            Box::new(|module| {
                module.functions[0].blocks[0].instructions[6].operation =
                    Operation::TaskScopeOpen { scope: 0 };
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // The scope table rejects duplicate declaration ids.
        (
            Box::new(|module| {
                module.task_scopes = Some(BTreeMap::from([(
                    FunctionId(1),
                    vec![
                        TaskScope {
                            scope: 0,
                            parent: None,
                        },
                        TaskScope {
                            scope: 0,
                            parent: None,
                        },
                    ],
                )]));
            }),
            VerifyErrorKind::DuplicateId,
        ),
        // Scope table keys must reference declared functions.
        (
            Box::new(|module| {
                module.task_scopes = Some(BTreeMap::from([(
                    FunctionId(9),
                    vec![TaskScope {
                        scope: 0,
                        parent: None,
                    }],
                )]));
            }),
            VerifyErrorKind::UnknownTarget,
        ),
        // The spawned callee must return Future[T].
        (
            Box::new(|module| {
                module.functions[1].parameters[0].ty = Type::Int;
                module.functions[1].return_type = Type::Int;
                module.functions[1].blocks[0].instructions[0].ty = Type::Int;
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // Callee effects must be a subset of the caller's declared effects.
        (
            Box::new(|module| {
                module.functions[1].effects = vec!["net".into()];
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // Collect requires a List[Task[T]] operand.
        (
            Box::new(|module| {
                module.functions[0].blocks[0].instructions[5].operation = Operation::TaskCollect {
                    scope: 0,
                    tasks: ValueId(0),
                };
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // A Task handle consumed twice.
        (
            Box::new(|module| {
                let instructions = &mut module.functions[0].blocks[0].instructions;
                instructions.insert(
                    4,
                    Instruction {
                        result: ValueId(5),
                        ty: Type::Int,
                        operation: Operation::Await(ValueId(2)),
                        range: range(91, 100),
                    },
                );
                for (index, instruction) in instructions.iter_mut().enumerate() {
                    instruction.result = ValueId(u32::try_from(index + 1).unwrap());
                }
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // A Task handle used after its scope closed.
        (
            Box::new(|module| {
                let function = &mut module.functions[0];
                function.return_type = Type::Unit;
                function.blocks[0].instructions = vec![
                    Instruction {
                        result: ValueId(1),
                        ty: Type::Unit,
                        operation: Operation::TaskScopeOpen { scope: 0 },
                        range: range(40, 54),
                    },
                    Instruction {
                        result: ValueId(2),
                        ty: Type::Task(Box::new(Type::Int)),
                        operation: Operation::Spawn {
                            scope: 0,
                            callee: FunctionId(2),
                            arguments: vec![ValueId(0)],
                        },
                        range: range(55, 72),
                    },
                    Instruction {
                        result: ValueId(3),
                        ty: Type::Unit,
                        operation: Operation::TaskScopeClose { scope: 0 },
                        range: range(131, 140),
                    },
                    Instruction {
                        result: ValueId(4),
                        ty: Type::Int,
                        operation: Operation::Await(ValueId(2)),
                        range: range(91, 100),
                    },
                ];
                function.blocks[0].terminator = Terminator::Return(None);
            }),
            VerifyErrorKind::TaskViolation,
        ),
    ];
    for (mutate, expected) in mutations {
        let mut module = task_module();
        mutate(&mut module);
        let errors = verify(&module);
        assert!(
            errors.iter().any(|error| error.kind == expected),
            "{errors:?}"
        );
        assert!(canonical_json(&module).is_err());
    }
}

#[test]
fn task_verifier_rejects_region_and_limit_mutations() {
    let mutations: Vec<Mutation> = vec![
        // IR v1: a scope region must not span blocks.
        (
            Box::new(|module| {
                let function = &mut module.functions[0];
                function.return_type = Type::Unit;
                function.blocks = vec![
                    Block {
                        id: BlockId(0),
                        instructions: vec![Instruction {
                            result: ValueId(1),
                            ty: Type::Unit,
                            operation: Operation::TaskScopeOpen { scope: 0 },
                            range: range(40, 54),
                        }],
                        terminator: Terminator::Jump(BlockId(1)),
                        range: range(40, 60),
                    },
                    Block {
                        id: BlockId(1),
                        instructions: vec![Instruction {
                            result: ValueId(2),
                            ty: Type::Unit,
                            operation: Operation::TaskScopeClose { scope: 0 },
                            range: range(61, 70),
                        }],
                        terminator: Terminator::Return(None),
                        range: range(61, 75),
                    },
                ];
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // Parent-chain depth is bounded by 64.
        (
            Box::new(|module| {
                module.task_scopes = Some(BTreeMap::from([(
                    FunctionId(1),
                    (0..65)
                        .map(|scope| TaskScope {
                            scope,
                            parent: scope.checked_sub(1),
                        })
                        .collect(),
                )]));
            }),
            VerifyErrorKind::TaskViolation,
        ),
        // A single scope admits at most 1,024 spawns.
        (
            Box::new(|module| {
                let function = &mut module.functions[0];
                function.return_type = Type::Unit;
                let mut instructions = vec![Instruction {
                    result: ValueId(1),
                    ty: Type::Unit,
                    operation: Operation::TaskScopeOpen { scope: 0 },
                    range: range(40, 54),
                }];
                for index in 0..=MAX_TASK_SPAWNS_PER_SCOPE {
                    instructions.push(Instruction {
                        result: ValueId(u32::try_from(index + 2).unwrap()),
                        ty: Type::Task(Box::new(Type::Int)),
                        operation: Operation::Spawn {
                            scope: 0,
                            callee: FunctionId(2),
                            arguments: vec![ValueId(0)],
                        },
                        range: range(55, 72),
                    });
                }
                instructions.push(Instruction {
                    result: ValueId(u32::try_from(MAX_TASK_SPAWNS_PER_SCOPE + 3).unwrap()),
                    ty: Type::Unit,
                    operation: Operation::TaskScopeClose { scope: 0 },
                    range: range(131, 140),
                });
                function.blocks[0].instructions = instructions;
                function.blocks[0].terminator = Terminator::Return(None);
            }),
            VerifyErrorKind::Limit,
        ),
    ];
    for (mutate, expected) in mutations {
        let mut module = task_module();
        mutate(&mut module);
        let errors = verify(&module);
        assert!(
            errors.iter().any(|error| error.kind == expected),
            "{errors:?}"
        );
        assert!(canonical_json(&module).is_err());
    }
}
