use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
    Parameter, SourceRange, Terminator, Type, ValueId,
};
use sico_observability::{canonical_json, parse_runtime_fault};
use sico_runner::scheduler::{
    CompletionRecord, MAX_COMPLETION_BYTES, MAX_COMPLETION_RECORDS, MAX_LIVE_TASKS,
    MAX_READY_QUEUE, MAX_SCOPE_CHILDREN, MAX_SCOPE_DEPTH, OperationId, ReadinessClass, RecvOutcome,
    RunIdentity, SchedulerCore, SchedulerFault, ScopeId, SendOutcome, TaskId, TerminalKind,
};
use sico_runner::{
    CancelToken, FsGrants, ObservedRun, RunOutcome, Runner, RunnerLimits, ScriptInput, ScriptOutput,
};
use sico_tooling_protocol::{DapSession, decode_dap_frame, encode_dap_frame};

fn runner() -> Runner {
    Runner::new().unwrap()
}

fn no_cancel() -> CancelToken {
    CancelToken::new()
}

fn limits() -> RunnerLimits {
    RunnerLimits::default()
}

fn run(component: &[u8], input: &ScriptInput) -> RunOutcome {
    runner()
        .run_program(
            component,
            input,
            &limits(),
            &no_cancel(),
            &FsGrants::default(),
        )
        .unwrap()
}

#[test]
fn echo_roundtrips_and_exact_guest_exit_values_pass_through() {
    let echo = echo_component(0);
    for input in [
        ScriptInput::default(),
        ScriptInput {
            arguments: vec!["unicode-雪".into(), "emoji-\u{1f600}".into()],
            stdin: (0..1024_u32)
                .map(|index| [0, 1, 0xff, 0xfe, b'x'][(index % 5) as usize])
                .collect(),
        },
        ScriptInput {
            arguments: Vec::new(),
            stdin: vec![7; 1024 * 1024],
        },
    ] {
        let outcome = run(&echo, &input);
        assert_eq!(
            outcome,
            RunOutcome::Output(ScriptOutput {
                stdout: input.stdin.clone(),
                stderr: input.stdin.clone(),
                exit_code: 0,
            })
        );
    }

    let exit42 = echo_component(42);
    assert_eq!(
        run(&exit42, &ScriptInput::default()),
        RunOutcome::Output(ScriptOutput {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: 42,
        })
    );
    assert_eq!(run(&exit42, &ScriptInput::default()).exit_code(), 42);
}

#[test]
fn out_of_range_guest_exit_is_rejected_not_truncated() {
    let outcome = run(&echo_component(120), &ScriptInput::default());
    assert!(matches!(outcome, RunOutcome::Trap(_)), "{outcome:?}");
    assert_eq!(outcome.exit_code(), 125);
}

#[test]
fn domain_error_maps_to_122_with_code_and_message() {
    let outcome = run(&error_component(), &ScriptInput::default());
    assert_eq!(
        outcome,
        RunOutcome::Domain {
            code: "domain-error".to_owned(),
            message: "rejected".to_owned(),
        }
    );
    assert_eq!(outcome.exit_code(), 122);
}

#[test]
fn cancelled_script_error_maps_to_control_exit_123() {
    let outcome = run(&cancelled_component(), &ScriptInput::default());
    assert_eq!(outcome, RunOutcome::Cancelled);
    assert_eq!(outcome.exit_code(), 123);
}

#[test]
fn trap_fuel_memory_and_malformed_guests_fail_closed_and_host_survives() {
    let runner = runner();
    let no_cancel = no_cancel();
    let limits = limits();

    // Guest trap: unreachable.
    let outcome = runner
        .run_program(
            &trap_component(),
            &ScriptInput::default(),
            &limits,
            &no_cancel,
            &FsGrants::default(),
        )
        .unwrap();
    assert!(matches!(outcome, RunOutcome::Trap(_)), "{outcome:?}");

    // Host survives the trap and runs a normal guest.
    assert!(matches!(
        runner
            .run_program(
                &echo_component(0),
                &ScriptInput::default(),
                &limits,
                &no_cancel,
                &FsGrants::default()
            )
            .unwrap(),
        RunOutcome::Output(_)
    ));

    // Non-termination: deterministic fuel exhaustion.
    let tight = RunnerLimits {
        fuel: 1_000,
        ..limits.clone()
    };
    let outcome = runner
        .run_program(
            &spin_component(),
            &ScriptInput::default(),
            &tight,
            &no_cancel,
            &FsGrants::default(),
        )
        .unwrap();
    assert_eq!(outcome, RunOutcome::FuelExhausted);
    assert_eq!(outcome.exit_code(), 125);

    // Timeout: tiny deadline, effectively unbounded fuel.
    let instant = RunnerLimits {
        fuel: u64::MAX,
        timeout: std::time::Duration::from_millis(30),
        ..limits.clone()
    };
    let outcome = runner
        .run_program(
            &spin_component(),
            &ScriptInput::default(),
            &instant,
            &no_cancel,
            &FsGrants::default(),
        )
        .unwrap();
    assert_eq!(outcome, RunOutcome::Timeout);
    assert_eq!(outcome.exit_code(), 124);

    // Memory ceiling below the program's declared minimum.
    let tiny = RunnerLimits {
        memory_bytes: 8 * 1024 * 1024,
        ..limits.clone()
    };
    let outcome = runner
        .run_program(
            &echo_component(0),
            &ScriptInput::default(),
            &tiny,
            &no_cancel,
            &FsGrants::default(),
        )
        .unwrap();
    assert!(
        matches!(
            outcome,
            RunOutcome::MemoryLimit | RunOutcome::Launch(_) | RunOutcome::Incompatible(_)
        ),
        "{outcome:?}"
    );

    // Malformed result from a malicious core guest.
    let outcome = runner
        .run_program(
            &malformed_component(),
            &ScriptInput::default(),
            &limits,
            &no_cancel,
            &FsGrants::default(),
        )
        .unwrap();
    assert!(matches!(outcome, RunOutcome::Trap(_)), "{outcome:?}");

    // Host is still healthy after every malicious case.
    assert!(matches!(
        runner
            .run_program(
                &echo_component(0),
                &ScriptInput::default(),
                &limits,
                &no_cancel,
                &FsGrants::default()
            )
            .unwrap(),
        RunOutcome::Output(_)
    ));
}

#[test]
fn observed_trap_uses_verified_exact_source_frames() {
    let artifact = trap_debug_artifact();
    let runner = runner();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let observed = prepared
        .run_observed(
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            "run-observed-1",
            7,
        )
        .unwrap();
    assert!(matches!(observed.outcome, RunOutcome::Trap(_)));
    let fault = observed.fault.unwrap();
    assert_eq!(fault.class, "trap");
    assert_eq!(fault.generation_id, 7);
    assert!(!fault.frames.is_empty());
    assert!(fault.frames.iter().any(|frame| {
        frame.source.as_ref().is_some_and(|source| {
            source.document_id == "doc.trap" && source.start == 10 && source.end == 20
        })
    }));
    let bytes = canonical_json(&fault).unwrap();
    assert_eq!(parse_runtime_fault(&bytes).unwrap(), fault);

    let mut stale_map = artifact.debug_map.clone();
    stale_map.push(b' ');
    assert!(matches!(
        runner.prepare_program_with_debug(
            &artifact.component,
            &stale_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        ),
        Err(RunOutcome::Incompatible(_))
    ));
}

#[test]
fn source_breakpoint_stops_real_debug_store_then_continues_to_one_terminal() {
    let artifact = trap_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let bindings = prepared.bind_source_breakpoints("doc.trap", &[10]);
    assert_eq!(bindings.len(), 1);
    let breakpoint = bindings[0]
        .breakpoint
        .clone()
        .expect("the exact source row must bind to a guest module PC");
    assert_eq!(breakpoint.source_offset, 10);

    let session = prepared
        .start_debug(
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            &[breakpoint],
            "debug-source-breakpoint",
            9,
        )
        .unwrap();
    let stop = session
        .wait_for_stop(std::time::Duration::from_secs(5))
        .expect("the Wasmtime Store must stop at the installed source breakpoint");
    assert_eq!(stop.reason, "breakpoint");
    assert!(!stop.frames.is_empty());
    assert!(
        stop.frames
            .iter()
            .flat_map(|frame| &frame.locals)
            .any(|local| local.available
                && matches!(local.type_name, "i32" | "i64" | "f32" | "f64" | "v128")),
        "the stopped Wasmtime frame must expose at least one bounded scalar local"
    );
    assert!(session.continue_execution());

    let observed = session.finish(std::time::Duration::from_secs(5)).unwrap();
    assert!(matches!(observed.outcome, RunOutcome::Trap(_)));
    let fault = observed.fault.expect("trap has one strict terminal fault");
    assert_eq!(fault.run_id, "debug-source-breakpoint");
    assert_eq!(fault.generation_id, 9);
    assert!(fault.frames.iter().any(|frame| {
        frame.source.as_ref().is_some_and(|source| {
            source.document_id == "doc.trap" && source.start == 10 && source.end == 20
        })
    }));
}

