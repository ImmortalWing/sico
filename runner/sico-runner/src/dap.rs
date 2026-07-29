use std::time::Duration;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sico_observability::{EventRedactor, parse_execution_event};
use sico_tooling_protocol::{DapBackend, DapBackendReply};

use crate::{
    CancelToken, DebugBreakpoint, DebugStop, PreparedProgram, RunnerLimits, RuntimeDebugSession,
    ScriptInput,
};

const THREAD_ID: u64 = 1;
const MAX_STACK_FRAMES: usize = 256;
const MAX_VARIABLES: usize = 256;
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

pub struct RuntimeDapConfig<'a> {
    pub document_id: &'a str,
    pub display_uri: &'a str,
    pub source: Vec<u8>,
    pub input: ScriptInput,
    pub limits: RunnerLimits,
    pub cancel: CancelToken,
    pub run_id: &'a str,
    pub generation_id: u64,
}

/// Runner-owned implementation behind the exact M10 DAP allowlist. The
/// constructor binds the client-visible source bytes to the verified debug
/// map before any protocol request can launch a Store.
pub struct RuntimeDapBackend {
    prepared: PreparedProgram,
    document_id: String,
    display_uri: String,
    line_starts: Vec<u64>,
    input: ScriptInput,
    limits: RunnerLimits,
    cancel: CancelToken,
    breakpoints: Vec<DebugBreakpoint>,
    session: Option<RuntimeDebugSession>,
    stop: Option<DebugStop>,
    launched: bool,
    stop_on_entry: bool,
    run_id: String,
    generation_id: u64,
}

impl RuntimeDapBackend {
    /// Creates an authority-neutral adapter over an already prepared Program.
    /// Source bytes must exactly match the digest and length in the verified
    /// debug map; the display URI is never used to open a file.
    pub fn new(prepared: PreparedProgram, config: RuntimeDapConfig<'_>) -> Result<Self, String> {
        let RuntimeDapConfig {
            document_id,
            display_uri,
            source,
            input,
            limits,
            cancel,
            run_id,
            generation_id,
        } = config;
        let map = prepared
            .debug_map
            .as_deref()
            .ok_or_else(|| "debug-map-unavailable".to_owned())?;
        let document = map
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .ok_or_else(|| "source-document-mismatch".to_owned())?;
        let digest = format!("{:x}", Sha256::digest(&source));
        if document.byte_length != source.len() as u64 || document.sha256 != digest {
            return Err("source-identity-mismatch".to_owned());
        }
        let mut line_starts = vec![0];
        for (index, byte) in source.iter().enumerate() {
            if *byte == b'\n' {
                line_starts.push((index + 1) as u64);
            }
        }
        Ok(Self {
            prepared,
            document_id: document_id.to_owned(),
            display_uri: display_uri.to_owned(),
            line_starts,
            input,
            limits,
            cancel,
            breakpoints: Vec::new(),
            session: None,
            stop: None,
            launched: false,
            stop_on_entry: false,
            run_id: run_id.to_owned(),
            generation_id,
        })
    }

    fn set_breakpoints(&mut self, arguments: &Value) -> Result<DapBackendReply, String> {
        let requested_document = arguments
            .pointer("/source/documentId")
            .and_then(Value::as_str)
            .ok_or_else(|| "source-document-required".to_owned())?;
        if requested_document != self.document_id {
            return Err("source-identity-mismatch".to_owned());
        }
        let requested = arguments
            .get("breakpoints")
            .and_then(Value::as_array)
            .ok_or_else(|| "breakpoints-required".to_owned())?;
        if requested.len() > 256 {
            return Err("breakpoint-limit".to_owned());
        }
        let mut response = Vec::with_capacity(requested.len());
        let mut accepted = Vec::new();
        for item in requested {
            if item.get("condition").is_some()
                || item.get("hitCondition").is_some()
                || item.get("logMessage").is_some()
            {
                return Err("unsupported-breakpoint-mode".to_owned());
            }
            let line = item
                .get("line")
                .and_then(Value::as_u64)
                .filter(|line| *line > 0)
                .ok_or_else(|| "invalid-source-line".to_owned())?;
            let offset = self
                .line_starts
                .get((line - 1) as usize)
                .copied()
                .ok_or_else(|| "invalid-source-line".to_owned())?;
            let binding = self
                .prepared
                .bind_source_breakpoints(&self.document_id, &[offset])
                .into_iter()
                .next()
                .and_then(|binding| binding.breakpoint);
            if let Some(breakpoint) = binding {
                accepted.push(breakpoint);
                response.push(json!({"verified": true, "line": line, "source": {"name": self.display_uri, "documentId": self.document_id}}));
            } else {
                response.push(
                    json!({"verified": false, "line": line, "message": "unbound-source-location"}),
                );
            }
        }
        self.breakpoints = accepted;
        Ok(reply(json!({"breakpoints": response})))
    }

