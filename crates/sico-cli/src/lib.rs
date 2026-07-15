//! Command-line integration for the Sico frontend and static semantics.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};
use serde_json::{Value, json};
use sico_codegen_wasm::{CodegenError, compile_component};
use sico_diagnostics::{render_syntax_json, render_syntax_text, syntax_identity};
use sico_format::format as canonical_format;
use sico_ir::{CoreLowerError, Module, Type, lower_core};
use sico_package::{
    AuthorizedPackage, BuildInput, RuntimeLimits, TrustPolicy, TrustStatus, authorize,
    build_unsigned, sha256_hex, sign_development, verify, verify_trusted,
};
use sico_parser::{DeclarationKind, Parse, parse};
use sico_runtime::{AppStorage, FaultClass, HostLimits, prepare_storage, run_authorized_package};
use sico_semantics::{Analysis, DiagnosticArgument, analyze};
use sico_source::{SourceFile, SourceId, TextRange};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_DIAGNOSTIC: i32 = 1;
pub const EXIT_TOOL_ERROR: i32 = 2;
pub const EXIT_DOMAIN_ERROR: i32 = 3;
pub const EXIT_CANCELLED: i32 = 4;
pub const EXIT_TIMEOUT: i32 = 5;
pub const EXIT_RUNTIME_FAULT: i32 = 6;

static ARTIFACT_ID: AtomicU64 = AtomicU64::new(0);

/// Runs the CLI with injectable streams for binary-level contract tests.
pub fn run<I, T>(
    args: I,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = match command().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            let exit = error.exit_code();
            let rendered = error.render().to_string();
            let written = if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                stdout.write_all(rendered.as_bytes())
            } else {
                stderr.write_all(rendered.as_bytes())
            };
            return if written.is_ok() {
                exit
            } else {
                EXIT_TOOL_ERROR
            };
        }
    };

    match matches.subcommand() {
        Some(("check", command)) => run_check(command, stdin, stdout, stderr),
        Some(("format", command)) => run_format(command, stdin, stdout, stderr),
        Some(("outline", command)) => run_outline(command, stdin, stdout, stderr),
        Some(("build", command)) => run_build(command, stdin, stdout, stderr),
        Some(("run", command)) => run_program(command, stdin, stdout, stderr),
        Some(("inspect", command)) => run_inspect(command, stdout, stderr),
        _ => EXIT_TOOL_ERROR,
    }
}

fn command() -> Command {
    Command::new("sico")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Sico compiler toolchain")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Check source syntax and static semantics")
                .arg(input_arg())
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Emit RFC-0001 JSON")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("format")
                .about("Produce canonical source layout")
                .arg(input_arg())
                .arg(
                    Arg::new("check")
                        .long("check")
                        .help("Exit 1 when source is not canonical")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("write"),
                )
                .arg(
                    Arg::new("write")
                        .long("write")
                        .help("Rewrite a source file in place")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("check"),
                ),
        )
        .subcommand(
            Command::new("outline")
                .about("List top-level parser declarations")
                .arg(input_arg())
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Emit sico.outline.v0 JSON")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(build_command())
        .subcommand(run_command())
        .subcommand(inspect_command())
}

fn build_command() -> Command {
    Command::new("build")
        .about("Build a deterministic .sapp application package")
        .arg(input_arg())
        .arg(output_arg())
        .arg(app_id_arg())
        .arg(app_version_arg())
        .arg(sign_key_arg())
        .arg(
            Arg::new("raw-component")
                .long("raw-component")
                .help("Internal M3 evidence boundary: emit raw Component")
                .action(ArgAction::SetTrue)
                .conflicts_with("sign-key"),
        )
}