#[test]
fn exact_dap_session_drives_real_component_stack_scopes_and_terminal() {
    let artifact = trap_debug_artifact();
    let source = trap_debug_source();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let backend = sico_runner::RuntimeDapBackend::new(
        prepared,
        sico_runner::RuntimeDapConfig {
            document_id: "doc.trap",
            display_uri: "workspace://trap.sico",
            source: source.to_vec(),
            input: ScriptInput::default(),
            limits: limits(),
            cancel: no_cancel(),
            run_id: "dap-real-component",
            generation_id: 11,
        },
    )
    .unwrap();
    let mut dap = DapSession::new(backend);
    let request = |seq, command: &str, arguments: serde_json::Value| serde_json::json!({"seq": seq, "type": "request", "command": command, "arguments": arguments});

    let initialized = dap
        .handle(&request(1, "initialize", serde_json::json!({})))
        .unwrap();
    assert_eq!(initialized[1]["event"], "initialized");
    let breakpoints = dap
        .handle(&request(
            2,
            "setBreakpoints",
            serde_json::json!({
                "source": {"documentId": "doc.trap"},
                "breakpoints": [{"line": 2}]
            }),
        ))
        .unwrap();
    assert_eq!(breakpoints[0]["body"]["breakpoints"][0]["verified"], true);
    assert!(
        dap.handle(&request(3, "launch", serde_json::json!({})))
            .unwrap()[0]["success"]
            .as_bool()
            .unwrap()
    );
    let configured = dap
        .handle(&request(4, "configurationDone", serde_json::json!({})))
        .unwrap();
    assert_eq!(configured[1]["event"], "stopped");
    assert_eq!(configured[1]["body"]["reason"], "breakpoint");

    let threads = dap
        .handle(&request(5, "threads", serde_json::json!({})))
        .unwrap();
    assert_eq!(threads[0]["body"]["threads"][0]["id"], 1);
    let stack = dap
        .handle(&request(
            6,
            "stackTrace",
            serde_json::json!({"threadId": 1}),
        ))
        .unwrap();
    assert!(stack[0]["body"]["totalFrames"].as_u64().unwrap() > 0);
    assert_eq!(stack[0]["body"]["stackFrames"][0]["line"], 2);
    let scopes = dap
        .handle(&request(7, "scopes", serde_json::json!({"frameId": 1})))
        .unwrap();
    let locals_reference = scopes[0]["body"]["scopes"][1]["variablesReference"]
        .as_u64()
        .unwrap();
    let arguments_reference = scopes[0]["body"]["scopes"][0]["variablesReference"]
        .as_u64()
        .unwrap();
    let arguments = dap
        .handle(&request(
            8,
            "variables",
            serde_json::json!({"variablesReference": arguments_reference}),
        ))
        .unwrap();
    assert_eq!(
        arguments[0]["body"]["variables"][0]["name"],
        "argumentCount"
    );
    assert_eq!(arguments[0]["body"]["variables"][0]["type"], "i64");
    let variables = dap
        .handle(&request(
            9,
            "variables",
            serde_json::json!({"variablesReference": locals_reference}),
        ))
        .unwrap();
    let locals = variables[0]["body"]["variables"].as_array().unwrap();
    assert!(locals.iter().any(|local| {
        local["value"] != "<unavailable>"
            && matches!(
                local["type"].as_str(),
                Some("i32" | "i64" | "f32" | "f64" | "v128")
            )
    }));

    let continued = dap
        .handle(&request(10, "continue", serde_json::json!({"threadId": 1})))
        .unwrap();
    assert_eq!(continued[1]["event"], "continued");
    let terminated = dap
        .handle(&request(11, "terminate", serde_json::json!({})))
        .unwrap();
    assert_eq!(terminated[1]["event"], "terminated");
    assert_eq!(terminated[2]["event"], "exited");
    assert_eq!(terminated[2]["body"]["exitCode"], 125);
}

#[test]
fn dap_stop_on_entry_is_a_real_breakpoint_and_disconnect_owns_teardown() {
    let artifact = trap_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let backend = sico_runner::RuntimeDapBackend::new(
        prepared,
        sico_runner::RuntimeDapConfig {
            document_id: "doc.trap",
            display_uri: "workspace://trap.sico",
            source: trap_debug_source().to_vec(),
            input: ScriptInput::default(),
            limits: limits(),
            cancel: no_cancel(),
            run_id: "dap-entry",
            generation_id: 12,
        },
    )
    .unwrap();
    let mut dap = DapSession::new(backend);
    let request = |seq, command: &str, arguments: serde_json::Value| serde_json::json!({"seq": seq, "type": "request", "command": command, "arguments": arguments});
    dap.handle(&request(1, "initialize", serde_json::json!({})))
        .unwrap();
    dap.handle(&request(
        2,
        "launch",
        serde_json::json!({"stopOnEntry": true}),
    ))
    .unwrap();
    let configured = dap
        .handle(&request(3, "configurationDone", serde_json::json!({})))
        .unwrap();
    assert_eq!(configured[1]["event"], "stopped");
    assert_eq!(configured[1]["body"]["reason"], "entry");
    let disconnected = dap
        .handle(&request(4, "disconnect", serde_json::json!({})))
        .unwrap();
    assert_eq!(disconnected[1]["event"], "terminated");
    assert_eq!(disconnected[2]["event"], "exited");
    assert_eq!(disconnected[2]["body"]["exitCode"], 123);
}

#[test]
fn dap_pause_stops_a_busy_real_component_at_a_safe_epoch() {
    let artifact = spin_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let backend = sico_runner::RuntimeDapBackend::new(
        prepared,
        sico_runner::RuntimeDapConfig {
            document_id: "doc.spin",
            display_uri: "workspace://spin.sico",
            source: vec![b' '; 64],
            input: ScriptInput::default(),
            limits: RunnerLimits {
                fuel: u64::MAX,
                timeout: std::time::Duration::from_secs(10),
                ..limits()
            },
            cancel: no_cancel(),
            run_id: "dap-pause",
            generation_id: 13,
        },
    )
    .unwrap();
    let mut dap = DapSession::new(backend);
    let request = |seq, command: &str| serde_json::json!({"seq": seq, "type": "request", "command": command, "arguments": {}});
    dap.handle(&request(1, "initialize")).unwrap();
    dap.handle(&request(2, "launch")).unwrap();
    let configured = dap.handle(&request(3, "configurationDone")).unwrap();
    assert_eq!(
        configured.len(),
        1,
        "no breakpoint means no fabricated stop"
    );
    let paused = dap.handle(&request(4, "pause")).unwrap();
    assert_eq!(paused[1]["event"], "stopped");
    assert_eq!(paused[1]["body"]["reason"], "pause");
    let terminated = dap.handle(&request(5, "terminate")).unwrap();
    assert_eq!(terminated[1]["event"], "terminated");
    assert_eq!(terminated[2]["body"]["exitCode"], 123);
}