    fn configure(&mut self) -> Result<DapBackendReply, String> {
        if !self.launched || self.session.is_some() {
            return Err("invalid-state".to_owned());
        }
        let mut breakpoints = self.breakpoints.clone();
        if self.stop_on_entry {
            let map = self
                .prepared
                .debug_map
                .as_deref()
                .ok_or_else(|| "debug-map-unavailable".to_owned())?;
            let core_base = self
                .prepared
                .debug_core_base
                .ok_or_else(|| "debug-map-unavailable".to_owned())?;
            let entry = map
                .mappings
                .iter()
                .find(|mapping| !mapping.generated && mapping.source.is_some())
                .and_then(|mapping| {
                    Some(DebugBreakpoint {
                        core_module: mapping.core_module.clone(),
                        module_pc: u32::try_from(mapping.instruction_start.checked_sub(core_base)?)
                            .ok()?,
                        source_offset: mapping.source.as_ref()?.start,
                    })
                })
                .ok_or_else(|| "entry-location-unavailable".to_owned())?;
            if !breakpoints.iter().any(|candidate| {
                candidate.core_module == entry.core_module && candidate.module_pc == entry.module_pc
            }) {
                breakpoints.push(entry);
            }
        }
        let session = self
            .prepared
            .start_debug(
                &self.input,
                &self.limits,
                &self.cancel,
                &breakpoints,
                &self.run_id,
                self.generation_id,
            )
            .map_err(|_| "debug-launch-failed".to_owned())?;
        let mut stop = (!breakpoints.is_empty())
            .then(|| session.wait_for_stop(STOP_TIMEOUT))
            .flatten();
        if self.stop_on_entry
            && let Some(stop) = &mut stop
        {
            stop.reason = "entry";
        }
        self.stop = stop.clone();
        self.session = Some(session);
        let events = stop
            .map(|stop| vec![event("stopped", json!({"reason": stop.reason, "threadId": THREAD_ID, "allThreadsStopped": true}))])
            .unwrap_or_default();
        Ok(DapBackendReply {
            body: json!({}),
            events,
        })
    }

    fn stack_trace(&self) -> Result<DapBackendReply, String> {
        let stop = self.stop.as_ref().ok_or_else(|| "not-stopped".to_owned())?;
        let map = self
            .prepared
            .debug_map
            .as_deref()
            .ok_or_else(|| "debug-map-unavailable".to_owned())?;
        let core_base = self
            .prepared
            .debug_core_base
            .ok_or_else(|| "debug-map-unavailable".to_owned())?;
        let frames = stop
            .frames
            .iter()
            .take(MAX_STACK_FRAMES)
            .enumerate()
            .map(|(index, frame)| {
                let absolute_pc = core_base + u64::from(frame.module_pc);
                let mapping = map.mappings.iter().find(|mapping| {
                    frame
                        .core_module
                        .as_deref()
                        .is_none_or(|name| mapping.core_module == name)
                        && mapping.instruction_start <= absolute_pc
                        && absolute_pc < mapping.instruction_end
                });
                let (name, line) = mapping
                    .and_then(|mapping| {
                        mapping
                            .source
                            .as_ref()
                            .map(|source| (&mapping.function_id, source.start))
                    })
                    .map(|(name, offset)| (name.clone(), self.line_for_offset(offset)))
                    .unwrap_or_else(|| ("<generated>".to_owned(), 1));
                json!({
                    "id": index + 1,
                    "name": name,
                    "line": line,
                    "column": 1,
                    "source": {"name": self.display_uri, "documentId": self.document_id}
                })
            })
            .collect::<Vec<_>>();
        Ok(reply(
            json!({"stackFrames": frames, "totalFrames": frames.len()}),
        ))
    }

