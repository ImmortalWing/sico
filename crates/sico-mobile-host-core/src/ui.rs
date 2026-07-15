use sico_host_core::{EventGate, RenderPlan, UiError, UiEvent, UiEventKind, UiNodeKind};

pub const MAX_ANDROID_INPUT_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidWidgetKind {
    VerticalLayout,
    HorizontalLayout,
    TextView,
    Button,
    EditText,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndroidWidget {
    pub id: String,
    pub kind: AndroidWidgetKind,
    pub text: Option<String>,
    pub content_description: Option<String>,
    pub depth: usize,
    pub focusable: bool,
    pub max_input_bytes: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndroidRenderPlan {
    pub widgets: Vec<AndroidWidget>,
    pub accessibility_order: Vec<String>,
}

impl AndroidRenderPlan {
    #[must_use]
    pub fn from_shared(plan: &RenderPlan) -> Self {
        let widgets = plan
            .nodes
            .iter()
            .map(|node| AndroidWidget {
                id: node.id.clone(),
                kind: widget_kind(node.kind),
                text: node.raw_text.clone(),
                content_description: node.accessibility_label.clone(),
                depth: node.depth,
                focusable: matches!(node.kind, UiNodeKind::Button | UiNodeKind::Input),
                max_input_bytes: (node.kind == UiNodeKind::Input)
                    .then_some(MAX_ANDROID_INPUT_BYTES),
            })
            .collect();
        Self {
            widgets,
            accessibility_order: plan.accessibility_order.clone(),
        }
    }
}

pub struct AndroidUiSession {
    render_plan: AndroidRenderPlan,
    event_gate: EventGate,
}

impl AndroidUiSession {
    #[must_use]
    pub fn from_shared(plan: &RenderPlan) -> Self {
        Self {
            render_plan: AndroidRenderPlan::from_shared(plan),
            event_gate: plan.event_gate(),
        }
    }

    #[must_use]
    pub const fn render_plan(&self) -> &AndroidRenderPlan {
        &self.render_plan
    }

    /// Queues one native button activation through the shared event gate.
    ///
    /// # Errors
    ///
    /// Preserves the shared node, type, rate and queue refusal contract.
    pub fn click(&mut self, now_ms: u64, node_id: String) -> Result<(), UiError> {
        self.event_gate.enqueue(
            now_ms,
            UiEvent {
                node_id,
                kind: UiEventKind::Click,
                value: None,
            },
        )
    }

    /// Queues one committed IME value through the shared event gate.
    ///
    /// # Errors
    ///
    /// Rejects values over 4 KiB UTF-8 plus every shared event refusal.
    pub fn input(&mut self, now_ms: u64, node_id: String, value: String) -> Result<(), UiError> {
        if value.len() > MAX_ANDROID_INPUT_BYTES {
            return Err(UiError::EventMismatch);
        }
        self.event_gate.enqueue(
            now_ms,
            UiEvent {
                node_id,
                kind: UiEventKind::Input,
                value: Some(value),
            },
        )
    }

    pub fn pop(&mut self) -> Option<UiEvent> {
        self.event_gate.pop()
    }
}

const fn widget_kind(kind: UiNodeKind) -> AndroidWidgetKind {
    match kind {
        UiNodeKind::Window | UiNodeKind::Column => AndroidWidgetKind::VerticalLayout,
        UiNodeKind::Row => AndroidWidgetKind::HorizontalLayout,
        UiNodeKind::Text | UiNodeKind::Status => AndroidWidgetKind::TextView,
        UiNodeKind::Button => AndroidWidgetKind::Button,
        UiNodeKind::Input => AndroidWidgetKind::EditText,
    }
}