fn run_command() -> Command {
    Command::new("run")
        .about("Verify and run a .sapp, or build/cache source then run it")
        .arg(input_arg())
        .arg(app_id_arg())
        .arg(app_version_arg())
        .arg(sign_key_arg())
        .arg(trusted_key_arg())
        .arg(
            Arg::new("allow-unsigned-dev")
                .long("allow-unsigned-dev")
                .help("Enter explicit local development trust mode")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("grant")
                .long("grant")
                .value_name("CAPABILITY")
                .help("Grant one requested capability")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("storage-root")
                .long("storage-root")
                .value_name("DIRECTORY")
                .help("Host-owned base for isolated app storage"),
        )
        .arg(
            Arg::new("cache-dir")
                .long("cache-dir")
                .value_name("DIRECTORY")
                .help("Source-run cache (default: SICO_CACHE_DIR or OS temp)"),
        )
        .arg(
            Arg::new("no-cache")
                .long("no-cache")
                .help("Disable the source-run cache")
                .action(ArgAction::SetTrue)
                .conflicts_with("cache-dir"),
        )
        .arg(
            Arg::new("runtime")
                .long("runtime")
                .value_name("WASMTIME")
                .help("Wasmtime executable (otherwise SICO_WASMTIME or PATH)"),
        )
        .arg(
            Arg::new("args")
                .value_name("ARG")
                .help("Reserved application arguments after --")
                .num_args(0..)
                .last(true)
                .allow_hyphen_values(true),
        )
}

fn inspect_command() -> Command {
    Command::new("inspect")
        .about("Verify and inspect a .sapp without executing it")
        .arg(input_arg())
        .arg(trusted_key_arg())
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit sico.sapp.inspect.v0 JSON")
                .action(ArgAction::SetTrue),
        )
}

fn input_arg() -> Arg {
    Arg::new("input")
        .value_name("FILE|-")
        .help("Source file, or - for stdin")
        .required(true)
        .allow_hyphen_values(true)
}

fn output_arg() -> Arg {
    Arg::new("output")
        .short('o')
        .long("output")
        .value_name("ARTIFACT")
        .help("Output path; required for stdin")
}

fn app_id_arg() -> Arg {
    Arg::new("app-id")
        .long("app-id")
        .value_name("ID")
        .default_value("dev.sico.app")
}

fn app_version_arg() -> Arg {
    Arg::new("app-version")
        .long("app-version")
        .value_name("VERSION")
        .default_value("0.0.0")
}

fn sign_key_arg() -> Arg {
    Arg::new("sign-key")
        .long("sign-key")
        .value_name("SEED_FILE")
        .help("File containing a 32-byte lowercase-hex development seed")
}

fn trusted_key_arg() -> Arg {
    Arg::new("trusted-key")
        .long("trusted-key")
        .value_name("PUBLIC_KEY_FILE")
        .help("File containing a trusted 32-byte lowercase-hex public key")
}

fn run_build(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let raw = matches.get_flag("raw-component");
    let output = match build_output_path(input, matches.get_one::<String>("output"), raw) {
        Ok(output) => output,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return EXIT_TOOL_ERROR;
        }
    };
    let component = match compile_source(input, stdin, stdout, stderr) {
        Ok(component) => component,
        Err(exit) => return exit,
    };
    let artifact = if raw {
        component
    } else {
        match package_component(matches, component) {
            Ok(package) => package,
            Err(message) => {
                let _ = writeln!(stderr, "{message}");
                return EXIT_TOOL_ERROR;
            }
        }
    };
    match write_new_artifact(&output, &artifact) {
        Ok(()) => {
            if writeln!(stdout, "built {}", output.display()).is_err() {
                let _ = fs::remove_file(&output);
                EXIT_TOOL_ERROR
            } else {
                EXIT_SUCCESS
            }
        }
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            EXIT_TOOL_ERROR
        }
    }
}

