//! Shared, non-executing editor/AI execution plans for STEP-0093.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

pub const EXECUTION_PLAN_SCHEMA: &str = "sico.execution-plan.v0";
pub const DEBUG_LAUNCH_PLAN_SCHEMA: &str = "sico.debug-launch-plan.v0";
pub const MAX_PROGRAM_BYTES: usize = 4 * 1024;
pub const MAX_ARGUMENTS: usize = 64;
pub const MAX_ARGUMENT_BYTES: usize = 4 * 1024;
pub const MAX_TOTAL_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_LOG_BYTES: usize = 1024 * 1024;
pub const MAX_DAP_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_DAP_HEADER_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DapProtocolError {
    Header,
    DuplicateLength,
    MissingLength,
    FrameLimit,
    Truncated,
    TrailingBytes,
    Json,
    Shape,
    UnknownClaim,
}

/// Decodes exactly one bounded DAP Content-Length frame.
///
/// # Errors
///
/// Rejects non-CRLF headers, duplicate/missing/invalid lengths, oversized or
/// truncated bodies, trailing bytes, invalid JSON and non-object payloads.
pub fn decode_dap_frame(frame: &[u8]) -> Result<Value, DapProtocolError> {
    let header_end = frame
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or(DapProtocolError::Header)?;
    if header_end > MAX_DAP_HEADER_BYTES
        || frame[..header_end].contains(&b'\n')
            && !frame[..header_end]
                .windows(2)
                .any(|window| window == b"\r\n")
    {
        return Err(DapProtocolError::Header);
    }
    let header = std::str::from_utf8(&frame[..header_end]).map_err(|_| DapProtocolError::Header)?;
    let mut length = None;
    for line in header.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            return Err(DapProtocolError::Header);
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(DapProtocolError::DuplicateLength);
            }
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|_| DapProtocolError::Header)?;
            if parsed > MAX_DAP_FRAME_BYTES {
                return Err(DapProtocolError::FrameLimit);
            }
            length = Some(parsed);
        } else if !name.eq_ignore_ascii_case("Content-Type") {
            return Err(DapProtocolError::Header);
        }
    }
    let length = length.ok_or(DapProtocolError::MissingLength)?;
    let body_start = header_end + 4;
    let body_end = body_start
        .checked_add(length)
        .ok_or(DapProtocolError::FrameLimit)?;
    if frame.len() < body_end {
        return Err(DapProtocolError::Truncated);
    }
    if frame.len() != body_end {
        return Err(DapProtocolError::TrailingBytes);
    }
    let value: Value =
        serde_json::from_slice(&frame[body_start..body_end]).map_err(|_| DapProtocolError::Json)?;
    if !value.is_object() {
        return Err(DapProtocolError::Shape);
    }
    Ok(value)
}

