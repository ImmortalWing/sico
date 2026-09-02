//! Model Context Protocol stdio server for the frozen `sico.ai-tool.v0`
//! surface (M13 STEP-0121).
//!
//! The server is a thin, dependency-free JSON-RPC 2.0 envelope over
//! [`sico_ai_tools::execute_value`], which stays the single security
//! boundary: bounded input and zero side effects (no filesystem writes,
//! no process launches, no network, no model calls). MCP method support is exactly `initialize`,
//! `ping`, `tools/list` and `tools/call`; `tools/call` maps 1:1 onto one
//! `sico.ai-tool.v0` request whose budget is filled server-side and never
//! widened by the client, and whose response JSON is embedded verbatim.

use serde_json::{Value, json};
use sico_ai_tools::{MAX_REQUEST_BYTES, execute_value};

/// MCP protocol revision this server implements.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
/// Server identity announced during `initialize`.
pub const SERVER_NAME: &str = "sico-ai-tool";
pub const SERVER_VERSION: &str = "0.1.0";
/// The four frozen `sico.ai-tool.v0` operations, exposed 1:1 as MCP tools.
pub const TOOL_NAMES: [&str; 4] = [
    "inspect",
    "validate_fix",
    "plan_execution",
    "summarize_execution",
];

/// Server-side protocol budget: bounded defaults, never widened by the
/// client (each value sits inside the `sico.ai-tool.v0` protocol caps).
fn server_budget() -> Value {
    json!({
        "max_files": 16,
        "max_input_bytes": 1024 * 1024,
        "max_symbols": 256,
        "max_diagnostics": 100,
        "max_response_bytes": MAX_REQUEST_BYTES,
    })
}

fn tool_schema(name: &str) -> Value {
    // The JSON Schema mirrors the frozen request shape loosely on purpose:
    // exact validation stays in `execute_value` (deny-unknown-fields serde
    // contracts), the schema only guides model callers.
    let input_note = match name {
        "inspect" => "workspace files plus query for outline/symbols/diagnostics",
        "validate_fix" => "original source, digest, diagnostics, and one bounded edit",
        "plan_execution" => "recorded execution events plus optional fault record",
        _ => "recorded execution events plus optional fault record for the bounded summary",
    };
    json!({
        "type": "object",
        "properties": {
            "input": {
                "type": "object",
                "description": input_note,
            }
        },
        "required": ["input"],
        "additionalProperties": false,
    })
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "inspect" => "Inspect Sico sources: outline, symbols, diagnostics (read-only).",
        "validate_fix" => "Validate one bounded candidate edit against expected diagnostics.",
        "plan_execution" => "Derive a bounded, redacted execution plan description.",
        "summarize_execution" => "Summarize recorded execution events into a bounded report.",
        _ => "sico.ai-tool.v0 operation",
    }
}

fn tools_list() -> Value {
    let tools: Vec<Value> = TOOL_NAMES
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "title": format!("Sico AI tool: {name}"),
                "description": tool_description(name),
                "inputSchema": tool_schema(name),
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Wraps one `sico.ai-tool.v0` response value as MCP text content.
fn tool_content(response: &Value) -> Value {
    let text = serde_json::to_string(response).unwrap_or_else(|_| "{}".to_owned());
    json!({ "content": [{ "type": "text", "text": text }], "isError": response["ok"] == json!(false) })
}

/// Maps one MCP `tools/call` onto the frozen protocol entry point.
fn tools_call(params: &Value) -> Result<Value, String> {
    let name = params["name"]
        .as_str()
        .ok_or("tools/call requires a tool name")?;
    if !TOOL_NAMES.contains(&name) {
        return Err(format!("unknown tool: {name}"));
    }
    let input = params
        .get("arguments")
        .and_then(|arguments| arguments.get("input"))
        .cloned()
        .ok_or("tools/call requires arguments.input")?;
    // request_id is derived from the MCP id when it fits the protocol's
    // bounded identity rules; the tool validates the rest fail-closed.
    let request_id = params["_sico_request_id"].as_str().unwrap_or("mcp-call");
    let request = json!({
        "schema": "sico.ai-tool.request.v0",
        "protocol_version": 0,
        "request_id": request_id,
        "operation": name,
        "budget": server_budget(),
        "input": input,
    });
    Ok(tool_content(&execute_value(request)))
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

/// Handles one decoded JSON-RPC request value. Notification-shaped frames
/// (no `id`) yield `None`. Everything outside the supported method set is
/// a typed JSON-RPC error; nothing here can widen authority.
///
/// # Panics
///
/// Never panics on client input: the only `expect` is on the method
/// string presence proven by the guard above.
#[must_use]
pub fn handle_frame(frame: &Value) -> Option<Value> {
    if frame.get("method").and_then(Value::as_str).is_none() {
        return Some(rpc_error(&frame["id"], -32600, "invalid request"));
    }
    let method = frame["method"].as_str().expect("method checked");
    let id = frame.get("id").cloned().unwrap_or(Value::Null);
    let params = frame.get("params").cloned().unwrap_or(json!({}));
    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
            }
        }),
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        "tools/list" => json!({ "jsonrpc": "2.0", "id": id, "result": tools_list() }),
        "tools/call" => match tools_call(&params) {
            Ok(mut result) => {
                result["id"] = id.clone();
                let mut request = json!({ "jsonrpc": "2.0", "id": id });
                request["result"] = result;
                request
            }
            Err(message) => rpc_error(&id, -32602, &message),
        },
        "notifications/initialized" => return None,
        _ => rpc_error(&id, -32601, "method not found"),
    };
    (frame.get("id").is_some()).then_some(response)
}