#[test]
fn dap_output_is_bounded_redacted_and_derived_before_terminal_events() {
    let artifact = echo_debug_artifact(0);
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let backend = sico_runner::RuntimeDapBackend::new(
        prepared,
        sico_runner::RuntimeDapConfig {
            document_id: "doc.echo",
            display_uri: "workspace://echo.sico",
            source: vec![b' '; 64],
            input: ScriptInput {
                arguments: Vec::new(),
                stdin: b"must-not-leak".to_vec(),
            },
            limits: limits(),
            cancel: no_cancel(),
            run_id: "dap-output",
            generation_id: 14,
        },
    )
    .unwrap();
    let mut dap = DapSession::new(backend);
    let request = |seq, command: &str| serde_json::json!({"seq": seq, "type": "request", "command": command, "arguments": {}});
    dap.handle(&request(1, "initialize")).unwrap();
    dap.handle(&request(2, "launch")).unwrap();
    dap.handle(&request(3, "configurationDone")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let messages = dap.poll().unwrap();
    let output = messages
        .iter()
        .filter(|message| message["event"] == "output")
        .collect::<Vec<_>>();
    assert_eq!(output.len(), 2);
    assert!(
        output
            .iter()
            .all(|message| message["body"]["output"] == "<redacted-output>")
    );
    assert!(
        !serde_json::to_string(&messages)
            .unwrap()
            .contains("must-not-leak")
    );
    assert_eq!(messages[messages.len() - 2]["event"], "terminated");
    assert_eq!(messages[messages.len() - 1]["event"], "exited");
    assert_eq!(messages[messages.len() - 1]["body"]["exitCode"], 0);
}

#[test]
fn dap_stdio_server_uses_exact_bounded_frames_against_a_real_component() {
    let artifact = trap_debug_artifact();
    let unique = format!(
        "sico-dap-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir(&directory).unwrap();
    let component = directory.join("trap.component.wasm");
    let map = directory.join("trap.debug-map.json");
    let identity = directory.join("trap.debug-identity.json");
    let source = directory.join("trap.sico");
    std::fs::write(&component, &artifact.component).unwrap();
    std::fs::write(&map, &artifact.debug_map).unwrap();
    std::fs::write(&identity, &artifact.identity).unwrap();
    std::fs::write(&source, trap_debug_source()).unwrap();

    let requests = [
        serde_json::json!({"seq": 1, "type": "request", "command": "initialize", "arguments": {}}),
        serde_json::json!({"seq": 2, "type": "request", "command": "setBreakpoints", "arguments": {"source": {"documentId": "doc.trap"}, "breakpoints": [{"line": 2}]}}),
        serde_json::json!({"seq": 3, "type": "request", "command": "launch", "arguments": {}}),
        serde_json::json!({"seq": 4, "type": "request", "command": "configurationDone", "arguments": {}}),
        serde_json::json!({"seq": 5, "type": "request", "command": "continue", "arguments": {"threadId": 1}}),
        serde_json::json!({"seq": 6, "type": "request", "command": "terminate", "arguments": {}}),
    ];
    let mut input = Vec::new();
    for request in requests {
        input.extend_from_slice(&encode_dap_frame(&request).unwrap());
    }
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_sico-dap"))
        .arg(&component)
        .arg(&map)
        .arg(&identity)
        .arg(&source)
        .arg("doc.trap")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write as _;
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let messages = split_dap_frames(&output.stdout);
    assert!(messages.iter().any(|message| message["event"] == "stopped"));
    assert!(
        messages
            .iter()
            .any(|message| message["event"] == "continued")
    );
    assert!(
        messages
            .iter()
            .any(|message| message["event"] == "terminated")
    );
    assert!(messages.iter().any(|message| message["event"] == "exited"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn dap_nested_call_stack_is_source_mapped_in_innermost_first_order() {
    let artifact = nested_trap_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let backend = sico_runner::RuntimeDapBackend::new(
        prepared,
        sico_runner::RuntimeDapConfig {
            document_id: "doc.nested",
            display_uri: "workspace://nested.sico",
            source: nested_debug_source().to_vec(),
            input: ScriptInput::default(),
            limits: limits(),
            cancel: no_cancel(),
            run_id: "dap-nested",
            generation_id: 15,
        },
    )
    .unwrap();
    let mut dap = DapSession::new(backend);
    let request = |seq, command: &str, arguments: serde_json::Value| serde_json::json!({"seq": seq, "type": "request", "command": command, "arguments": arguments});
    dap.handle(&request(1, "initialize", serde_json::json!({})))
        .unwrap();
    dap.handle(&request(
        2,
        "setBreakpoints",
        serde_json::json!({"source": {"documentId": "doc.nested"}, "breakpoints": [{"line": 3}]}),
    ))
    .unwrap();
    dap.handle(&request(3, "launch", serde_json::json!({})))
        .unwrap();
    dap.handle(&request(4, "configurationDone", serde_json::json!({})))
        .unwrap();
    let stack = dap
        .handle(&request(
            5,
            "stackTrace",
            serde_json::json!({"threadId": 1}),
        ))
        .unwrap();
    let frames = stack[0]["body"]["stackFrames"].as_array().unwrap();
    assert!(
        frames.len() >= 2,
        "nested guest call must produce at least two frames: {frames:?}"
    );
    assert_eq!(frames[0]["line"], 3, "callee frame must be innermost");
    assert!(
        frames.iter().skip(1).any(|frame| frame["line"] == 2),
        "caller frame must follow the callee: {frames:?}"
    );
    dap.handle(&request(6, "terminate", serde_json::json!({})))
        .unwrap();
}

#[test]
fn dap_source_identity_and_breakpoint_modes_fail_closed_with_typed_codes() {
    let artifact = trap_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepare = || {
        runner
            .prepare_program_with_debug(
                &artifact.component,
                &artifact.debug_map,
                &artifact.identity,
                &FsGrants::default(),
                &sico_runner::NetGrants::default(),
            )
            .unwrap()
    };
    assert_eq!(
        sico_runner::RuntimeDapBackend::new(
            prepare(),
            sico_runner::RuntimeDapConfig {
                document_id: "doc.trap",
                display_uri: "workspace://trap.sico",
                source: vec![b'x'; 64],
                input: ScriptInput::default(),
                limits: limits(),
                cancel: no_cancel(),
                run_id: "dap-stale-source",
                generation_id: 16,
            },
        )
        .err(),
        Some("source-identity-mismatch".to_owned())
    );

    let backend = sico_runner::RuntimeDapBackend::new(
        prepare(),
        sico_runner::RuntimeDapConfig {
            document_id: "doc.trap",
            display_uri: "workspace://trap.sico",
            source: trap_debug_source().to_vec(),
            input: ScriptInput::default(),
            limits: limits(),
            cancel: no_cancel(),
            run_id: "dap-negative",
            generation_id: 17,
        },
    )
    .unwrap();
    let mut dap = DapSession::new(backend);
    let request = |seq, breakpoints: serde_json::Value| serde_json::json!({"seq": seq, "type": "request", "command": "setBreakpoints", "arguments": {"source": {"documentId": "doc.trap"}, "breakpoints": breakpoints}});
    dap.handle(
        &serde_json::json!({"seq": 1, "type": "request", "command": "initialize", "arguments": {}}),
    )
    .unwrap();
    let conditional = dap
        .handle(&request(
            2,
            serde_json::json!([{"line": 2, "condition": "true"}]),
        ))
        .unwrap();
    assert_eq!(conditional[0]["success"], false);
    assert_eq!(
        conditional[0]["body"]["code"],
        "unsupported-breakpoint-mode"
    );
    let invalid_line = dap
        .handle(&request(3, serde_json::json!([{"line": 999}])))
        .unwrap();
    assert_eq!(invalid_line[0]["success"], false);
    assert_eq!(invalid_line[0]["body"]["code"], "invalid-source-line");
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn dap_hundred_sequential_sessions_have_bounded_rss_handles_and_no_poisoning() {
    let artifact = trap_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let breakpoint = prepared.bind_source_breakpoints("doc.trap", &[10])[0]
        .breakpoint
        .clone()
        .unwrap();
    let run_one = |generation_id| {
        let session = prepared
            .start_debug(
                &ScriptInput::default(),
                &limits(),
                &no_cancel(),
                std::slice::from_ref(&breakpoint),
                &format!("dap-repeat-{generation_id}"),
                generation_id,
            )
            .unwrap();
        assert_eq!(
            session
                .wait_for_stop(std::time::Duration::from_secs(2))
                .unwrap()
                .reason,
            "breakpoint"
        );
        assert!(session.continue_execution());
        assert!(matches!(
            session
                .finish(std::time::Duration::from_secs(2))
                .unwrap()
                .outcome,
            RunOutcome::Trap(_)
        ));
    };
    run_one(1);
    let (baseline_handles, baseline_rss) = process_metrics();
    for generation_id in 2..=101 {
        run_one(generation_id);
    }
    let (final_handles, final_rss) = process_metrics();
    println!(
        "DAP_REPEAT_100 baseline_handles={baseline_handles} final_handles={final_handles} baseline_rss={baseline_rss} final_rss={final_rss}"
    );
    assert!(
        final_handles <= baseline_handles + 8,
        "handle growth: {baseline_handles} -> {final_handles}"
    );
    assert!(
        final_rss <= baseline_rss + 64 * 1024 * 1024,
        "RSS growth: {baseline_rss} -> {final_rss}"
    );
}

#[test]
fn dap_pause_continue_latency_is_measured_separately_from_launch() {
    let artifact = spin_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let debug_limits = RunnerLimits {
        fuel: u64::MAX,
        timeout: std::time::Duration::from_secs(10),
        ..limits()
    };
    let mut pause_samples = Vec::new();
    let mut continue_samples = Vec::new();
    for generation_id in 1..=20 {
        let session = prepared
            .start_debug(
                &ScriptInput::default(),
                &debug_limits,
                &no_cancel(),
                &[],
                &format!("dap-latency-{generation_id}"),
                generation_id,
            )
            .unwrap();
        let pause_started = std::time::Instant::now();
        session.request_pause();
        assert_eq!(
            session
                .wait_for_stop(std::time::Duration::from_secs(2))
                .unwrap()
                .reason,
            "pause"
        );
        pause_samples.push(pause_started.elapsed());
        let continue_started = std::time::Instant::now();
        assert!(session.continue_execution());
        continue_samples.push(continue_started.elapsed());
        session.terminate();
        assert_eq!(
            session
                .finish(std::time::Duration::from_secs(2))
                .unwrap()
                .outcome,
            RunOutcome::Cancelled
        );
    }
    pause_samples.sort_unstable();
    continue_samples.sort_unstable();
    println!(
        "DAP_LATENCY pause_median_us={} pause_p95_us={} continue_median_us={} continue_p95_us={}",
        pause_samples[10].as_micros(),
        pause_samples[18].as_micros(),
        continue_samples[10].as_micros(),
        continue_samples[18].as_micros()
    );
}

#[test]
fn dap_debug_build_overhead_is_measured_against_same_ir() {
    let mut normal = Vec::new();
    let mut debug = Vec::new();
    let source = [b' '; 64];
    let compiler_sha256 = "f".repeat(64);
    for _ in 0..20 {
        let mut module = echo_module(0);
        module.source_len = 64;
        let started = std::time::Instant::now();
        let component = sico_codegen_wasm::compile_script_program(&module).unwrap();
        normal.push(started.elapsed());
        let started = std::time::Instant::now();
        let artifact = sico_codegen_wasm::compile_script_program_with_debug(
            &module,
            &sico_codegen_wasm::DebugBuildInput {
                document_id: "doc.build-overhead",
                source_bytes: &source,
                display_uri: Some("workspace://build-overhead.sico"),
                compiler_package: "sico-compiler",
                compiler_version: "0.0.2-dev",
                compiler_executable_sha256: &compiler_sha256,
                adapter_identities: vec!["sico:script-adapter@0.1.0".into()],
                wit_identities: vec!["sico:script@0.1.0".into()],
            },
        )
        .unwrap();
        debug.push(started.elapsed());
        assert!(artifact.component.len() > component.len());
    }
    normal.sort_unstable();
    debug.sort_unstable();
    println!(
        "DAP_BUILD normal_median_us={} debug_median_us={} normal_p95_us={} debug_p95_us={}",
        normal[10].as_micros(),
        debug[10].as_micros(),
        normal[18].as_micros(),
        debug[18].as_micros()
    );
}

#[cfg(windows)]
fn process_metrics() -> (u64, u64) {
    let script = format!(
        "$p=Get-Process -Id {}; Write-Output \"$($p.HandleCount),$($p.WorkingSet64)\"",
        std::process::id()
    );
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let (handles, rss) = text.trim().split_once(',').unwrap();
    (handles.parse().unwrap(), rss.parse().unwrap())
}

/// Linux counterpart (STEP-0109): open fd count plus VmRSS from /proc.
#[cfg(target_os = "linux")]
fn process_metrics() -> (u64, u64) {
    let fds = std::fs::read_dir("/proc/self/fd").unwrap().count() as u64;
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let rss_kb: u64 = status
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .unwrap();
    (fds, rss_kb * 1024)
}

fn split_dap_frames(mut bytes: &[u8]) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let header = std::str::from_utf8(&bytes[..header_end]).unwrap();
        let length = header
            .split("\r\n")
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let end = header_end + 4 + length;
        messages.push(decode_dap_frame(&bytes[..end]).unwrap());
        bytes = &bytes[end..];
    }
    messages
}

#[test]
fn observed_outcomes_have_stable_classes_and_unmapped_frames_are_explicit() {
    let runner = runner();
    let prepared = runner
        .prepare_program_with_net(
            &trap_component(),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let observed = prepared
        .run_observed(
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            "run-no-map",
            1,
        )
        .unwrap();
    let fault = observed.fault.unwrap();
    assert_eq!(fault.class, "trap");
    assert!(!fault.frames.is_empty());
    assert!(fault.frames.iter().all(|frame| {
        frame.source.is_none()
            && frame.unavailable_reason.as_deref() == Some("debug-map-unavailable")
    }));

    let cases = [
        (
            RunOutcome::Domain {
                code: "domain-error".into(),
                message: "not exposed".into(),
            },
            "domain-error",
        ),
        (RunOutcome::Cancelled, "cancelled"),
        (RunOutcome::Timeout, "timeout"),
        (RunOutcome::FuelExhausted, "resource-limit.fuel"),
        (RunOutcome::MemoryLimit, "resource-limit.memory"),
        (
            RunOutcome::HostProviderFailure {
                provider_id: "sico.streams.stdout".into(),
            },
            "host-provider-failure",
        ),
        (RunOutcome::Trap("engine prose".into()), "trap"),
        (RunOutcome::Launch("engine prose".into()), "launch-failure"),
        (
            RunOutcome::Incompatible("engine prose".into()),
            "incompatible-artifact",
        ),
    ];
    for (outcome, expected_class) in cases {
        let observed = ObservedRun::from_outcome(outcome, "run-class-matrix", 2).unwrap();
        let fault = observed.fault.unwrap();
        assert_eq!(fault.class, expected_class);
        assert!(!fault.message.contains("engine prose"));
        if fault.class == "host-provider-failure" {
            assert_eq!(fault.provider_id.as_deref(), Some("sico.streams.stdout"));
        }
        parse_runtime_fault(&canonical_json(&fault).unwrap()).unwrap();
    }
}

#[test]
fn runner_cli_renders_verified_fault_json_text_and_stale_refusal() {
    let artifact = trap_debug_artifact();
    let unique = format!(
        "sico-step0097-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir(&directory).unwrap();
    let component = directory.join("trap.component.wasm");
    let map = directory.join("trap.debug-map.json");
    let identity = directory.join("trap.debug-identity.json");
    std::fs::write(&component, &artifact.component).unwrap();
    std::fs::write(&map, &artifact.debug_map).unwrap();
    std::fs::write(&identity, &artifact.identity).unwrap();

    let run = |json: bool| {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_sico-runner"));
        if json {
            command.arg("--json");
        }
        command
            .arg("--debug-map")
            .arg(&map)
            .arg("--debug-identity")
            .arg(&identity)
            .arg(&component)
            .output()
            .unwrap()
    };

    let json_output = run(true);
    assert_eq!(json_output.status.code(), Some(125));
    let fault = parse_runtime_fault(
        json_output
            .stderr
            .strip_suffix(b"\n")
            .unwrap_or(&json_output.stderr),
    )
    .unwrap();
    assert_eq!(fault.class, "trap");
    assert!(fault.frames.iter().any(|frame| {
        frame
            .source
            .as_ref()
            .is_some_and(|source| source.document_id == "doc.trap")
    }));

    let text_output = run(false);
    assert_eq!(text_output.status.code(), Some(125));
    let text = String::from_utf8(text_output.stderr).unwrap();
    assert!(text.contains("trap [runtime.trap]: guest execution trapped"));
    assert!(text.contains("doc.trap:10..20"));
    assert!(!text.contains("unreachable"));

    let mut stale = artifact.identity;
    stale.push(b' ');
    std::fs::write(&identity, stale).unwrap();
    let stale_output = run(true);
    assert_eq!(stale_output.status.code(), Some(127));
    let stale_fault = parse_runtime_fault(
        stale_output
            .stderr
            .strip_suffix(b"\n")
            .unwrap_or(&stale_output.stderr),
    )
    .unwrap();
    assert_eq!(stale_fault.class, "incompatible-artifact");
    assert!(stale_fault.frames.is_empty());

    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn runner_cli_accepts_identity_bound_client_cancellation_file() {
    let unique = format!(
        "sico-step0098-client-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir(&directory).unwrap();
    let component = directory.join("spin.component.wasm");
    let request = directory.join("cancel.json");
    std::fs::write(&component, spin_component()).unwrap();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_sico-runner"))
        .args(["--fuel", "1000000000000"])
        .arg("--cancel-request-file")
        .arg(&request)
        .arg(&component)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    std::fs::write(
        &request,
        br#"{"schema":"sico.cancel-request.v0","run_id":"run-0","generation_id":1,"cause":"client"}"#,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert!(
        child.try_wait().unwrap().is_none(),
        "stale generation cancelled the run"
    );

    std::fs::write(
        &request,
        br#"{"schema":"sico.cancel-request.v0","run_id":"run-0","generation_id":0,"cause":"client"}"#,
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(123));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("\"class\":\"cancelled\""), "{stderr}");

    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn runner_watch_accepts_generation_bound_client_cancellation() {
    let unique = format!(
        "sico-step0098-watch-client-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir(&directory).unwrap();
    let component = directory.join("spin.component.wasm");
    let request = directory.join("cancel.json");
    std::fs::write(&component, spin_component()).unwrap();
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_sico-runner"))
        .args(["--watch", "--watch-runs", "1", "--fuel", "1000000000000"])
        .arg("--cancel-request-file")
        .arg(&request)
        .arg(&component)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::fs::write(
        &request,
        br#"{"schema":"sico.cancel-request.v0","run_id":"watch","generation_id":1,"cause":"client"}"#,
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(123));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("\"event\":\"run-complete\""), "{stderr}");
    assert!(stderr.contains("\"exit\":123"), "{stderr}");
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(windows)]
#[test]
fn runner_cli_observes_real_windows_console_control() {
    let unique = format!(
        "sico-step0098-signal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir(&directory).unwrap();
    let component = directory.join("blocked-read.component.wasm");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/end-to-end/script-stream-read-once.sico");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component.as_os_str().to_owned(),
            source.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));

    let script = r#"
import os, signal, subprocess, sys, time
# STATUS_CONTROL_C_EXIT with empty stderr means the CTRL_BREAK arrived
# before the child installed its console handler: a fixture-side startup
# race, not a runner failure. Only that exact signature may retry; every
# other mismatch is a real failure.
for attempt in range(6):
    p = subprocess.Popen(
        [sys.argv[1], sys.argv[2]],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
    )
    time.sleep(0.3)
    os.kill(p.pid, signal.CTRL_BREAK_EVENT)
    stdout, stderr = p.communicate(timeout=10)
    if p.returncode == 123 and b'\"class\":\"cancelled\"' in stderr:
        break
    if p.returncode == 3221225786 and not stderr:
        continue
    sys.stderr.buffer.write(stderr)
    raise SystemExit(1)
else:
    sys.stderr.buffer.write(b'console-control fixture never observed a handler-installed run')
    raise SystemExit(1)
"#;
    let output = std::process::Command::new("python")
        .arg("-c")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_sico-runner"))
        .arg(&component)
        .output()
        .expect("Python is required for the Windows console-control fixture");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(windows)]
#[test]
fn runner_watch_observes_console_control_while_idle() {
    let unique = format!(
        "sico-step0098-watch-signal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir(&directory).unwrap();
    let component = directory.join("echo.component.wasm");
    std::fs::write(&component, echo_component(0)).unwrap();
    let script = r#"
import os, signal, subprocess, sys, time
# Same fixture-side race as the CLI fixture: retry only on the exact
# STATUS_CONTROL_C_EXIT-before-handler-install signature.
for attempt in range(6):
    p = subprocess.Popen(
        [sys.argv[1], '--watch', sys.argv[2]],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
    )
    time.sleep(0.3)
    os.kill(p.pid, signal.CTRL_BREAK_EVENT)
    stdout, stderr = p.communicate(timeout=10)
    if p.returncode == 123 and b'\"schema\":\"sico.runner.watch.v0\"' in stderr:
        break
    if p.returncode == 3221225786 and not stderr:
        continue
    sys.stderr.buffer.write(stderr)
    raise SystemExit(1)
else:
    sys.stderr.buffer.write(b'watch console-control fixture never observed a handler-installed run')
    raise SystemExit(1)
"#;
    let output = std::process::Command::new("python")
        .arg("-c")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_sico-runner"))
        .arg(&component)
        .output()
        .expect("Python is required for the Windows watch signal fixture");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancellation_reaches_a_blocked_guest() {
    let token = CancelToken::new();
    let token_thread = token.clone();
    let instant = RunnerLimits {
        fuel: u64::MAX,
        timeout: std::time::Duration::from_secs(60),
        ..limits()
    };
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = token_thread.request(sico_runner::CancellationSource::Signal);
    });
    let runner = runner();
    let prepared = runner
        .prepare_program_with_net(
            &spin_component(),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let observed = prepared
        .run_observed(&ScriptInput::default(), &instant, &token, "run-signal", 0)
        .unwrap();
    assert_eq!(observed.outcome, RunOutcome::Cancelled);
    assert_eq!(observed.outcome.exit_code(), 123);
    assert_eq!(
        observed.cancellation_source,
        Some(sico_runner::CancellationSource::Signal)
    );
}

#[test]
fn prepared_program_reruns_are_isolated_and_survive_a_trap() {
    let runner = runner();
    let prepared = runner
        .prepare_program_with_net(
            &echo_component(0),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let mut elapsed = Vec::new();
    for index in 0..32_u8 {
        let input = ScriptInput {
            arguments: vec![index.to_string()],
            stdin: vec![index; 128],
        };
        let started = std::time::Instant::now();
        let outcome = prepared.run(&input, &limits(), &no_cancel()).unwrap();
        elapsed.push(started.elapsed());
        assert_eq!(
            outcome,
            RunOutcome::Output(ScriptOutput {
                stdout: input.stdin.clone(),
                stderr: input.stdin,
                exit_code: 0,
            })
        );
    }
    elapsed.sort_unstable();
    println!(
        "PERSISTENT_WARM_MEDIAN_US={}",
        elapsed[elapsed.len() / 2].as_micros()
    );
    assert!(
        elapsed[elapsed.len() / 2] <= std::time::Duration::from_millis(20),
        "warm median {:?}",
        elapsed[elapsed.len() / 2]
    );

    let trapped = runner
        .prepare_program_with_net(
            &trap_component(),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap()
        .run(&ScriptInput::default(), &limits(), &no_cancel())
        .unwrap();
    assert!(matches!(trapped, RunOutcome::Trap(_)));
    assert!(matches!(
        prepared
            .run(&ScriptInput::default(), &limits(), &no_cancel())
            .unwrap(),
        RunOutcome::Output(_)
    ));
}

#[test]
fn incompatible_artifacts_and_input_bounds_fail_closed() {
    let outcome = run(b"not-a-component", &ScriptInput::default());
    assert!(matches!(outcome, RunOutcome::Incompatible(_)));
    assert_eq!(outcome.exit_code(), 127);

    let oversized = ScriptInput {
        arguments: Vec::new(),
        stdin: vec![0; 8 * 1024 * 1024 + 1],
    };
    assert!(
        runner()
            .run_program(
                &echo_component(0),
                &oversized,
                &limits(),
                &no_cancel(),
                &FsGrants::default()
            )
            .is_err()
    );

    // A composed command (WASI imports) is not a runnable Program for the
    // direct runner and must not be executed.
    let adapter = sico_app_cli_compose_adapter();
    let composed = sico_app_cli_compose(&echo_component(0), &adapter);
    let outcome = run(&composed, &ScriptInput::default());
    assert!(
        matches!(outcome, RunOutcome::Launch(_) | RunOutcome::Incompatible(_)),
        "{outcome:?}"
    );
}

fn sico_app_cli_compose_adapter() -> Vec<u8> {
    sico_app_cli::compose::script_adapter_component()
}

fn sico_app_cli_compose(program: &[u8], adapter: &[u8]) -> Vec<u8> {
    sico_app_cli::compose::compose_script_command(program, adapter)
}

fn script_result() -> Type {
    Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    }
}

fn run_function(name: &str, instructions: Vec<Instruction>, result: ValueId) -> Function {
    let range = SourceRange { start: 0, end: 0 };
    Function {
        id: FunctionId(1),
        name: name.to_owned(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "input".into(),
            ty: Type::Named("ScriptInput".into()),
            range,
        }],
        return_type: script_result(),
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions,
            terminator: Terminator::Return(Some(result)),
            range,
        }],
        range,
    }
}

fn echo_component(exit_code: i64) -> Vec<u8> {
    sico_codegen_wasm::compile_script_program(&echo_module(exit_code)).unwrap()
}

fn echo_module(exit_code: i64) -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("echo.sico", 0);
    module.functions.push(run_function(
        "run",
        vec![
            Instruction {
                result: ValueId(1),
                ty: Type::Bytes,
                operation: Operation::Project {
                    base: ValueId(0),
                    field: "stdin".into(),
                },
                range,
            },
            Instruction {
                result: ValueId(2),
                ty: Type::Bytes,
                operation: Operation::Copy(ValueId(1)),
                range,
            },
            Instruction {
                result: ValueId(3),
                ty: Type::I64,
                operation: Operation::ConstI64(exit_code),
                range,
            },
            Instruction {
                result: ValueId(4),
                ty: Type::Named("ScriptOutput".into()),
                operation: Operation::Construct {
                    name: "ScriptOutput".into(),
                    fields: vec![
                        ConstructField {
                            name: "stdout".into(),
                            value: ValueId(1),
                        },
                        ConstructField {
                            name: "stderr".into(),
                            value: ValueId(2),
                        },
                        ConstructField {
                            name: "exit_code".into(),
                            value: ValueId(3),
                        },
                    ],
                },
                range,
            },
            Instruction {
                result: ValueId(5),
                ty: script_result(),
                operation: Operation::Variant {
                    name: "ok".into(),
                    payload: vec![ValueId(4)],
                },
                range,
            },
        ],
        ValueId(5),
    ));
    module
}

fn echo_debug_artifact(exit_code: i64) -> sico_codegen_wasm::DebugArtifact {
    let mut module = echo_module(exit_code);
    module.source_len = 64;
    let source = [b' '; 64];
    let compiler_sha256 = "d".repeat(64);
    sico_codegen_wasm::compile_script_program_with_debug(
        &module,
        &sico_codegen_wasm::DebugBuildInput {
            document_id: "doc.echo",
            source_bytes: &source,
            display_uri: Some("workspace://echo.sico"),
            compiler_package: "sico-compiler",
            compiler_version: "0.0.2-dev",
            compiler_executable_sha256: &compiler_sha256,
            adapter_identities: vec!["sico:script-adapter@0.1.0".into()],
            wit_identities: vec!["sico:script@0.1.0".into()],
        },
    )
    .unwrap()
}

fn error_component() -> Vec<u8> {
    script_error_component("DomainError", "rejected")
}

fn cancelled_component() -> Vec<u8> {
    script_error_component("Cancelled", "scope cancelled")
}

fn script_error_component(case: &str, message: &str) -> Vec<u8> {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("error.sico", 0);
    module.functions.push(run_function(
        "run",
        vec![
            Instruction {
                result: ValueId(1),
                ty: Type::Named("ScriptErrorCode".into()),
                operation: Operation::Variant {
                    name: format!("ScriptErrorCode.{case}"),
                    payload: Vec::new(),
                },
                range,
            },
            Instruction {
                result: ValueId(2),
                ty: Type::String,
                operation: Operation::ConstString(message.into()),
                range,
            },
            Instruction {
                result: ValueId(3),
                ty: Type::Named("ScriptError".into()),
                operation: Operation::Construct {
                    name: "ScriptError".into(),
                    fields: vec![
                        ConstructField {
                            name: "code".into(),
                            value: ValueId(1),
                        },
                        ConstructField {
                            name: "message".into(),
                            value: ValueId(2),
                        },
                    ],
                },
                range,
            },
            Instruction {
                result: ValueId(4),
                ty: script_result(),
                operation: Operation::Variant {
                    name: "error".into(),
                    payload: vec![ValueId(3)],
                },
                range,
            },
        ],
        ValueId(4),
    ));
    sico_codegen_wasm::compile_script_program(&module).unwrap()
}

fn trap_component() -> Vec<u8> {
    let mut module = Module::new("trap.sico", 0);
    let mut function = run_function("run", Vec::new(), ValueId(0));
    function.blocks[0].terminator = Terminator::Unreachable;
    module.functions.push(function);
    sico_codegen_wasm::compile_script_program(&module).unwrap()
}

fn spin_component() -> Vec<u8> {
    sico_codegen_wasm::compile_script_program(&spin_module()).unwrap()
}

fn spin_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("spin.sico", 0);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "input".into(),
            ty: Type::Named("ScriptInput".into()),
            range,
        }],
        return_type: script_result(),
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![
            Block {
                id: BlockId(0),
                instructions: Vec::new(),
                terminator: Terminator::Jump(BlockId(1)),
                range,
            },
            Block {
                id: BlockId(1),
                instructions: Vec::new(),
                terminator: Terminator::Jump(BlockId(1)),
                range,
            },
        ],
        range,
    });
    module
}

