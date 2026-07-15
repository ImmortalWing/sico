use std::{fs, path::Path};

use sico_ir::{Operation, Terminator, VerifyErrorKind, canonical_json, lower_core, verify};
use sico_source::{SourceFile, SourceId};

const CASES: &[(&str, &str)] = &[
    (
        "CAP-001",
        "effects-capabilities/valid/explicit-boundary.sico",
    ),
    ("RES-001", "affine-resources/valid/close-once.sico"),
    ("RES-002", "affine-resources/valid/scoped-cleanup.sico"),
    ("REV-001", "revision/valid/versioned-commit.sico"),
    ("REV-002", "revision/valid/stale-load-ignored.sico"),
];

#[test]
fn five_flow_cases_lower_to_deterministic_verified_snapshots() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut snapshots = Vec::new();
    for (index, (case, relative)) in CASES.iter().enumerate() {
        let path = repository.join("syntax-candidates/b").join(relative);
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            path.display().to_string(),
            fs::read_to_string(path).unwrap(),
        )
        .unwrap();
        let module = lower_core(&source).unwrap_or_else(|error| panic!("{case}: {error:?}"));
        assert!(verify(&module).is_empty(), "{case}: {:?}", verify(&module));
        assert_eq!(module, lower_core(&source).unwrap());
        assert_eq!(
            canonical_json(&module).unwrap(),
            canonical_json(&module).unwrap()
        );
        snapshots.push(format!("{case}={}", shape(&module)));
    }
    let actual = format!("{}\n", snapshots.join("\n"));
    if std::env::var_os("SICO_DUMP_FLOW_IR").is_some() {
        print!("{actual}");
    } else {
        assert_eq!(actual, include_str!("../../../tests/ir/flow-lowering.snap"));
    }
}

#[test]
fn effect_resource_and_revision_mutations_are_rejected() {
    let mut capability = load("effects-capabilities/valid/explicit-boundary.sico");
    capability.functions[0].effects.clear();
    assert_kind(&capability, &VerifyErrorKind::UndeclaredEffect);

    let mut resource = load("affine-resources/valid/scoped-cleanup.sico");
    resource.functions[0].blocks[0]
        .instructions
        .retain(|instruction| !matches!(instruction.operation, Operation::ResourceDrop(_)));
    assert_kind(&resource, &VerifyErrorKind::ResourceLeak);

    let mut double_use = load("affine-resources/valid/close-once.sico");
    let parameter = double_use.functions[0].parameters[0].id;
    let result = double_use.functions[0].blocks[0].instructions[0].result;
    let range = double_use.functions[0].blocks[0].instructions[0].range;
    double_use.functions[0].blocks[0].instructions.insert(
        1,
        sico_ir::Instruction {
            result: sico_ir::ValueId(result.0 + 1),
            ty: sico_ir::Type::Unit,
            operation: Operation::ResourceDrop(parameter),
            range,
        },
    );
    assert_kind(&double_use, &VerifyErrorKind::ResourceViolation);

    let mut revision = load("revision/valid/stale-load-ignored.sico");
    let model = revision.functions[0].parameters[0].id;
    let Terminator::Branch { condition, .. } = &mut revision.functions[0].blocks[0].terminator
    else {
        panic!("expected revision branch")
    };
    *condition = model;
    assert_kind(&revision, &VerifyErrorKind::RevisionGuard);
}

#[test]
fn cumulative_capability_is_seventeen_lowered_and_eight_typed_refused() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    collect_sico(&repository.join("syntax-candidates/b"), &mut paths);
    paths.sort();
    let mut lowered = 0;
    let mut unsupported = 0;
    for (index, path) in paths.iter().enumerate() {
        let text = fs::read_to_string(path).unwrap();
        if !text.contains("// expect: accept") {
            continue;
        }
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            path.display().to_string(),
            text,
        )
        .unwrap();
        match lower_core(&source) {
            Ok(module) => {
                lowered += 1;
                assert!(verify(&module).is_empty());
            }
            Err(sico_ir::CoreLowerError::Unsupported { .. }) => unsupported += 1,
            Err(error) => panic!("{}: {error:?}", path.display()),
        }
    }
    assert_eq!((lowered, unsupported), (17, 8));
}

fn load(relative: &str) -> sico_ir::Module {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = repository.join("syntax-candidates/b").join(relative);
    let source = SourceFile::from_text(
        SourceId::new(0),
        path.display().to_string(),
        fs::read_to_string(path).unwrap(),
    )
    .unwrap();
    lower_core(&source).unwrap()
}

fn assert_kind(module: &sico_ir::Module, kind: &VerifyErrorKind) {
    let errors = verify(module);
    assert!(errors.iter().any(|error| &error.kind == kind), "{errors:?}");
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
                        .map(|instruction| operation_name(&instruction.operation))
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

fn operation_name(operation: &Operation) -> &'static str {
    match operation {
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
    }
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
