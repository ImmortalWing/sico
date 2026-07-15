use sico_ir::{
    Block, BlockId, EntryError, Function, FunctionId, Instruction, Module, Operation, Parameter,
    SourceRange, Terminator, Type, ValueId, VerifyErrorKind, canonical_json,
    require_semantic_success, verify,
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
        Type::Named("UserId".into()),
        Type::Option(Box::new(Type::Int)),
        Type::Result {
            ok: Box::new(Type::Int),
            error: Box::new(Type::Named("Failure".into())),
        },
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
