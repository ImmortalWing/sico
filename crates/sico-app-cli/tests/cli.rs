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
