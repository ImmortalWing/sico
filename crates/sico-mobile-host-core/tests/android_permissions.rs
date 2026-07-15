use std::collections::BTreeSet;

use sico_host_core::{app_identity_key, capability_fingerprint};
use sico_mobile_host_core::{
    AndroidPermissionError, AndroidScope, UriGrantSession, map_android_permissions,
};

#[test]
fn all_five_v0_capabilities_have_exact_minimal_mappings() {
    let capabilities = BTreeSet::from([
        "clock.read".to_owned(),
        "log.write".to_owned(),
        "network.connect".to_owned(),
        "random.read".to_owned(),
        "storage.read-write".to_owned(),
    ]);
    let plan = map_android_permissions(&capabilities).unwrap();
    assert_eq!(plan.capabilities, capabilities);
    assert_eq!(
        plan.manifest_permissions,
        BTreeSet::from(["android.permission.INTERNET".to_owned()])
    );
    assert!(plan.runtime_permissions.is_empty());
    assert_eq!(plan.scopes.len(), 5);
    assert!(plan.scopes.contains(&AndroidScope::AppPrivateStorage));
}

#[test]
fn unknown_capability_is_not_promoted_to_android_permission() {
    let error =
        map_android_permissions(&BTreeSet::from(["camera.capture".to_owned()])).unwrap_err();
    assert_eq!(
        error,
        AndroidPermissionError::UnknownCapability("camera.capture".to_owned())
    );
}

#[test]
fn uri_grants_are_ephemeral_host_ingress_not_guest_capabilities() {
    let mut grants = UriGrantSession::default();
    grants.grant_read("content://provider/app.sapp").unwrap();
    assert!(grants.can_read("content://provider/app.sapp"));
    assert!(
        map_android_permissions(&BTreeSet::new())
            .unwrap()
            .capabilities
            .is_empty()
    );
    grants.clear();
    assert!(!grants.can_read("content://provider/app.sapp"));
    assert!(grants.grant_read("file:///sdcard/app.sapp").is_err());
}

#[test]
fn signer_and_capability_drift_change_authority_keys() {
    let app = "dev.sico.android-permission";
    assert_ne!(
        app_identity_key(app, "signer-a"),
        app_identity_key(app, "signer-b")
    );
    assert_ne!(
        capability_fingerprint(&BTreeSet::from(["clock.read".to_owned()])),
        capability_fingerprint(&BTreeSet::from([
            "clock.read".to_owned(),
            "network.connect".to_owned()
        ]))
    );
}
