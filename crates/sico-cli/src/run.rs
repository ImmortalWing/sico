//! `sico run` and `sico eval`: compile through the Script profile, execute
//! through `sico-runner` across the executable boundary, and cache compiled
//! Program Components under the frozen RFC-0029 cache identity.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::PathBuf,
    process::{Command as ProcessCommand, Stdio},
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;
use sha2::{Digest, Sha256};
use sico_ir::{Operation, Terminator, Type, ValueId};
use sico_package::compose;
use sico_source::{SourceFile, SourceId};

use crate::cache::{CacheKeyParts, SourceCache, cache_key};
use crate::{EXIT_TOOL_ERROR, compile_source_bytes, lower_error, read_input_bytes};

pub const EXIT_COMPILE_DIAGNOSTIC: i32 = 120;
pub const EXIT_CLI_FAILURE: i32 = 121;
pub const EXIT_RESOURCE_LIMIT: i32 = 125;
pub const EXIT_RUNNER_INCOMPATIBLE: i32 = 127;

const MAX_GUEST_STDIN: usize = 8 * 1024 * 1024;

pub fn run_command() -> Command {
    Command::new("run")
        .about("Compile and execute a Script source through sico-runner")
        .arg(
            Arg::new("input")
                .value_name("FILE|-")
                .help("Script source file, or - to read source from stdin (guest stdin is then empty)")
                .required(true)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit JSON tool diagnostics on stderr")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("fs-read-root")
                .long("fs-read-root")
                .value_name("PATH")
                .help("Grant a storage.read root for sico.fs.read/exists (repeatable)")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("fs-write-root")
                .long("fs-write-root")
                .value_name("PATH")
                .help("Grant a storage.write root for sico.fs.write (repeatable)")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("args")
                .value_name("ARG")
                .help("Script arguments after --")
                .num_args(0..)
                .last(true)
                .allow_hyphen_values(true),
        )
}

pub fn eval_command() -> Command {
    Command::new("eval")
        .about("Evaluate a compile-time constant Int expression as a Script")
        .arg(
            Arg::new("expression")
                .value_name("EXPR")
                .help("Compile-time constant Int expression")
                .required(true),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit JSON tool diagnostics on stderr")
                .action(ArgAction::SetTrue),
        )
}

pub fn run_run(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let json = matches.get_flag("json");
    let input = matches.get_one::<String>("input").unwrap();
    let arguments: Vec<String> = matches
        .get_many::<String>("args")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();
    let (name, source) = match read_input_bytes(input, stdin) {
        Ok(source) => source,
        Err(message) => return tool_failure(stderr, json, "cli", &message),
    };
    let identity = match CacheIdentity::new(&source) {
        Ok(identity) => identity,
        Err(message) => return tool_failure(stderr, json, "cli", &message),
    };
    let component_path = match identity.cached_component(&name, &source, json, stdout, stderr) {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    // RFC-0030 mixing rule: when the compiled component imports the streams
    // interface, the CLI never buffers stdin — the runner inherits it so
    // backpressure reaches the producer; the 8 MiB bound does not apply.
    let streams_component = component_imports_streams(&component_path);
    let guest_stdin = if streams_component || input == "-" {
        Vec::new()
    } else {
        let mut bytes = Vec::new();
        if let Err(error) = stdin
            .take((MAX_GUEST_STDIN + 1) as u64)
            .read_to_end(&mut bytes)
        {
            return tool_failure(
                stderr,
                json,
                "cli",
                &format!("sico: cannot read stdin: {error}"),
            );
        }
        if bytes.len() > MAX_GUEST_STDIN {
            return tool_failure(
                stderr,
                json,
                "resource-limit.input",
                "sico: stdin exceeds the 8 MiB Script v0 bound",
            );
        }
        bytes
    };
    let mut fs_flags: Vec<String> = Vec::new();
    for (flag, values) in [
        ("--fs-read-root", matches.get_many::<String>("fs-read-root")),
        (
            "--fs-write-root",
            matches.get_many::<String>("fs-write-root"),
        ),
    ] {
        if let Some(values) = values {
            for value in values {
                fs_flags.push(flag.to_owned());
                fs_flags.push(value.clone());
            }
        }
    }
    execute_runner(
        &component_path,
        &fs_flags,
        &arguments,
        &guest_stdin,
        streams_component,
        json,
        stdout,
        stderr,
    )
}

/// True when the compiled component imports `sico:script/streams@0.1.0`.
/// An unreadable artifact reports false and fails later at the runner.
fn component_imports_streams(component: &std::path::Path) -> bool {
    let Ok(bytes) = std::fs::read(component) else {
        return false;
    };
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        match payload {
            Ok(wasmparser::Payload::ComponentImportSection(section)) => {
                for import in section.into_iter().flatten() {
                    if import.name.name == "sico:script/streams@0.1.0" {
                        return true;
                    }
                }
            }
            Ok(wasmparser::Payload::End(_)) => break,
            Err(_) => return false,
            _ => {}
        }
    }
    false
}