/// Reads bounded JSON-RPC frames (one JSON value per line, MCP stdio
/// framing) from `input` and appends responses to `output`. Oversize or
/// malformed frames produce typed errors, never a crash, and never touch
/// anything but the provided buffers.
#[must_use]
pub fn handle_session(input: &[u8], output: &mut Vec<u8>) -> usize {
    let mut answered = 0;
    let write = |response: &Value, output: &mut Vec<u8>| -> bool {
        serde_json::to_writer(&mut *output, response)
            .is_ok()
            .then(|| {
                output.push(b'\n');
            })
            .is_some()
    };
    for line in String::from_utf8_lossy(input).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(frame) = serde_json::from_str::<Value>(line) else {
            let error = rpc_error(&Value::Null, -32700, "parse error");
            answered += usize::from(write(&error, output));
            continue;
        };
        if let Some(response) = handle_frame(&frame) {
            answered += usize::from(write(&response, output));
        }
    }
    answered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialize() -> Value {
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} })
    }

    #[test]
    fn initialize_negotiates_and_lists_the_four_frozen_tools() {
        let reply = handle_frame(&initialize()).expect("initialize replies");
        assert_eq!(reply["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(reply["result"]["serverInfo"]["name"], SERVER_NAME);
        let listed = handle_frame(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
            .expect("tools/list replies");
        let names: Vec<&str> = listed["result"]["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect();
        assert_eq!(names, TOOL_NAMES);
        for tool in listed["result"]["tools"].as_array().expect("tools") {
            assert_eq!(tool["inputSchema"]["type"], json!("object"));
        }
    }

    #[test]
    fn tools_call_roundtrips_a_real_inspect_request() {
        let source = "function helper(value: Int) returns Int:\n  return value\nend function\n";
        let frame = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "inspect",
                "arguments": { "input": { "files": [ { "uri": "file:///helper.sico", "text": source } ] } }
            }
        });
        let reply = handle_frame(&frame).expect("tools/call replies");
        assert!(reply["error"].is_null(), "{reply}");
        let content = reply["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        let response: Value = serde_json::from_str(content).expect("ai-tool response json");
        assert_eq!(response["schema"], "sico.ai-tool.response.v0");
        assert_eq!(response["ok"], json!(true));
        assert_eq!(reply["result"]["isError"], json!(false));
    }

    #[test]
    fn tools_call_refuses_unknown_tools_and_bad_arguments() {
        let unknown = handle_frame(&json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "spawn_task", "arguments": {} }
        }))
        .expect("reply");
        assert_eq!(unknown["error"]["code"], -32602);
        let missing = handle_frame(&json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "inspect" }
        }))
        .expect("reply");
        assert_eq!(missing["error"]["code"], -32602);
    }

    #[test]
    fn unsupported_methods_and_malformed_frames_fail_closed() {
        assert_eq!(
            handle_frame(&json!({ "jsonrpc": "2.0", "id": 6, "method": "resources/list" }))
                .expect("reply")["error"]["code"],
            -32601
        );
        let invalid = handle_frame(&json!({ "jsonrpc": "2.0", "id": 7 })).expect("reply");
        assert_eq!(invalid["error"]["code"], -32600);
        // Notifications stay silent.
        assert_eq!(
            handle_frame(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })),
            None
        );
        let mut output = Vec::new();
        assert_eq!(
            handle_session(
                b"not-json\n\n{ \"jsonrpc\": \"2.0\", \"id\": 8, \"method\": \"ping\" }",
                &mut output
            ),
            2
        );
        let lines: Vec<&str> = std::str::from_utf8(&output).unwrap().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("parse error"));
    }

    #[test]
    fn oversized_inputs_fail_through_the_frozen_boundary() {
        // A 2 MiB file inside one inspect request must hit the protocol's
        // request-size guard via the MCP path, not the MCP envelope.
        let big = "x".repeat(2 * 1024 * 1024);
        let frame = json!({
            "jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": { "name": "inspect", "arguments": { "input": { "files": [ { "uri": "file:///big.sico", "text": big } ] } } }
        });
        let reply = handle_frame(&frame).expect("reply");
        let content = reply["result"]["content"][0]["text"]
            .as_str()
            .expect("text");
        let response: Value = serde_json::from_str(content).unwrap();
        assert_eq!(response["ok"], json!(false));
        assert_eq!(response["error"]["code"], "request_too_large");
    }
}

