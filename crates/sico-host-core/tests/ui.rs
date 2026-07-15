use std::path::Path;

use sico_host_core::{UiError, UiEvent, UiEventKind, UiModel, UiNode, UiNodeKind};

#[test]
fn typed_tree_escapes_text_and_freezes_accessibility_order() {
    let model = UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: node(
            "window",
            UiNodeKind::Window,
            None,
            None,
            vec![
                node(
                    "title",
                    UiNodeKind::Text,
                    Some("<script>x</script>"),
                    None,
                    vec![],
                ),
                node(
                    "submit",
                    UiNodeKind::Button,
                    Some("Go"),
                    Some("Submit form"),
                    vec![],
                ),
                node("name", UiNodeKind::Input, None, Some("Name"), vec![]),
            ],
        ),
    };
    let plan = model.validate().unwrap();
    assert_eq!(
        plan.nodes[1].escaped_text.as_deref(),
        Some("&lt;script&gt;x&lt;/script&gt;")
    );
    assert_eq!(plan.accessibility_order, ["submit", "name"]);
}

#[test]
fn duplicate_deep_control_and_missing_accessibility_inputs_are_refused() {
    let duplicate = UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: node(
            "root",
            UiNodeKind::Window,
            None,
            None,
            vec![
                node("same", UiNodeKind::Text, None, None, vec![]),
                node("same", UiNodeKind::Status, None, None, vec![]),
            ],
        ),
    };
    assert_eq!(duplicate.validate().unwrap_err(), UiError::DuplicateId);
    let missing = UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: node(
            "root",
            UiNodeKind::Window,
            None,
            None,
            vec![node("button", UiNodeKind::Button, Some("go"), None, vec![])],
        ),
    };
    assert_eq!(
        missing.validate().unwrap_err(),
        UiError::MissingAccessibilityLabel
    );
    let control = UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: node(
            "root",
            UiNodeKind::Window,
            Some("bad\u{0007}"),
            None,
            vec![],
        ),
    };
    assert_eq!(control.validate().unwrap_err(), UiError::InvalidText);
    let mut deep = node("leaf", UiNodeKind::Column, None, None, vec![]);
    for index in 0..32 {
        deep = node(
            &format!("depth-{index}"),
            UiNodeKind::Column,
            None,
            None,
            vec![deep],
        );
    }
    deep.kind = UiNodeKind::Window;
    assert_eq!(
        UiModel {
            schema: "sico.ui.model.v0".to_owned(),
            root: deep,
        }
        .validate()
        .unwrap_err(),
        UiError::DepthLimit
    );
}

#[test]
fn event_kind_payload_rate_and_queue_are_bounded() {
    let plan = basic_model().validate().unwrap();
    let mut gate = plan.event_gate();
    assert_eq!(
        gate.enqueue(
            0,
            UiEvent {
                node_id: "input".to_owned(),
                kind: UiEventKind::Click,
                value: None,
            }
        )
        .unwrap_err(),
        UiError::EventMismatch
    );
    for _ in 0..120 {
        gate.enqueue(0, click()).unwrap();
    }
    assert_eq!(gate.enqueue(0, click()).unwrap_err(), UiError::EventRate);
    for _ in 0..120 {
        gate.enqueue(1_000, click()).unwrap();
    }
    for _ in 0..16 {
        gate.enqueue(2_000, click()).unwrap();
    }
    assert_eq!(gate.len(), 256);
    assert_eq!(
        gate.enqueue(2_000, click()).unwrap_err(),
        UiError::EventQueue
    );
}

#[test]
fn ui_wit_package_parses() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("wit");
    let mut resolve = wit_parser::Resolve::default();
    let (package, _sources) = resolve.push_dir(path).unwrap();
    assert!(!resolve.packages[package].worlds.is_empty());
}

fn basic_model() -> UiModel {
    UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: node(
            "root",
            UiNodeKind::Window,
            None,
            None,
            vec![
                node("button", UiNodeKind::Button, Some("Go"), Some("Go"), vec![]),
                node("input", UiNodeKind::Input, None, Some("Input"), vec![]),
            ],
        ),
    }
}

fn click() -> UiEvent {
    UiEvent {
        node_id: "button".to_owned(),
        kind: UiEventKind::Click,
        value: None,
    }
}

fn node(
    id: &str,
    kind: UiNodeKind,
    text: Option<&str>,
    accessibility_label: Option<&str>,
    children: Vec<UiNode>,
) -> UiNode {
    UiNode {
        id: id.to_owned(),
        kind,
        text: text.map(str::to_owned),
        accessibility_label: accessibility_label.map(str::to_owned),
        children,
    }
}