    fn line_for_offset(&self, offset: u64) -> usize {
        self.line_starts
            .partition_point(|start| *start <= offset)
            .max(1)
    }

    fn scopes(&self, arguments: &Value) -> Result<DapBackendReply, String> {
        let frame = arguments
            .get("frameId")
            .and_then(Value::as_u64)
            .filter(|frame| *frame > 0)
            .ok_or_else(|| "invalid-frame".to_owned())?;
        let stop = self.stop.as_ref().ok_or_else(|| "not-stopped".to_owned())?;
        if frame as usize > stop.frames.len().min(MAX_STACK_FRAMES) {
            return Err("invalid-frame".to_owned());
        }
        Ok(reply(json!({"scopes": [
            {"name": "Arguments", "variablesReference": frame * 10 + 1, "expensive": false, "presentationHint": "arguments"},
            {"name": "Locals", "variablesReference": frame * 10 + 2, "expensive": false, "presentationHint": "locals"},
            {"name": "Result", "variablesReference": frame * 10 + 3, "expensive": false}
        ]})))
    }

    fn variables(&self, arguments: &Value) -> Result<DapBackendReply, String> {
        let reference = arguments
            .get("variablesReference")
            .and_then(Value::as_u64)
            .ok_or_else(|| "invalid-scope".to_owned())?;
        let frame_id = reference / 10;
        let scope = reference % 10;
        let stop = self.stop.as_ref().ok_or_else(|| "not-stopped".to_owned())?;
        let frame = stop
            .frames
            .get(frame_id.saturating_sub(1) as usize)
            .ok_or_else(|| "invalid-scope".to_owned())?;
        let variables = match scope {
            1 => {
                let mut values = vec![
                    scalar("argumentCount", "i64", self.input.arguments.len().to_string()),
                    scalar("stdinBytes", "i64", self.input.stdin.len().to_string()),
                ];
                values.extend(
                    self.input
                        .arguments
                        .iter()
                        .take(MAX_VARIABLES.saturating_sub(values.len()))
                        .enumerate()
                        .map(|(index, argument)| {
                            scalar(
                                &format!("argument{index}Bytes"),
                                "i64",
                                argument.len().to_string(),
                            )
                        }),
                );
                values
            }
            2 => frame
                .locals
                .iter()
                .take(MAX_VARIABLES)
                .map(|local| json!({"name": local.name, "value": local.value, "type": local.type_name, "variablesReference": 0, "presentationHint": {"attributes": ["readOnly"], "visibility": if local.available { "public" } else { "internal" }}}))
                .collect(),
            3 => vec![unavailable("result")],
            _ => return Err("invalid-scope".to_owned()),
        };
        Ok(reply(json!({"variables": variables})))
    }

    fn continue_execution(&mut self) -> Result<DapBackendReply, String> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| "not-launched".to_owned())?;
        if self.stop.is_none() || !session.continue_execution() {
            return Err("not-stopped".to_owned());
        }
        self.stop = None;
        Ok(DapBackendReply {
            body: json!({"allThreadsContinued": true}),
            events: vec![event(
                "continued",
                json!({"threadId": THREAD_ID, "allThreadsContinued": true}),
            )],
        })
    }

    fn pause(&mut self) -> Result<DapBackendReply, String> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| "not-launched".to_owned())?;
        if self.stop.is_some() {
            return Err("already-stopped".to_owned());
        }
        session.request_pause();
        let stop = session
            .wait_for_stop(STOP_TIMEOUT)
            .ok_or_else(|| "safe-pause-unavailable".to_owned())?;
        self.stop = Some(stop);
        Ok(DapBackendReply {
            body: json!({}),
            events: vec![event(
                "stopped",
                json!({"reason": "pause", "threadId": THREAD_ID, "allThreadsStopped": true}),
            )],
        })
    }

    fn terminate(&mut self) -> Result<DapBackendReply, String> {
        let Some(session) = self.session.take() else {
            return Ok(reply(json!({})));
        };
        session.terminate();
        let observed = session
            .finish(STOP_TIMEOUT)
            .map_err(|_| "debug-termination-failed".to_owned())?;
        self.stop = None;
        Ok(DapBackendReply {
            body: json!({}),
            events: terminal_events(&observed, &self.run_id, self.generation_id)?,
        })
    }

    fn poll_terminal(&mut self) -> Result<DapBackendReply, String> {
        if !self
            .session
            .as_ref()
            .is_some_and(RuntimeDebugSession::is_finished)
        {
            return Ok(DapBackendReply::default());
        }
        let session = self
            .session
            .take()
            .ok_or_else(|| "not-launched".to_owned())?;
        let observed = session
            .finish(STOP_TIMEOUT)
            .map_err(|_| "debug-terminal-failed".to_owned())?;
        self.stop = None;
        Ok(DapBackendReply {
            body: json!({}),
            events: terminal_events(&observed, &self.run_id, self.generation_id)?,
        })
    }
}

