use std::{fs, path::Path};

use sico_ir::{
    BlockId, CoreLowerError, Operation, Terminator, Type, VerifyErrorKind, canonical_json,
    lower_core, verify,
};
use sico_source::{SourceFile, SourceId};

const SUPPORTED: &[&str] = &[
    "CAP-002",
    "MATCH-001",
    "MATCH-002",
    "MATCH-003",
    "NOM-001",
    "NOM-002",
    "NOM-003",
    "NUM-001",
    "NUM-002",
    "NUM-003",
    "NUM-004",
    "RESULT-001",
];

#[test]
fn twelve_core_cases_lower_deterministically_and_remaining_valid_cases_refuse() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    collect_sico(&repository.join("syntax-candidates/b"), &mut paths);
    paths.sort();
    let mut lowered = 0;
    let mut snapshots = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let text = fs::read_to_string(path).unwrap();
        if !text.contains("// expect: accept") {
            continue;
        }
        let case = metadata(&text, "case").to_owned();
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            path.display().to_string(),
            text,
        )
        .unwrap();
        let result = lower_core(&source);
        if !SUPPORTED.contains(&case.as_str()) {
            assert!(
                matches!(result, Ok(_) | Err(CoreLowerError::Unsupported { .. })),
                "unexpected non-capability result for {case}: {result:?}"
            );
            continue;
        }
        match result {
            Ok(module) => {
                lowered += 1;
                assert!(verify(&module).is_empty());
                assert_eq!(module, lower_core(&source).unwrap());
                assert_eq!(
                    canonical_json(&module).unwrap(),
                    canonical_json(&module).unwrap()
                );
                snapshots.push(format!("{case}={}", shape(&module)));
            }
            Err(CoreLowerError::Unsupported { .. }) => {
                panic!("unexpected refusal for {case}");
            }
            Err(error) => panic!("{case}: {error:?}"),
        }
    }
    assert_eq!(lowered, 12);
    snapshots.sort();
    let actual = format!("{}\n", snapshots.join("\n"));
    if std::env::var_os("SICO_DUMP_CORE_IR").is_some() {
        print!("{actual}");
    } else {
        assert_eq!(actual, include_str!("../../../tests/ir/core-lowering.snap"));
    }
}

#[test]
fn expression_operands_and_constructor_fields_lower_left_to_right_once() {
    let text = "record Pair:\n  field left: Int\n  field right: Int\nend record\n\nfunction main() returns Pair:\n  return Pair(left: 1 + 2, right: 3 + 4)\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "order.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    let operations: Vec<_> = module.functions[0].blocks[0]
        .instructions
        .iter()
        .map(|instruction| &instruction.operation)
        .collect();
    assert!(matches!(operations[0], Operation::ConstInt(value) if value == "1"));
    assert!(matches!(operations[1], Operation::ConstInt(value) if value == "2"));
    assert!(matches!(operations[2], Operation::AddInt { .. }));
    assert!(matches!(operations[3], Operation::ConstInt(value) if value == "3"));
    assert!(matches!(operations[4], Operation::ConstInt(value) if value == "4"));
    assert!(matches!(operations[5], Operation::AddInt { .. }));
    assert!(matches!(operations[6], Operation::Construct { name, .. } if name == "Pair"));
}

#[test]
fn semantic_error_wins_before_backend_support_classification() {
    let source = SourceFile::from_text(
        SourceId::new(0),
        "invalid-async.sico",
        "async function bad() returns Int:\n  return \"wrong\"\nend function\n",
    )
    .unwrap();
    let CoreLowerError::Semantic(diagnostics) = lower_core(&source).unwrap_err() else {
        panic!("semantic gate must run first")
    };
    assert_eq!(diagnostics[0].code, "E2001");
}

#[test]
fn all_twenty_nine_invalid_cases_stop_before_ir_support_dispatch() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let map: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repository.join("diagnostics/semantic-case-map.json")).unwrap(),
    )
    .unwrap();
    let expected: std::collections::BTreeMap<_, _> = map["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            (
                case["case"].as_str().unwrap().to_owned(),
                case["code"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let mut paths = Vec::new();
    collect_sico(&repository.join("syntax-candidates/b"), &mut paths);
    paths.sort();
    let mut rejected = 0;
    for (index, path) in paths.iter().enumerate() {
        let text = fs::read_to_string(path).unwrap();
        if !text.contains("// expect: reject") {
            continue;
        }
        rejected += 1;
        let case = metadata(&text, "case").to_owned();
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            path.display().to_string(),
            text,
        )
        .unwrap();
        let CoreLowerError::Semantic(diagnostics) = lower_core(&source).unwrap_err() else {
            panic!("{case} reached IR support dispatch")
        };
        assert_eq!(diagnostics.len(), 1, "{case}");
        assert_eq!(diagnostics[0].code, expected[case.as_str()], "{case}");
    }
    assert_eq!(rejected, 29);
}