fn run_program(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if matches
        .get_many::<String>("args")
        .is_some_and(|mut arguments| arguments.next().is_some())
    {
        let _ = writeln!(
            stderr,
            "sico: scalar package entry main() does not accept application arguments"
        );
        return EXIT_TOOL_ERROR;
    }
    let input = matches.get_one::<String>("input").unwrap();
    let authorized = match authorize_run_input(matches, input, stdin, stdout, stderr) {
        Ok(authorized) => authorized,
        Err(exit) => return exit,
    };
    let host_limits = HostLimits::default();
    let storage = match storage_for_run(matches, &authorized, &host_limits, stderr) {
        Ok(storage) => storage,
        Err(exit) => return exit,
    };
    let runtime = matches
        .get_one::<String>("runtime")
        .map(OsString::from)
        .or_else(|| std::env::var_os("SICO_WASMTIME"))
        .unwrap_or_else(|| OsString::from("wasmtime"));
    match run_authorized_package(&runtime, &authorized, storage.as_ref(), &host_limits) {
        Ok(output) => {
            if stdout.write_all(&output.stdout).is_err()
                || stderr.write_all(&output.stderr).is_err()
            {
                return EXIT_TOOL_ERROR;
            }
            match output.fault {
                None if output.status.success() => EXIT_SUCCESS,
                Some(FaultClass::DomainError) => EXIT_DOMAIN_ERROR,
                Some(FaultClass::Cancelled) => EXIT_CANCELLED,
                Some(FaultClass::Timeout) => EXIT_TIMEOUT,
                Some(FaultClass::ResourceLimit | FaultClass::Trap) => EXIT_RUNTIME_FAULT,
                Some(FaultClass::CapabilityDenied | FaultClass::HostFatal) | None => {
                    EXIT_TOOL_ERROR
                }
            }
        }
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: cannot start Runtime {}: {} ({:?})",
                runtime.to_string_lossy(),
                error,
                error.fault_class()
            );
            EXIT_TOOL_ERROR
        }
    }
}

