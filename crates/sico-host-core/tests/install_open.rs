use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_host_core::{HostError, HostStore, InstallOptions};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sign_development};
use wasm_encoder::Component;

static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn signed_install_and_reverified_open_close_over_identity() {
    let root = test_directory();
    let store = HostStore::open(&root).unwrap();
    let (package, key) = package("dev.sico.host-test", "1.0.0", 3);
    let trusted = BTreeSet::from([key]);
    let installed = store
        .install_bytes(&package, &trusted, InstallOptions::default())
        .unwrap();
    assert_eq!(installed.app_id, "dev.sico.host-test");
    let opened = store
        .open_installed(
            &installed.app_identity,
            &installed.revision_digest,
            &trusted,
            &BTreeSet::new(),
        )
        .unwrap();
    assert_eq!(opened.metadata, installed);
    assert!(opened.package.granted_capabilities.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn signer_identity_tamper_and_downgrade_are_refused() {
    let root = test_directory();
    let store = HostStore::open(&root).unwrap();
    let (version_two, key_a) = package("dev.sico.host-test", "2.0.0", 4);
    let trusted_a = BTreeSet::from([key_a]);
    let installed = store
        .install_bytes(&version_two, &trusted_a, InstallOptions::default())
        .unwrap();
    let (other_signer, key_b) = package("dev.sico.host-test", "2.0.0", 5);
    let second = store
        .install_bytes(
            &other_signer,
            &BTreeSet::from([key_b]),
            InstallOptions::default(),
        )
        .unwrap();
    assert_ne!(installed.app_identity, second.app_identity);

    let (version_one, _) = package("dev.sico.host-test", "1.0.0", 4);
    assert!(matches!(
        store.install_bytes(&version_one, &trusted_a, InstallOptions::default()),
        Err(HostError::Downgrade { .. })
    ));

    fs::write(
        store
            .root()
            .join("apps")
            .join(&installed.app_identity)
            .join("revisions")
            .join(&installed.revision_digest)
            .join("app.sapp"),
        b"tampered",
    )
    .unwrap();
    assert!(matches!(
        store.open_installed(
            &installed.app_identity,
            &installed.revision_digest,
            &trusted_a,
            &BTreeSet::new()
        ),
        Err(HostError::DigestMismatch)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn untrusted_unsigned_and_non_sapp_paths_are_refused() {
    let root = test_directory();
    let store = HostStore::open(&root).unwrap();
    let unsigned = build_unsigned(input("dev.sico.host-test", "1.0.0")).unwrap();
    assert!(matches!(
        store.install_bytes(&unsigned, &BTreeSet::new(), InstallOptions::default()),
        Err(HostError::Package(_))
    ));
    let path = root.join("app.bin");
    fs::write(&path, unsigned).unwrap();
    assert!(matches!(
        store.install_path(&path, &BTreeSet::new(), InstallOptions::default()),
        Err(HostError::InvalidPackagePath)
    ));
    fs::remove_dir_all(root).unwrap();
}

fn package(app_id: &str, version: &str, seed: u8) -> (Vec<u8>, [u8; 32]) {
    let unsigned = build_unsigned(input(app_id, version)).unwrap();
    let seed = [seed; 32];
    let key = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    (sign_development(&unsigned, &seed).unwrap(), key)
}

fn input(app_id: &str, version: &str) -> BuildInput {
    BuildInput {
        app_id: app_id.to_owned(),
        app_version: version.to_owned(),
        component: Component::new().finish(),
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    }
}

fn test_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-host-core-test-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