/// Encodes one DAP object with a deterministic compact JSON body.
///
/// # Errors
///
/// Rejects non-object or oversized payloads.
pub fn encode_dap_frame(value: &Value) -> Result<Vec<u8>, DapProtocolError> {
    if !value.is_object() {
        return Err(DapProtocolError::Shape);
    }
    let body = serde_json::to_vec(value).map_err(|_| DapProtocolError::Json)?;
    if body.len() > MAX_DAP_FRAME_BYTES {
        return Err(DapProtocolError::FrameLimit);
    }
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(&body);
    Ok(frame)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DapClaimState {
    Supported,
    Refused,
}

/// Looks up the sole machine-readable DAP allowlist. Missing or malformed
/// rows are treated as unknown; callers must never infer support.
///
/// # Errors
///
/// Rejects an absent, duplicate, malformed or unsupported claim state.
pub fn dap_claim_state(kind: &str, name: &str) -> Result<DapClaimState, DapProtocolError> {
    let contract: Value = serde_json::from_str(include_str!(
        "../../../observability/contracts/dap-claimed-subset-v0.json"
    ))
    .map_err(|_| DapProtocolError::Json)?;
    let claims = contract["claims"]
        .as_array()
        .ok_or(DapProtocolError::Shape)?;
    let mut matched = claims
        .iter()
        .filter(|claim| claim["kind"] == kind && claim["name"] == name);
    let claim = matched.next().ok_or(DapProtocolError::UnknownClaim)?;
    if matched.next().is_some() {
        return Err(DapProtocolError::Shape);
    }
    match claim["state"].as_str() {
        Some("supported") => Ok(DapClaimState::Supported),
        Some("refused") => Ok(DapClaimState::Refused),
        _ => Err(DapProtocolError::Shape),
    }
}

/// Produces the stable typed response for refused and unclaimed requests.
/// This helper cannot accidentally advertise or execute a capability.
///
/// # Errors
///
/// Rejects malformed envelopes and requests that are actually supported.
pub fn refuse_dap_request(request: &Value) -> Result<Value, DapProtocolError> {
    let sequence = request["seq"].as_u64().ok_or(DapProtocolError::Shape)?;
    let command = request["command"].as_str().ok_or(DapProtocolError::Shape)?;
    if request["type"] != "request" {
        return Err(DapProtocolError::Shape);
    }
    match dap_claim_state("request", command) {
        Ok(DapClaimState::Supported) => return Err(DapProtocolError::Shape),
        Ok(DapClaimState::Refused) | Err(DapProtocolError::UnknownClaim) => {}
        Err(error) => return Err(error),
    }
    Ok(json!({
        "seq": 0,
        "type": "response",
        "request_seq": sequence,
        "success": false,
        "command": command,
        "message": "unsupported-request",
        "body": { "code": "unsupported-request" }
    }))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DapBackendReply {
    pub body: Value,
    pub events: Vec<Value>,
}

pub trait DapBackend {
    /// Handles one allowlisted request.
    ///
    /// # Errors
    ///
    /// Returns a stable backend error code for a failed request.
    fn request(&mut self, command: &str, arguments: &Value) -> Result<DapBackendReply, String>;

    /// Polls bounded asynchronous events.
    ///
    /// # Errors
    ///
    /// Returns a stable backend error code when Runtime polling fails.
    fn poll(&mut self) -> Result<DapBackendReply, String> {
        Ok(DapBackendReply::default())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DapSessionState {
    New,
    Initialized,
    Launched,
    Configured,
    Terminated,
}

/// Exact-allowlist DAP state machine. Runtime-bearing requests are delegated
/// to an owned backend; protocol code never fabricates breakpoint, pause,
/// frame, variable, output or terminal evidence.
pub struct DapSession<B> {
    backend: B,
    state: DapSessionState,
    sequence: u64,
}

impl<B: DapBackend> DapSession<B> {
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: DapSessionState::New,
            sequence: 1,
        }
    }

    /// Handles one decoded DAP request and returns a response followed by any
    /// allowlisted events emitted by the backend.
    ///
    /// # Errors
    ///
    /// Rejects malformed request envelopes and backend attempts to emit an
    /// unclaimed event or an oversized payload.
    pub fn handle(&mut self, request: &Value) -> Result<Vec<Value>, DapProtocolError> {
        if request["type"] != "request" {
            return Err(DapProtocolError::Shape);
        }
        let request_seq = request["seq"].as_u64().ok_or(DapProtocolError::Shape)?;
        let command = request["command"].as_str().ok_or(DapProtocolError::Shape)?;
        if dap_claim_state("request", command) != Ok(DapClaimState::Supported) {
            let mut response = refuse_dap_request(request)?;
            response["seq"] = self.next_sequence().into();
            return Ok(vec![response]);
        }
        if command == "initialize" {
            if self.state != DapSessionState::New {
                return Ok(vec![self.failure(request_seq, command, "invalid-state")]);
            }
            self.state = DapSessionState::Initialized;
            let response = self.success(
                request_seq,
                command,
                &json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsTerminateRequest": true,
                    "supportsConditionalBreakpoints": false,
                    "supportsHitConditionalBreakpoints": false,
                    "supportsLogPoints": false,
                    "supportsEvaluateForHovers": false,
                    "supportsStepBack": false,
                    "supportsRestartRequest": false,
                    "supportsSetVariable": false,
                    "supportsReadMemoryRequest": false,
                    "supportsWriteMemoryRequest": false,
                    "supportsDisassembleRequest": false
                }),
            );
            let event = self.event("initialized", &json!({}))?;
            return Ok(vec![response, event]);
        }
        if !self.precondition(command) {
            return Ok(vec![self.failure(request_seq, command, "invalid-state")]);
        }
        let arguments = request.get("arguments").unwrap_or(&Value::Null);
        let reply = match self.backend.request(command, arguments) {
            Ok(reply) => reply,
            Err(code) => return Ok(vec![self.failure(request_seq, command, &code)]),
        };
        self.transition(command);
        let mut messages = vec![self.success(request_seq, command, &reply.body)];
        messages.extend(self.encode_events(reply.events)?);
        Ok(messages)
    }

    /// Polls asynchronous Runtime output/terminal events without fabricating
    /// a request or response envelope.
    ///
    /// # Errors
    ///
    /// Applies the same sole allowlist and frame bound as request replies.
    pub fn poll(&mut self) -> Result<Vec<Value>, DapProtocolError> {
        if matches!(
            self.state,
            DapSessionState::New | DapSessionState::Terminated
        ) {
            return Ok(Vec::new());
        }
        let reply = self.backend.poll().map_err(|_| DapProtocolError::Shape)?;
        let terminal = reply
            .events
            .iter()
            .any(|event| event["event"] == "terminated");
        let messages = self.encode_events(reply.events)?;
        if terminal {
            self.state = DapSessionState::Terminated;
        }
        Ok(messages)
    }

    fn encode_events(&mut self, events: Vec<Value>) -> Result<Vec<Value>, DapProtocolError> {
        let mut messages = Vec::with_capacity(events.len());
        for event in events {
            let name = event["event"].as_str().ok_or(DapProtocolError::Shape)?;
            if dap_claim_state("event", name) != Ok(DapClaimState::Supported) {
                return Err(DapProtocolError::UnknownClaim);
            }
            if serde_json::to_vec(&event)
                .map_err(|_| DapProtocolError::Json)?
                .len()
                > MAX_DAP_FRAME_BYTES
            {
                return Err(DapProtocolError::FrameLimit);
            }
            messages.push(self.event(name, event.get("body").unwrap_or(&Value::Null))?);
        }
        Ok(messages)
    }

    fn precondition(&self, command: &str) -> bool {
        match command {
            "launch" => self.state == DapSessionState::Initialized,
            "setBreakpoints" => matches!(
                self.state,
                DapSessionState::Initialized | DapSessionState::Launched
            ),
            "configurationDone" => self.state == DapSessionState::Launched,
            "threads" | "disconnect" => {
                self.state != DapSessionState::New && self.state != DapSessionState::Terminated
            }
            "terminate" => matches!(
                self.state,
                DapSessionState::Launched | DapSessionState::Configured
            ),
            "stackTrace" | "scopes" | "variables" | "continue" | "pause" => {
                self.state == DapSessionState::Configured
            }
            _ => false,
        }
    }

    fn transition(&mut self, command: &str) {
        match command {
            "launch" => self.state = DapSessionState::Launched,
            "configurationDone" => self.state = DapSessionState::Configured,
            "disconnect" | "terminate" => self.state = DapSessionState::Terminated,
            _ => {}
        }
    }

    fn success(&mut self, request_seq: u64, command: &str, body: &Value) -> Value {
        json!({"seq": self.next_sequence(), "type": "response", "request_seq": request_seq, "success": true, "command": command, "body": body})
    }

    fn failure(&mut self, request_seq: u64, command: &str, code: &str) -> Value {
        json!({"seq": self.next_sequence(), "type": "response", "request_seq": request_seq, "success": false, "command": command, "message": code, "body": {"code": code}})
    }

    fn event(&mut self, name: &str, body: &Value) -> Result<Value, DapProtocolError> {
        if dap_claim_state("event", name) != Ok(DapClaimState::Supported) {
            return Err(DapProtocolError::UnknownClaim);
        }
        Ok(json!({"seq": self.next_sequence(), "type": "event", "event": name, "body": body}))
    }

    fn next_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        sequence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionMode {
    Check,
    Run,
    Watch,
    Repl,
}

impl ExecutionMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Run => "run",
            Self::Watch => "watch",
            Self::Repl => "repl",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanError {
    InvalidProgram,
    UnexpectedProgram,
    InvalidArguments,
    InvalidLogLimit,
}

/// Builds a bounded direct-process plan. The returned value is data only: this
/// crate never reads source, spawns a process, opens a shell, or applies grants.
///
/// # Errors
///
/// Returns [`PlanError`] when a path, argument, mode combination, or capture
/// limit exceeds the frozen protocol bounds.
pub fn execution_plan(
    mode: ExecutionMode,
    program: Option<&str>,
    guest_arguments: &[String],
    max_log_bytes: usize,
) -> Result<Value, PlanError> {
    if max_log_bytes == 0 || max_log_bytes > MAX_LOG_BYTES {
        return Err(PlanError::InvalidLogLimit);
    }
    let program = match mode {
        ExecutionMode::Repl if program.is_some() => return Err(PlanError::UnexpectedProgram),
        ExecutionMode::Repl => None,
        _ => Some(
            program
                .filter(|value| valid_program(value))
                .ok_or(PlanError::InvalidProgram)?,
        ),
    };
    if mode == ExecutionMode::Check && !guest_arguments.is_empty() {
        return Err(PlanError::InvalidArguments);
    }
    if mode == ExecutionMode::Repl && !guest_arguments.is_empty() {
        return Err(PlanError::InvalidArguments);
    }
    if guest_arguments.len() > MAX_ARGUMENTS
        || guest_arguments
            .iter()
            .any(|argument| !valid_argument(argument))
        || guest_arguments.iter().map(String::len).sum::<usize>() > MAX_TOTAL_ARGUMENT_BYTES
    {
        return Err(PlanError::InvalidArguments);
    }

    let mut arguments = vec![mode.as_str().to_owned(), "--json".to_owned()];
    if let Some(program) = program {
        arguments.push(program.to_owned());
    }
    if !guest_arguments.is_empty() {
        arguments.push("--".to_owned());
        arguments.extend(guest_arguments.iter().cloned());
    }
    let (stdout, stderr_schemas, runtime_locations) = match mode {
        ExecutionMode::Check => ("sico.diagnostics.v0-json", Vec::<&str>::new(), false),
        ExecutionMode::Run => ("guest-bytes", vec!["sico.runner.outcome.v0"], false),
        ExecutionMode::Watch => (
            "guest-bytes",
            vec!["sico.runner.outcome.v0", "sico.runner.watch.v0"],
            false,
        ),
        ExecutionMode::Repl => (
            "sico.repl.event.v0-jsonl",
            vec!["sico.repl.event.v0"],
            false,
        ),
    };
    Ok(json!({
        "schema": EXECUTION_PLAN_SCHEMA,
        "mode": mode.as_str(),
        "executable": "sico",
        "arguments": arguments,
        "shell": false,
        "cwd": null,
        "output": {
            "stdout": stdout,
            "stderr_schemas": stderr_schemas,
            "capture_limit_bytes": max_log_bytes,
            "overflow": "truncate-and-mark"
        },
        "cancellation": {
            "owner": "client",
            "action": "terminate-direct-child-tree",
            "grace_ms": 1000,
            "typed_runner_exit_guaranteed": false
        },
        "source_map": {
            "compile_coordinates": "utf8-byte-half-open",
            "source": program,
            "runtime_locations": runtime_locations,
            "debug_adapter": false
        }
    }))
}

/// Builds a data-only, shell-free launch plan for the bounded `sico-dap`
/// adapter. Every path is a direct argv value; the editor remains responsible
/// for producing the verified debug triplet and owning the child lifecycle.
///
/// # Errors
///
/// Rejects missing, over-limit or control-character-bearing paths and IDs.
pub fn debug_launch_plan(
    component: &str,
    debug_map: &str,
    debug_identity: &str,
    source: &str,
    document_id: &str,
) -> Result<Value, PlanError> {
    if [component, debug_map, debug_identity, source]
        .into_iter()
        .any(|value| !valid_program(value))
        || !valid_argument(document_id)
        || document_id.is_empty()
    {
        return Err(PlanError::InvalidProgram);
    }
    Ok(json!({
        "schema": DEBUG_LAUNCH_PLAN_SCHEMA,
        "mode": "debug",
        "executable": "sico-dap",
        "arguments": [component, debug_map, debug_identity, source, document_id],
        "shell": false,
        "cwd": null,
        "protocol": {
            "transport": "stdio",
            "framing": "dap-content-length-crlf",
            "max_frame_bytes": MAX_DAP_FRAME_BYTES,
            "claimed_subset": "sico.dap-claimed-subset.v0"
        },
        "identity": {
            "source_coordinates": "utf8-byte-half-open",
            "component_debug_map_source": "sha256-bound",
            "display_paths_are_authority": false
        },
        "cancellation": {
            "owner": "editor",
            "action": "dap-terminate-or-disconnect",
            "typed_runner_exit_guaranteed": true,
            "fallback": "terminate-owned-child-tree"
        }
    }))
}

fn valid_program(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PROGRAM_BYTES && valid_characters(value)
}

fn valid_argument(value: &str) -> bool {
    value.len() <= MAX_ARGUMENT_BYTES && valid_characters(value)
}

fn valid_characters(value: &str) -> bool {
    !value.contains('\0') && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metacharacters_and_spaces_remain_one_direct_argument() {
        let program = "path with spaces/$(touch nope);`name`.sico";
        let guest = vec!["; echo injected".to_owned(), "$(whoami)".to_owned()];
        let plan = execution_plan(ExecutionMode::Run, Some(program), &guest, 64 * 1024).unwrap();
        assert_eq!(
            plan["arguments"],
            json!([
                "run",
                "--json",
                program,
                "--",
                "; echo injected",
                "$(whoami)"
            ])
        );
        assert_eq!(plan["shell"], false);
        assert_eq!(plan["cwd"], Value::Null);

        let empty =
            execution_plan(ExecutionMode::Run, Some("app.sico"), &[String::new()], 1).unwrap();
        assert_eq!(
            empty["arguments"],
            json!(["run", "--json", "app.sico", "--", ""])
        );
    }

    #[test]
    fn mode_protocols_and_honest_debug_boundary_are_explicit() {
        let watch =
            execution_plan(ExecutionMode::Watch, Some("app.sico"), &[], MAX_LOG_BYTES).unwrap();
        assert_eq!(watch["schema"], EXECUTION_PLAN_SCHEMA);
        assert_eq!(watch["output"]["capture_limit_bytes"], MAX_LOG_BYTES);
        assert_eq!(
            watch["output"]["stderr_schemas"],
            json!(["sico.runner.outcome.v0", "sico.runner.watch.v0"])
        );
        assert_eq!(watch["source_map"]["runtime_locations"], false);
        assert_eq!(watch["source_map"]["debug_adapter"], false);

        let repl = execution_plan(ExecutionMode::Repl, None, &[], 1).unwrap();
        assert_eq!(repl["arguments"], json!(["repl", "--json"]));
        assert_eq!(repl["output"]["stdout"], "sico.repl.event.v0-jsonl");
    }

    #[test]
    fn debug_launch_is_direct_argv_and_identity_bound_by_contract() {
        let plan = debug_launch_plan(
            "out path/app.component.wasm",
            "out path/app.debug-map.json",
            "out path/app.debug-identity.json",
            "source path/app.sico",
            "doc.app",
        )
        .unwrap();
        assert_eq!(plan["schema"], DEBUG_LAUNCH_PLAN_SCHEMA);
        assert_eq!(plan["executable"], "sico-dap");
        assert_eq!(plan["shell"], false);
        assert_eq!(plan["arguments"].as_array().unwrap().len(), 5);
        assert_eq!(plan["protocol"]["max_frame_bytes"], MAX_DAP_FRAME_BYTES);
        assert_eq!(
            plan["identity"]["component_debug_map_source"],
            "sha256-bound"
        );
        assert!(debug_launch_plan("bad\npath", "m", "i", "s", "doc").is_err());

        let schema: Value = serde_json::from_str(include_str!(
            "../../../tooling/schema/debug-launch-plan-v0.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            DEBUG_LAUNCH_PLAN_SCHEMA
        );
        assert_eq!(schema["properties"]["shell"]["const"], false);
    }

    #[test]
    fn every_input_and_capture_bound_fails_closed() {
        assert_eq!(
            execution_plan(ExecutionMode::Run, None, &[], 1),
            Err(PlanError::InvalidProgram)
        );
        assert_eq!(
            execution_plan(ExecutionMode::Repl, Some("app.sico"), &[], 1),
            Err(PlanError::UnexpectedProgram)
        );
        assert_eq!(
            execution_plan(ExecutionMode::Run, Some("app.sico"), &[], MAX_LOG_BYTES + 1),
            Err(PlanError::InvalidLogLimit)
        );
        assert_eq!(
            execution_plan(
                ExecutionMode::Run,
                Some("app.sico"),
                &vec!["x".to_owned(); MAX_ARGUMENTS + 1],
                1,
            ),
            Err(PlanError::InvalidArguments)
        );
        assert_eq!(
            execution_plan(ExecutionMode::Run, Some("bad\npath"), &[], 1),
            Err(PlanError::InvalidProgram)
        );
    }

    #[test]
    fn repository_schema_tracks_the_frozen_identity_and_bounds() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../tooling/schema/execution-plan-v0.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            EXECUTION_PLAN_SCHEMA
        );
        assert_eq!(
            schema["properties"]["output"]["properties"]["capture_limit_bytes"]["maximum"],
            MAX_LOG_BYTES
        );
        assert_eq!(schema["properties"]["shell"]["const"], false);
        assert_eq!(
            schema["properties"]["source_map"]["properties"]["debug_adapter"]["const"],
            false
        );
    }

    #[test]
    fn dap_framing_is_exact_bounded_and_object_only() {
        let value = json!({"seq": 1, "type": "request", "command": "initialize"});
        let frame = encode_dap_frame(&value).unwrap();
        assert_eq!(decode_dap_frame(&frame).unwrap(), value);
        for invalid in [
            b"Content-Length: 2\n\n{}".to_vec(),
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
            b"Content-Type: application/json\r\n\r\n{}".to_vec(),
            b"Content-Length: 3\r\n\r\n{}".to_vec(),
            b"Content-Length: 2\r\n\r\n{}x".to_vec(),
            b"Content-Length: 2\r\n\r\n[]".to_vec(),
        ] {
            assert!(decode_dap_frame(&invalid).is_err(), "{invalid:?}");
        }
        let oversized = format!("Content-Length: {}\r\n\r\n", MAX_DAP_FRAME_BYTES + 1);
        assert_eq!(
            decode_dap_frame(oversized.as_bytes()),
            Err(DapProtocolError::FrameLimit)
        );
    }

    #[test]
    fn every_refused_claim_and_unknown_request_has_one_typed_response() {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../observability/contracts/dap-claimed-subset-v0.json"
        ))
        .unwrap();
        for claim in contract["claims"].as_array().unwrap() {
            let name = claim["name"].as_str().unwrap();
            let state = dap_claim_state(claim["kind"].as_str().unwrap(), name).unwrap();
            assert_eq!(
                state == DapClaimState::Supported,
                claim["state"] == "supported"
            );
            if claim["kind"] == "request" && state == DapClaimState::Refused {
                let response =
                    refuse_dap_request(&json!({"seq": 7, "type": "request", "command": name}))
                        .unwrap();
                assert_eq!(response["success"], false);
                assert_eq!(response["message"], "unsupported-request");
                assert_eq!(response["request_seq"], 7);
            }
        }
        let response =
            refuse_dap_request(&json!({"seq": 8, "type": "request", "command": "futureRequest"}))
                .unwrap();
        assert_eq!(response["body"]["code"], "unsupported-request");
    }

    #[derive(Default)]
    struct MockDapBackend {
        commands: Vec<String>,
    }

    impl DapBackend for MockDapBackend {
        fn request(
            &mut self,
            command: &str,
            _arguments: &Value,
        ) -> Result<DapBackendReply, String> {
            self.commands.push(command.to_owned());
            let events = match command {
                "continue" => vec![
                    json!({"event": "continued", "body": {"threadId": 1, "allThreadsContinued": true}}),
                ],
                "terminate" => vec![
                    json!({"event": "terminated", "body": {}}),
                    json!({"event": "exited", "body": {"exitCode": 123}}),
                ],
                _ => Vec::new(),
            };
            Ok(DapBackendReply {
                body: json!({}),
                events,
            })
        }
    }

    fn dap_request(sequence: u64, command: &str) -> Value {
        json!({"seq": sequence, "type": "request", "command": command, "arguments": {}})
    }

    #[test]
    fn dap_session_advertises_exactly_and_never_delegates_refused_requests() {
        let mut session = DapSession::new(MockDapBackend::default());
        let initialized = session.handle(&dap_request(1, "initialize")).unwrap();
        assert_eq!(initialized.len(), 2);
        assert_eq!(initialized[0]["body"]["supportsStepBack"], false);
        assert_eq!(initialized[1]["event"], "initialized");
        let refused = session.handle(&dap_request(2, "evaluate")).unwrap();
        assert_eq!(refused[0]["message"], "unsupported-request");
        let launch = session.handle(&dap_request(3, "launch")).unwrap();
        assert_eq!(launch[0]["success"], true);
        session
            .handle(&dap_request(4, "configurationDone"))
            .unwrap();
        let continued = session.handle(&dap_request(5, "continue")).unwrap();
        assert_eq!(continued[1]["event"], "continued");
        let terminated = session.handle(&dap_request(6, "terminate")).unwrap();
        assert_eq!(terminated[1]["event"], "terminated");
        assert_eq!(terminated[2]["event"], "exited");
        let after = session.handle(&dap_request(7, "threads")).unwrap();
        assert_eq!(after[0]["message"], "invalid-state");
        assert_eq!(
            session.backend.commands,
            ["launch", "configurationDone", "continue", "terminate"]
        );
    }

    #[test]
    fn dap_backend_cannot_emit_unclaimed_or_oversized_events() {
        struct BadBackend;
        impl DapBackend for BadBackend {
            fn request(&mut self, _: &str, _: &Value) -> Result<DapBackendReply, String> {
                Ok(DapBackendReply {
                    body: json!({}),
                    events: vec![json!({"event": "process", "body": {}})],
                })
            }
        }
        let mut session = DapSession::new(BadBackend);
        session.handle(&dap_request(1, "initialize")).unwrap();
        assert_eq!(
            session.handle(&dap_request(2, "launch")),
            Err(DapProtocolError::UnknownClaim)
        );
    }
}