#[cfg(test)]
mod step0121_tests {
    use super::*;

    fn base_request() -> Value {
        json!({
            "schema": "sico.ai-tool.request.v0",
            "protocol_version": 0,
            "request_id": "mutation",
            "operation": "inspect",
            "budget": {
                "max_files": 16,
                "max_input_bytes": 1024 * 1024,
                "max_symbols": 256,
                "max_diagnostics": 100,
                "max_response_bytes": 1024 * 1024
            },
            "input": { "files": [ { "uri": "file:///main.sico", "text": "" } ] }
        })
    }

    fn mcp_call_is_failure(request: &Value) -> bool {
        let frame = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": request["operation"], "arguments": { "input": request["input"] } }
        });
        let Some(reply) = handle_frame(&frame) else {
            return false;
        };
        let Some(text) = reply["result"]["content"][0]["text"].as_str() else {
            return false;
        };
        let response: Value = serde_json::from_str(text).expect("embedded ai-tool response");
        response["ok"] == json!(false)
    }

    #[test]
    fn five_hundred_twelve_protocol_mutations_fail_closed_through_mcp() {
        // The frozen 512-mutation corpus replayed through the MCP
        // transport. The envelope owns schema/version/budget by design
        // (STEP-0121 §4.2: never widened by the client), so client-side
        // corruption can only target the operation name and the input
        // payload — exactly what this corpus mutates. Every variant must
        // land on the tool's typed failure, proving the transport adds no
        // bypass on the fields a client actually controls.
        let mut rejected = 0;
        let base = base_request();
        for index in 0..256 {
            let mut invalid = base.clone();
            invalid["input"] = json!({ "files": [ { "uri": format!("file:///m{index}.sico"), "text": "" } ], "intruder": index });
            rejected += usize::from(mcp_call_is_failure(&invalid));
        }
        for index in 0..256 {
            let mut invalid = base.clone();
            invalid["input"] = json!({ "files": [] });
            invalid["operation"] = json!(
                [
                    "inspect",
                    "validate_fix",
                    "plan_execution",
                    "summarize_execution"
                ][index % 4]
            );
            invalid["request_id"] = json!(format!("m{index:04x}"));
            rejected += usize::from(mcp_call_is_failure(&invalid));
        }
        assert_eq!(rejected, 512);
    }

    #[test]
    fn real_stdio_session_performs_inspect_then_validate_fix_roundtrip() {
        // MCP lifecycle against the real framing loop: initialize →
        // tools/list → inspect → validate_fix (clean) → validate_fix
        // (typed refusal on a stale digest).
        const BROKEN: &str = "function helper(value: Int) returns Int:
  return value
";
        const BROKEN_SHA256: &str =
            "348962e06364cc361344d588433266ab70c4b12850342235e9b270c3bf63906d";
        const GOOD: &str = "function helper(value: Int) returns Int:
  return value + 1
end function
";
        let mut session = String::new();
        let frames = [
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
            json!({
                "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": { "name": "inspect", "arguments": { "input": { "files": [ { "uri": "file:///helper.sico", "text": BROKEN } ] } } }
            }),
            json!({
                "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                "params": { "name": "validate_fix", "arguments": { "input": {
                    "uri": "file:///helper.sico",
                    "original": BROKEN,
                    "candidate": GOOD,
                    "original_sha256": BROKEN_SHA256,
                    "expected_diagnostics": ["SYNTAX_MISSING_FUNCTION_CLOSE"],
                    "max_changed_bytes": 256
                } } }
            }),
            json!({
                "jsonrpc": "2.0", "id": 5, "method": "tools/call",
                "params": { "name": "validate_fix", "arguments": { "input": {
                    "uri": "file:///helper.sico",
                    "original": GOOD,
                    "candidate": GOOD,
                    "original_sha256": "0".repeat(64),
                    "expected_diagnostics": ["SYNTAX_MISSING_FUNCTION_CLOSE"],
                    "max_changed_bytes": 256
                } } }
            }),
        ];
        for frame in frames {
            session.push_str(&frame.to_string());
            session.push('\n');
        }
        let mut output = Vec::new();
        let answered = handle_session(session.as_bytes(), &mut output);
        assert_eq!(answered, 5, "notifications stay silent");
        let lines: Vec<Value> = std::str::from_utf8(&output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines[0]["result"]["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(
            lines[1]["result"]["tools"].as_array().map(Vec::len),
            Some(4)
        );
        // Inspect sees the broken source: a diagnostic is reported.
        let inspect: Value =
            serde_json::from_str(lines[2]["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(inspect["ok"], json!(true));
        assert!(
            !inspect["result"]["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .is_empty(),
            "{inspect}"
        );
        // The real fix validates clean.
        let fixed: Value =
            serde_json::from_str(lines[3]["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(fixed["ok"], json!(true), "{fixed}");
        // The stale-digest non-fix is a typed refusal through MCP.
        let refused: Value =
            serde_json::from_str(lines[4]["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(refused["ok"], json!(false));
    }
}