fn authorize_run_input(
    matches: &ArgMatches,
    input: &str,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<AuthorizedPackage, i32> {
    let packaged = is_package_path(input);
    let package = if packaged {
        fs::read(input).map_err(|error| {
            let _ = writeln!(stderr, "sico: cannot read package {input}: {error}");
            EXIT_TOOL_ERROR
        })?
    } else {
        source_run_package(matches, input, stdin, stdout, stderr)?
    };
    let policy = if packaged {
        trust_policy(matches).map_err(|message| {
            let _ = writeln!(stderr, "{message}");
            EXIT_TOOL_ERROR
        })?
    } else {
        TrustPolicy::AllowUnsignedDevelopment
    };
    let trusted = verify_trusted(&package, &policy).map_err(|error| {
        let _ = writeln!(stderr, "sico: package trust verification failed: {error}");
        EXIT_TOOL_ERROR
    })?;
    let grants: BTreeSet<String> = matches
        .get_many::<String>("grant")
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    authorize(trusted, &grants).map_err(|error| {
        let _ = writeln!(stderr, "sico: package authorization failed: {error}");
        EXIT_TOOL_ERROR
    })
}

fn storage_for_run(
    matches: &ArgMatches,
    package: &AuthorizedPackage,
    limits: &HostLimits,
    stderr: &mut dyn Write,
) -> Result<Option<AppStorage>, i32> {
    if !package.granted_capabilities.contains("storage.read-write") {
        return Ok(None);
    }
    let root = matches.get_one::<String>("storage-root").ok_or_else(|| {
        let _ = writeln!(stderr, "sico: storage.read-write requires --storage-root");
        EXIT_TOOL_ERROR
    })?;
    prepare_storage(Path::new(root), package, limits.storage_bytes)
        .map(Some)
        .map_err(|error| {
            let _ = writeln!(stderr, "sico: cannot prepare isolated storage: {error}");
            EXIT_TOOL_ERROR
        })
}

fn run_inspect(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    if input == "-" {
        let _ = writeln!(stderr, "sico: inspect from stdin is not supported in v0");
        return EXIT_TOOL_ERROR;
    }
    let bytes = match fs::read(input) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "sico: cannot read package {input}: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let structural = match verify(&bytes) {
        Ok(package) => package,
        Err(error) => {
            let _ = writeln!(stderr, "sico: package verification failed: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let trusted_key = matches.get_one::<String>("trusted-key");
    let policy = match trusted_key {
        Some(path) => match read_hex_key::<32>(Path::new(path), "trusted public key") {
            Ok(key) => TrustPolicy::RequireDevelopment(BTreeSet::from([key])),
            Err(message) => {
                let _ = writeln!(stderr, "{message}");
                return EXIT_TOOL_ERROR;
            }
        },
        None => TrustPolicy::AllowUnsignedDevelopment,
    };
    let trusted = match verify_trusted(&bytes, &policy) {
        Ok(trusted) => trusted,
        Err(error) => {
            let _ = writeln!(stderr, "sico: signature verification failed: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let (trust, public_key) = match trusted.trust {
        TrustStatus::UnsignedDevelopment => ("unsigned-development", Value::Null),
        TrustStatus::Development { public_key } if trusted_key.is_some() => {
            ("development-trusted", json!(public_key))
        }
        TrustStatus::Development { public_key } => {
            ("development-valid-untrusted", json!(public_key))
        }
    };
    let manifest = structural.manifest;
    if matches.get_flag("json") {
        let output = serde_json::to_string_pretty(&json!({
            "schema": "sico.sapp.inspect.v0",
            "package_sha256": sha256_hex(&bytes),
            "app": manifest.app,
            "component": manifest.component,
            "resources": manifest.resources,
            "source_effects": manifest.source_effects,
            "capabilities": manifest.capabilities,
            "component_imports": structural.component_imports,
            "limits": manifest.limits,
            "trust": { "status": trust, "public_key": public_key }
        }))
        .expect("inspect JSON serialization cannot fail");
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else if writeln!(stdout, "{} {}", manifest.app.id, manifest.app.version).is_err()
        || writeln!(stdout, "package sha256 {}", sha256_hex(&bytes)).is_err()
        || writeln!(
            stdout,
            "component {} {} bytes",
            manifest.component.sha256, manifest.component.bytes
        )
        .is_err()
        || writeln!(stdout, "trust {trust}").is_err()
        || writeln!(stdout, "capabilities {}", manifest.capabilities.join(",")).is_err()
    {
        return EXIT_TOOL_ERROR;
    }
    EXIT_SUCCESS
}

fn package_component(matches: &ArgMatches, component: Vec<u8>) -> Result<Vec<u8>, String> {
    let unsigned = build_unsigned(BuildInput {
        app_id: matches.get_one::<String>("app-id").unwrap().clone(),
        app_version: matches.get_one::<String>("app-version").unwrap().clone(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .map_err(|error| format!("sico: cannot build package: {error}"))?;
    let Some(path) = matches.get_one::<String>("sign-key") else {
        return Ok(unsigned);
    };
    let seed = read_hex_key::<32>(Path::new(path), "development signing seed")?;
    sign_development(&unsigned, &seed)
        .map_err(|error| format!("sico: cannot sign package: {error}"))
}

fn source_run_package(
    matches: &ArgMatches,
    input: &str,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<Vec<u8>, i32> {
    let (name, source_bytes) = match read_input_bytes(input, stdin) {
        Ok(input) => input,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return Err(EXIT_TOOL_ERROR);
        }
    };
    let seed = match matches.get_one::<String>("sign-key") {
        Some(path) => match read_hex_key::<32>(Path::new(path), "development signing seed") {
            Ok(seed) => Some(seed),
            Err(message) => {
                let _ = writeln!(stderr, "{message}");
                return Err(EXIT_TOOL_ERROR);
            }
        },
        None => None,
    };
    let app_id = matches.get_one::<String>("app-id").unwrap();
    let app_version = matches.get_one::<String>("app-version").unwrap();
    let cache_path = if matches.get_flag("no-cache") {
        None
    } else {
        let directory = matches
            .get_one::<String>("cache-dir")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("SICO_CACHE_DIR").map(PathBuf::from))
            .unwrap_or_else(|| std::env::temp_dir().join("sico-source-cache-v0"));
        let mut key_material = b"SICO-SOURCE-CACHE-V0\0".to_vec();
        key_material.extend_from_slice(app_id.as_bytes());
        key_material.push(0);
        key_material.extend_from_slice(app_version.as_bytes());
        key_material.push(0);
        key_material.extend_from_slice(&source_bytes);
        if let Some(seed) = seed {
            key_material.extend_from_slice(&seed);
        }
        Some(directory.join(format!("{}.sapp", sha256_hex(&key_material))))
    };
    if let Some(path) = cache_path.as_ref()
        && path.exists()
    {
        let cached = fs::read(path).map_err(|error| {
            let _ = writeln!(stderr, "sico: cannot read source cache: {error}");
            EXIT_TOOL_ERROR
        })?;
        let verified = verify(&cached).map_err(|error| {
            let _ = writeln!(stderr, "sico: refusing corrupt source cache: {error}");
            EXIT_TOOL_ERROR
        })?;
        if verified.manifest.app.id != *app_id || verified.manifest.app.version != *app_version {
            let _ = writeln!(stderr, "sico: refusing stale source cache identity");
            return Err(EXIT_TOOL_ERROR);
        }
        return Ok(cached);
    }

    let component = compile_source_bytes(&name, &source_bytes, stdout, stderr)?;
    let unsigned = build_unsigned(BuildInput {
        app_id: app_id.clone(),
        app_version: app_version.clone(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .map_err(|error| {
        let _ = writeln!(stderr, "sico: cannot build source package: {error}");
        EXIT_TOOL_ERROR
    })?;
    let package = match seed {
        Some(seed) => sign_development(&unsigned, &seed).map_err(|error| {
            let _ = writeln!(stderr, "sico: cannot sign source package: {error}");
            EXIT_TOOL_ERROR
        })?,
        None => unsigned,
    };
    if let Some(path) = cache_path {
        let parent = path.parent().expect("cache entry has a directory");
        fs::create_dir_all(parent).map_err(|error| {
            let _ = writeln!(stderr, "sico: cannot create source cache: {error}");
            EXIT_TOOL_ERROR
        })?;
        if let Err(message) = write_new_artifact(&path, &package)
            && !path.exists()
        {
            let _ = writeln!(stderr, "{message}");
            return Err(EXIT_TOOL_ERROR);
        }
    }
    Ok(package)
}

fn trust_policy(matches: &ArgMatches) -> Result<TrustPolicy, String> {
    if let Some(path) = matches.get_one::<String>("trusted-key") {
        let key = read_hex_key::<32>(Path::new(path), "trusted public key")?;
        Ok(TrustPolicy::RequireDevelopment(BTreeSet::from([key])))
    } else if matches.get_flag("allow-unsigned-dev") {
        Ok(TrustPolicy::AllowUnsignedDevelopment)
    } else {
        Err("sico: package run requires --trusted-key or explicit --allow-unsigned-dev".to_owned())
    }
}

fn read_hex_key<const N: usize>(path: &Path, label: &str) -> Result<[u8; N], String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("sico: cannot read {label} {}: {error}", path.display()))?;
    let text = text.trim();
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "sico: {label} must contain exactly {} lowercase hex characters",
            N * 2
        ));
    }
    let mut bytes = [0_u8; N];
    for (index, output) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *output = u8::from_str_radix(&text[offset..offset + 2], 16)
            .expect("validated lowercase hex pair");
    }
    Ok(bytes)
}

fn is_package_path(input: &str) -> bool {
    input != "-"
        && Path::new(input)
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sapp"))
}

fn compile_source(
    input: &str,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<Vec<u8>, i32> {
    let (name, bytes) = match read_input_bytes(input, stdin) {
        Ok(input) => input,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return Err(EXIT_TOOL_ERROR);
        }
    };
    compile_source_bytes(&name, &bytes, stdout, stderr)
}

fn compile_source_bytes(
    name: &str,
    bytes: &[u8],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<Vec<u8>, i32> {
    let source = match SourceFile::from_bytes(SourceId::new(0), name.to_owned(), bytes) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: {name}: source contract error {:?} at bytes {}..{}",
                error.kind,
                u32::from(error.range.start()),
                u32::from(error.range.end())
            );
            return Err(EXIT_TOOL_ERROR);
        }
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return Err(emit_frontend_failure(
            &source, &parsed, false, stdout, stderr,
        ));
    }
    let analysis = analyze(&source).expect("successful parse must lower for semantic analysis");
    if !analysis.is_success() {
        return Err(emit_semantic_result(
            &source, &analysis, false, stdout, stderr,
        ));
    }
    let module = match lower_core(&source) {
        Ok(module) => module,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: cannot lower {}: {}",
                source.name(),
                lower_error(&error)
            );
            return Err(EXIT_TOOL_ERROR);
        }
    };
    if let Err(message) = validate_entry(&module) {
        let _ = writeln!(stderr, "sico: cannot build {}: {message}", source.name());
        return Err(EXIT_TOOL_ERROR);
    }
    match compile_component(&module) {
        Ok(component) => Ok(component),
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: cannot generate Component for {}: {}",
                source.name(),
                codegen_error(&error)
            );
            Err(EXIT_TOOL_ERROR)
        }
    }
}

