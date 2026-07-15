use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_host_core::{
    HostStore, InstallOptions, PermissionChoice, PermissionError, PermissionOutcome,
    PermissionSession, PermissionStore,
};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sign_development};
use wasm_encoder::{
    Component, ComponentExportKind, ComponentExportSection, ComponentImportSection,
    ComponentTypeRef, ComponentValType, PrimitiveValType,
};

static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn persistent_permission_roundtrips_for_exact_identity_and_capabilities() {
    let fixture = fixture();
    let prompt = PermissionStore::prompt(&fixture.opened);
    assert_eq!(prompt.capabilities, ["clock.read"]);
    assert_eq!(
        prompt.signer_fingerprint,
        fixture.opened.metadata.trust_identity
    );
    assert!(matches!(
        fixture
            .permissions
            .resolve(&fixture.opened, &PermissionSession::default())
            .unwrap(),
        PermissionOutcome::PromptRequired(_)
    ));
    let decisions = BTreeMap::from([("clock.read".to_owned(), PermissionChoice::AllowPersistent)]);
    fixture
        .permissions
        .apply(
            &fixture.opened,
            &decisions,
            &mut PermissionSession::default(),
        )
        .unwrap();
    assert!(matches!(
        fixture.permissions.resolve(&fixture.opened, &PermissionSession::default()).unwrap(),
        PermissionOutcome::Granted(grants) if grants == BTreeSet::from(["clock.read".to_owned()])
    ));
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn allow_once_expires_and_deny_has_no_side_effect() {
    let fixture = fixture();
    let mut session = PermissionSession::default();
    let once = BTreeMap::from([("clock.read".to_owned(), PermissionChoice::AllowOnce)]);
    assert!(matches!(
        fixture
            .permissions
            .apply(&fixture.opened, &once, &mut session)
            .unwrap(),
        PermissionOutcome::Granted(_)
    ));
    assert!(matches!(
        fixture
            .permissions
            .resolve(&fixture.opened, &session)
            .unwrap(),
        PermissionOutcome::Granted(_)
    ));
    session.clear();
    assert!(matches!(
        fixture
            .permissions
            .resolve(&fixture.opened, &session)
            .unwrap(),
        PermissionOutcome::PromptRequired(_)
    ));
    let deny = BTreeMap::from([("clock.read".to_owned(), PermissionChoice::Deny)]);
    assert!(matches!(
        fixture.permissions.apply(&fixture.opened, &deny, &mut session).unwrap(),
        PermissionOutcome::Denied(denied) if denied == ["clock.read"]
    ));
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn corrupt_record_and_incomplete_decisions_fail_closed() {
    let fixture = fixture();
    assert!(matches!(
        fixture.permissions.apply(
            &fixture.opened,
            &BTreeMap::new(),
            &mut PermissionSession::default()
        ),
        Err(PermissionError::DecisionSetMismatch)
    ));
    let decisions = BTreeMap::from([("clock.read".to_owned(), PermissionChoice::AllowPersistent)]);
    fixture
        .permissions
        .apply(
            &fixture.opened,
            &decisions,
            &mut PermissionSession::default(),
        )
        .unwrap();
    let record = fixture
        .root
        .join("permissions")
        .join(&fixture.opened.metadata.app_identity)
        .join(format!(
            "{}.json",
            fixture.opened.metadata.capability_fingerprint
        ));
    fs::write(record, b"{\"schema\":\"unknown\"}").unwrap();
    assert!(
        fixture
            .permissions
            .resolve(&fixture.opened, &PermissionSession::default())
            .is_err()
    );
    fs::remove_dir_all(fixture.root).unwrap();
}

struct Fixture {
    root: PathBuf,
    permissions: PermissionStore,
    opened: sico_host_core::OpenedPackage,
}

fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "sico-permission-test-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let host = HostStore::open(&root).unwrap();
    let seed = [31_u8; 32];
    let key = *SigningKey::from_bytes(&seed).verifying_key().as_bytes();
    let unsigned = build_unsigned(BuildInput {
        app_id: "dev.sico.permission-test".to_owned(),
        app_version: "1.0.0".to_owned(),
        component: clock_component(),
        resources: Vec::new(),
        source_effects: vec!["clock.read".to_owned()],
        capabilities: vec!["clock.read".to_owned()],
        limits: RuntimeLimits::default(),
    })
    .unwrap();
    let package = sign_development(&unsigned, &seed).unwrap();
    let trusted = BTreeSet::from([key]);
    let installed = host
        .install_bytes(&package, &trusted, InstallOptions::default())
        .unwrap();
    let opened = host
        .open_installed(
            &installed.app_identity,
            &installed.revision_digest,
            &trusted,
            &BTreeSet::from(["clock.read".to_owned()]),
        )
        .unwrap();
    let permissions = PermissionStore::open(host.root()).unwrap();
    Fixture {
        root,
        permissions,
        opened,
    }
}

fn clock_component() -> Vec<u8> {
    let mut imports = ComponentImportSection::new();
    imports.import(
        "wasi:clocks/monotonic-clock@0.2.0",
        ComponentTypeRef::Value(ComponentValType::Primitive(PrimitiveValType::Bool)),
    );
    let mut component = Component::new();
    component.section(&imports);
    let mut exports = ComponentExportSection::new();
    exports.export("capability-probe", ComponentExportKind::Value, 0, None);
    component.section(&exports);
    component.finish()
}