fn spin_debug_artifact() -> sico_codegen_wasm::DebugArtifact {
    let mut module = spin_module();
    module.source_len = 64;
    let source = [b' '; 64];
    let compiler_sha256 = "c".repeat(64);
    sico_codegen_wasm::compile_script_program_with_debug(
        &module,
        &sico_codegen_wasm::DebugBuildInput {
            document_id: "doc.spin",
            source_bytes: &source,
            display_uri: Some("workspace://spin.sico"),
            compiler_package: "sico-compiler",
            compiler_version: "0.0.2-dev",
            compiler_executable_sha256: &compiler_sha256,
            adapter_identities: vec!["sico:script-adapter@0.1.0".into()],
            wit_identities: vec!["sico:script@0.1.0".into()],
        },
    )
    .unwrap()
}

fn trap_debug_artifact() -> sico_codegen_wasm::DebugArtifact {
    let parameter_range = SourceRange { start: 0, end: 10 };
    let mut module = Module::new("trap.sico", 64);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "input".into(),
            ty: Type::Named("ScriptInput".into()),
            range: parameter_range,
        }],
        return_type: script_result(),
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            range: SourceRange { start: 10, end: 20 },
        }],
        range: SourceRange { start: 0, end: 20 },
    });
    let source = trap_debug_source();
    let compiler_sha256 = "b".repeat(64);
    sico_codegen_wasm::compile_script_program_with_debug(
        &module,
        &sico_codegen_wasm::DebugBuildInput {
            document_id: "doc.trap",
            source_bytes: &source,
            display_uri: Some("workspace://trap.sico"),
            compiler_package: "sico-compiler",
            compiler_version: "0.0.2-dev",
            compiler_executable_sha256: &compiler_sha256,
            adapter_identities: vec!["sico:script-adapter@0.1.0".into()],
            wit_identities: vec!["sico:script@0.1.0".into()],
        },
    )
    .unwrap()
}