fn validate_entry(module: &Module) -> Result<(), &'static str> {
    let mut entries = module
        .functions
        .iter()
        .filter(|function| function.name == "main");
    let Some(main) = entries.next() else {
        return Err("synchronous scalar entry function main() is missing");
    };
    if entries.next().is_some() {
        return Err("multiple main functions are not supported");
    }
    if !main.parameters.is_empty() {
        return Err("main must not declare parameters in M3");
    }
    if !main.effects.is_empty() {
        return Err("main effects require an M4 capability host");
    }
    if !matches!(main.return_type, Type::Unit | Type::Bool | Type::Int) {
        return Err("main result must be Unit, Bool, or a compile-time proven fitting Int");
    }
    Ok(())
}

fn build_output_path(input: &str, output: Option<&String>, raw: bool) -> Result<PathBuf, String> {
    if let Some(output) = output {
        return Ok(PathBuf::from(output));
    }
    if input == "-" {
        return Err("sico: build from stdin requires --output".to_owned());
    }
    Ok(Path::new(input).with_extension(if raw { "component.wasm" } else { "sapp" }))
}

fn write_new_artifact(output: &Path, bytes: &[u8]) -> Result<(), String> {
    if output.exists() {
        return Err(format!(
            "sico: refusing to overwrite existing artifact {}",
            output.display()
        ));
    }
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = output
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("component.wasm");
    let temporary = parent.join(format!(
        ".{file_name}.sico-{}-{}.tmp",
        std::process::id(),
        ARTIFACT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| {
            format!(
                "sico: cannot create temporary artifact {}: {error}",
                temporary.display()
            )
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "sico: cannot write temporary artifact {}: {error}",
            temporary.display()
        ));
    }
    drop(file);
    fs::rename(&temporary, output).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!(
            "sico: cannot install artifact {}: {error}",
            output.display()
        )
    })
}

