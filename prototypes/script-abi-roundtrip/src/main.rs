//! STEP-0079 Script aggregate Canonical ABI roundtrip harness.
//!
//! Drives compiler-generated Program Components through Wasmtime 46.0.1:
//! 10,000 seeded boundary roundtrips, a repeated-call cleanup oracle and
//! malicious-memory fixtures that must fail closed.

use std::collections::BTreeMap;
use std::error::Error;

use sha2::{Digest, Sha256};
use sico_ir::{
    Block, BlockId, ConstructField, Function, FunctionId, Instruction as IrInstruction, Module,
    Operation, Parameter, SourceRange, Terminator, Type, ValueId,
};
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, DataSegment, DataSegmentMode, ExportKind, ExportSection,
    Function as WasmFunction, FunctionSection, GlobalSection, GlobalType, Instruction, MemArg,
    MemorySection, MemoryType, Module as CoreModule, TypeSection, ValType,
};
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

type AnyError = Box<dyn Error + Send + Sync>;
type AnyResult<T> = Result<T, AnyError>;

fn compile(module: &Module) -> AnyResult<Vec<u8>> {
    sico_codegen_wasm::compile_script_program(module)
        .map_err(|error| format!("script backend refused fixture: {error:?}").into())
}

const ROUNDTRIPS: usize = 10_000;
const CLEANUP_CALLS: usize = 512;