fn trap_debug_source() -> [u8; 64] {
    let mut source = [b' '; 64];
    source[9] = b'\n';
    source
}

fn nested_trap_debug_artifact() -> sico_codegen_wasm::DebugArtifact {
    let run_range = SourceRange { start: 0, end: 20 };
    let mut module = Module::new("nested.sico", 64);
    module.functions.push(Function {
        id: FunctionId(1),
        name: "run".into(),
        parameters: vec![Parameter {
            id: ValueId(0),
            name: "input".into(),
            ty: Type::Named("ScriptInput".into()),
            range: SourceRange { start: 0, end: 10 },
        }],
        return_type: script_result(),
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(1),
                ty: script_result(),
                operation: Operation::Call {
                    function: FunctionId(2),
                    arguments: Vec::new(),
                },
                range: SourceRange { start: 10, end: 20 },
            }],
            terminator: Terminator::Return(Some(ValueId(1))),
            range: SourceRange { start: 10, end: 20 },
        }],
        range: run_range,
    });
    module.functions.push(Function {
        id: FunctionId(2),
        name: "nested_trap".into(),
        parameters: Vec::new(),
        return_type: script_result(),
        effects: Vec::new(),
        entry: BlockId(0),
        blocks: vec![Block {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            range: SourceRange { start: 20, end: 30 },
        }],
        range: SourceRange { start: 20, end: 30 },
    });
    let source = nested_debug_source();
    sico_codegen_wasm::compile_script_program_with_debug(
        &module,
        &sico_codegen_wasm::DebugBuildInput {
            document_id: "doc.nested",
            source_bytes: &source,
            display_uri: Some("workspace://nested.sico"),
            compiler_package: "sico-compiler",
            compiler_version: "0.0.2-dev",
            compiler_executable_sha256: &"e".repeat(64),
            adapter_identities: vec!["sico:script-adapter@0.1.0".into()],
            wit_identities: vec!["sico:script@0.1.0".into()],
        },
    )
    .unwrap()
}