fn lower_error(error: &CoreLowerError) -> String {
    match error {
        CoreLowerError::Unsupported { feature, range } => format!(
            "unsupported {feature} at bytes {}..{}",
            range.start, range.end
        ),
        CoreLowerError::Frontend(_) => "frontend gate failed unexpectedly".to_owned(),
        CoreLowerError::Semantic(diagnostics) => format!(
            "semantic gate failed unexpectedly with {} diagnostic(s)",
            diagnostics.len()
        ),
        CoreLowerError::InvalidIr(errors) => format!(
            "IR verifier rejected compiler output with {} error(s)",
            errors.len()
        ),
    }
}

fn codegen_error(error: &CodegenError) -> String {
    match error {
        CodegenError::InvalidIr(errors) => {
            format!("IR verifier rejected input with {} error(s)", errors.len())
        }
        CodegenError::ModuleTooLarge { functions } => {
            format!("module has too many functions ({functions})")
        }
        CodegenError::AsyncUnsupported {
            function,
            feature,
            contract,
        } => format!("function {function}: unsupported {feature}: {contract}"),
        CodegenError::Unsupported { function, feature } => {
            format!("function {function}: unsupported {feature}")
        }
        CodegenError::IntegerOutsideProvenI64 { function, bytes } => {
            format!("function {function}: Int is outside the proven i64 probe ({bytes} bytes)")
        }
    }
}

fn run_check(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let json_output = matches.get_flag("json");
    let Ok(source) = load_source(input, stdin).map_err(|message| writeln!(stderr, "{message}"))
    else {
        return EXIT_TOOL_ERROR;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return emit_frontend_failure(&source, &parsed, json_output, stdout, stderr);
    }
    let analysis = analyze(&source).expect("successful parse must lower for semantic analysis");
    emit_semantic_result(&source, &analysis, json_output, stdout, stderr)
}

