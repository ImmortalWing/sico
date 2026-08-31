//! Versioned, bounded, data-only observability contracts.

#![forbid(unsafe_code)]

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, VecDeque},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_encoder::{Component, CustomSection};

pub const DEBUG_IDENTITY_SCHEMA: &str = "sico.debug.identity.v0";
pub const DEBUG_MAP_SCHEMA: &str = "sico.debug-map.v0";
pub const DEBUG_LINK_SCHEMA: &str = "sico.debug-link.v0";
pub const DEBUG_LINK_SECTION: &str = "sico.debug-link.v0";
pub const MAX_DEBUG_MAP_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DOCUMENTS: usize = 256;
pub const MAX_FUNCTIONS: usize = 100_000;
pub const MAX_MAPPINGS: usize = 1_000_000;
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub const MAX_RUNTIME_FRAMES: usize = 256;
pub const MAX_MESSAGE_BYTES: usize = 65_536;
pub const MAX_CANCEL_REQUEST_BYTES: usize = 4_096;
pub const EXECUTION_EVENT_SCHEMA: &str = "sico.execution-event.v0";
pub const MAX_EXECUTION_EVENT_BYTES: usize = 1024 * 1024;
pub const MAX_OUTPUT_CHUNK_BYTES: usize = 64 * 1024;
pub const MAX_CAPTURED_CHANNEL_BYTES: usize = 1024 * 1024;
pub const MAX_EVENT_QUEUE_ITEMS: usize = 256;
pub const MAX_EVENT_QUEUE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugIdentity {
    pub schema: String,
    pub source: SourceIdentity,
    pub compiler: CompilerIdentity,
    pub language_semantics: String,
    pub ir_schema: String,
    pub component_code_sha256: String,
    pub component_sha256: String,
    pub debug_map_sha256: String,
    pub adapter_identities: Vec<String>,
    pub wit_identities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub document_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub display_uri: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerIdentity {
    pub package: String,
    pub version: String,
    pub executable_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugMap {
    pub schema: String,
    pub binding: DebugBinding,
    pub coordinate_system: String,
    pub documents: Vec<DebugDocument>,
    pub functions: Vec<DebugFunction>,
    pub mappings: Vec<DebugMapping>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugBinding {
    pub source_sha256: String,
    pub compiler_executable_sha256: String,
    pub component_code_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugDocument {
    pub id: String,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugFunction {
    pub id: String,
    pub core_module: String,
    pub component_function: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugMapping {
    pub core_module: String,
    pub component_function: u64,
    pub instruction_start: u64,
    pub instruction_end: u64,
    pub function_id: String,
    pub source: Option<DebugSpan>,
    pub call_site: Option<DebugSpan>,
    pub inline_parent: Option<String>,
    pub generated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugSpan {
    pub document_id: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFault {
    pub schema: String,
    pub run_id: String,
    pub generation_id: u64,
    pub class: String,
    pub code: String,
    pub key: String,
    pub message: String,
    pub provider_id: Option<String>,
    pub frames: Vec<RuntimeFrame>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFrame {
    pub function_id: String,
    pub source: Option<DebugSpan>,
    pub generated: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRequest {
    pub schema: String,
    pub run_id: String,
    pub generation_id: u64,
    pub cause: String,
}

/// A strict, binary-safe event record crossing the Runtime/client boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvent {
    pub schema: String,
    pub run_id: String,
    pub generation_id: u64,
    pub sequence: u64,
    pub kind: String,
    pub task_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub scope_id: Option<String>,
    pub cause: String,
    pub payload: ExecutionEventPayload,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEventPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dropped_events: Option<u64>,
}

/// Redaction is deliberately injected: no event producer may cross the runner
/// boundary without an explicit policy owning the transformation.
pub trait EventRedactor {
    fn redact(&self, bytes: &[u8]) -> Vec<u8>;
}

/// Bounded, ordered frames awaiting a client. Overflow discards ordinary
/// records but reserves space for a deterministic `dropped` marker and the
/// sole terminal record.
#[derive(Debug)]
pub struct EventQueue {
    frames: VecDeque<Vec<u8>>,
    bytes: usize,
    dropped: u64,
    terminal: bool,
    identity: Option<(String, u64)>,
    last_sequence: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineFrame<'a> {
    pub core_module: Option<&'a str>,
    pub component_function: u64,
    pub instruction_offset: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugLink {
    pub schema: String,
    pub source_sha256: String,
    pub compiler_executable_sha256: String,
    pub component_code_sha256: String,
    pub debug_map_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    Json(String),
    UnknownSchema(String),
    InvalidDigest(String),
    InvalidIdentity(String),
    InvalidCoordinateSystem(String),
    Limit(&'static str),
    NonCanonical(&'static str),
    Duplicate(&'static str),
    UnknownReference(String),
    InvalidRange(String),
    StaleBinding(&'static str),
    MissingDebugLink,
    DuplicateDebugLink,
    InvalidComponent,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContractError {}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Serializes a contract value with the repository's compact deterministic JSON form.
///
/// # Errors
///
/// Returns a JSON error when the value cannot be serialized.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, ContractError> {
    serde_json::to_vec(value).map_err(|error| ContractError::Json(error.to_string()))
}

/// Parses and strictly validates canonical debug-map bytes.
///
/// # Errors
///
/// Rejects invalid JSON, unknown fields/schema, non-canonical bytes, broken
/// references, ordering/range violations and hard-limit overflows.
pub fn parse_debug_map(bytes: &[u8]) -> Result<DebugMap, ContractError> {
    if bytes.len() > MAX_DEBUG_MAP_BYTES {
        return Err(ContractError::Limit("debug-map-bytes"));
    }
    let map: DebugMap =
        serde_json::from_slice(bytes).map_err(|error| ContractError::Json(error.to_string()))?;
    validate_debug_map(&map)?;
    if canonical_json(&map)? != bytes {
        return Err(ContractError::NonCanonical("debug-map-json"));
    }
    Ok(map)
}

/// Parses and strictly validates canonical debug-identity bytes.
///
/// # Errors
///
/// Rejects invalid JSON, unknown fields/schema, non-canonical bytes, malformed
/// identities/digests and hard-limit overflows.
pub fn parse_debug_identity(bytes: &[u8]) -> Result<DebugIdentity, ContractError> {
    let identity: DebugIdentity =
        serde_json::from_slice(bytes).map_err(|error| ContractError::Json(error.to_string()))?;
    validate_debug_identity(&identity)?;
    if canonical_json(&identity)? != bytes {
        return Err(ContractError::NonCanonical("debug-identity-json"));
    }
    Ok(identity)
}

/// Parses and strictly validates canonical Runtime fault bytes.
///
/// # Errors
///
/// Rejects invalid JSON, unknown fields/schema, non-canonical bytes and all
/// identity, class, range or hard-limit violations.
pub fn parse_runtime_fault(bytes: &[u8]) -> Result<RuntimeFault, ContractError> {
    let fault: RuntimeFault =
        serde_json::from_slice(bytes).map_err(|error| ContractError::Json(error.to_string()))?;
    validate_runtime_fault(&fault)?;
    if canonical_json(&fault)? != bytes {
        return Err(ContractError::NonCanonical("runtime-fault-json"));
    }
    Ok(fault)
}

/// Parses a bounded canonical persistent-client cancellation request.
///
/// # Errors
///
/// Rejects oversized/non-canonical JSON, unknown fields, invalid identities,
/// unsafe generation numbers and causes outside the client/debugger boundary.
pub fn parse_cancel_request(bytes: &[u8]) -> Result<CancelRequest, ContractError> {
    if bytes.len() > MAX_CANCEL_REQUEST_BYTES {
        return Err(ContractError::Limit("cancel-request-bytes"));
    }
    let request: CancelRequest =
        serde_json::from_slice(bytes).map_err(|error| ContractError::Json(error.to_string()))?;
    if request.schema != "sico.cancel-request.v0" {
        return Err(ContractError::UnknownSchema(request.schema));
    }
    validate_id(&request.run_id)?;
    if request.generation_id > MAX_SAFE_INTEGER {
        return Err(ContractError::Limit("generation-id"));
    }
    if !matches!(request.cause.as_str(), "client" | "debugger") {
        return Err(ContractError::InvalidIdentity("cancel-cause".into()));
    }
    if canonical_json(&request)? != bytes {
        return Err(ContractError::NonCanonical("cancel-request-json"));
    }
    Ok(request)
}

/// Parses one canonical execution-event frame before a client consumes it.
///
/// # Errors
///
/// Rejects oversized, non-canonical, unknown-field and invalid identity/event
/// records. This decoder is intentionally independent of any transport.
pub fn parse_execution_event(bytes: &[u8]) -> Result<ExecutionEvent, ContractError> {
    if bytes.len() > MAX_EXECUTION_EVENT_BYTES {
        return Err(ContractError::Limit("execution-event-bytes"));
    }
    let event: ExecutionEvent =
        serde_json::from_slice(bytes).map_err(|error| ContractError::Json(error.to_string()))?;
    validate_execution_event(&event)?;
    if canonical_json(&event)? != bytes {
        return Err(ContractError::NonCanonical("execution-event-json"));
    }
    Ok(event)
}

/// Validates every stable execution-event identity and payload bound.
///
/// # Errors
///
/// Rejects unknown kinds/causes, incompatible output payloads and unbounded
/// message or terminal metadata before the event becomes observable.
pub fn validate_execution_event(event: &ExecutionEvent) -> Result<(), ContractError> {
    if event.schema != EXECUTION_EVENT_SCHEMA {
        return Err(ContractError::UnknownSchema(event.schema.clone()));
    }
    validate_id(&event.run_id)?;
    if event.generation_id > MAX_SAFE_INTEGER || event.sequence > MAX_SAFE_INTEGER {
        return Err(ContractError::Limit("event-integer"));
    }
    if !matches!(
        event.kind.as_str(),
        "accepted"
            | "started"
            | "generation-published"
            | "stdout"
            | "stderr"
            | "breakpoint"
            | "stopped"
            | "continued"
            | "cancellation-requested"
            | "terminal"
            | "fault"
            | "truncated"
            | "dropped"
    ) {
        return Err(ContractError::InvalidIdentity("event-kind".into()));
    }
    for value in [&event.task_id, &event.parent_task_id, &event.scope_id]
        .into_iter()
        .flatten()
    {
        validate_id(value)?;
    }
    if !matches!(
        event.cause.as_str(),
        "launch"
            | "guest"
            | "host"
            | "timeout"
            | "signal"
            | "client"
            | "debugger"
            | "overflow"
            | "internal"
    ) {
        return Err(ContractError::InvalidIdentity("event-cause".into()));
    }
    let payload = &event.payload;
    if let Some(bytes) = &payload.bytes_base64 {
        let decoded = decode_base64(bytes)
            .ok_or_else(|| ContractError::InvalidIdentity("event-base64".into()))?;
        if decoded.len() > MAX_OUTPUT_CHUNK_BYTES {
            return Err(ContractError::Limit("event-output-chunk"));
        }
        if !matches!(event.kind.as_str(), "stdout" | "stderr") {
            return Err(ContractError::InvalidIdentity("event-bytes-kind".into()));
        }
    }
    if let Some(message) = &payload.message
        && message.len() > MAX_MESSAGE_BYTES
    {
        return Err(ContractError::Limit("event-message"));
    }
    if let Some(id) = &payload.breakpoint_id {
        validate_id(id)?;
    }
    if let Some(class) = &payload.terminal_class
        && class.len() > 256
    {
        return Err(ContractError::Limit("event-terminal-class"));
    }
    if payload
        .dropped_events
        .is_some_and(|value| value > MAX_SAFE_INTEGER)
    {
        return Err(ContractError::Limit("event-dropped"));
    }
    if canonical_json(event)?.len() > MAX_EXECUTION_EVENT_BYTES {
        return Err(ContractError::Limit("execution-event-bytes"));
    }
    Ok(())
}

impl EventQueue {
    #[must_use]
    pub fn new() -> Self {
        Self {
            frames: VecDeque::new(),
            bytes: 0,
            dropped: 0,
            terminal: false,
            identity: None,
            last_sequence: None,
        }
    }

    /// Applies redaction, chunks output and queues canonical frames. Output
    /// is binary-safe Base64; no UTF-8 conversion is performed.
    ///
    /// # Errors
    ///
    /// Rejects an invalid output kind, redacted chunk or sequence overflow.
    pub fn push_output<R: EventRedactor>(
        &mut self,
        mut event: ExecutionEvent,
        bytes: &[u8],
        redactor: &R,
    ) -> Result<u64, ContractError> {
        if !matches!(event.kind.as_str(), "stdout" | "stderr") {
            return Err(ContractError::InvalidIdentity("event-output-kind".into()));
        }
        let mut emitted = 0_u64;
        for chunk in redactor.redact(bytes).chunks(MAX_OUTPUT_CHUNK_BYTES) {
            event.payload = ExecutionEventPayload {
                bytes_base64: Some(encode_base64(chunk)),
                ..ExecutionEventPayload::default()
            };
            self.push(event.clone())?;
            emitted += 1;
            event.sequence = event
                .sequence
                .checked_add(1)
                .ok_or(ContractError::Limit("event-sequence"))?;
        }
        Ok(emitted)
    }

    /// Queues a validated record. A duplicate terminal is rejected.
    ///
    /// # Errors
    ///
    /// Rejects invalid identities, ordering, limits and duplicate terminals.
    #[allow(clippy::needless_pass_by_value)]
    pub fn push(&mut self, event: ExecutionEvent) -> Result<(), ContractError> {
        validate_execution_event(&event)?;
        if self.terminal {
            return Err(ContractError::InvalidIdentity(
                "event-after-terminal".into(),
            ));
        }
        if let Some((run_id, generation_id)) = &self.identity {
            if run_id != &event.run_id || *generation_id != event.generation_id {
                return Err(ContractError::InvalidIdentity(
                    "event-run-generation".into(),
                ));
            }
        } else {
            self.identity = Some((event.run_id.clone(), event.generation_id));
        }
        if self
            .last_sequence
            .is_some_and(|previous| event.sequence <= previous)
        {
            return Err(ContractError::NonCanonical("event-sequence"));
        }
        let terminal = event.kind == "terminal" || event.kind == "fault";
        let frame = canonical_json(&event)?;
        if terminal {
            self.reserve_terminal(&event, frame)?;
            self.terminal = true;
        } else if !self.fits(frame.len()) {
            self.dropped = self.dropped.saturating_add(1);
        } else {
            self.bytes += frame.len();
            self.frames.push_back(frame);
        }
        self.last_sequence = Some(event.sequence);
        Ok(())
    }

    #[must_use]
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        let frame = self.frames.pop_front()?;
        self.bytes -= frame.len();
        Some(frame)
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    #[must_use]
    pub const fn queued_bytes(&self) -> usize {
        self.bytes
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Vec<u8>> + '_ {
        std::iter::from_fn(|| self.pop())
    }

    fn fits(&self, length: usize) -> bool {
        self.frames.len() < MAX_EVENT_QUEUE_ITEMS
            && self.bytes.saturating_add(length) <= MAX_EVENT_QUEUE_BYTES
    }

    fn reserve_terminal(
        &mut self,
        event: &ExecutionEvent,
        frame: Vec<u8>,
    ) -> Result<(), ContractError> {
        let marker_for = |dropped| ExecutionEvent {
            schema: EXECUTION_EVENT_SCHEMA.into(),
            run_id: event.run_id.clone(),
            generation_id: event.generation_id,
            sequence: event.sequence.saturating_sub(1),
            kind: "dropped".into(),
            task_id: event.task_id.clone(),
            parent_task_id: event.parent_task_id.clone(),
            scope_id: event.scope_id.clone(),
            cause: "overflow".into(),
            payload: ExecutionEventPayload {
                dropped_events: Some(dropped),
                ..ExecutionEventPayload::default()
            },
        };
        let marker_present = self.dropped != 0 || !self.fits(frame.len());
        let marker_len = marker_present
            .then(|| canonical_json(&marker_for(self.dropped)).map(|value| value.len()))
            .transpose()?
            .unwrap_or(0);
        let needed = frame.len() + marker_len;
        let marker_items = usize::from(marker_present);
        while !self.frames.is_empty()
            && (self.frames.len() + 1 + marker_items > MAX_EVENT_QUEUE_ITEMS
                || self.bytes.saturating_add(needed) > MAX_EVENT_QUEUE_BYTES)
        {
            let removed = self.frames.pop_front().expect("checked nonempty");
            self.bytes -= removed.len();
            self.dropped = self.dropped.saturating_add(1);
        }
        if needed > MAX_EVENT_QUEUE_BYTES {
            return Err(ContractError::Limit("terminal-event-bytes"));
        }
        if self.dropped != 0 {
            let marker = canonical_json(&marker_for(self.dropped))?;
            self.bytes += marker.len();
            self.frames.push_back(marker);
        }
        self.bytes += frame.len();
        self.frames.push_back(frame);
        Ok(())
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates a decoded debug identity.
///
/// # Errors
///
/// Rejects invalid schema, identities, digests, ordering, URI policy or limits.
pub fn validate_debug_identity(identity: &DebugIdentity) -> Result<(), ContractError> {
    if identity.schema != DEBUG_IDENTITY_SCHEMA {
        return Err(ContractError::UnknownSchema(identity.schema.clone()));
    }
    validate_id(&identity.source.document_id)?;
    validate_digest(&identity.source.sha256)?;
    validate_id(&identity.compiler.package)?;
    validate_id(&identity.compiler.version)?;
    validate_digest(&identity.compiler.executable_sha256)?;
    validate_id(&identity.language_semantics)?;
    validate_id(&identity.ir_schema)?;
    validate_digest(&identity.component_code_sha256)?;
    validate_digest(&identity.component_sha256)?;
    validate_digest(&identity.debug_map_sha256)?;
    validate_sorted_ids(&identity.adapter_identities)?;
    validate_sorted_ids(&identity.wit_identities)?;
    if identity.source.byte_length > MAX_SAFE_INTEGER {
        return Err(ContractError::Limit("source-byte-length"));
    }
    if let Some(uri) = &identity.source.display_uri
        && (uri.len() > 65_536
            || (!uri.starts_with("workspace://") && !uri.starts_with("sico-source://")))
    {
        return Err(ContractError::InvalidIdentity("display-uri".into()));
    }
    Ok(())
}

/// Validates a decoded Runtime fault record.
///
/// # Errors
///
/// Rejects unknown classes, invalid identities/ranges, oversized text and
/// more than 256 frames.
pub fn validate_runtime_fault(fault: &RuntimeFault) -> Result<(), ContractError> {
    if fault.schema != "sico.runtime-fault.v0" {
        return Err(ContractError::UnknownSchema(fault.schema.clone()));
    }
    validate_id(&fault.run_id)?;
    validate_id(&fault.code)?;
    validate_id(&fault.key)?;
    if fault.generation_id > MAX_SAFE_INTEGER {
        return Err(ContractError::Limit("generation-id"));
    }
    if !matches!(
        fault.class.as_str(),
        "domain-error"
            | "cancelled"
            | "timeout"
            | "resource-limit.fuel"
            | "resource-limit.memory"
            | "resource-limit.other"
            | "trap"
            | "host-provider-failure"
            | "incompatible-artifact"
            | "launch-failure"
            | "external-termination"
            | "internal-invariant"
    ) {
        return Err(ContractError::InvalidIdentity("fault-class".into()));
    }
    if fault.message.len() > MAX_MESSAGE_BYTES {
        return Err(ContractError::Limit("fault-message"));
    }
    if let Some(provider) = &fault.provider_id {
        validate_id(provider)?;
    }
    if fault.frames.len() > MAX_RUNTIME_FRAMES {
        return Err(ContractError::Limit("runtime-frames"));
    }
    for frame in &fault.frames {
        validate_id(&frame.function_id)?;
        if let Some(span) = &frame.source {
            validate_id(&span.document_id)?;
            if span.start > span.end || span.end > MAX_SAFE_INTEGER {
                return Err(ContractError::InvalidRange("runtime-frame-source".into()));
            }
        }
        if let Some(reason) = &frame.unavailable_reason {
            validate_id(reason)?;
        }
        if frame.source.is_some() && frame.unavailable_reason.is_some() {
            return Err(ContractError::InvalidIdentity(
                "frame-source-and-unavailable".into(),
            ));
        }
    }
    Ok(())
}

/// Validates all decoded debug-map identities, bindings, ranges and limits.
///
/// # Errors
///
/// Rejects any contract, reference, ordering, overlap or hard-limit violation.
#[allow(clippy::too_many_lines)]
pub fn validate_debug_map(map: &DebugMap) -> Result<(), ContractError> {
    if map.schema != DEBUG_MAP_SCHEMA {
        return Err(ContractError::UnknownSchema(map.schema.clone()));
    }
    if map.coordinate_system != "utf8-byte-half-open" {
        return Err(ContractError::InvalidCoordinateSystem(
            map.coordinate_system.clone(),
        ));
    }
    for digest in [
        &map.binding.source_sha256,
        &map.binding.compiler_executable_sha256,
        &map.binding.component_code_sha256,
    ] {
        validate_digest(digest)?;
    }
    if map.documents.is_empty() || map.documents.len() > MAX_DOCUMENTS {
        return Err(ContractError::Limit("debug-documents"));
    }
    if map.functions.len() > MAX_FUNCTIONS {
        return Err(ContractError::Limit("debug-functions"));
    }
    if map.mappings.len() > MAX_MAPPINGS {
        return Err(ContractError::Limit("debug-mappings"));
    }
    let mut document_ids = BTreeSet::new();
    let mut previous_document = None;
    for document in &map.documents {
        validate_id(&document.id)?;
        validate_digest(&document.sha256)?;
        if document.byte_length > MAX_SAFE_INTEGER {
            return Err(ContractError::Limit("document-byte-length"));
        }
        if !document_ids.insert(document.id.as_str()) {
            return Err(ContractError::Duplicate("document-id"));
        }
        if previous_document.is_some_and(|previous| previous > document.id.as_str()) {
            return Err(ContractError::NonCanonical("documents"));
        }
        previous_document = Some(document.id.as_str());
    }
    let mut function_ids = BTreeMap::new();
    let mut previous_function = None;
    for function in &map.functions {
        validate_id(&function.id)?;
        validate_id(&function.core_module)?;
        if function.component_function > MAX_SAFE_INTEGER {
            return Err(ContractError::Limit("component-function"));
        }
        if function_ids
            .insert(
                function.id.as_str(),
                (function.core_module.as_str(), function.component_function),
            )
            .is_some()
        {
            return Err(ContractError::Duplicate("function-id"));
        }
        if previous_function.is_some_and(|previous| previous > function.id.as_str()) {
            return Err(ContractError::NonCanonical("functions"));
        }
        previous_function = Some(function.id.as_str());
    }
    let mut previous_key: Option<(&str, u64, u64, u64, &str)> = None;
    let mut previous_range: Option<(&str, u64, u64)> = None;
    for mapping in &map.mappings {
        validate_id(&mapping.core_module)?;
        validate_id(&mapping.function_id)?;
        let function_binding = function_ids
            .get(mapping.function_id.as_str())
            .ok_or_else(|| ContractError::UnknownReference(mapping.function_id.clone()))?;
        if *function_binding != (mapping.core_module.as_str(), mapping.component_function) {
            return Err(ContractError::InvalidIdentity(
                "mapping-function-binding".into(),
            ));
        }
        for value in [
            mapping.component_function,
            mapping.instruction_start,
            mapping.instruction_end,
        ] {
            if value > MAX_SAFE_INTEGER {
                return Err(ContractError::Limit("mapping-integer"));
            }
        }
        if mapping.instruction_start >= mapping.instruction_end {
            return Err(ContractError::InvalidRange("instruction".into()));
        }
        if let Some(span) = &mapping.source {
            validate_span(span, &map.documents)?;
        }
        if let Some(span) = &mapping.call_site {
            validate_span(span, &map.documents)?;
        }
        if let Some(parent) = &mapping.inline_parent
            && !function_ids.contains_key(parent.as_str())
        {
            return Err(ContractError::UnknownReference(parent.clone()));
        }
        let key = (
            mapping.core_module.as_str(),
            mapping.component_function,
            mapping.instruction_start,
            mapping.instruction_end,
            mapping.function_id.as_str(),
        );
        if previous_key.is_some_and(|previous| previous >= key) {
            return Err(if previous_key == Some(key) {
                ContractError::Duplicate("mapping")
            } else {
                ContractError::NonCanonical("mappings")
            });
        }
        if previous_range.is_some_and(|(module, function, end)| {
            module == mapping.core_module
                && function == mapping.component_function
                && mapping.instruction_start < end
        }) {
            return Err(ContractError::InvalidRange(
                "overlapping-instruction".into(),
            ));
        }
        previous_key = Some(key);
        previous_range = Some((
            mapping.core_module.as_str(),
            mapping.component_function,
            mapping.instruction_end,
        ));
    }
    Ok(())
}

/// Resolves one engine frame only when its module/function/offset identifies
/// exactly one accepted debug-map row.
///
/// Missing or ambiguous engine metadata produces an explicit unavailable
/// frame; it never guesses a source span.
#[must_use]
pub fn resolve_engine_frame(map: &DebugMap, frame: &EngineFrame<'_>) -> RuntimeFrame {
    let offset = frame.instruction_offset;
    let mut modules = BTreeSet::new();
    for function in &map.functions {
        if function.component_function != frame.component_function {
            continue;
        }
        if frame
            .core_module
            .is_none_or(|module| module == function.core_module)
        {
            modules.insert(function.core_module.as_str());
        }
    }
    if modules.len() != 1 && frame.core_module.is_some() {
        modules.clear();
        for function in &map.functions {
            if function.component_function == frame.component_function {
                modules.insert(function.core_module.as_str());
            }
        }
    }
    let Some(core_module) = modules
        .iter()
        .copied()
        .next()
        .filter(|_| modules.len() == 1)
    else {
        return unavailable_frame("runtime.unmapped", "module-unresolved", true);
    };
    let Some(function) = map.functions.iter().find(|function| {
        function.core_module == core_module
            && function.component_function == frame.component_function
    }) else {
        return unavailable_frame("runtime.unmapped", "function-unresolved", true);
    };
    let Some(offset) = offset else {
        return unavailable_frame(
            &function.id,
            "instruction-offset-unavailable",
            function.id.starts_with("generated."),
        );
    };
    let upper = map.mappings.partition_point(|mapping| {
        (
            mapping.core_module.as_str(),
            mapping.component_function,
            mapping.instruction_start,
        ) <= (core_module, frame.component_function, offset)
    });
    let Some(mapping) = upper
        .checked_sub(1)
        .and_then(|index| map.mappings.get(index))
    else {
        return unavailable_frame(
            &function.id,
            "instruction-unmapped",
            function.id.starts_with("generated."),
        );
    };
    if mapping.core_module != core_module
        || mapping.component_function != frame.component_function
        || mapping.instruction_start > offset
        || offset >= mapping.instruction_end
    {
        return unavailable_frame(
            &function.id,
            "instruction-unmapped",
            function.id.starts_with("generated."),
        );
    }
    RuntimeFrame {
        function_id: mapping.function_id.clone(),
        source: mapping.source.clone(),
        generated: mapping.generated,
        unavailable_reason: mapping
            .source
            .is_none()
            .then(|| "generated-code".to_owned()),
    }
}

fn unavailable_frame(function_id: &str, reason: &str, generated: bool) -> RuntimeFrame {
    RuntimeFrame {
        function_id: function_id.into(),
        source: None,
        generated,
        unavailable_reason: Some(reason.into()),
    }
}

/// Appends the canonical digest link to an otherwise complete Component.
///
/// # Errors
///
/// Rejects invalid link fields, stale code digests and malformed Components.
pub fn link_component(component_code: &[u8], link: &DebugLink) -> Result<Vec<u8>, ContractError> {
    validate_debug_link(link)?;
    if sha256_hex(component_code) != link.component_code_sha256 {
        return Err(ContractError::StaleBinding("component-code"));
    }
    let bytes = canonical_json(link)?;
    let mut section = Component::new();
    section.section(&CustomSection {
        name: Cow::Borrowed(DEBUG_LINK_SECTION),
        data: Cow::Borrowed(&bytes),
    });
    let section = section.finish();
    if component_code.len() < 8 || section.len() < 8 || component_code[..8] != section[..8] {
        return Err(ContractError::InvalidComponent);
    }
    let mut linked = component_code.to_vec();
    linked.extend_from_slice(&section[8..]);
    Ok(linked)
}

/// Verifies the complete Component/map/identity digest and source-document chain.
///
/// # Errors
///
/// Rejects any malformed artifact, stale/mixed digest, unknown source document
/// or non-canonical contract bytes.
pub fn verify_debug_artifacts(
    component: &[u8],
    map_bytes: &[u8],
    identity_bytes: &[u8],
) -> Result<(DebugMap, DebugIdentity), ContractError> {
    let map = parse_debug_map(map_bytes)?;
    let identity = parse_debug_identity(identity_bytes)?;
    let (component_code, link) = split_debug_link(component)?;
    validate_debug_link(&link)?;
    if identity.source.sha256 != map.binding.source_sha256
        || identity.compiler.executable_sha256 != map.binding.compiler_executable_sha256
        || identity.component_code_sha256 != map.binding.component_code_sha256
        || identity.source.sha256 != link.source_sha256
        || identity.compiler.executable_sha256 != link.compiler_executable_sha256
        || identity.component_code_sha256 != link.component_code_sha256
        || identity.debug_map_sha256 != link.debug_map_sha256
    {
        return Err(ContractError::StaleBinding("identity-chain"));
    }
    if sha256_hex(&component_code) != identity.component_code_sha256
        || sha256_hex(component) != identity.component_sha256
        || sha256_hex(map_bytes) != identity.debug_map_sha256
    {
        return Err(ContractError::StaleBinding("artifact-digest"));
    }
    let source_document = map
        .documents
        .iter()
        .find(|document| document.id == identity.source.document_id)
        .ok_or_else(|| ContractError::UnknownReference(identity.source.document_id.clone()))?;
    if source_document.sha256 != identity.source.sha256
        || source_document.byte_length != identity.source.byte_length
    {
        return Err(ContractError::StaleBinding("source-document"));
    }
    Ok((map, identity))
}

/// Removes and decodes the unique Sico debug-link custom section.
///
/// # Errors
///
/// Rejects malformed Components, missing/duplicate links and non-canonical JSON.
pub fn split_debug_link(component: &[u8]) -> Result<(Vec<u8>, DebugLink), ContractError> {
    if component.len() < 8 || &component[..4] != b"\0asm" {
        return Err(ContractError::InvalidComponent);
    }
    let mut cursor = 8;
    let mut code = component[..8].to_vec();
    let mut link = None;
    while cursor < component.len() {
        let section_start = cursor;
        let id = component[cursor];
        cursor += 1;
        let (section_len, length_bytes) = decode_u32(&component[cursor..])?;
        cursor += length_bytes;
        let section_end = cursor
            .checked_add(section_len as usize)
            .filter(|end| *end <= component.len())
            .ok_or(ContractError::InvalidComponent)?;
        let mut is_debug_link = false;
        if id == 0 {
            let (name_len, name_bytes) = decode_u32(&component[cursor..section_end])?;
            let name_start = cursor + name_bytes;
            let name_end = name_start
                .checked_add(name_len as usize)
                .filter(|end| *end <= section_end)
                .ok_or(ContractError::InvalidComponent)?;
            let name = std::str::from_utf8(&component[name_start..name_end])
                .map_err(|_| ContractError::InvalidComponent)?;
            if name == DEBUG_LINK_SECTION {
                if link.is_some() {
                    return Err(ContractError::DuplicateDebugLink);
                }
                let parsed: DebugLink = serde_json::from_slice(&component[name_end..section_end])
                    .map_err(|error| ContractError::Json(error.to_string()))?;
                if canonical_json(&parsed)? != component[name_end..section_end] {
                    return Err(ContractError::NonCanonical("debug-link-json"));
                }
                link = Some(parsed);
                is_debug_link = true;
            }
        }
        if !is_debug_link {
            code.extend_from_slice(&component[section_start..section_end]);
        }
        cursor = section_end;
    }
    Ok((code, link.ok_or(ContractError::MissingDebugLink)?))
}

fn decode_u32(bytes: &[u8]) -> Result<(u32, usize), ContractError> {
    let mut value = 0_u32;
    let mut shift = 0_u32;
    for (index, byte) in bytes.iter().copied().take(5).enumerate() {
        value |= u32::from(byte & 0x7f)
            .checked_shl(shift)
            .ok_or(ContractError::InvalidComponent)?;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
    }
    Err(ContractError::InvalidComponent)
}

fn validate_span(span: &DebugSpan, documents: &[DebugDocument]) -> Result<(), ContractError> {
    validate_id(&span.document_id)?;
    if span.start > span.end || span.end > MAX_SAFE_INTEGER {
        return Err(ContractError::InvalidRange("source".into()));
    }
    let document = documents
        .iter()
        .find(|document| document.id == span.document_id)
        .ok_or_else(|| ContractError::UnknownReference(span.document_id.clone()))?;
    if span.end > document.byte_length {
        return Err(ContractError::InvalidRange("source-document".into()));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::InvalidDigest(value.into()));
    }
    Ok(())
}

fn validate_debug_link(link: &DebugLink) -> Result<(), ContractError> {
    if link.schema != DEBUG_LINK_SCHEMA {
        return Err(ContractError::UnknownSchema(link.schema.clone()));
    }
    for digest in [
        &link.source_sha256,
        &link.compiler_executable_sha256,
        &link.component_code_sha256,
        &link.debug_map_sha256,
    ] {
        validate_digest(digest)?;
    }
    Ok(())
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = u32::from(chunk[0]) << 16
            | u32::from(*chunk.get(1).unwrap_or(&0)) << 8
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(char::from(TABLE[((value >> 18) & 63) as usize]));
        output.push(char::from(TABLE[((value >> 12) & 63) as usize]));
        output.push(if chunk.len() > 1 {
            char::from(TABLE[((value >> 6) & 63) as usize])
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            char::from(TABLE[(value & 63) as usize])
        } else {
            '='
        });
    }
    output
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(4) {
        return None;
    }
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    for chunk in value.as_bytes().as_chunks::<4>().0 {
        let padding = usize::from(chunk[2] == b'=') + usize::from(chunk[3] == b'=');
        if padding > 0 && chunk.as_slice() != &value.as_bytes()[value.len() - 4..] {
            return None;
        }
        let sextet = |byte| match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        };
        let first = sextet(chunk[0])?;
        let second = sextet(chunk[1])?;
        let third = if chunk[2] == b'=' {
            0
        } else {
            sextet(chunk[2])?
        };
        let fourth = if chunk[3] == b'=' {
            0
        } else {
            sextet(chunk[3])?
        };
        if padding == 2 && chunk[3] != b'=' || padding == 1 && chunk[2] == b'=' {
            return None;
        }
        let packed = u32::from(first) << 18
            | u32::from(second) << 12
            | u32::from(third) << 6
            | u32::from(fourth);
        let bytes = packed.to_be_bytes();
        output.push(bytes[1]);
        if padding < 2 {
            output.push(bytes[2]);
        }
        if padding == 0 {
            output.push(bytes[3]);
        }
    }
    Some(output)
}

fn validate_id(value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'@' | b'-')
        })
    {
        return Err(ContractError::InvalidIdentity(value.into()));
    }
    Ok(())
}

fn validate_sorted_ids(values: &[String]) -> Result<(), ContractError> {
    if values.len() > 64 {
        return Err(ContractError::Limit("identity-list"));
    }
    let mut previous = None;
    for value in values {
        validate_id(value)?;
        if previous.is_some_and(|previous| previous >= value.as_str()) {
            return Err(if previous == Some(value.as_str()) {
                ContractError::Duplicate("identity-list")
            } else {
                ContractError::NonCanonical("identity-list")
            });
        }
        previous = Some(value.as_str());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn artifact_set(document_sha256: String) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let component_code = Component::new().finish();
        let source_sha256 = digest(0xaa);
        let compiler_sha256 = digest(0xbb);
        let map = DebugMap {
            schema: DEBUG_MAP_SCHEMA.into(),
            binding: DebugBinding {
                source_sha256: source_sha256.clone(),
                compiler_executable_sha256: compiler_sha256.clone(),
                component_code_sha256: sha256_hex(&component_code),
            },
            coordinate_system: "utf8-byte-half-open".into(),
            documents: vec![DebugDocument {
                id: "doc.main".into(),
                sha256: document_sha256,
                byte_length: 1,
            }],
            functions: Vec::new(),
            mappings: Vec::new(),
        };
        let map_bytes = canonical_json(&map).unwrap();
        let link = DebugLink {
            schema: DEBUG_LINK_SCHEMA.into(),
            source_sha256: source_sha256.clone(),
            compiler_executable_sha256: compiler_sha256.clone(),
            component_code_sha256: sha256_hex(&component_code),
            debug_map_sha256: sha256_hex(&map_bytes),
        };
        let component = link_component(&component_code, &link).unwrap();
        let identity = DebugIdentity {
            schema: DEBUG_IDENTITY_SCHEMA.into(),
            source: SourceIdentity {
                document_id: "doc.main".into(),
                sha256: source_sha256,
                byte_length: 1,
                display_uri: Some("workspace://main.sico".into()),
            },
            compiler: CompilerIdentity {
                package: "sico-compiler".into(),
                version: "0.0.2-dev".into(),
                executable_sha256: compiler_sha256,
            },
            language_semantics: "sico.semantics.v0".into(),
            ir_schema: "sico.ir.v0".into(),
            component_code_sha256: sha256_hex(&component_code),
            component_sha256: sha256_hex(&component),
            debug_map_sha256: sha256_hex(&map_bytes),
            adapter_identities: Vec::new(),
            wit_identities: Vec::new(),
        };
        (component, map_bytes, canonical_json(&identity).unwrap())
    }

    #[test]
    fn link_roundtrip_is_non_circular_and_strict() {
        let component = Component::new().finish();
        let link = DebugLink {
            schema: DEBUG_LINK_SCHEMA.into(),
            source_sha256: digest(0xaa),
            compiler_executable_sha256: digest(0xbb),
            component_code_sha256: sha256_hex(&component),
            debug_map_sha256: digest(0xdd),
        };
        let linked = link_component(&component, &link).unwrap();
        let (stripped, parsed) = split_debug_link(&linked).unwrap();
        assert_eq!(stripped, component);
        assert_eq!(parsed, link);
        assert_ne!(sha256_hex(&linked), sha256_hex(&stripped));
    }

    #[test]
    fn duplicate_link_and_stale_code_fail_closed() {
        let component = Component::new().finish();
        let mut link = DebugLink {
            schema: DEBUG_LINK_SCHEMA.into(),
            source_sha256: digest(0xaa),
            compiler_executable_sha256: digest(0xbb),
            component_code_sha256: digest(0xcc),
            debug_map_sha256: digest(0xdd),
        };
        assert_eq!(
            link_component(&component, &link),
            Err(ContractError::StaleBinding("component-code"))
        );
        link.component_code_sha256 = sha256_hex(&component);
        let once = link_component(&component, &link).unwrap();
        let twice = link_component(&once, &link).unwrap_err();
        assert_eq!(twice, ContractError::StaleBinding("component-code"));
    }

    #[test]
    fn artifact_mix_and_malformed_inputs_fail_closed() {
        let (component, map, identity) = artifact_set(digest(0xaa));
        verify_debug_artifacts(&component, &map, &identity).unwrap();

        assert!(matches!(
            verify_debug_artifacts(&component[..component.len() - 1], &map, &identity),
            Err(ContractError::InvalidComponent)
        ));
        assert!(matches!(
            verify_debug_artifacts(&component, b"{", &identity),
            Err(ContractError::Json(_))
        ));

        let (_, foreign_map, _) = artifact_set(digest(0xcc));
        assert!(matches!(
            verify_debug_artifacts(&component, &foreign_map, &identity),
            Err(ContractError::StaleBinding(_))
        ));

        let (mismatched_component, mismatched_map, mismatched_identity) =
            artifact_set(digest(0xcc));
        assert_eq!(
            verify_debug_artifacts(&mismatched_component, &mismatched_map, &mismatched_identity),
            Err(ContractError::StaleBinding("source-document"))
        );
    }

    fn runtime_fault_with_frames(count: usize) -> RuntimeFault {
        RuntimeFault {
            schema: "sico.runtime-fault.v0".into(),
            run_id: "run-test".into(),
            generation_id: 1,
            class: "trap".into(),
            code: "runtime.trap".into(),
            key: "runtime_trap".into(),
            message: "guest execution trapped".into(),
            provider_id: None,
            frames: (0..count)
                .map(|index| RuntimeFrame {
                    function_id: format!("fn.{index}"),
                    source: None,
                    generated: false,
                    unavailable_reason: Some("debug-map-unavailable".into()),
                })
                .collect(),
        }
    }

    #[test]
    fn runtime_fault_limits_and_exclusive_frame_location_fail_closed() {
        let accepted = runtime_fault_with_frames(MAX_RUNTIME_FRAMES);
        validate_runtime_fault(&accepted).unwrap();
        assert_eq!(
            parse_runtime_fault(&canonical_json(&accepted).unwrap()).unwrap(),
            accepted
        );

        let limit_plus_one = runtime_fault_with_frames(MAX_RUNTIME_FRAMES + 1);
        assert_eq!(
            validate_runtime_fault(&limit_plus_one),
            Err(ContractError::Limit("runtime-frames"))
        );

        let mut oversized = runtime_fault_with_frames(0);
        oversized.message = "x".repeat(MAX_MESSAGE_BYTES + 1);
        assert_eq!(
            validate_runtime_fault(&oversized),
            Err(ContractError::Limit("fault-message"))
        );

        let mut conflicting = runtime_fault_with_frames(1);
        conflicting.frames[0].source = Some(DebugSpan {
            document_id: "doc.main".into(),
            start: 0,
            end: 1,
        });
        assert!(matches!(
            validate_runtime_fault(&conflicting),
            Err(ContractError::InvalidIdentity(_))
        ));

        let mut unknown = runtime_fault_with_frames(0);
        unknown.class = "engine-text-derived".into();
        assert!(matches!(
            validate_runtime_fault(&unknown),
            Err(ContractError::InvalidIdentity(_))
        ));
    }

    #[test]
    fn cancellation_request_is_canonical_bounded_and_identity_typed() {
        let request = CancelRequest {
            schema: "sico.cancel-request.v0".into(),
            run_id: "run-7".into(),
            generation_id: 3,
            cause: "client".into(),
        };
        let bytes = canonical_json(&request).unwrap();
        assert_eq!(parse_cancel_request(&bytes).unwrap(), request);

        let mut non_canonical = bytes.clone();
        non_canonical.push(b'\n');
        assert_eq!(
            parse_cancel_request(&non_canonical),
            Err(ContractError::NonCanonical("cancel-request-json"))
        );
        assert_eq!(
            parse_cancel_request(&vec![b'x'; MAX_CANCEL_REQUEST_BYTES + 1]),
            Err(ContractError::Limit("cancel-request-bytes"))
        );
        let mut invalid_cause = request;
        invalid_cause.cause = "signal".into();
        assert!(matches!(
            parse_cancel_request(&canonical_json(&invalid_cause).unwrap()),
            Err(ContractError::InvalidIdentity(_))
        ));
    }

    struct TestRedactor;
    impl EventRedactor for TestRedactor {
        fn redact(&self, bytes: &[u8]) -> Vec<u8> {
            bytes
                .windows(6)
                .enumerate()
                .fold(bytes.to_vec(), |mut output, (index, window)| {
                    if window == b"secret" {
                        output[index..index + 6].copy_from_slice(b"******");
                    }
                    output
                })
        }
    }

    fn event(sequence: u64, kind: &str) -> ExecutionEvent {
        ExecutionEvent {
            schema: EXECUTION_EVENT_SCHEMA.into(),
            run_id: "run-1".into(),
            generation_id: 0,
            sequence,
            kind: kind.into(),
            task_id: Some("task-0".into()),
            parent_task_id: None,
            scope_id: Some("scope-0".into()),
            cause: "guest".into(),
            payload: ExecutionEventPayload::default(),
        }
    }

    #[test]
    fn execution_events_are_canonical_binary_safe_and_bounded() {
        let mut queue = EventQueue::new();
        queue
            .push_output(
                event(0, "stdout"),
                &[0, 255, b's', b'e', b'c', b'r', b'e', b't'],
                &TestRedactor,
            )
            .unwrap();
        let frame = queue.pop().unwrap();
        let decoded = parse_execution_event(&frame).unwrap();
        assert_eq!(decoded.kind, "stdout");
        assert_eq!(
            decode_base64(decoded.payload.bytes_base64.as_deref().unwrap()).unwrap(),
            [0, 255, b'*', b'*', b'*', b'*', b'*', b'*']
        );
        let mut invalid = event(1, "started");
        invalid.payload.bytes_base64 = Some("AA==".into());
        assert!(matches!(
            validate_execution_event(&invalid),
            Err(ContractError::InvalidIdentity(_))
        ));
    }

    #[test]
    fn overflow_has_a_marker_and_terminal_is_never_lost() {
        let mut queue = EventQueue::new();
        for sequence in 0..=MAX_EVENT_QUEUE_ITEMS as u64 {
            queue.push(event(sequence, "started")).unwrap();
        }
        let mut terminal = event(999, "terminal");
        terminal.cause = "internal".into();
        terminal.payload.terminal_class = Some("success".into());
        queue.push(terminal).unwrap();
        let frames: Vec<_> = std::iter::from_fn(|| queue.pop())
            .map(|frame| parse_execution_event(&frame).unwrap())
            .collect();
        assert!(frames.iter().any(|frame| frame.kind == "dropped"));
        assert_eq!(frames.last().unwrap().kind, "terminal");
        assert!(queue.is_terminal());
    }

    #[test]
    fn source_lookup_over_one_hundred_thousand_rows_is_measured_and_exact() {
        let mappings = (0..100_000_u64)
            .map(|offset| DebugMapping {
                core_module: "core".into(),
                component_function: 7,
                instruction_start: offset,
                instruction_end: offset + 1,
                function_id: "fn.main".into(),
                source: Some(DebugSpan {
                    document_id: "doc".into(),
                    start: offset,
                    end: offset + 1,
                }),
                call_site: None,
                inline_parent: None,
                generated: false,
            })
            .collect();
        let map = DebugMap {
            schema: DEBUG_MAP_SCHEMA.into(),
            binding: DebugBinding {
                source_sha256: "a".repeat(64),
                compiler_executable_sha256: "b".repeat(64),
                component_code_sha256: "c".repeat(64),
            },
            coordinate_system: "utf8-byte-half-open".into(),
            documents: vec![DebugDocument {
                id: "doc".into(),
                sha256: "a".repeat(64),
                byte_length: 100_000,
            }],
            functions: vec![DebugFunction {
                id: "fn.main".into(),
                core_module: "core".into(),
                component_function: 7,
            }],
            mappings,
        };
        validate_debug_map(&map).unwrap();
        let frame = EngineFrame {
            core_module: Some("core"),
            component_function: 7,
            instruction_offset: Some(99_999),
        };
        let mut samples = Vec::new();
        for _ in 0..20 {
            let started = std::time::Instant::now();
            for _ in 0..1_000 {
                let resolved = resolve_engine_frame(&map, &frame);
                assert_eq!(resolved.source.as_ref().unwrap().start, 99_999);
            }
            samples.push(started.elapsed().as_nanos() / 1_000);
        }
        samples.sort_unstable();
        let p95 = samples[18];
        println!("DEBUG_LOOKUP_100K_P95_NS={p95}");
        assert!(
            p95 < 5_000_000_000,
            "lookup batch unexpectedly stalled: {p95}ns"
        );
    }
}
