use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_codegen_wasm::compile_component;
use sico_host_core::{HostStore, InstallOptions, InstalledRevision};
use sico_ir::lower_core;
use sico_mobile_host_core::{
    BRIDGE_SCHEMA, BridgeOperation, BridgeRequest, BridgeResponse, MobileHostCore, ResponseStatus,
};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sha256_hex, sign_development};
use sico_runtime::{HostLimits, run_authorized_package};
use sico_source::{SourceFile, SourceId};

static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn exact_signed_package_has_desktop_mobile_identity_and_permission_parity() {
    let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") else {
        return;
    };
    let root = test_directory();
    let desktop_root = root.join("desktop");
    let mobile_root = root.join("mobile");
    let seed = [61_u8; 32];
    let key = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let trusted_keys = BTreeSet::from([key]);
    let package = signed_answer(&seed);
    let exact_digest = sha256_hex(&package);

    let desktop = HostStore::open(&desktop_root).unwrap();
    let desktop_installed = desktop
        .install_bytes(&package, &trusted_keys, InstallOptions::default())
        .unwrap();
    let desktop_opened = desktop
        .open_installed(
            &desktop_installed.app_identity,
            &desktop_installed.revision_digest,
            &trusted_keys,
            &BTreeSet::new(),
        )
        .unwrap();
    let output = run_authorized_package(
        OsStr::new(&runtime),
        &desktop_opened.package,
        None,
        &HostLimits::default(),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "42");

    let mobile = MobileHostCore::open(&mobile_root, trusted_keys).unwrap();
    let installed = dispatch(
        &mobile,
        &request("install", BridgeOperation::Install),
        Some(&package),
    );
    assert_eq!(installed.status, ResponseStatus::Ok);
    let mobile_installed: InstalledRevision = serde_json::from_value(installed.data).unwrap();
    assert_eq!(mobile_installed, desktop_installed);
    assert_eq!(mobile_installed.revision_digest, exact_digest);

    let opened = dispatch(
        &mobile,
        &request(
            "open",
            BridgeOperation::Open {
                app_identity: mobile_installed.app_identity.clone(),
                revision_digest: mobile_installed.revision_digest.clone(),
                host_grants: BTreeSet::new(),
            },
        ),
        None,
    );
    assert_eq!(opened.status, ResponseStatus::Ok);
    let mobile_opened: InstalledRevision = serde_json::from_value(opened.data).unwrap();
    assert_eq!(mobile_opened, desktop_opened.metadata);
    assert!(desktop_opened.package.granted_capabilities.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_bytes_and_identity_are_refused_on_the_mobile_boundary() {
    let root = test_directory();
    let seed = [62_u8; 32];
    let key = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let mobile = MobileHostCore::open(&root, BTreeSet::from([key])).unwrap();
    let mut package = signed_answer(&seed);
    package[0] ^= 1;
    let rejected = dispatch(
        &mobile,
        &request("install", BridgeOperation::Install),
        Some(&package),
    );
    assert_eq!(rejected.code, "INSTALL_REJECTED");
    let rejected = dispatch(
        &mobile,
        &request(
            "open",
            BridgeOperation::Open {
                app_identity: "a".repeat(64),
                revision_digest: "b".repeat(64),
                host_grants: BTreeSet::new(),
            },
        ),
        None,
    );
    assert_eq!(rejected.code, "OPEN_REJECTED");
    fs::remove_dir_all(root).unwrap();
}

fn signed_answer(seed: &[u8; 32]) -> Vec<u8> {
    let source = SourceFile::from_text(
        SourceId::new(1),
        "android-parity.sico",
        "function main() returns Int:\n  return 40 + 2\nend function\n",
    )
    .unwrap();
    let component = compile_component(&lower_core(&source).unwrap()).unwrap();
    let unsigned = build_unsigned(BuildInput {
        app_id: "dev.sico.android-parity".to_owned(),
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

fn request(request_id: &str, operation: BridgeOperation) -> BridgeRequest {
    BridgeRequest {
        schema: BRIDGE_SCHEMA.to_owned(),
        request_id: request_id.to_owned(),
        operation,
    }
}

fn dispatch(
    core: &MobileHostCore,
    request: &BridgeRequest,
    package: Option<&[u8]>,
) -> BridgeResponse {
    serde_json::from_slice(&core.dispatch(&serde_json::to_vec(&request).unwrap(), package)).unwrap()
}

fn test_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-desktop-android-parity-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