fn nested_debug_source() -> [u8; 64] {
    let mut source = [b' '; 64];
    source[9] = b'\n';
    source[19] = b'\n';
    source
}

/// A malicious core guest whose result discriminant is out of range, wrapped
/// in the frozen Program Component shell.
fn malformed_component() -> Vec<u8> {
    use wasm_encoder::{
        CodeSection, ConstExpr, ExportKind, ExportSection, Function as WasmFunction,
        FunctionSection, GlobalSection, GlobalType, Instruction as Wasm, MemArg, MemorySection,
        MemoryType, Module as CoreModule, TypeSection, ValType,
    };
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function([ValType::I32], []);
    let mut functions = FunctionSection::new();
    functions.function(0);
    functions.function(0);
    functions.function(1);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(1024),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(4096),
    );
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("run", ExportKind::Func, 0);
    exports.export("cabi_realloc", ExportKind::Func, 1);
    exports.export("cabi_post_run", ExportKind::Func, 2);

    let mut run = WasmFunction::new(Vec::new());
    run.instruction(&Wasm::I32Const(0));
    run.instruction(&Wasm::I32Const(2));
    run.instruction(&Wasm::I32Store8(MemArg {
        offset: 1024,
        align: 0,
        memory_index: 0,
    }));
    run.instruction(&Wasm::I32Const(1024));
    run.instruction(&Wasm::End);

    let mut realloc = WasmFunction::new(vec![(1, ValType::I32)]);
    realloc.instruction(&Wasm::GlobalGet(0));
    realloc.instruction(&Wasm::LocalGet(2));
    realloc.instruction(&Wasm::I32Add);
    realloc.instruction(&Wasm::I32Const(-1));
    realloc.instruction(&Wasm::I32Add);
    realloc.instruction(&Wasm::I32Const(0));
    realloc.instruction(&Wasm::LocalGet(2));
    realloc.instruction(&Wasm::I32Sub);
    realloc.instruction(&Wasm::I32And);
    realloc.instruction(&Wasm::LocalTee(4));
    realloc.instruction(&Wasm::LocalGet(3));
    realloc.instruction(&Wasm::I32Add);
    realloc.instruction(&Wasm::GlobalSet(0));
    realloc.instruction(&Wasm::LocalGet(4));
    realloc.instruction(&Wasm::End);

    let mut post = WasmFunction::new(Vec::new());
    post.instruction(&Wasm::I32Const(4096));
    post.instruction(&Wasm::GlobalSet(0));
    post.instruction(&Wasm::End);

    let mut code = CodeSection::new();
    code.function(&run);
    code.function(&realloc);
    code.function(&post);
    let mut module = CoreModule::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    sico_codegen_wasm::wrap_script_component(
        &module.finish(),
        sico_codegen_wasm::FsUse::default(),
        sico_codegen_wasm::StreamUse::default(),
        false,
        0,
    )
}

// ---- STEP-0105: single-Store cooperative scheduler core (M11) ----

fn step0105_scheduler(run_id: &str) -> SchedulerCore {
    SchedulerCore::new(RunIdentity {
        run_id: run_id.to_owned(),
        generation_id: 1,
    })
}

fn step0105_record(
    scheduler: &SchedulerCore,
    task: TaskId,
    operation: OperationId,
    payload_bytes: usize,
) -> CompletionRecord {
    CompletionRecord {
        run_id: scheduler.identity().run_id.clone(),
        generation_id: scheduler.identity().generation_id,
        task,
        operation,
        payload_bytes,
    }
}

/// Drives one full synthetic workload through the core: open a scope, spawn
/// `spawned` tasks, run each through `created → runnable → suspended →
/// runnable → completing → succeeded` with one Host operation per task, then
/// close and tear down. Returns the canonical turn order observed.
fn step0105_drive(scheduler: &mut SchedulerCore, spawned: usize) -> Vec<OperationId> {
    let root = scheduler.root_task();
    assert_eq!(scheduler.parent_of(root), None);
    scheduler.make_runnable(root).unwrap();
    assert_eq!(scheduler.next_ready(), Some(root));
    let scope = scheduler.open_scope(ScopeId(0)).unwrap();
    let mut entries = Vec::new();
    for _ in 0..spawned {
        let task = scheduler.spawn_task(scope, root).unwrap();
        assert_eq!(scheduler.parent_of(task), Some(root));
        scheduler.make_runnable(task).unwrap();
        assert_eq!(scheduler.next_ready(), Some(task));
        let operation = scheduler
            .register_operation(task, ReadinessClass::HostCompletion)
            .unwrap();
        scheduler.suspend(task).unwrap();
        scheduler
            .publish_completion(step0105_record(scheduler, task, operation, 16))
            .unwrap();
        entries.push((task, operation));
    }
    let turn: Vec<_> = scheduler
        .drain_turn()
        .iter()
        .map(|record| record.operation)
        .collect();
    for (task, _) in &entries {
        scheduler.make_runnable(*task).unwrap();
        assert_eq!(scheduler.next_ready(), Some(*task));
        scheduler.begin_completion(*task).unwrap();
        scheduler
            .commit_terminal(*task, TerminalKind::Succeeded)
            .unwrap();
    }
    scheduler.close_scope(scope).unwrap();
    scheduler.commit_root(TerminalKind::Succeeded).unwrap();
    scheduler.teardown().unwrap();
    turn
}

#[test]
fn scheduler_scale_workloads_execute_within_run_bounds() {
    // `total` counts every live task including the root, so the largest
    // workload sits exactly on the ADR-0010 1,024 live-task bound.
    for total in [1_usize, 2, 16, 256, MAX_LIVE_TASKS] {
        let mut scheduler = step0105_scheduler(&format!("run-scale-{total}"));
        let started = std::time::Instant::now();
        let turn = step0105_drive(&mut scheduler, total - 1);
        println!(
            "SCHEDULER_SCALE tasks={total} wall_us={}",
            started.elapsed().as_micros()
        );
        // Single-class turn: canonical order reduces to registration order.
        let expected: Vec<_> = (1..total as u64).map(OperationId).collect();
        assert_eq!(turn, expected);
        assert_eq!(scheduler.live_tasks(), 0);
        assert_eq!(scheduler.host_operations_total as usize, total - 1);
    }
}

