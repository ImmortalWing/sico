use std::collections::{BTreeSet, HashSet};

use sico_host_core::{
    OpenRequest, UiError, UiModel, UiNode, UiNodeKind, app_identity_key, capability_fingerprint,
};

#[test]
fn app_and_signer_identity_is_deterministic_and_separated_for_4096_inputs() {
    let mut identities = HashSet::new();
    for index in 0..4_096 {
        let app_id = format!("dev.sico.property-{index}");
        let trust = format!("{index:064x}");
        let first = app_identity_key(&app_id, &trust);
        assert_eq!(first, app_identity_key(&app_id, &trust));
        assert!(identities.insert(first));
        assert_ne!(
            app_identity_key(&app_id, &trust),
            app_identity_key(&app_id, &format!("{:064x}", index + 1))
        );
    }
}

#[test]
fn capability_fingerprint_is_order_stable_and_separated_for_4096_inputs() {
    let mut fingerprints = HashSet::new();
    for index in 0..4_096 {
        let left = BTreeSet::from([
            "storage.read-write".to_owned(),
            format!("property.capability.{index}"),
        ]);
        let right: BTreeSet<_> = left.iter().rev().cloned().collect();
        let fingerprint = capability_fingerprint(&left);
        assert_eq!(fingerprint, capability_fingerprint(&right));
        assert!(fingerprints.insert(fingerprint));
    }
}

#[test]
fn hostile_ui_and_open_requests_fail_closed_for_2048_inputs_each() {
    for index in 0..2_048 {
        let model = UiModel {
            schema: "sico.ui.model.v0".to_owned(),
            root: UiNode {
                id: "root".to_owned(),
                kind: UiNodeKind::Window,
                text: Some(format!("payload-{index}\u{0007}")),
                accessibility_label: None,
                children: Vec::new(),
            },
        };
        assert_eq!(model.validate().unwrap_err(), UiError::InvalidText);
        assert!(OpenRequest::new(format!("payload-{index}.txt").into()).is_err());
    }
}
