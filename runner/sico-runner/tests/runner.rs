use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
    Parameter, SourceRange, Terminator, Type, ValueId,
};
use sico_observability::{canonical_json, parse_runtime_fault};
use sico_runner::{
    CancelToken, FsGrants, ObservedRun, RunOutcome, Runner, RunnerLimits, ScriptInput, ScriptOutput,
};

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
p = subprocess.Popen(
    [sys.argv[1], sys.argv[2]],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
)
time.sleep(0.15)
os.kill(p.pid, signal.CTRL_BREAK_EVENT)
stdout, stderr = p.communicate(timeout=10)
if p.returncode != 123 or b'\"class\":\"cancelled\"' not in stderr:
    sys.stderr.buffer.write(stderr)
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
p = subprocess.Popen(
    [sys.argv[1], '--watch', sys.argv[2]],
    stdin=subprocess.DEVNULL,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
)
time.sleep(0.2)
os.kill(p.pid, signal.CTRL_BREAK_EVENT)
stdout, stderr = p.communicate(timeout=10)
if p.returncode != 123 or b'\"schema\":\"sico.runner.watch.v0\"' not in stderr:
    sys.stderr.buffer.write(stderr)
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
    sico_codegen_wasm::compile_script_program(&module).unwrap()
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
    sico_codegen_wasm::compile_script_program(&module).unwrap()
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
    let source = [b' '; 64];
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