impl DapBackend for RuntimeDapBackend {
    fn request(&mut self, command: &str, arguments: &Value) -> Result<DapBackendReply, String> {
        match command {
            "launch" => {
                if self.launched {
                    return Err("already-launched".to_owned());
                }
                if arguments
                    .get("noDebug")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    return Err("debug-launch-required".to_owned());
                }
                self.stop_on_entry = arguments
                    .get("stopOnEntry")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.launched = true;
                Ok(reply(json!({})))
            }
            "setBreakpoints" => self.set_breakpoints(arguments),
            "configurationDone" => self.configure(),
            "threads" => Ok(reply(
                json!({"threads": [{"id": THREAD_ID, "name": "Sico Script"}]}),
            )),
            "stackTrace" => self.stack_trace(),
            "scopes" => self.scopes(arguments),
            "variables" => self.variables(arguments),
            "continue" => self.continue_execution(),
            "pause" => self.pause(),
            "disconnect" | "terminate" => self.terminate(),
            _ => Err("unsupported-request".to_owned()),
        }
    }

    fn poll(&mut self) -> Result<DapBackendReply, String> {
        self.poll_terminal()
    }
}

fn reply(body: Value) -> DapBackendReply {
    DapBackendReply {
        body,
        events: Vec::new(),
    }
}

fn event(name: &str, body: Value) -> Value {
    json!({"event": name, "body": body})
}

fn unavailable(name: &str) -> Value {
    json!({"name": name, "value": "<unavailable>", "type": "unavailable", "variablesReference": 0, "presentationHint": {"attributes": ["readOnly"]}})
}

fn scalar(name: &str, type_name: &str, value: String) -> Value {
    json!({"name": name, "value": value, "type": type_name, "variablesReference": 0, "presentationHint": {"attributes": ["readOnly"]}})
}

struct DapOutputRedactor;

impl EventRedactor for DapOutputRedactor {
    fn redact(&self, bytes: &[u8]) -> Vec<u8> {
        if bytes.is_empty() {
            Vec::new()
        } else {
            b"<redacted-output>".to_vec()
        }
    }
}

fn dap_output_events(
    observed: &crate::ObservedRun,
    run_id: &str,
    generation_id: u64,
) -> Result<Vec<Value>, String> {
    let frames = crate::execution_events(observed, run_id, generation_id, &DapOutputRedactor)
        .map_err(|_| "execution-event-failed".to_owned())?;
    let mut events = Vec::new();
    for frame in frames {
        let decoded =
            parse_execution_event(&frame).map_err(|_| "execution-event-failed".to_owned())?;
        if matches!(decoded.kind.as_str(), "stdout" | "stderr") {
            events.push(event(
                "output",
                json!({
                    "category": decoded.kind,
                    "output": "<redacted-output>",
                    "data": {"runId": decoded.run_id, "generationId": decoded.generation_id, "sequence": decoded.sequence}
                }),
            ));
        }
    }
    Ok(events)
}

fn terminal_events(
    observed: &crate::ObservedRun,
    run_id: &str,
    generation_id: u64,
) -> Result<Vec<Value>, String> {
    let mut events = dap_output_events(observed, run_id, generation_id)?;
    events.extend([
        event("terminated", json!({})),
        event("exited", json!({"exitCode": observed.outcome.exit_code()})),
    ]);
    Ok(events)
}