pub fn run_eval(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let json = matches.get_flag("json");
    let expression = matches.get_one::<String>("expression").unwrap();
    let value = match eval_constant(expression) {
        Ok(value) => value,
        Err(message) => return tool_failure(stderr, json, "compile", &message),
    };
    let mut synthesized = b"eval\0".to_vec();
    synthesized.extend_from_slice(expression.as_bytes());
    let identity = match CacheIdentity::new(&synthesized) {
        Ok(identity) => identity,
        Err(message) => return tool_failure(stderr, json, "cli", &message),
    };
    let component_path = match identity.cached_component_eval(value, json, stderr) {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    execute_runner(&component_path, &[], &[], &[], false, json, stdout, stderr)
}

/// Evaluates a compile-time constant `Int` expression through the scalar
/// frontend, refusing anything else without executing code.
fn eval_constant(expression: &str) -> Result<i64, String> {
    let text = format!("function main() returns Int:\n  return {expression}\nend function\n");
    let source = SourceFile::from_text(SourceId::new(0), "eval.sico", text)
        .map_err(|error| format!("sico: eval source contract error {error:?}"))?;
    let module = sico_ir::lower_core(&source).map_err(|error| {
        format!(
            "sico: eval expression is not valid: {}",
            lower_error(&error)
        )
    })?;
    let mut constants: BTreeMap<ValueId, i64> = BTreeMap::new();
    let mut result = None;
    for function in &module.functions {
        if function.name != "main" {
            return Err("sico: eval synthesizes exactly one main function".to_owned());
        }
        for block in &function.blocks {
            for instruction in &block.instructions {
                let value = match &instruction.operation {
                    Operation::ConstInt(value) => value
                        .parse::<i64>()
                        .map_err(|_| "sico: eval integer literal is outside i64".to_owned())?,
                    Operation::Copy(source) => {
                        *constants.get(source).ok_or_else(unsupported_eval)?
                    }
                    Operation::AddInt { left, right } => constants
                        .get(left)
                        .and_then(|left| {
                            constants
                                .get(right)
                                .and_then(|right| left.checked_add(*right))
                        })
                        .ok_or_else(unsupported_eval)?,
                    _ => return Err(unsupported_eval()),
                };
                constants.insert(instruction.result, value);
            }
            if let Terminator::Return(Some(value)) = block.terminator {
                result = constants.get(&value).copied();
            }
        }
    }
    result.ok_or_else(unsupported_eval)
}

fn unsupported_eval() -> String {
    "sico: eval v0 supports compile-time constant Int expressions only".to_owned()
}

struct CacheIdentity {
    key: [u8; 32],
}

impl CacheIdentity {
    fn new(source: &[u8]) -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("sico: cannot locate the compiler executable: {error}"))?;
        let executable_bytes = std::fs::read(&executable)
            .map_err(|error| format!("sico: cannot read the compiler executable: {error}"))?;
        let compiler_digest: [u8; 32] = Sha256::digest(&executable_bytes).into();
        let adapter = compose::script_adapter_component();
        let adapter_digest: [u8; 32] = Sha256::digest(&adapter).into();
        Ok(Self {
            key: cache_key(&CacheKeyParts {
                compiler_digest: &compiler_digest,
                compiler_build_id: env!("CARGO_PKG_VERSION"),
                semantics_id: sico_ir::SCHEMA,
                script_wit_version: "sico:script@0.1.0",
                adapter_digest: &adapter_digest,
                source,
                app_id: "sico",
                app_version: env!("CARGO_PKG_VERSION"),
                profile_id: "script-v0",
                manifest_schema: sico_package::MANIFEST_SCHEMA_V1,
                codegen_options: &[],
            }),
        })
    }

    fn cached_component(
        &self,
        name: &str,
        source: &[u8],
        json: bool,
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
    ) -> Result<PathBuf, i32> {
        let Some(cache) = SourceCache::open() else {
            return compile_and_cache(None, &self.key, || {
                compile_source_bytes(name, source, "script-v0", stdout, stderr)
            });
        };
        match cache.get(&self.key) {
            Ok(Some(_)) => Ok(cache.entry_for(&self.key)),
            Ok(None) => compile_and_cache(Some(&cache), &self.key, || {
                compile_source_bytes(name, source, "script-v0", stdout, stderr)
            }),
            Err(error) => Err(cache_failure(stderr, json, &error)),
        }
    }

    fn cached_component_eval(
        &self,
        value: i64,
        json: bool,
        stderr: &mut dyn Write,
    ) -> Result<PathBuf, i32> {
        let Some(cache) = SourceCache::open() else {
            return compile_and_cache(None, &self.key, || compile_eval_component(value, stderr));
        };
        match cache.get(&self.key) {
            Ok(Some(_)) => Ok(cache.entry_for(&self.key)),
            Ok(None) => compile_and_cache(Some(&cache), &self.key, || {
                compile_eval_component(value, stderr)
            }),
            Err(error) => Err(cache_failure(stderr, json, &error)),
        }
    }
}

