use std::{
    ffi::OsString,
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_codegen_wasm::compile_component;
use sico_desktop_host::{EXIT_ERROR, EXIT_SUCCESS, run, windows_association_plan};
use sico_host_core::InstalledRevision;
use sico_ir::lower_core;
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sign_development};
use sico_source::{SourceFile, SourceId};

static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn signed_package_installs_runs_and_uninstalls_through_desktop_entrypoint() {
    let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") else {
        return;
    };
    let root = test_directory();
    fs::create_dir_all(&root).unwrap();
    let package_path = root.join("answer.sapp");
    let trusted_key = root.join("trusted-key.hex");
    let seed = [51_u8; 32];
    fs::write(&package_path, signed_answer(&seed)).unwrap();
    fs::write(
        &trusted_key,
        format!(
            "{}\n",
            hex(SigningKey::from_bytes(&seed).verifying_key().as_bytes())
        ),
    )
    .unwrap();
    let store = root.join("store");

    let (status, output, error) = invoke(vec![
        "host".into(),
        "install".into(),
        package_path.as_os_str().into(),
        "--store".into(),
        store.as_os_str().into(),
        "--trusted-key".into(),
        trusted_key.as_os_str().into(),
    ]);
    assert_eq!(status, EXIT_SUCCESS, "{error}");
    let installed: InstalledRevision = serde_json::from_str(output.trim()).unwrap();

    let (status, output, error) = invoke(vec![
        "host".into(),
        "open".into(),
        package_path.as_os_str().into(),
        "--store".into(),
        store.as_os_str().into(),
        "--trusted-key".into(),
        trusted_key.as_os_str().into(),
        "--runtime".into(),
        runtime,
    ]);
    assert_eq!(status, EXIT_SUCCESS, "{error}");
    assert_eq!(output.trim(), "42");

    let (status, output, error) = invoke(vec![
        "host".into(),
        "uninstall".into(),
        installed.app_identity.into(),
        "--store".into(),
        store.as_os_str().into(),
    ]);
    assert_eq!(status, EXIT_SUCCESS, "{error}");
    assert_eq!(output.trim(), "removed=true");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn association_plan_is_per_user_and_passes_one_quoted_path() {
    let executable = std::env::current_exe().unwrap();
    let plan = windows_association_plan(&executable).unwrap();
    assert_eq!(plan.len(), 5);
    assert!(
        plan.iter()
            .all(|operation| operation.key.starts_with("HKCU\\"))
    );
    let open = plan
        .iter()
        .find(|operation| operation.key.ends_with("shell\\open\\command"))
        .unwrap();
    assert!(open.value.ends_with(" open \"%1\""));
    assert!(!open.value.contains("cmd.exe"));
}

#[test]
fn typed_ui_preview_validates_without_starting_a_window() {
    let root = test_directory();
    fs::create_dir_all(&root).unwrap();
    let model = root.join("model.json");
    fs::write(
        &model,
        r#"{"schema":"sico.ui.model.v0","root":{"id":"root","kind":"window","text":null,"accessibility_label":null,"children":[{"id":"message","kind":"text","text":"Hello from Sico","accessibility_label":null,"children":[]}]}}"#,
    )
    .unwrap();
    let (status, output, error) = invoke(vec![
        "host".into(),
        "ui-preview".into(),
        model.as_os_str().into(),
        "--validate-only".into(),
    ]);
    assert_eq!(status, EXIT_SUCCESS, "{error}");
    assert_eq!(output.trim(), "validated-ui-nodes=2");

    fs::write(&model, b"{}\n").unwrap();
    let (status, _, _) = invoke(vec![
        "host".into(),
        "ui-preview".into(),
        model.as_os_str().into(),
        "--validate-only".into(),
    ]);
    assert_eq!(status, EXIT_ERROR);
    fs::remove_dir_all(root).unwrap();
}

fn signed_answer(seed: &[u8; 32]) -> Vec<u8> {
    let source = SourceFile::from_text(
        SourceId::new(1),
        "answer.sico",
        "function main() returns Int:\n  return 40 + 2\nend function\n",
    )
    .unwrap();
    let component = compile_component(&lower_core(&source).unwrap()).unwrap();
    let unsigned = build_unsigned(BuildInput {
        app_id: "dev.sico.desktop-answer".to_owned(),
        app_version: "1.0.0".to_owned(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .unwrap();
    sign_development(&unsigned, seed).unwrap()
}

fn invoke(args: Vec<OsString>) -> (i32, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run(args, &mut stdout, &mut stderr);
    (
        status,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").unwrap();
        output
    })
}

fn test_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-desktop-host-test-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
