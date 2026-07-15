use std::{fmt::Write as _, fs, path::Path};

use sico_codegen_wasm::{CodegenError, compile, compile_component};
use sico_ir::{
    Block, BlockId, Function, FunctionId, Instruction, Module, Operation, Parameter, SourceRange,
    Terminator, Type, ValueId,
};
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
fn boundary_probe_wit_parses_with_result_record_and_resource_shapes() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["boundary-probe-v0", "async-flow-v0"] {
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

fn validate(bytes: &[u8]) {
    wasmparser::Validator::new()
        .validate_all(bytes)
        .expect("backend output must pass the real Wasm validator");
}

fn snapshot(name: &str, bytes: &[u8]) {
    let actual = bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").unwrap();
            output
        },
    );
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