#[test]
fn intrinsic_try_and_match_mutations_are_rejected_independently() {
    let intrinsic_source = SourceFile::from_text(
        SourceId::new(0),
        "intrinsic.sico",
        "function main() returns Float64:\n  return Float64.from_int(1)\nend function\n",
    )
    .unwrap();
    let mut intrinsic = lower_core(&intrinsic_source).unwrap();
    let Operation::Intrinsic { name, .. } =
        &mut intrinsic.functions[0].blocks[0].instructions[1].operation
    else {
        panic!("expected intrinsic")
    };
    *name = "unknown".into();
    assert!(
        verify(&intrinsic)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::UnknownTarget)
    );

    let match_source = SourceFile::from_text(
        SourceId::new(1),
        "match.sico",
        "enum Flag:\n  case On\n  case Off\nend enum\n\nfunction value(flag: Flag) returns Bool:\n  match flag:\n    case Flag.On:\n      return true\n    case Flag.Off:\n      return false\n  end match\nend function\n",
    )
    .unwrap();
    let mut matched = lower_core(&match_source).unwrap();
    let Terminator::Match { arms, .. } = &mut matched.functions[0].blocks[0].terminator else {
        panic!("expected match")
    };
    arms[0].target = BlockId(99);
    assert!(
        verify(&matched)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::UnknownTarget)
    );

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result_path =
        repository.join("syntax-candidates/b/result-mapping/valid/same-error-try.sico");
    let result_source = SourceFile::from_text(
        SourceId::new(2),
        result_path.display().to_string(),
        fs::read_to_string(result_path).unwrap(),
    )
    .unwrap();
    let mut result = lower_core(&result_source).unwrap();
    let try_instruction = result.functions[1].blocks[0]
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction.operation, Operation::Try(_)))
        .unwrap();
    try_instruction.ty = Type::Bool;
    assert!(
        verify(&result)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::TypeMismatch)
    );
}

fn shape(module: &sico_ir::Module) -> String {
    module
        .functions
        .iter()
        .map(|function| {
            let blocks = function
                .blocks
                .iter()
                .map(|block| {
                    let operations = block
                        .instructions
                        .iter()
                        .map(|instruction| match &instruction.operation {
                            Operation::ConstInt(_) => "const-int",
                            Operation::ConstBool(_) => "const-bool",
                            Operation::ConstString(_) => "const-string",
                            Operation::Copy(_) => "copy",
                            Operation::AddInt { .. } => "add-int",
                            Operation::Call { .. } => "call",
                            Operation::Intrinsic { .. } => "intrinsic",
                            Operation::Construct { .. } => "construct",
                            Operation::Project { .. } => "project",
                            Operation::Variant { .. } => "variant",
                            Operation::EffectCall { .. } => "effect-call",
                            Operation::ResourceCall { .. } => "resource-call",
                            Operation::ResourceMove(_) => "resource-move",
                            Operation::ResourceBorrow(_) => "resource-borrow",
                            Operation::ResourceDrop(_) => "resource-drop",
                            Operation::RevisionCheck { .. } => "revision-check",
                            Operation::Try(_) => "try",
                            Operation::Await(_) => "await",
                            Operation::StreamNext(_) => "stream-next",
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let terminator = match block.terminator {
                        Terminator::Return(_) => "return",
                        Terminator::Jump(_) => "jump",
                        Terminator::Branch { .. } => "branch",
                        Terminator::Match { .. } => "match",
                        Terminator::Unreachable => "unreachable",
                    };
                    format!("b{}[{operations}]>{terminator}", block.id.0)
                })
                .collect::<Vec<_>>()
                .join(";");
            format!("f{}:{}({blocks})", function.id.0, function.name)
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn metadata<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("// {key}: ")))
        .unwrap()
}

fn collect_sico(root: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_sico(&path, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "sico")
        {
            output.push(path);
        }
    }
}
