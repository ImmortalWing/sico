use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
    Parameter, SourceRange, Terminator, Type, ValueId,
};
use sico_runner::{
    CancelToken, FsGrants, RunOutcome, Runner, RunnerLimits, ScriptInput, ScriptOutput,
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
        token_thread.cancel();
    });
    let outcome = runner()
        .run_program(
            &spin_component(),
            &ScriptInput::default(),
            &instant,
            &token,
            &FsGrants::default(),
        )
        .unwrap();
    assert_eq!(outcome, RunOutcome::Cancelled);
    assert_eq!(outcome.exit_code(), 123);
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
