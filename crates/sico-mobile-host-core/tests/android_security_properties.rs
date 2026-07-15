use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use sico_host_core::{UiError, UiModel, UiNode, UiNodeKind};
use sico_mobile_host_core::{
    AndroidIntent, AndroidIntentAction, AndroidLifecycle, AndroidUiSession, BRIDGE_SCHEMA,
    BridgeOperation, BridgeRequest, BridgeResponse, LifecycleDescriptor, MobileHostCore,
    ResponseStatus, validate_intent,
};

const INPUTS_PER_PROPERTY: usize = 2_048;
static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn bridge_intent_lifecycle_and_ui_properties_cover_8192_inputs() {
    let root = test_directory();
    let core = MobileHostCore::open(&root, BTreeSet::new()).unwrap();

    for index in 0..INPUTS_PER_PROPERTY {
        let request = BridgeRequest {
            schema: format!("{BRIDGE_SCHEMA}.{index}"),
            request_id: format!("property-{index}"),
            operation: BridgeOperation::Probe,
        };
        let response: BridgeResponse =
            serde_json::from_slice(&core.dispatch(&serde_json::to_vec(&request).unwrap(), None))
                .unwrap();
        assert_eq!(response.status, ResponseStatus::Error);
        assert_eq!(response.code, "INVALID_REQUEST");
    }

    for index in 0..INPUTS_PER_PROPERTY {
        let intent = AndroidIntent {
            action: AndroidIntentAction::View,
            uri: Some(format!("file:///untrusted/{index}.sapp")),
            mime_type: Some("application/vnd.sico.sapp".to_owned()),
            display_name: Some(format!("property-{index}.sapp")),
            clip_items: 0,
        };
        assert!(validate_intent(&intent).is_err());
    }

    for index in 0..INPUTS_PER_PROPERTY {
        let descriptor = LifecycleDescriptor {
            app_identity: format!("{index:063x}g"),
            revision_digest: format!("{index:064x}"),
        };
        assert!(AndroidLifecycle::create(descriptor).is_err());
    }

    let plan = ui_model().validate().unwrap();
    for index in 0..INPUTS_PER_PROPERTY {
        let mut session = AndroidUiSession::from_shared(&plan);
        let value = format!("{index:04x}{}", "x".repeat(4_093));
        assert_eq!(
            session.input(index as u64, "input".to_owned(), value),
            Err(UiError::EventMismatch)
        );
    }

    fs::remove_dir_all(root).unwrap();
}

fn ui_model() -> UiModel {
    UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: UiNode {
            id: "root".to_owned(),
            kind: UiNodeKind::Window,
            text: None,
            accessibility_label: None,
            children: vec![UiNode {
                id: "input".to_owned(),
                kind: UiNodeKind::Input,
                text: None,
                accessibility_label: Some("Property input".to_owned()),
                children: Vec::new(),
            }],
        },
    }
}

fn test_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-android-security-properties-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
