use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

pub const MAX_UI_NODES: usize = 1_024;
pub const MAX_UI_DEPTH: usize = 32;
pub const MAX_TOTAL_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_TEXT_BYTES: usize = 4 * 1024;
pub const MAX_QUEUED_UI_EVENTS: usize = 256;
pub const MAX_UI_EVENTS_PER_SECOND: usize = 120;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiNodeKind {
    Window,
    Column,
    Row,
    Text,
    Button,
    Input,
    Status,
}

impl UiNodeKind {
    const fn is_container(self) -> bool {
        matches!(self, Self::Window | Self::Column | Self::Row)
    }

    const fn is_interactive(self) -> bool {
        matches!(self, Self::Button | Self::Input)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiNode {
    pub id: String,
    pub kind: UiNodeKind,
    pub text: Option<String>,
    pub accessibility_label: Option<String>,
    pub children: Vec<UiNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiModel {
    pub schema: String,
    pub root: UiNode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderNode {
    pub id: String,
    pub kind: UiNodeKind,
    pub escaped_text: Option<String>,
    pub accessibility_label: Option<String>,
    pub depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderPlan {
    pub nodes: Vec<RenderNode>,
    pub accessibility_order: Vec<String>,
    interactive: BTreeMap<String, UiNodeKind>,
}

impl RenderPlan {
    /// Creates an empty bounded event gate for this validated render plan.
    #[must_use]
    pub fn event_gate(&self) -> EventGate {
        EventGate {
            interactive: self.interactive.clone(),
            queue: VecDeque::new(),
            accepted_times: VecDeque::new(),
            last_time_ms: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiEventKind {
    Click,
    Input,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiEvent {
    pub node_id: String,
    pub kind: UiEventKind,
    pub value: Option<String>,
}

pub struct EventGate {
    interactive: BTreeMap<String, UiNodeKind>,
    queue: VecDeque<UiEvent>,
    accepted_times: VecDeque<u64>,
    last_time_ms: Option<u64>,
}

impl EventGate {
    /// Validates and queues one deterministic-timestamp UI event.
    ///
    /// # Errors
    ///
    /// Rejects unknown/mismatched nodes, invalid payloads, time reversal,
    /// queue overflow and more than 120 accepted events in a rolling second.
    pub fn enqueue(&mut self, now_ms: u64, event: UiEvent) -> Result<(), UiError> {
        if self.last_time_ms.is_some_and(|last| now_ms < last) {
            return Err(UiError::TimeReversal);
        }
        self.last_time_ms = Some(now_ms);
        while self
            .accepted_times
            .front()
            .is_some_and(|accepted| now_ms.saturating_sub(*accepted) >= 1_000)
        {
            self.accepted_times.pop_front();
        }
        if self.accepted_times.len() >= MAX_UI_EVENTS_PER_SECOND {
            return Err(UiError::EventRate);
        }
        if self.queue.len() >= MAX_QUEUED_UI_EVENTS {
            return Err(UiError::EventQueue);
        }
        let kind = self
            .interactive
            .get(&event.node_id)
            .ok_or(UiError::UnknownEventNode)?;
        match (kind, event.kind, event.value.as_deref()) {
            (UiNodeKind::Button, UiEventKind::Click, None) => {}
            (UiNodeKind::Input, UiEventKind::Input, Some(value))
                if valid_text(value) && value.len() <= MAX_TEXT_BYTES => {}
            _ => return Err(UiError::EventMismatch),
        }
        self.accepted_times.push_back(now_ms);
        self.queue.push_back(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<UiEvent> {
        self.queue.pop_front()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiError {
    UnknownSchema,
    RootNotWindow,
    NestedWindow,
    NodeLimit,
    DepthLimit,
    InvalidId,
    DuplicateId,
    TextLimit,
    InvalidText,
    InvalidChildren,
    MissingAccessibilityLabel,
    UnknownEventNode,
    EventMismatch,
    EventQueue,
    EventRate,
    TimeReversal,
}

impl std::fmt::Display for UiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "UI contract rejected input: {self:?}")
    }
}

impl std::error::Error for UiError {}

impl UiModel {
    /// Validates the complete model before returning a renderer-neutral plan.
    ///
    /// # Errors
    ///
    /// Rejects unknown schema/nodes, invalid IDs/text/tree shape, duplicate
    /// IDs, missing accessibility labels and every fixed resource ceiling.
    pub fn validate(&self) -> Result<RenderPlan, UiError> {
        if self.schema != "sico.ui.model.v0" {
            return Err(UiError::UnknownSchema);
        }
        if self.root.kind != UiNodeKind::Window {
            return Err(UiError::RootNotWindow);
        }
        let mut state = ValidationState::default();
        validate_node(&self.root, 1, true, &mut state)?;
        Ok(RenderPlan {
            nodes: state.nodes,
            accessibility_order: state.accessibility_order,
            interactive: state.interactive,
        })
    }
}

#[derive(Default)]
struct ValidationState {
    ids: BTreeSet<String>,
    nodes: Vec<RenderNode>,
    accessibility_order: Vec<String>,
    interactive: BTreeMap<String, UiNodeKind>,
    total_text: usize,
}

fn validate_node(
    node: &UiNode,
    depth: usize,
    root: bool,
    state: &mut ValidationState,
) -> Result<(), UiError> {
    if state.nodes.len() >= MAX_UI_NODES {
        return Err(UiError::NodeLimit);
    }
    if depth > MAX_UI_DEPTH {
        return Err(UiError::DepthLimit);
    }
    if !valid_id(&node.id) {
        return Err(UiError::InvalidId);
    }
    if !state.ids.insert(node.id.clone()) {
        return Err(UiError::DuplicateId);
    }
    if !root && node.kind == UiNodeKind::Window {
        return Err(UiError::NestedWindow);
    }
    if !node.kind.is_container() && !node.children.is_empty() {
        return Err(UiError::InvalidChildren);
    }
    let text = validate_optional_text(node.text.as_deref(), state)?;
    let label = validate_optional_text(node.accessibility_label.as_deref(), state)?;
    if node.kind.is_interactive() && label.is_none() {
        return Err(UiError::MissingAccessibilityLabel);
    }
    if node.kind.is_interactive() {
        state.accessibility_order.push(node.id.clone());
        state.interactive.insert(node.id.clone(), node.kind);
    }
    state.nodes.push(RenderNode {
        id: node.id.clone(),
        kind: node.kind,
        escaped_text: text.map(escape_text),
        accessibility_label: label.map(str::to_owned),
        depth,
    });
    for child in &node.children {
        validate_node(child, depth + 1, false, state)?;
    }
    Ok(())
}

fn validate_optional_text<'a>(
    text: Option<&'a str>,
    state: &mut ValidationState,
) -> Result<Option<&'a str>, UiError> {
    let Some(text) = text else {
        return Ok(None);
    };
    if text.len() > MAX_TEXT_BYTES {
        return Err(UiError::TextLimit);
    }
    if !valid_text(text) {
        return Err(UiError::InvalidText);
    }
    state.total_text = state
        .total_text
        .checked_add(text.len())
        .ok_or(UiError::TextLimit)?;
    if state.total_text > MAX_TOTAL_TEXT_BYTES {
        return Err(UiError::TextLimit);
    }
    Ok(Some(text))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_text(text: &str) -> bool {
    text.chars()
        .all(|character| !character.is_control() || matches!(character, '\n' | '\t'))
}

fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}