fn main() -> AnyResult<()> {
    let echo_bytes = compile(&echo_module())?;
    let error_bytes = compile(&error_module())?;
    if echo_bytes != compile(&echo_module())? || error_bytes != compile(&error_module())? {
        return Err("script program encoding is not deterministic".into());
    }

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let echo = Component::new(&engine, &echo_bytes)?;
    let error_component = Component::new(&engine, &error_bytes)?;

    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let echo_instance = linker.instantiate(&mut store, &echo)?;
    let echo_run = echo_instance
        .get_func(&mut store, "run")
        .ok_or("echo component does not export run")?;
    for _ in 0..ROUNDTRIPS {
        let input = rng.input(120, 96, 4096);
        let output = call(&mut store, &echo_run, &input)?;
        let expected = ScriptOutput {
            stdout: input.stdin.clone(),
            stderr: Vec::new(),
            exit_code: 0,
        };
        if output != ScriptOutcome::Output(expected) {
            return Err(format!("roundtrip mismatch: {output:?}").into());
        }
    }

    // Repeated-call cleanup oracle: per call the guest lowers at least
    // stdin + stdout + result area, so 512 calls of 1 MiB each (plus eight
    // 4 MiB calls) would cross the 64 MiB arena ceiling without working
    // post-return cleanup.
    for _ in 0..CLEANUP_CALLS {
        let input = rng.input(8, 64, 1024 * 1024);
        let output = call(&mut store, &echo_run, &input)?;
        if output
            != (ScriptOutcome::Output(ScriptOutput {
                stdout: input.stdin.clone(),
                stderr: Vec::new(),
                exit_code: 0,
            }))
        {
            return Err("cleanup oracle mismatch".into());
        }
    }
    for _ in 0..8 {
        let input = rng.input(0, 0, 4 * 1024 * 1024);
        let output = call(&mut store, &echo_run, &input)?;
        if output
            != (ScriptOutcome::Output(ScriptOutput {
                stdout: input.stdin.clone(),
                stderr: Vec::new(),
                exit_code: 0,
            }))
        {
            return Err("large-call cleanup mismatch".into());
        }
    }

    // Boundary-size evidence: 1,024 arguments and an 8 MiB channel.
    let boundary_input = ScriptInput {
        arguments: (0..1024).map(|index| format!("arg-{index:04}")).collect(),
        stdin: rng.bytes(8 * 1024 * 1024),
    };
    let output = call(&mut store, &echo_run, &boundary_input)?;
    if output
        != (ScriptOutcome::Output(ScriptOutput {
            stdout: boundary_input.stdin.clone(),
            stderr: Vec::new(),
            exit_code: 0,
        }))
    {
        return Err("boundary-size case mismatch".into());
    }

    let error_instance = linker.instantiate(&mut store, &error_component)?;
    let error_run = error_instance
        .get_func(&mut store, "run")
        .ok_or("error component does not export run")?;
    let output = call(&mut store, &error_run, &ScriptInput::default())?;
    if output
        != (ScriptOutcome::Error {
            code: "domain-error".to_owned(),
            message: "rejected".to_owned(),
        })
    {
        return Err(format!("error fixture mismatch: {output:?}").into());
    }

    let mut malicious = BTreeMap::new();
    for (name, component) in malicious_components() {
        let component = Component::new(&engine, &component)
            .map_err(|error| format!("{name}: malicious component must validate: {error}"))?;
        let instance = linker.instantiate(&mut store, &component)?;
        let run = instance
            .get_func(&mut store, "run")
            .ok_or("malicious component does not export run")?;
        match call(&mut store, &run, &ScriptInput::default()) {
            Ok(value) => {
                return Err(format!("{name}: malformed memory was accepted: {value:?}").into());
            }
            Err(error) => {
                malicious.insert(
                    name.to_owned(),
                    format!("{error}").chars().take(160).collect::<String>(),
                );
            }
        }
    }

    let report = serde_json::json!({
        "harness": "script-abi-roundtrip-v0",
        "wasmtime": "46.0.1",
        "echo_component": {
            "bytes": echo_bytes.len(),
            "sha256": hex_digest(&echo_bytes),
        },
        "error_component": {
            "bytes": error_bytes.len(),
            "sha256": hex_digest(&error_bytes),
        },
        "seeded_roundtrips": ROUNDTRIPS,
        "cleanup_oracle": {
            "one_mib_calls": CLEANUP_CALLS,
            "four_mib_calls": 8,
            "single_instance": true,
        },
        "boundary_case": { "arguments": 1024, "stdin_bytes": 8 * 1024 * 1024 },
        "error_fixture": "domain-error/rejected",
        "malicious_fail_closed": malicious,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ScriptInput {
    arguments: Vec<String>,
    stdin: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScriptOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ScriptOutcome {
    Output(ScriptOutput),
    Error { code: String, message: String },
}

fn call(
    store: &mut Store<()>,
    run: &wasmtime::component::Func,
    input: &ScriptInput,
) -> AnyResult<ScriptOutcome> {
    // Wasmtime accounts canonical-ABI transport against per-Store hostcall
    // fuel: lowering charges len * size_of::<T>(), lifting charges
    // len * size_of::<Val>() per element. Top up to the engine default
    // (128 Mi) or the measured need of this call, whichever is larger.
    let needed = input
        .stdin
        .len()
        .saturating_mul(64)
        .saturating_add(64 << 20);
    store.set_hostcall_fuel(needed.max(128 << 20));
    let params = [Val::Record(vec![
        (
            "arguments".to_owned(),
            Val::List(input.arguments.iter().cloned().map(Val::String).collect()),
        ),
        (
            "stdin".to_owned(),
            Val::List(input.stdin.iter().copied().map(Val::U8).collect()),
        ),
    ])];
    let mut results = [Val::Result(Ok(None))];
    run.call(&mut *store, &params, &mut results)?;
    result_from_val(&results[0])
}

fn result_from_val(value: &Val) -> AnyResult<ScriptOutcome> {
    match value {
        Val::Result(Ok(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err("script output is not a record".into());
            };
            let stdout = bytes_field(fields, "stdout")?;
            let stderr = bytes_field(fields, "stderr")?;
            let exit_code = match field(fields, "exit-code")? {
                Val::S64(exit) => *exit,
                _ => return Err("exit-code is not s64".into()),
            };
            Ok(ScriptOutcome::Output(ScriptOutput {
                stdout,
                stderr,
                exit_code,
            }))
        }
        Val::Result(Err(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err("script error is not a record".into());
            };
            let code = match field(fields, "code")? {
                Val::Enum(code) => code.clone(),
                _ => return Err("code is not an enum".into()),
            };
            let message = match field(fields, "message")? {
                Val::String(message) => message.clone(),
                _ => return Err("message is not a string".into()),
            };
            Ok(ScriptOutcome::Error { code, message })
        }
        _ => Err("script result has an invalid shape".into()),
    }
}

fn field<'a>(fields: &'a [(String, Val)], name: &str) -> AnyResult<&'a Val> {
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .ok_or_else(|| format!("missing field {name}").into())
}

fn bytes_field(fields: &[(String, Val)], name: &str) -> AnyResult<Vec<u8>> {
    let Val::List(values) = field(fields, name)? else {
        return Err(format!("{name} is not a list").into());
    };
    values
        .iter()
        .map(|value| match value {
            Val::U8(byte) => Ok(*byte),
            _ => Err(format!("{name} item is not u8").into()),
        })
        .collect()
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next() & 0xff) as u8).collect()
    }

    fn text(&mut self, len: usize) -> String {
        let mut output = String::new();
        while output.len() < len {
            let candidate = match self.below(5) {
                0 => char::from_u32(0x61 + self.below(26) as u32),
                1 => char::from_u32(0x4e2d + self.below(0x100) as u32),
                2 => char::from_u32(0x1f600 + self.below(0x50) as u32),
                3 => Some('\u{0}'),
                _ => Some('%'),
            };
            if let Some(candidate) = candidate {
                output.push(candidate);
            }
        }
        output
    }

    fn input(&mut self, max_args: usize, max_arg_len: usize, max_stdin: usize) -> ScriptInput {
        let count = self.below(max_args + 1);
        let arguments = (0..count)
            .map(|_| {
                let len = self.below(max_arg_len + 1);
                self.text(len)
            })
            .collect();
        let stdin_len = self.below(max_stdin + 1);
        let stdin = self.bytes(stdin_len);
        ScriptInput { arguments, stdin }
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn script_result() -> Type {
    Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    }
}