#[test]
fn scheduler_limit_plus_one_cases_fail_with_typed_outcomes() {
    // Task table: root + 1,023 spawned fills 1,024 live tasks; the 1,025th
    // is a typed refusal.
    let mut scheduler = step0105_scheduler("run-limit-tasks");
    let root = scheduler.root_task();
    let scope = scheduler.open_scope(ScopeId(0)).unwrap();
    for _ in 0..MAX_LIVE_TASKS - 1 {
        scheduler.spawn_task(scope, root).unwrap();
    }
    assert_eq!(scheduler.live_tasks(), MAX_LIVE_TASKS);
    assert_eq!(
        scheduler.spawn_task(scope, root),
        Err(SchedulerFault::TaskTableFull)
    );

    // Scope children: scope 0 holds the root plus 1,023 spawned tasks.
    let mut scheduler = step0105_scheduler("run-limit-children");
    let root = scheduler.root_task();
    for _ in 1..MAX_SCOPE_CHILDREN {
        scheduler.spawn_task(ScopeId(0), root).unwrap();
    }
    assert_eq!(
        scheduler.spawn_task(ScopeId(0), root),
        Err(SchedulerFault::ScopeChildrenExceeded)
    );

    // Scope depth 65.
    let mut scheduler = step0105_scheduler("run-limit-depth");
    let mut scope = ScopeId(0);
    for _ in 1..MAX_SCOPE_DEPTH {
        scope = scheduler.open_scope(scope).unwrap();
    }
    assert_eq!(
        scheduler.open_scope(scope),
        Err(SchedulerFault::ScopeDepthExceeded)
    );

    // Ready queue: with the queue cap equal to the task cap, every entry is
    // deduped per task, so the bound is only reachable through a stale entry
    // left behind by a task that went terminal while still queued: the
    // replacement task's first enqueue is the typed limit+1, and popping the
    // stale entry afterwards frees the slot exactly once.
    let mut scheduler = step0105_scheduler("run-limit-ready");
    let root = scheduler.root_task();
    scheduler.make_runnable(root).unwrap();
    let scope = scheduler.open_scope(ScopeId(0)).unwrap();
    let mut victim = None;
    for index in 0..MAX_READY_QUEUE - 1 {
        let task = scheduler.spawn_task(scope, root).unwrap();
        scheduler.make_runnable(task).unwrap();
        if index == 0 {
            victim = Some(task);
        }
    }
    scheduler
        .commit_terminal(victim.unwrap(), TerminalKind::Cancelled)
        .unwrap();
    let replacement = scheduler.spawn_task(scope, root).unwrap();
    assert_eq!(
        scheduler.make_runnable(replacement),
        Err(SchedulerFault::ReadyQueueFull)
    );
    let _ = scheduler.next_ready();
    scheduler.make_runnable(replacement).unwrap();

    // Completion records: 1,024 delivered, the 1,025th refused.
    let mut scheduler = step0105_scheduler("run-limit-completions");
    let root = scheduler.root_task();
    for _ in 0..MAX_COMPLETION_RECORDS {
        let operation = scheduler
            .register_operation(root, ReadinessClass::HostCompletion)
            .unwrap();
        scheduler
            .publish_completion(step0105_record(&scheduler, root, operation, 0))
            .unwrap();
    }
    let overflow = scheduler
        .register_operation(root, ReadinessClass::HostCompletion)
        .unwrap();
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, root, overflow, 0)),
        Err(SchedulerFault::CompletionQueueFull)
    );

    // Completion bytes: exactly 16 MiB delivered, one more byte refused.
    let mut scheduler = step0105_scheduler("run-limit-bytes");
    let root = scheduler.root_task();
    let big = scheduler
        .register_operation(root, ReadinessClass::HostCompletion)
        .unwrap();
    scheduler
        .publish_completion(step0105_record(&scheduler, root, big, MAX_COMPLETION_BYTES))
        .unwrap();
    let extra = scheduler
        .register_operation(root, ReadinessClass::HostCompletion)
        .unwrap();
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, root, extra, 1)),
        Err(SchedulerFault::CompletionBytesExceeded)
    );
    // The 16 MiB metadata budget is structurally unreachable with every
    // count capped at 1,024 (hundreds of KiB worst case), so it has no
    // limit+1 case; it is enforced against future larger records.
}

