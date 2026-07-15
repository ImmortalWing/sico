use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_mobile_host_core::{
    BRIDGE_SCHEMA, BridgeOperation, BridgeRequest, BridgeResponse, MobileHostCore, ResponseStatus,
};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sign_development};
use wasm_encoder::Component;

static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn probe_is_bounded_typed_and_shared_core_owned() {
    let (root, core, _) = core();
    let response = dispatch(&core, &request("probe", BridgeOperation::Probe), None);
    assert_eq!(response.status, ResponseStatus::Ok);
    assert_eq!(response.data["trust_owner"], "sico-host-core");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn signed_bytes_install_open_and_uninstall_through_bridge() {
    let (root, core, seed) = core();
    let package = package(&seed);
    let installed = dispatch(
        &core,
        &request("install", BridgeOperation::Install),
        Some(&package),
    );
    assert_eq!(installed.status, ResponseStatus::Ok);
    let app_identity = installed.data["app_identity"].as_str().unwrap().to_owned();
    let revision_digest = installed.data["revision_digest"]
        .as_str()
        .unwrap()
        .to_owned();
    let opened = dispatch(
        &core,
        &request(
            "open",
            BridgeOperation::Open {
                app_identity: app_identity.clone(),
                revision_digest,
                host_grants: BTreeSet::new(),
            },
        ),
        None,
    );
    assert_eq!(opened.status, ResponseStatus::Ok);
    let removed = dispatch(
        &core,
        &request("remove", BridgeOperation::Uninstall { app_identity }),
        None,
    );
    assert_eq!(removed.data["removed"], true);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_unknown_oversized_and_missing_package_fail_closed() {
    let (root, core, _) = core();
    let invalid: BridgeResponse = serde_json::from_slice(&core.dispatch(b"{}", None)).unwrap();
    assert_eq!(invalid.code, "INVALID_REQUEST");
    let oversized = core.dispatch(&vec![b'x'; 65_537], None);
    let oversized: BridgeResponse = serde_json::from_slice(&oversized).unwrap();
    assert_eq!(oversized.code, "REQUEST_TOO_LARGE");
    let missing = dispatch(&core, &request("install", BridgeOperation::Install), None);
    assert_eq!(missing.code, "PACKAGE_REQUIRED");
    fs::remove_dir_all(root).unwrap();
}

fn core() -> (PathBuf, MobileHostCore, [u8; 32]) {
    let root = std::env::temp_dir().join(format!(
        "sico-mobile-core-test-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let seed = [71_u8; 32];
    let key = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let core = MobileHostCore::open(&root, BTreeSet::from([key])).unwrap();
    (root, core, seed)
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

fn package(seed: &[u8; 32]) -> Vec<u8> {
    let unsigned = build_unsigned(BuildInput {
        app_id: "dev.sico.mobile-bridge".to_owned(),
        app_version: "1.0.0".to_owned(),
        component: Component::new().finish(),
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .unwrap();
    sign_development(&unsigned, seed).unwrap()
}