fn run_format(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let write = matches.get_flag("write");
    if write && input == "-" {
        let _ = writeln!(stderr, "sico: format --write requires a file path");
        return EXIT_TOOL_ERROR;
    }
    let Ok(source) = load_source(input, stdin).map_err(|message| writeln!(stderr, "{message}"))
    else {
        return EXIT_TOOL_ERROR;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return emit_frontend_failure(&source, &parsed, false, stdout, stderr);
    }
    let formatted = canonical_format(&source).expect("successful parse must format");

    if matches.get_flag("check") {
        if formatted == source.text() {
            return EXIT_SUCCESS;
        }
        let _ = writeln!(stderr, "{} is not canonically formatted", source.name());
        return EXIT_DIAGNOSTIC;
    }
    if write {
        return match fs::write(input, formatted.as_bytes()) {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => {
                let _ = writeln!(stderr, "sico: cannot write {input}: {error}");
                EXIT_TOOL_ERROR
            }
        };
    }
    if stdout.write_all(formatted.as_bytes()).is_err() {
        return EXIT_TOOL_ERROR;
    }
    EXIT_SUCCESS
}

fn run_outline(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let json_output = matches.get_flag("json");
    let Ok(source) = load_source(input, stdin).map_err(|message| writeln!(stderr, "{message}"))
    else {
        return EXIT_TOOL_ERROR;
    };
    let parsed = parse(&source);
    if !parsed.is_success() {
        return emit_frontend_failure(&source, &parsed, json_output, stdout, stderr);
    }
    let ast = parsed.ast().unwrap();
    if json_output {
        let declarations: Vec<_> = ast
            .declarations()
            .iter()
            .map(|declaration| {
                json!({
                    "kind": declaration_kind(declaration.kind),
                    "name": declaration.name,
                    "range": {
                        "start_byte": u32::from(declaration.range.start()),
                        "end_byte": u32::from(declaration.range.end())
                    }
                })
            })
            .collect();
        let output = serde_json::to_string_pretty(&json!({
            "schema": "sico.outline.v0",
            "file": source.name(),
            "type_checker": "not-run",
            "declarations": declarations
        }))
        .unwrap();
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else {
        for declaration in ast.declarations() {
            if writeln!(
                stdout,
                "{}\t{}\t{}..{}",
                declaration_kind(declaration.kind),
                declaration.name,
                u32::from(declaration.range.start()),
                u32::from(declaration.range.end())
            )
            .is_err()
            {
                return EXIT_TOOL_ERROR;
            }
        }
    }
    EXIT_SUCCESS
}

fn load_source(input: &str, stdin: &mut dyn Read) -> Result<SourceFile, String> {
    let (name, bytes) = read_input_bytes(input, stdin)?;
    SourceFile::from_bytes(SourceId::new(0), name.clone(), &bytes).map_err(|error| {
        format!(
            "sico: {name}: source contract error {:?} at bytes {}..{}",
            error.kind,
            u32::from(error.range.start()),
            u32::from(error.range.end())
        )
    })
}

fn read_input_bytes(input: &str, stdin: &mut dyn Read) -> Result<(String, Vec<u8>), String> {
    if input == "-" {
        let mut bytes = Vec::new();
        stdin
            .read_to_end(&mut bytes)
            .map_err(|error| format!("sico: cannot read stdin: {error}"))?;
        Ok(("<stdin>".to_owned(), bytes))
    } else {
        let bytes =
            fs::read(input).map_err(|error| format!("sico: cannot read {input}: {error}"))?;
        Ok((input.to_owned(), bytes))
    }
}

fn emit_frontend_failure(
    source: &SourceFile,
    parsed: &Parse,
    json_output: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if !parsed.lex_errors().is_empty() {
        let first = &parsed.lex_errors()[0];
        let _ = writeln!(
            stderr,
            "sico: {}: unregistered lexical error {:?} at bytes {}..{}",
            source.name(),
            first.kind,
            u32::from(first.range.start()),
            u32::from(first.range.end())
        );
        return EXIT_TOOL_ERROR;
    }
    if !parsed
        .errors()
        .iter()
        .all(|error| syntax_identity(&error.kind).is_some())
    {
        let _ = writeln!(
            stderr,
            "sico: {}: unregistered parser error {:?}",
            source.name(),
            parsed.errors()[0].kind
        );
        return EXIT_TOOL_ERROR;
    }

    if json_output {
        let output = syntax_check_json(source, parsed).unwrap();
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else {
        let output = render_syntax_text(source, source.name(), parsed.errors());
        if writeln!(stderr, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    }
    EXIT_DIAGNOSTIC
}

fn syntax_check_json(source: &SourceFile, parsed: &Parse) -> Option<String> {
    let mut value: Value =
        serde_json::from_str(&render_syntax_json(source, source.name(), parsed.errors())?).ok()?;
    value["status"] = json!({
        "syntax": "error",
        "type_checker": "not-run",
        "semantic_checks_performed": false
    });
    serde_json::to_string_pretty(&value).ok()
}

fn emit_semantic_result(
    source: &SourceFile,
    analysis: &Analysis,
    json_output: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if json_output {
        let output = semantic_check_json(source, analysis);
        if writeln!(stdout, "{output}").is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else if analysis.is_success() {
        if writeln!(stdout, "check ok: {} (syntax + semantics)", source.name()).is_err() {
            return EXIT_TOOL_ERROR;
        }
    } else {
        for diagnostic in &analysis.diagnostics {
            let Some(position) = source
                .line_index()
                .line_col(source.text(), diagnostic.range.start())
            else {
                return EXIT_TOOL_ERROR;
            };
            if writeln!(
                stderr,
                "{} {}:{}:{} {}",
                diagnostic.code,
                source.name(),
                position.line,
                position.column,
                diagnostic.message
            )
            .is_err()
            {
                return EXIT_TOOL_ERROR;
            }
        }
    }
    if analysis.is_success() {
        EXIT_SUCCESS
    } else {
        EXIT_DIAGNOSTIC
    }
}

fn semantic_check_json(source: &SourceFile, analysis: &Analysis) -> String {
    let diagnostics: Vec<_> = analysis
        .diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            json!({
                "id": format!("d{}", index + 1),
                "code": diagnostic.code,
                "key": diagnostic.key,
                "severity": "error",
                "kind": "root",
                "message": diagnostic.message,
                "arguments": diagnostic.arguments.iter().map(|(name, value)| {
                    let value = match value {
                        DiagnosticArgument::Text(text) => json!(text),
                        DiagnosticArgument::List(items) => json!(items),
                    };
                    (name.clone(), value)
                }).collect::<serde_json::Map<_, _>>(),
                "file": source.name(),
                "range": diagnostic_range(source, diagnostic.range)
            })
        })
        .collect();
    let errors = diagnostics.len();
    serde_json::to_string_pretty(&json!({
        "schema": "sico.diagnostics.v0",
        "protocol_version": 0,
        "tool": { "name": "sico", "version": env!("CARGO_PKG_VERSION") },
        "coordinate_system": {
            "encoding": "utf-8",
            "byte_base": 0,
            "byte_end": "exclusive",
            "line_base": 1,
            "column_base": 1,
            "column_unit": "unicode-scalar-value"
        },
        "diagnostics": diagnostics,
        "summary": {
            "emitted": errors,
            "errors": errors,
            "warnings": 0,
            "info": 0,
            "suppressed": 0,
            "truncated": 0
        },
        "status": {
            "syntax": "ok",
            "type_checker": if errors == 0 { "ok" } else { "error" },
            "semantic_checks_performed": true
        }
    }))
    .expect("diagnostic JSON serialization cannot fail")
}

fn diagnostic_range(source: &SourceFile, range: TextRange) -> Value {
    let start = source
        .line_index()
        .line_col(source.text(), range.start())
        .expect("semantic range start is a scalar boundary");
    let end = source
        .line_index()
        .line_col(source.text(), range.end())
        .expect("semantic range end is a scalar boundary");
    json!({
        "start": {
            "byte": u32::from(range.start()),
            "line": start.line,
            "column": start.column
        },
        "end": {
            "byte": u32::from(range.end()),
            "line": end.line,
            "column": end.column
        }
    })
}

const fn declaration_kind(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Newtype => "newtype",
        DeclarationKind::Record => "record",
        DeclarationKind::Enum => "enum",
        DeclarationKind::Capability => "capability",
        DeclarationKind::Resource => "resource",
        DeclarationKind::Interface => "interface",
        DeclarationKind::Function => "function",
    }
}