fn run_function(instructions: Vec<IrInstruction>, result: ValueId) -> Function {
    let range = SourceRange { start: 0, end: 0 };
    Function {
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
        blocks: vec![Block {
            id: BlockId(0),
            instructions,
            terminator: Terminator::Return(Some(result)),
            range,
        }],
        range,
    }
}

/// `run` echoing stdin to stdout with empty stderr and exit code zero.
fn echo_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("echo.sico", 0);
    module.functions.push(run_function(
        vec![
            IrInstruction {
                result: ValueId(1),
                ty: Type::Bytes,
                operation: Operation::Project {
                    base: ValueId(0),
                    field: "stdin".into(),
                },
                range,
            },
            IrInstruction {
                result: ValueId(2),
                ty: Type::Bytes,
                operation: Operation::ConstBytes(Vec::new()),
                range,
            },
            IrInstruction {
                result: ValueId(3),
                ty: Type::I64,
                operation: Operation::ConstI64(0),
                range,
            },
            IrInstruction {
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
            IrInstruction {
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

/// `run` returning `ScriptError { code: domain-error, message: "rejected" }`.
fn error_module() -> Module {
    let range = SourceRange { start: 0, end: 0 };
    let mut module = Module::new("error.sico", 0);
    module.functions.push(run_function(
        vec![
            IrInstruction {
                result: ValueId(1),
                ty: Type::Named("ScriptErrorCode".into()),
                operation: Operation::Variant {
                    name: "ScriptErrorCode.DomainError".into(),
                    payload: Vec::new(),
                },
                range,
            },
            IrInstruction {
                result: ValueId(2),
                ty: Type::String,
                operation: Operation::ConstString("rejected".into()),
                range,
            },
            IrInstruction {
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
            IrInstruction {
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
    module
}

/// Malicious core guests: constant result areas with one defect each. Every
/// defect must be rejected by the engine's Canonical ABI lift.
fn malicious_components() -> Vec<(&'static str, Vec<u8>)> {
    let mut fixtures = Vec::new();
    // tag = 2: invalid result discriminant.
    fixtures.push((
        "invalid-result-tag",
        malicious_core(&[(1024, 2, 1)], &[], 1024),
    ));
    // err tag with enum code = 9: invalid enum discriminant.
    fixtures.push((
        "invalid-enum-tag",
        malicious_core(
            &[(1024, 1, 1), (1032, 9, 1), (1036, 2048, 4), (1040, 1, 4)],
            b"x",
            1024,
        ),
    ));
    // err tag with invalid UTF-8 message bytes.
    fixtures.push((
        "invalid-utf8-message",
        malicious_core(
            &[(1024, 1, 1), (1032, 0, 1), (1036, 2048, 4), (1040, 2, 4)],
            &[0xff, 0xfe],
            1024,
        ),
    ));
    // ok tag with an out-of-bounds stdout pointer.
    fixtures.push((
        "out-of-bounds-payload",
        malicious_core(
            &[
                (1024, 0, 1),
                (1032, 0xffff_fff0, 4),
                (1036, 16, 4),
                (1040, 0, 4),
                (1044, 0, 4),
            ],
            &[],
            1024,
        ),
    ));
    // ok tag with an overflowing stdout length.
    fixtures.push((
        "overflowing-length",
        malicious_core(
            &[
                (1024, 0, 1),
                (1032, 2048, 4),
                (1036, 0xffff_ffff, 4),
                (1040, 0, 4),
                (1044, 0, 4),
            ],
            &[],
            1024,
        ),
    ));
    // A well-formed result area returned at a misaligned address.
    fixtures.push((
        "misaligned-result-area",
        malicious_core(
            &[
                (1024, 0, 1),
                (1032, 0, 4),
                (1036, 0, 4),
                (1040, 0, 4),
                (1044, 0, 4),
            ],
            &[],
            1025,
        ),
    ));
    fixtures
        .into_iter()
        .map(|(name, core)| {
            (
                name,
                sico_codegen_wasm::wrap_script_component(
                    &core,
                    sico_codegen_wasm::FsUse::default(),
                    sico_codegen_wasm::StreamUse::default(),
                    false,
                    0,
                ),
            )
        })
        .collect()
}

/// Emits a core module exporting `memory`, `cabi_realloc`, `cabi_post_run`
/// and a `run` that stores constant scalars at constant offsets (a 32-byte
/// result area at 1024, message bytes in a data segment at 2048) and returns
/// the given pointer. `(offset, value, width)` entries are little-endian.
fn malicious_core(writes: &[(u32, u32, u32)], data: &[u8], result_pointer: u32) -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.ty().function([ValType::I32], []);
    let mut functions = FunctionSection::new();
    functions.function(0);
    functions.function(1);
    functions.function(2);

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
    for (offset, value, width) in writes {
        run.instruction(&Instruction::I32Const(0));
        run.instruction(&Instruction::I32Const(i32::from_le_bytes(
            value.to_le_bytes(),
        )));
        let store = match width {
            1 => Instruction::I32Store8(MemArg {
                offset: u64::from(*offset),
                align: 0,
                memory_index: 0,
            }),
            _ => Instruction::I32Store(MemArg {
                offset: u64::from(*offset),
                align: 2,
                memory_index: 0,
            }),
        };
        run.instruction(&store);
    }
    run.instruction(&Instruction::I32Const(i32::from_le_bytes(
        result_pointer.to_le_bytes(),
    )));
    run.instruction(&Instruction::End);

    let mut realloc = WasmFunction::new(vec![(1, ValType::I32)]);
    realloc.instruction(&Instruction::GlobalGet(0));
    realloc.instruction(&Instruction::LocalGet(2));
    realloc.instruction(&Instruction::I32Add);
    realloc.instruction(&Instruction::I32Const(-1));
    realloc.instruction(&Instruction::I32Add);
    realloc.instruction(&Instruction::I32Const(0));
    realloc.instruction(&Instruction::LocalGet(2));
    realloc.instruction(&Instruction::I32Sub);
    realloc.instruction(&Instruction::I32And);
    realloc.instruction(&Instruction::LocalTee(4));
    realloc.instruction(&Instruction::LocalGet(3));
    realloc.instruction(&Instruction::I32Add);
    realloc.instruction(&Instruction::GlobalSet(0));
    realloc.instruction(&Instruction::LocalGet(4));
    realloc.instruction(&Instruction::End);

    let mut post_return = WasmFunction::new(Vec::new());
    post_return.instruction(&Instruction::I32Const(4096));
    post_return.instruction(&Instruction::GlobalSet(0));
    post_return.instruction(&Instruction::End);

    let mut code = CodeSection::new();
    code.function(&run);
    code.function(&realloc);
    code.function(&post_return);

    let mut module = CoreModule::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    if !data.is_empty() {
        let mut data_section = DataSection::new();
        data_section.segment(DataSegment {
            mode: DataSegmentMode::Active {
                memory_index: 0,
                offset: &ConstExpr::i32_const(2048),
            },
            data: data.to_vec(),
        });
        module.section(&data_section);
    }
    module.finish()
}
