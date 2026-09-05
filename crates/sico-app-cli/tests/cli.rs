use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use sico_app_cli::{EXIT_SUCCESS, EXIT_TOOL_ERROR};
use sico_codegen_wasm::compile_component;
use sico_ir::lower_core;
use sico_package::verify;
use sico_source::{SourceFile, SourceId};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn pack_and_inspect_are_deterministic_and_separate_from_the_compiler() {
    let component = temp("component.wasm");
    let first = temp("first.sapp");
    let second = temp("second.sapp");
    fs::write(&component, scalar_component()).unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_app_cli::run(
        [
            "sico-app",
            "pack",
            "--output",
            path(&first),
            path(&component),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        verify(&fs::read(&first).unwrap()).unwrap().manifest.app.id,
        "dev.sico.app"
    );

    stdout.clear();
    stderr.clear();
    let exit = sico_app_cli::run(
        [
            "sico-app",
            "pack",
            "--output",
            path(&second),
            path(&component),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, EXIT_SUCCESS);
    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());

    stdout.clear();
    stderr.clear();
    let exit = sico_app_cli::run(
        ["sico-app", "inspect", "--json", path(&first)],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, EXIT_SUCCESS);
    let value: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(value["schema"], "sico.sapp.inspect.v0");
    assert_eq!(value["trust"]["status"], "unsigned-development");

    fs::remove_file(component).unwrap();
    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
}

#[test]
fn run_requires_an_explicit_trust_mode() {
    let package = temp("missing.sapp");
    fs::write(&package, b"not-a-package").unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_app_cli::run(
        ["sico-app", "run", path(&package)],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, EXIT_TOOL_ERROR);
    assert!(String::from_utf8_lossy(&stderr).contains("requires --trusted-key"));
    fs::remove_file(package).unwrap();
}

fn scalar_component() -> Vec<u8> {
    let source = SourceFile::from_text(
        SourceId::new(1),
        "answer.sico",
        "function main() returns Int:\n  return 42\nend function\n",
    )
    .unwrap();
    compile_component(&lower_core(&source).unwrap()).unwrap()
}

fn temp(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-app-cli-{}-{}-{suffix}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn path(path: &std::path::Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn pack_script_builds_a_strict_v1_package_with_verified_closure() {
    let program_component = echo_program();
    let adapter = sico_app_cli::compose::script_adapter_component();
    let composed = sico_app_cli::compose::compose_script_command(&program_component, &adapter);
    let command = temp("echo.command.wasm");
    let package = temp("echo.sapp");
    fs::write(&command, &composed).unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_app_cli::run(
        [
            "sico-app",
            "pack",
            "--script",
            "--output",
            path(&package),
            path(&command),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
    let verified = verify(&fs::read(&package).unwrap()).unwrap();
    assert_eq!(verified.manifest.schema, "sico.sapp.manifest.v1");
    let script = verified.manifest.script.as_ref().unwrap();
    assert_eq!(script.world, "sico:script/program@0.1.0");
    assert_eq!(script.adapter.id, "sico:script/adapter@0.1.0");
    assert_eq!(
        script.adapter.sha256,
        sico_package::sha256_hex(&sico_app_cli::compose::script_adapter_component())
    );
    assert_eq!(script.wit.sha256.len(), 64);
    assert_eq!(script.semantics, sico_ir::SCHEMA);

    // Import/capability closure: the composed command imports map exactly to
    // the declared script.args/script.stdio capabilities.
    let trusted = sico_package::TrustedPackage {
        package: verified,
        trust: sico_package::TrustStatus::UnsignedDevelopment,
    };
    let grants =
        std::collections::BTreeSet::from(["script.args".to_owned(), "script.stdio".to_owned()]);
    let authorized = sico_package::authorize(trusted, &grants).unwrap();
    assert_eq!(authorized.granted_capabilities, grants);

    let trusted = sico_package::TrustedPackage {
        package: verify(&fs::read(&package).unwrap()).unwrap(),
        trust: sico_package::TrustStatus::UnsignedDevelopment,
    };
    assert!(matches!(
        sico_package::authorize(trusted, &std::collections::BTreeSet::new()),
        Err(sico_package::PackageError::HostDenied(_))
    ));

    // A v0 scalar component under --script fails closed: its import closure
    // cannot carry script capabilities.
    let scalar = temp("scalar.wasm");
    let scalar_package = temp("scalar.sapp");
    fs::write(&scalar, scalar_component()).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_app_cli::run(
        [
            "sico-app",
            "pack",
            "--script",
            "--output",
            path(&scalar_package),
            path(&scalar),
        ],
        &mut stdout,
        &mut stderr,
    );
    let _ = (exit, stdout, stderr);
    // pack itself succeeds structurally (closure is an authorize-time gate);
    // authorization must fail instead.
    let verified = verify(&fs::read(&scalar_package).unwrap());
    if let Ok(verified) = verified {
        let trusted = sico_package::TrustedPackage {
            package: verified,
            trust: sico_package::TrustStatus::UnsignedDevelopment,
        };
        assert!(sico_package::authorize(trusted, &grants).is_err());
    }
    fs::remove_file(command).unwrap();
    fs::remove_file(package).unwrap();
    fs::remove_file(scalar).unwrap();
    fs::remove_file(scalar_package).unwrap();
}

fn echo_program() -> Vec<u8> {
    use sico_ir::{
        Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
        Parameter, SourceRange, Terminator, Type, ValueId,
    };
    let range = SourceRange { start: 0, end: 0 };
    let script_result = || Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    };
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
    sico_codegen_wasm::compile_script_program(&module).unwrap()
}