fn compile_and_cache(
    cache: Option<&SourceCache>,
    key: &[u8; 32],
    compile: impl FnOnce() -> Result<Vec<u8>, i32>,
) -> Result<PathBuf, i32> {
    let component = compile()?;
    if let Some(cache) = cache {
        cache
            .put(key, &component)
            .map_err(|error| cache_failure(&mut std::io::stderr(), false, &error))
    } else {
        let temporary = std::env::temp_dir().join(format!(
            "sico-run-{}-{}.component.wasm",
            std::process::id(),
            key.iter().take(4).fold(String::new(), |mut out, byte| {
                use std::fmt::Write as _;
                write!(out, "{byte:02x}").expect("writing to a String cannot fail");
                out
            })
        ));
        if let Err(error) = std::fs::write(&temporary, &component) {
            let _ = writeln!(std::io::stderr(), "sico: cannot write component: {error}");
            return Err(EXIT_TOOL_ERROR);
        }
        Ok(temporary)
    }
}

fn cache_failure(stderr: &mut dyn Write, json: bool, error: &crate::cache::CacheError) -> i32 {
    tool_failure(
        stderr,
        json,
        "cache",
        &format!("sico: source cache failure: {error:?}"),
    )
}

fn compile_eval_component(value: i64, stderr: &mut dyn Write) -> Result<Vec<u8>, i32> {
    use sico_ir::{
        Block, BlockId, ConstructField, Function, FunctionId, Instruction, Module, Operation,
        Parameter, SourceRange, Terminator,
    };
    let range = SourceRange { start: 0, end: 0 };
    let script_result = || Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    };
    let text = value.to_string();
    let mut module = Module::new("eval.sico", 0);
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
        blocks: vec![Block {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(1),
                    ty: Type::Bytes,
                    operation: Operation::ConstBytes(text.into_bytes()),
                    range,
                },
                Instruction {
                    result: ValueId(2),
                    ty: Type::Bytes,
                    operation: Operation::ConstBytes(Vec::new()),
                    range,
                },
                Instruction {
                    result: ValueId(3),
                    ty: Type::I64,
                    operation: Operation::ConstI64(0),
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
            terminator: Terminator::Return(Some(ValueId(5))),
            range,
        }],
        range,
    });
    match sico_codegen_wasm::compile_script_program(&module) {
        Ok(component) => Ok(component),
        Err(error) => {
            let _ = writeln!(stderr, "sico: eval Component generation failed: {error:?}");
            Err(EXIT_TOOL_ERROR)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_runner(
    component: &std::path::Path,
    fs_flags: &[String],
    arguments: &[String],
    guest_stdin: &[u8],
    streams_component: bool,
    json: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let Some(runner) = find_runner() else {
        return tool_failure(
            stderr,
            json,
            "incompatible",
            "sico: sico-runner executable not found (SICO_RUNNER, sibling, or PATH)",
        );
    };
    let mut command = ProcessCommand::new(runner);
    command.args(fs_flags);
    command.arg(component);
    if !arguments.is_empty() {
        command.arg("--");
        command.args(arguments);
    }
    // Streaming components inherit stdin so backpressure reaches the
    // producer; buffered components receive one bounded pipe.
    if streams_component {
        command.stdin(Stdio::inherit());
    } else {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return tool_failure(
                stderr,
                json,
                "incompatible",
                &format!("sico: cannot launch sico-runner: {error}"),
            );
        }
    };
    if !streams_component
        && let Some(mut stdin) = child.stdin.take()
        && stdin.write_all(guest_stdin).is_err()
    {
        let _ = child.kill();
    }
    let _ = stdout.flush();
    match child.wait() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNNER_INCOMPATIBLE),
        Err(error) => tool_failure(
            stderr,
            json,
            "cli",
            &format!("sico: runner wait failed: {error}"),
        ),
    }
}

fn find_runner() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SICO_RUNNER") {
        // An explicit override is authoritative: a broken path fails closed
        // at launch instead of silently falling through to another runner.
        return Some(PathBuf::from(path));
    }
    if let Ok(executable) = std::env::current_exe() {
        for name in ["sico-runner.exe", "sico-runner"] {
            let sibling = executable.with_file_name(name);
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    which("sico-runner")
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let executable = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&path)
        .map(|dir| dir.join(&executable))
        .find(|candidate| candidate.is_file())
}

fn tool_failure(stderr: &mut dyn Write, json: bool, class: &str, message: &str) -> i32 {
    let exit = match class {
        "compile" => EXIT_COMPILE_DIAGNOSTIC,
        "resource-limit.input" => EXIT_RESOURCE_LIMIT,
        "incompatible" => EXIT_RUNNER_INCOMPATIBLE,
        _ => EXIT_CLI_FAILURE,
    };
    if json {
        let _ = writeln!(
            stderr,
            "{}",
            json!({
                "schema": "sico.run.diagnostic.v0",
                "class": class,
                "exit": exit,
                "detail": message,
            })
        );
    } else {
        let _ = writeln!(stderr, "{message}");
    }
    exit
}
