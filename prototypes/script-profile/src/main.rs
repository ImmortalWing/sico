use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use wasm_encoder::{
    ComponentBuilder, ComponentExportKind, ComponentTypeRef, ComponentValType, PrimitiveValType,
};
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

const STDIN_LIMIT: usize = 8 * 1024 * 1024;
const WARM_ITERATIONS: usize = 40;

type AnyError = Box<dyn Error + Send + Sync>;
type AnyResult<T> = Result<T, AnyError>;

#[derive(Debug, Deserialize)]
struct CaseFile {
    cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    JoinArgs,
    EchoStdin,
    SplitChannels,
    ScriptError,
}

#[derive(Clone, Debug, Deserialize)]
struct Case {
    name: String,
    mode: Mode,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    stdin_hex: String,
    stdin_pattern_hex: Option<String>,
    stdin_size: Option<usize>,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize)]
struct Expected {
    stdout_hex: Option<String>,
    stderr_hex: Option<String>,
    exit: Option<i64>,
    error_kind: Option<String>,
    error_message: Option<String>,
    echo_stdin: Option<bool>,
    runner_error: Option<String>,
    tool_exit: Option<u8>,
}

#[derive(Clone, Debug)]
struct ScriptInput {
    args: Vec<String>,
    stdin: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScriptResult {
    Output {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        exit: i64,
    },
    Error {
        kind: String,
        message: String,
    },
}

fn main() -> AnyResult<()> {
    let case_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cases.json");
    let cases: CaseFile = serde_json::from_slice(&fs::read(case_path)?)?;
    let component_bytes = build_program_component();

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    let cold_started = Instant::now();
    let component = Component::new(&engine, &component_bytes)?;
    let cold_compile = cold_started.elapsed();

    let mut passed = 0usize;
    for case in &cases.cases {
        run_case(&engine, &component, case)?;
        passed += 1;
    }

    let benchmark_case = cases
        .cases
        .iter()
        .find(|case| case.name == "binary-stdin")
        .ok_or("missing binary-stdin benchmark case")?;
    let benchmark_input = case_input(benchmark_case)?;
    let mut warm_samples = Vec::with_capacity(WARM_ITERATIONS);
    for _ in 0..WARM_ITERATIONS {
        let started = Instant::now();
        let result = invoke(
            &engine,
            &component,
            benchmark_case.mode.clone(),
            &benchmark_input,
        )?;
        validate_result(benchmark_case, &benchmark_input.stdin, &result)?;
        warm_samples.push(started.elapsed());
    }
    warm_samples.sort_unstable();

    let report = serde_json::json!({
        "prototype": "script-profile-direct-runner-v0",
        "wasmtime": "46.0.1",
        "component_bytes": component_bytes.len(),
        "cases_passed": passed,
        "cases_total": cases.cases.len(),
        "cold_component_compile_ms": milliseconds(cold_compile),
        "warm_instantiate_call_median_ms": milliseconds(percentile(&warm_samples, 50)),
        "warm_instantiate_call_p95_ms": milliseconds(percentile(&warm_samples, 95)),
        "warm_iterations": WARM_ITERATIONS,
        "scope": "dynamic Component values through a direct in-process Wasmtime runner; no guest core Canonical ABI and no adapter composition"
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn build_program_component() -> Vec<u8> {
    let mut builder = ComponentBuilder::default();
    let (args_ty, args) = builder.type_defined(Some("args"));
    args.list(PrimitiveValType::String);
    let (bytes_ty, bytes) = builder.type_defined(Some("bytes"));
    bytes.list(PrimitiveValType::U8);
    let (input_ty, input) = builder.type_defined(Some("script-input"));
    input.record([
        ("args", ComponentValType::Type(args_ty)),
        ("stdin", ComponentValType::Type(bytes_ty)),
    ]);
    let (output_ty, output) = builder.type_defined(Some("script-output"));
    output.record([
        ("stdout", ComponentValType::Type(bytes_ty)),
        ("stderr", ComponentValType::Type(bytes_ty)),
        ("exit", ComponentValType::Primitive(PrimitiveValType::S64)),
    ]);
    let (error_kind_ty, error_kind) = builder.type_defined(Some("script-error-kind"));
    error_kind.enum_type([
        "domain",
        "invalid-input",
        "permission-denied",
        "resource-limit",
        "internal",
    ]);
    let (error_ty, script_error) = builder.type_defined(Some("script-error"));
    script_error.record([
        ("kind", ComponentValType::Type(error_kind_ty)),
        (
            "message",
            ComponentValType::Primitive(PrimitiveValType::String),
        ),
    ]);
    let (result_ty, result) = builder.type_defined(Some("script-result"));
    result.result(
        Some(ComponentValType::Type(output_ty)),
        Some(ComponentValType::Type(error_ty)),
    );
    let (run_ty, mut run) = builder.type_function(Some("run"));
    run.params([("input", ComponentValType::Type(input_ty))]);
    run.result(Some(ComponentValType::Type(result_ty)));
    let imported = builder.import("host-run", ComponentTypeRef::Func(run_ty));
    builder.export("run", ComponentExportKind::Func, imported, None);
    builder.finish()
}

fn run_case(engine: &Engine, component: &Component, case: &Case) -> AnyResult<()> {
    let input = case_input(case)?;
    if input.stdin.len() > STDIN_LIMIT {
        if case.expected.runner_error.as_deref() != Some("stdin-limit")
            || case.expected.tool_exit != Some(125)
        {
            return Err(format!("{}: unexpected stdin limit expectation", case.name).into());
        }
        return Ok(());
    }
    let result = invoke(engine, component, case.mode.clone(), &input)?;
    validate_result(case, &input.stdin, &result)
}

fn invoke(
    engine: &Engine,
    component: &Component,
    mode: Mode,
    input: &ScriptInput,
) -> AnyResult<ScriptResult> {
    let mut linker = Linker::<()>::new(engine);
    linker
        .root()
        .func_new("host-run", move |_store, _ty, params, results| {
            let input = val_to_input(&params[0])?;
            results[0] = result_to_val(evaluate(&mode, input));
            Ok(())
        })?;
    let mut store = Store::new(engine, ());
    let instance = linker.instantiate(&mut store, component)?;
    let run = instance
        .get_func(&mut store, "run")
        .ok_or("component does not export run")?;
    let params = [input_to_val(input)];
    let mut results = [Val::Result(Ok(None))];
    run.call(&mut store, &params, &mut results)?;
    val_to_result(&results[0])
}

fn evaluate(mode: &Mode, input: ScriptInput) -> ScriptResult {
    match mode {
        Mode::JoinArgs => ScriptResult::Output {
            stdout: input.args.join("|").into_bytes(),
            stderr: Vec::new(),
            exit: 0,
        },
        Mode::EchoStdin => ScriptResult::Output {
            stdout: input.stdin,
            stderr: Vec::new(),
            exit: 0,
        },
        Mode::SplitChannels => ScriptResult::Output {
            stdout: input.stdin,
            stderr: b"warning".to_vec(),
            exit: 7,
        },
        Mode::ScriptError => ScriptResult::Error {
            kind: "invalid-input".to_owned(),
            message: "fixture rejected input".to_owned(),
        },
    }
}

fn case_input(case: &Case) -> AnyResult<ScriptInput> {
    let stdin = if let Some(size) = case.stdin_size {
        let pattern = decode_hex(case.stdin_pattern_hex.as_deref().unwrap_or_default())?;
        if pattern.is_empty() && size != 0 {
            return Err(format!("{}: non-empty generated stdin needs a pattern", case.name).into());
        }
        pattern.into_iter().cycle().take(size).collect()
    } else {
        decode_hex(&case.stdin_hex)?
    };
    Ok(ScriptInput {
        args: case.args.clone(),
        stdin,
    })
}

fn input_to_val(input: &ScriptInput) -> Val {
    Val::Record(vec![
        (
            "args".to_owned(),
            Val::List(input.args.iter().cloned().map(Val::String).collect()),
        ),
        (
            "stdin".to_owned(),
            Val::List(input.stdin.iter().copied().map(Val::U8).collect()),
        ),
    ])
}

fn result_to_val(result: ScriptResult) -> Val {
    match result {
        ScriptResult::Output {
            stdout,
            stderr,
            exit,
        } => Val::Result(Ok(Some(Box::new(Val::Record(vec![
            (
                "stdout".to_owned(),
                Val::List(stdout.into_iter().map(Val::U8).collect()),
            ),
            (
                "stderr".to_owned(),
                Val::List(stderr.into_iter().map(Val::U8).collect()),
            ),
            ("exit".to_owned(), Val::S64(exit)),
        ]))))),
        ScriptResult::Error { kind, message } => {
            Val::Result(Err(Some(Box::new(Val::Record(vec![
                ("kind".to_owned(), Val::Enum(kind)),
                ("message".to_owned(), Val::String(message)),
            ])))))
        }
    }
}

fn val_to_input(value: &Val) -> wasmtime::Result<ScriptInput> {
    let Val::Record(fields) = value else {
        return Err(wasmtime::Error::msg("script input is not a record"));
    };
    let args = field(fields, "args")?;
    let stdin = field(fields, "stdin")?;
    Ok(ScriptInput {
        args: val_to_strings(args)?,
        stdin: val_to_bytes(stdin)?,
    })
}

fn val_to_result(value: &Val) -> AnyResult<ScriptResult> {
    match value {
        Val::Result(Ok(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err("script output is not a record".into());
            };
            let exit = match field(fields, "exit")? {
                Val::S64(exit) => *exit,
                _ => return Err("script output exit is not s64".into()),
            };
            Ok(ScriptResult::Output {
                stdout: val_to_bytes(field(fields, "stdout")?)?,
                stderr: val_to_bytes(field(fields, "stderr")?)?,
                exit,
            })
        }
        Val::Result(Err(Some(value))) => {
            let Val::Record(fields) = value.as_ref() else {
                return Err("script error is not a record".into());
            };
            let kind = match field(fields, "kind")? {
                Val::Enum(kind) => kind.clone(),
                _ => return Err("script error kind is not an enum".into()),
            };
            let message = match field(fields, "message")? {
                Val::String(message) => message.clone(),
                _ => return Err("script error message is not text".into()),
            };
            Ok(ScriptResult::Error { kind, message })
        }
        _ => Err("script result has an invalid shape".into()),
    }
}

fn field<'a>(fields: &'a [(String, Val)], name: &str) -> wasmtime::Result<&'a Val> {
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .ok_or_else(|| wasmtime::Error::msg(format!("missing field {name}")))
}

fn val_to_strings(value: &Val) -> wasmtime::Result<Vec<String>> {
    let Val::List(values) = value else {
        return Err(wasmtime::Error::msg("args is not a list"));
    };
    values
        .iter()
        .map(|value| match value {
            Val::String(value) => Ok(value.clone()),
            _ => Err(wasmtime::Error::msg("args item is not text")),
        })
        .collect()
}

fn val_to_bytes(value: &Val) -> wasmtime::Result<Vec<u8>> {
    let Val::List(values) = value else {
        return Err(wasmtime::Error::msg("bytes is not a list"));
    };
    values
        .iter()
        .map(|value| match value {
            Val::U8(value) => Ok(*value),
            _ => Err(wasmtime::Error::msg("bytes item is not u8")),
        })
        .collect()
}

fn validate_result(case: &Case, stdin: &[u8], result: &ScriptResult) -> AnyResult<()> {
    match result {
        ScriptResult::Output {
            stdout,
            stderr,
            exit,
        } => {
            let expected_stdout = if case.expected.echo_stdin == Some(true) {
                stdin.to_vec()
            } else {
                decode_hex(case.expected.stdout_hex.as_deref().unwrap_or_default())?
            };
            let expected_stderr =
                decode_hex(case.expected.stderr_hex.as_deref().unwrap_or_default())?;
            if stdout != &expected_stdout
                || stderr != &expected_stderr
                || Some(*exit) != case.expected.exit
            {
                return Err(format!("{}: output mismatch", case.name).into());
            }
        }
        ScriptResult::Error { kind, message } => {
            if Some(kind.as_str()) != case.expected.error_kind.as_deref()
                || Some(message.as_str()) != case.expected.error_message.as_deref()
            {
                return Err(format!("{}: error mismatch", case.name).into());
            }
        }
    }
    Ok(())
}

fn decode_hex(value: &str) -> AnyResult<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex input has odd length".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| Ok(u8::from_str_radix(&value[index..index + 2], 16)?))
        .collect()
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let index = ((samples.len() - 1) * percentile).div_ceil(100);
    samples[index]
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