#[test]
fn scheduler_adversarial_completion_records_fail_closed() {
    let mut scheduler = step0105_scheduler("run-adversarial");
    let root = scheduler.root_task();
    let scope = scheduler.open_scope(ScopeId(0)).unwrap();
    let child = scheduler.spawn_task(scope, root).unwrap();
    let operation = scheduler
        .register_operation(root, ReadinessClass::HostCompletion)
        .unwrap();
    // Cross-run and cross-generation records never resolve.
    let mut wrong_run = step0105_record(&scheduler, root, operation, 0);
    wrong_run.run_id = "run-other".to_owned();
    assert_eq!(
        scheduler.publish_completion(wrong_run),
        Err(SchedulerFault::CrossRunCompletion)
    );
    let mut wrong_generation = step0105_record(&scheduler, root, operation, 0);
    wrong_generation.generation_id = 2;
    assert_eq!(
        scheduler.publish_completion(wrong_generation),
        Err(SchedulerFault::CrossRunCompletion)
    );
    // Unknown operation identity.
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, root, OperationId(999), 0)),
        Err(SchedulerFault::UnknownOperation(OperationId(999)))
    );
    // A record naming a different task than the registration is stale.
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, child, operation, 0)),
        Err(SchedulerFault::StaleCompletion(operation))
    );
    // First valid delivery lands; a replay is a duplicate.
    scheduler
        .publish_completion(step0105_record(&scheduler, root, operation, 0))
        .unwrap();
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, root, operation, 0)),
        Err(SchedulerFault::DuplicateCompletion(operation))
    );
    // An abandoned operation classifies late worker records as stale.
    let abandoned = scheduler
        .register_operation(root, ReadinessClass::HostCompletion)
        .unwrap();
    scheduler.abandon_operation(abandoned).unwrap();
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, root, abandoned, 0)),
        Err(SchedulerFault::StaleCompletion(abandoned))
    );
    // After teardown nothing resolves into the run, even with exact
    // identity.
    scheduler
        .commit_terminal(child, TerminalKind::Cancelled)
        .unwrap();
    scheduler.commit_root(TerminalKind::Succeeded).unwrap();
    scheduler.teardown().unwrap();
    assert_eq!(
        scheduler.publish_completion(step0105_record(&scheduler, root, operation, 0)),
        Err(SchedulerFault::CrossRunCompletion)
    );
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn repeated_runs_show_no_task_handle_or_rss_growth_across_store_teardown() {
    // Tasks, scopes, operations and queues are Store-scoped records and drop
    // with the Store; the observable leak surface is OS handles and RSS.
    // Each echo run registers and completes stdin/stdout/stderr Host
    // operations through the ingress before teardown.
    let runner = runner();
    let prepared = runner
        .prepare_program_with_net(
            &echo_component(0),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let run_one = |index: u32| {
        let input = ScriptInput {
            arguments: vec![index.to_string()],
            stdin: vec![index as u8; 128],
        };
        let outcome = prepared.run(&input, &limits(), &no_cancel()).unwrap();
        assert!(
            matches!(outcome, RunOutcome::Output(_)),
            "run {index}: {outcome:?}"
        );
    };
    run_one(0);
    let (baseline_handles, baseline_rss) = process_metrics();
    for index in 1..=100 {
        run_one(index);
    }
    let (final_handles, final_rss) = process_metrics();
    println!(
        "SCHEDULER_TEARDOWN_100 baseline_handles={baseline_handles} final_handles={final_handles} baseline_rss={baseline_rss} final_rss={final_rss}"
    );
    assert!(
        final_handles <= baseline_handles + 8,
        "handle growth: {baseline_handles} -> {final_handles}"
    );
    assert!(
        final_rss <= baseline_rss + 64 * 1024 * 1024,
        "RSS growth: {baseline_rss} -> {final_rss}"
    );
}

const STEP0105_SCRIPT_PRELUDE: &str = "record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

async function shout(word: Text) returns Text:
  return sico.text.concat(word, \"!\")
end function

";

fn step0105_guest_source(count: usize) -> String {
    let mut source = String::from(STEP0105_SCRIPT_PRELUDE);
    source.push_str(
        "function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n  task group:\n",
    );
    for index in 0..count {
        source.push_str(&format!("    let t{index} = spawn shout(\"w{index}\")\n"));
    }
    let list = (0..count)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    source.push_str(&format!(
        "    let all = await collect_tasks([{list}], order: input)\n"
    ));
    source.push_str("    return ok(ScriptOutput(stdout: sico.text.encode(sico.text.join(all, \" \")), stderr: sico.text.encode(\"\"), exit_code: I64.literal(0)))\n  end task\nend function\n");
    source
}

fn step0105_build_guest(source: &str, tag: usize) -> Vec<u8> {
    let directory = std::env::temp_dir().join(format!(
        "sico-step0105-scale-{}-{}",
        std::process::id(),
        tag
    ));
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("scale.sico");
    let component_path = directory.join("scale.component.wasm");
    std::fs::write(&source_path, source).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component_path.as_os_str().to_owned(),
            source_path.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

#[test]
fn guest_task_workloads_at_scale_run_within_bounds() {
    // Real guest spawn workloads (sequential-v1 profile, RFC-0036 §5.4):
    // source-level tasks execute eagerly inside the guest, so the run-level
    // fuel/memory/time bounds are what constrain them.
    for count in [1_usize, 2, 16, 256, 1_024] {
        let component = step0105_build_guest(&step0105_guest_source(count), count);
        let started = std::time::Instant::now();
        let outcome = run(&component, &ScriptInput::default());
        println!(
            "GUEST_TASK_SCALE tasks={count} wall_ms={}",
            started.elapsed().as_millis()
        );
        let expected = (0..count)
            .map(|index| format!("w{index}!"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            outcome,
            RunOutcome::Output(ScriptOutput {
                stdout: expected.into_bytes(),
                stderr: Vec::new(),
                exit_code: 0,
            })
        );
    }
}

// ---- STEP-0106: cancellation, timeout, race and select (M11) ----

#[test]
fn scheduler_cancellation_tree_scales_and_select_stays_canonical() {
    // Nested cancellation of a full 1,024-task chain: one reverse-ordered
    // commit pass, zero live tasks afterwards, teardown still exact.
    let mut scheduler = step0105_scheduler("run-step0106-chain");
    let root = scheduler.root_task();
    scheduler.make_runnable(root).unwrap();
    let _ = scheduler.next_ready();
    let mut parent = root;
    for _ in 1..MAX_LIVE_TASKS {
        parent = scheduler.spawn_task(ScopeId(0), parent).unwrap();
    }
    let started = std::time::Instant::now();
    let cancelled = scheduler.cancel_task(root).unwrap();
    println!(
        "CHAIN_CANCEL_1024 wall_us={}",
        started.elapsed().as_micros()
    );
    assert_eq!(cancelled.len(), MAX_LIVE_TASKS);
    assert_eq!(cancelled.last(), Some(&root));
    assert_eq!(scheduler.live_tasks(), 0);
    assert_eq!(scheduler.teardown().unwrap(), vec![ScopeId(0)]);

    // Collection ordering under repeated adversarial scheduling: shuffle
    // the publish order each round; the winner is always the canonical
    // first (registration order within the class), never the earliest
    // arrival.
    for round in 0..16_u32 {
        let mut scheduler = step0105_scheduler(&format!("run-step0106-select-{round}"));
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        let scope = scheduler.open_scope(ScopeId(0)).unwrap();
        let mut operands = Vec::new();
        let mut operations = Vec::new();
        for _ in 0..8 {
            let task = scheduler.spawn_task(scope, root).unwrap();
            let operation = scheduler
                .register_operation(task, ReadinessClass::HostCompletion)
                .unwrap();
            operands.push(task);
            operations.push(operation);
        }
        // Deterministic shuffle driven by the round number.
        let mut order: Vec<usize> = (0..8).collect();
        for index in 0..8 {
            let swap = ((round as usize) * 31 + index * 17 + 5) % 8;
            order.swap(index, swap);
        }
        for index in order {
            scheduler
                .publish_completion(step0105_record(
                    &scheduler,
                    operands[index],
                    operations[index],
                    0,
                ))
                .unwrap();
        }
        let outcome = scheduler.select(&operands).unwrap();
        assert_eq!(outcome.winner, operands[0], "round {round}");
        assert_eq!(outcome.losers.len(), 7, "round {round}");
        for loser in &outcome.losers {
            assert_eq!(
                scheduler.task_state(*loser),
                Some(sico_runner::scheduler::TaskState::Cancelled),
                "round {round}"
            );
        }
    }
}

#[test]
fn cancelled_guest_run_drives_the_scheduler_tree_to_teardown() {
    // Runner-level integration: a blocked guest cancelled via the M10
    // token path now resolves its scheduler root through the downward
    // tree and still tears the Store down cleanly (exit 123).
    let token = CancelToken::new();
    let token_thread = token.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = token_thread.request(sico_runner::CancellationSource::Signal);
    });
    let runner = runner();
    let prepared = runner
        .prepare_program_with_net(
            &spin_component(),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let patient = RunnerLimits {
        fuel: u64::MAX,
        timeout: std::time::Duration::from_secs(60),
        ..limits()
    };
    let observed = prepared
        .run_observed(
            &ScriptInput::default(),
            &patient,
            &token,
            "run-tree-cancel",
            1,
        )
        .unwrap();
    assert_eq!(observed.outcome, RunOutcome::Cancelled);
    assert_eq!(observed.outcome.exit_code(), 123);
    // A second run on the same prepared program proves teardown left no
    // scheduler state behind.
    let again = prepared
        .run_observed(
            &ScriptInput::default(),
            &limits(),
            &no_cancel(),
            "run-tree-after",
            2,
        )
        .unwrap();
    assert!(matches!(again.outcome, RunOutcome::FuelExhausted));
}

// ---- STEP-0107: bounded channels, streams and backpressure ----

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn channel_relay_keeps_rss_independent_of_stream_size() {
    // Relay 1 GiB of payload accounting through a 4-item / 4 MiB channel.
    // The v1 channel core stores `(sender, bytes)` records only — payload
    // contents stay in task memory, modeled here by one reusable 1 MiB
    // buffer — so scheduler metadata and process RSS must stay flat no
    // matter how much total volume flows through.
    let mut scheduler = step0105_scheduler("run-step0107-relay");
    let root = scheduler.root_task();
    scheduler.make_runnable(root).unwrap();
    let _ = scheduler.next_ready();
    let channel = scheduler.open_channel(root, 4, 4 << 20).unwrap();
    let payload = vec![0xAB_u8; 1 << 20];
    let metadata_before = scheduler.metadata_bytes();
    let (handles_before, rss_before) = process_metrics();
    let started = std::time::Instant::now();
    let mut relayed = 0_u64;
    for round in 0..256_u32 {
        // Fill to capacity, then drain completely: the slow-consumer
        // pattern that must never grow state.
        for _ in 0..4 {
            assert_eq!(
                scheduler.send(root, channel, payload.len()),
                Ok(SendOutcome::Buffered),
                "round {round}"
            );
        }
        for _ in 0..4 {
            let outcome = scheduler.recv(root, channel).unwrap();
            assert_eq!(
                outcome,
                RecvOutcome::Item {
                    sender: root,
                    bytes: payload.len()
                },
                "round {round}"
            );
            relayed += payload.len() as u64;
        }
    }
    let elapsed = started.elapsed();
    let (handles_after, rss_after) = process_metrics();
    assert_eq!(scheduler.metadata_bytes(), metadata_before);
    assert_eq!(relayed, 1 << 30);
    println!(
        "CHANNEL_RELAY_1GIB wall_ms={} metadata={} handles={}->{} rss={}->{}",
        elapsed.as_millis(),
        metadata_before,
        handles_before,
        handles_after,
        rss_before,
        rss_after
    );
    assert!(
        rss_after <= rss_before + 16 * 1024 * 1024,
        "RSS grew with stream size: {rss_before} -> {rss_after}"
    );
    // Teardown closes the still-open channel idempotently.
    scheduler.commit_root(TerminalKind::Succeeded).unwrap();
    scheduler.teardown().unwrap();
}

// ---- STEP-0108: persistent runner, watch, REPL and DAP task integration ----

#[test]
fn successive_generations_teardown_cleanly_before_the_next_store() {
    // Library-level watch-generation model: N generations of one prepared
    // program. A failed teardown would turn the outcome into Launch, so
    // identical successful outputs across generations prove each
    // generation's scheduler/Store settled before the next one published.
    let runner = runner();
    let prepared = runner
        .prepare_program_with_net(
            &echo_component(0),
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    for generation in 1..=8_u64 {
        let observed = prepared
            .run_observed(
                &ScriptInput::default(),
                &limits(),
                &no_cancel(),
                &format!("watch-gen-{generation}"),
                generation,
            )
            .unwrap();
        assert_eq!(
            observed.outcome,
            RunOutcome::Output(ScriptOutput {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit_code: 0,
            }),
            "generation {generation}"
        );
        assert_eq!(observed.cancellation_source, None);
    }
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn hundred_runs_with_changing_grants_show_no_leak_or_authority_drift() {
    // 100 generations alternating between granted and denied fs authority:
    // outcomes must be deterministic per grant parity (no authority leaks
    // across generations), with flat handles and RSS.
    let directory =
        std::env::temp_dir().join(format!("sico-step0108-grants-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("input.txt"), "alpha beta gamma").unwrap();
    let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/end-to-end/script-file-transform.sico");
    let component = step0105_build_guest(&std::fs::read_to_string(&source_path).unwrap(), 0);
    let runner = runner();
    // The runner's contract: grant roots are canonicalized by the caller.
    let canonical_root = directory.canonicalize().unwrap();
    let granted = FsGrants {
        read_roots: vec![canonical_root.clone()],
        write_roots: vec![canonical_root],
    };
    let run_one = |index: u32, grants: &FsGrants| {
        let prepared = runner
            .prepare_program_with_net(&component, grants, &sico_runner::NetGrants::default())
            .unwrap();
        prepared
            .run_observed(
                &ScriptInput::default(),
                &limits(),
                &no_cancel(),
                &format!("grant-gen-{index}"),
                u64::from(index),
            )
            .unwrap()
            .outcome
    };
    let granted_outcome = run_one(0, &granted);
    assert!(
        matches!(&granted_outcome, RunOutcome::Output(output) if output.exit_code == 0),
        "granted run: {granted_outcome:?}"
    );
    let denied_outcome = run_one(1, &FsGrants::default());
    assert!(
        !matches!(&denied_outcome, RunOutcome::Output(output) if output.exit_code == 0),
        "denied run must not succeed: {denied_outcome:?}"
    );
    let (handles_before, rss_before) = process_metrics();
    for index in 2..100_u32 {
        let outcome = if index % 2 == 0 {
            run_one(index, &granted)
        } else {
            run_one(index, &FsGrants::default())
        };
        let expected = if index % 2 == 0 {
            &granted_outcome
        } else {
            &denied_outcome
        };
        assert_eq!(&outcome, expected, "generation {index}");
    }
    let (handles_after, rss_after) = process_metrics();
    println!(
        "GRANT_MATRIX_100 handles={handles_before}->{handles_after} rss={rss_before}->{rss_after}"
    );
    assert!(
        handles_after <= handles_before + 8,
        "handle growth: {handles_before} -> {handles_after}"
    );
    assert!(
        rss_after <= rss_before + 64 * 1024 * 1024,
        "RSS growth: {rss_before} -> {rss_after}"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn debug_pause_terminate_leaves_no_stranded_tasks_or_workers() {
    // Twenty pause→terminate→finish cycles: every session's worker joins,
    // the scheduler inside the debug Store now tears down (STEP-0108), and
    // handles stay flat.
    let artifact = spin_debug_artifact();
    let runner = Runner::new_debug().unwrap();
    let prepared = runner
        .prepare_program_with_debug(
            &artifact.component,
            &artifact.debug_map,
            &artifact.identity,
            &FsGrants::default(),
            &sico_runner::NetGrants::default(),
        )
        .unwrap();
    let debug_limits = RunnerLimits {
        fuel: u64::MAX,
        timeout: std::time::Duration::from_secs(10),
        ..limits()
    };
    let cycle = |generation_id: u64| {
        let session = prepared
            .start_debug(
                &ScriptInput::default(),
                &debug_limits,
                &no_cancel(),
                &[],
                &format!("dap-strand-{generation_id}"),
                generation_id,
            )
            .unwrap();
        session.request_pause();
        assert_eq!(
            session
                .wait_for_stop(std::time::Duration::from_secs(2))
                .unwrap()
                .reason,
            "pause"
        );
        session.terminate();
        assert_eq!(
            session
                .finish(std::time::Duration::from_secs(2))
                .unwrap()
                .outcome,
            RunOutcome::Cancelled
        );
    };
    cycle(1);
    let (handles_before, rss_before) = process_metrics();
    for generation_id in 2..=20_u64 {
        cycle(generation_id);
    }
    let (handles_after, rss_after) = process_metrics();
    println!(
        "DAP_TERMINATE_20 handles={handles_before}->{handles_after} rss={rss_before}->{rss_after}"
    );
    assert!(
        handles_after <= handles_before + 8,
        "handle growth: {handles_before} -> {handles_after}"
    );
    assert!(
        rss_after <= rss_before + 64 * 1024 * 1024,
        "RSS growth: {rss_before} -> {rss_after}"
    );
}
