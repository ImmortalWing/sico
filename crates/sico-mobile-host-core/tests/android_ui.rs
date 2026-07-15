use sico_host_core::{MAX_UI_EVENTS_PER_SECOND, UiError, UiEventKind, UiModel, UiNode, UiNodeKind};
use sico_mobile_host_core::{AndroidUiSession, AndroidWidgetKind, MAX_BRIDGE_BYTES};

#[test]
fn maps_validated_nodes_to_native_widgets_without_html_interpretation() {
    let plan = model().validate().unwrap();
    let session = AndroidUiSession::from_shared(&plan);
    let widgets = &session.render_plan().widgets;
    assert_eq!(widgets[0].kind, AndroidWidgetKind::VerticalLayout);
    assert_eq!(widgets[1].kind, AndroidWidgetKind::TextView);
    assert_eq!(widgets[1].text.as_deref(), Some("<native & text>"));
    assert_eq!(widgets[2].kind, AndroidWidgetKind::Button);
    assert_eq!(widgets[3].kind, AndroidWidgetKind::EditText);
    assert!(
        widgets
            .iter()
            .all(|widget| !format!("{:?}", widget.kind).contains("WebView"))
    );
}

#[test]
fn talkback_order_and_focusability_follow_the_shared_plan() {
    let plan = model().validate().unwrap();
    let session = AndroidUiSession::from_shared(&plan);
    assert_eq!(session.render_plan().accessibility_order, ["go", "name"]);
    assert!(!session.render_plan().widgets[1].focusable);
    assert!(session.render_plan().widgets[2].focusable);
    assert_eq!(session.render_plan().widgets[3].max_input_bytes, Some(4096));
}

#[test]
fn native_touch_and_ime_events_use_the_shared_event_gate() {
    let plan = model().validate().unwrap();
    let mut session = AndroidUiSession::from_shared(&plan);
    session.click(1, "go".to_owned()).unwrap();
    session
        .input(2, "name".to_owned(), "张三".to_owned())
        .unwrap();
    assert_eq!(session.pop().unwrap().kind, UiEventKind::Click);
    assert_eq!(session.pop().unwrap().value.as_deref(), Some("张三"));
    assert_eq!(
        session.input(3, "name".to_owned(), "x".repeat(4097)),
        Err(UiError::EventMismatch)
    );
}

#[test]
fn mismatched_unknown_and_rate_excess_events_fail_closed() {
    let plan = model().validate().unwrap();
    let mut session = AndroidUiSession::from_shared(&plan);
    assert_eq!(
        session.click(0, "name".to_owned()),
        Err(UiError::EventMismatch)
    );
    assert_eq!(
        session.click(0, "missing".to_owned()),
        Err(UiError::UnknownEventNode)
    );
    for time in 0..MAX_UI_EVENTS_PER_SECOND {
        session.click(time as u64, "go".to_owned()).unwrap();
    }
    assert_eq!(
        session.click(MAX_UI_EVENTS_PER_SECOND as u64, "go".to_owned()),
        Err(UiError::EventRate)
    );
    assert_eq!(MAX_BRIDGE_BYTES, 64 * 1024);
}

fn model() -> UiModel {
    UiModel {
        schema: "sico.ui.model.v0".to_owned(),
        root: node(
            "root",
            UiNodeKind::Window,
            None,
            None,
            vec![
                node(
                    "text",
                    UiNodeKind::Text,
                    Some("<native & text>"),
                    None,
                    vec![],
                ),
                node("go", UiNodeKind::Button, Some("Go"), Some("Run"), vec![]),
                node("name", UiNodeKind::Input, None, Some("Name"), vec![]),
            ],
        ),
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
