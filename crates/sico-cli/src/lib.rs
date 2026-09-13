//! Command-line integration for the Sico frontend and static semantics.

#![forbid(unsafe_code)]

mod cache;
mod modules;
mod packages;
mod repl;
mod run;
mod test;

use std::{
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sico_codegen_wasm::{
    CodegenError, DebugArtifact, DebugBuildInput, compile_component, compile_component_with_debug,
    compile_script_program_with_debug,
};
use sico_diagnostics::{render_syntax_json, render_syntax_text, syntax_identity};
use sico_format::format as canonical_format;
use sico_ir::{CoreLowerError, Module, Type, lower_core, lower_core_modules};
use sico_parser::{DeclarationKind, Parse, parse};
use sico_semantics::{Analysis, DiagnosticArgument, analyze};
use sico_source::{SourceFile, SourceId, TextRange};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_DIAGNOSTIC: i32 = 1;
pub const EXIT_TOOL_ERROR: i32 = 2;

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
        Some(("run", command)) => run::run_run(command, stdin, stdout, stderr),
        Some(("watch", command)) => run::run_watch(command, stdout, stderr),
        Some(("repl", command)) => repl::run_repl(command, stdin, stdout, stderr),
        Some(("test", command)) => test::run_test(command, stdout, stderr),
        Some(("eval", command)) => run::run_eval(command, stdout, stderr),
        _ => EXIT_TOOL_ERROR,
    }
}

fn command() -> Command {
    Command::new("sico")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Sico compiler toolchain")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(test::test_command())
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
        .subcommand(run::run_command())
        .subcommand(run::watch_command())
        .subcommand(repl::repl_command())
        .subcommand(run::eval_command())
}

fn build_command() -> Command {
    Command::new("build")
        .about("Compile source to a deterministic WebAssembly Component")
        .arg(input_arg())
        .arg(output_arg())
        .arg(
            Arg::new("profile")
                .long("profile")
                .value_name("PROFILE")
                .help("Build profile: component-v0 (default) or script-v0")
                .default_value("component-v0"),
        )
        .arg(
            Arg::new("debug-info")
                .long("debug-info")
                .help("Emit a bound debug map and build identity beside the Component")
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

fn run_build(
    matches: &ArgMatches,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let input = matches.get_one::<String>("input").unwrap();
    let profile = matches.get_one::<String>("profile").unwrap();
    if profile != "component-v0" && profile != "script-v0" {
        let _ = writeln!(stderr, "sico: unknown build profile {profile}");
        return EXIT_TOOL_ERROR;
    }
    let output = match build_output_path(input, matches.get_one::<String>("output")) {
        Ok(output) => output,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return EXIT_TOOL_ERROR;
        }
    };
    if matches.get_flag("debug-info") {
        let artifact = match compile_source_debug(input, profile, stdin, stdout, stderr) {
            Ok(artifact) => artifact,
            Err(exit) => return exit,
        };
        return match write_debug_artifacts(&output, &artifact) {
            Ok(paths) => {
                if writeln!(
                    stdout,
                    "built {}\ndebug-map {}\ndebug-identity {}",
                    paths.component.display(),
                    paths.debug_map.display(),
                    paths.identity.display()
                )
                .is_err()
                {
                    remove_debug_artifacts(&paths);
                    EXIT_TOOL_ERROR
                } else {
                    EXIT_SUCCESS
                }
            }
            Err(message) => {
                let _ = writeln!(stderr, "{message}");
                EXIT_TOOL_ERROR
            }
        };
    }

    let component = match compile_source(input, profile, stdin, stdout, stderr) {
        Ok(component) => component,
        Err(exit) => return exit,
    };
    match write_new_artifact(&output, &component) {
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

fn compile_source(
    input: &str,
    profile: &str,
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
    compile_source_bytes(&name, &bytes, profile, stdout, stderr)
}

fn compile_source_debug(
    input: &str,
    profile: &str,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<DebugArtifact, i32> {
    let (name, bytes) = match read_input_bytes(input, stdin) {
        Ok(input) => input,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return Err(EXIT_TOOL_ERROR);
        }
    };
    // STEP-0143 limitation: debug artifacts stay single-file; module sets
    // refuse with a typed message instead of wrong cross-module maps.
    if modules::has_module_uses(&name, &bytes) {
        let _ = writeln!(
            stderr,
            "sico: --debug-info does not support module imports yet; build without --debug-info"
        );
        return Err(EXIT_TOOL_ERROR);
    }
    let (source, module) = prepare_source_bytes(&name, &bytes, profile, stdout, stderr)?;
    let executable = std::env::current_exe()
        .and_then(fs::read)
        .map_err(|error| {
            let _ = writeln!(stderr, "sico: cannot identify compiler executable: {error}");
            EXIT_TOOL_ERROR
        })?;
    let source_sha256 = sha256(&bytes);
    let compiler_sha256 = sha256(&executable);
    let display_uri = if input == "-" {
        "sico-source://stdin".to_owned()
    } else {
        format!(
            "workspace://{}",
            Path::new(input)
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("source.sico")
        )
    };
    let mut adapters = Vec::new();
    let mut wit = Vec::new();
    if profile == "script-v0" {
        adapters.push("sico:script-adapter@0.1.0".into());
        wit.push("sico:script@0.1.0".into());
    }
    let document_id = format!("document-{source_sha256}");
    let debug_input = DebugBuildInput {
        document_id: &document_id,
        source_bytes: &bytes,
        display_uri: Some(&display_uri),
        compiler_package: "sico-compiler",
        compiler_version: env!("CARGO_PKG_VERSION"),
        compiler_executable_sha256: &compiler_sha256,
        adapter_identities: adapters,
        wit_identities: wit,
    };
    let result = if profile == "script-v0" {
        let module = prepare_script_module(&module, &source, stderr)?;
        compile_script_program_with_debug(&module, &debug_input)
    } else {
        if let Err(message) = validate_entry(&module) {
            let _ = writeln!(stderr, "sico: cannot build {}: {message}", source.name());
            return Err(EXIT_TOOL_ERROR);
        }
        compile_component_with_debug(&module, &debug_input)
    };
    result.map_err(|error| {
        let _ = writeln!(
            stderr,
            "sico: cannot generate debug Component for {}: {}",
            source.name(),
            codegen_error(&error)
        );
        EXIT_TOOL_ERROR
    })
}

pub(crate) fn compile_source_bytes(
    name: &str,
    bytes: &[u8],
    profile: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<Vec<u8>, i32> {
    let assembled = match modules::assemble_from_bytes(name, bytes) {
        Ok(assembled) => assembled,
        Err(modules::AssembleError::Frontend(failure)) => {
            let modules::FrontendFailure { source, parsed } = *failure;
            return Err(emit_frontend_failure(
                &source, &parsed, false, stdout, stderr,
            ));
        }
        Err(modules::AssembleError::Diagnostics(lines)) => {
            for line in &lines {
                let _ = writeln!(stderr, "{line}");
            }
            return Err(EXIT_DIAGNOSTIC);
        }
        Err(modules::AssembleError::Tool(message)) => {
            let _ = writeln!(stderr, "{message}");
            return Err(EXIT_TOOL_ERROR);
        }
    };
    compile_assembled(&assembled, profile, stdout, stderr)
}

/// Compiles an assembled RFC-0039 module set (STEP-0143): per-file semantic
/// gates with file-attributed diagnostics, then the merged lowering. A
/// single-file set behaves exactly like the pre-modules path.
fn compile_assembled(
    assembled: &modules::Assembled,
    profile: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<Vec<u8>, i32> {
    let set = modules::analyze_set(assembled);
    if !set.entry.is_success() {
        return Err(emit_semantic_result(
            &assembled.entry,
            &set.entry,
            false,
            stdout,
            stderr,
        ));
    }
    for (name, analysis) in &set.imports {
        if analysis.is_success() {
            continue;
        }
        let source = assembled
            .imports
            .iter()
            .find(|import| &import.name == name)
            .map(|import| &import.source)
            .expect("analyzed imports come from the assembled set");
        return Err(emit_semantic_result(
            source, analysis, false, stdout, stderr,
        ));
    }
    if profile == "script-v0"
        && let Err(message) = validate_script_declarations(&set.entry)
    {
        let _ = writeln!(
            stderr,
            "sico: cannot build {}: {message}",
            assembled.entry.name()
        );
        return Err(EXIT_TOOL_ERROR);
    }
    let imports: Vec<sico_ir::ModuleImport> = assembled
        .imports
        .iter()
        .map(|import| sico_ir::ModuleImport {
            name: import.name.as_str(),
            source: &import.source,
        })
        .collect();
    let package_imports: Vec<sico_ir::PackageImport<'_>> = assembled
        .packages
        .iter()
        .map(|package| sico_ir::PackageImport {
            name: package.name.as_str(),
            version: package.version,
            interface: package.interface_kebab.as_str(),
            functions: &package.functions,
        })
        .collect();
    let module = match lower_core_modules(&assembled.entry, &imports, &package_imports) {
        Ok(module) => module,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: cannot lower {}: {}",
                assembled.entry.name(),
                lower_error(&error)
            );
            return Err(EXIT_TOOL_ERROR);
        }
    };
    if profile == "script-v0" {
        return compile_script_module(&module, &assembled.entry, stderr);
    }
    if let Err(message) = validate_entry(&module) {
        let _ = writeln!(
            stderr,
            "sico: cannot build {}: {message}",
            assembled.entry.name()
        );
        return Err(EXIT_TOOL_ERROR);
    }
    match compile_component(&module) {
        Ok(component) => Ok(component),
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: cannot generate Component for {}: {}",
                assembled.entry.name(),
                codegen_error(&error)
            );
            Err(EXIT_TOOL_ERROR)
        }
    }
}

fn prepare_source_bytes(
    name: &str,
    bytes: &[u8],
    profile: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(SourceFile, Module), i32> {
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
    if profile == "script-v0"
        && let Err(message) = validate_script_declarations(&analysis)
    {
        let _ = writeln!(stderr, "sico: cannot build {}: {message}", source.name());
        return Err(EXIT_TOOL_ERROR);
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
    Ok((source, module))
}

/// Compiles the validated Script profile module, mapping the source `main`
/// entry to the boundary `run` export.
fn compile_script_module(
    module: &Module,
    source: &SourceFile,
    stderr: &mut dyn Write,
) -> Result<Vec<u8>, i32> {
    let module = prepare_script_module(module, source, stderr)?;
    match sico_codegen_wasm::compile_script_program(&module) {
        Ok(component) => Ok(component),
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico: cannot generate Script Component for {}: {}",
                source.name(),
                codegen_error(&error)
            );
            Err(EXIT_TOOL_ERROR)
        }
    }
}

fn prepare_script_module(
    module: &Module,
    source: &SourceFile,
    stderr: &mut dyn Write,
) -> Result<Module, i32> {
    let mut module = module.clone();
    if let Err(message) = validate_script_entry(&module) {
        let _ = writeln!(stderr, "sico: cannot build {}: {message}", source.name());
        return Err(EXIT_TOOL_ERROR);
    }
    for function in &mut module.functions {
        if function.name == "main" {
            "run".clone_into(&mut function.name);
        }
    }
    Ok(module)
}

/// The frozen Script v0 entry: exactly one
/// `main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]`.
fn validate_script_entry(module: &Module) -> Result<(), &'static str> {
    let mut entries = module
        .functions
        .iter()
        .filter(|function| function.name == "main");
    let Some(main) = entries.next() else {
        return Err("script entry function main is missing");
    };
    if entries.next().is_some() {
        return Err("multiple main functions are not supported");
    }
    let script_input = Type::Named("ScriptInput".into());
    let script_result = Type::Result {
        ok: Box::new(Type::Named("ScriptOutput".into())),
        error: Box::new(Type::Named("ScriptError".into())),
    };
    if main.parameters.len() != 1 || main.parameters[0].ty != script_input {
        return Err("script main must take exactly one input: ScriptInput parameter");
    }
    if main.return_type != script_result {
        return Err("script main must return Result[ScriptOutput, ScriptError]");
    }
    if !main.effects.is_empty() {
        return Err("script v0 does not allow effect declarations");
    }
    Ok(())
}

/// Validates the four explicit Script declarations against the frozen ABI
/// shapes; unknown or mismatched shapes fail closed.
fn validate_script_declarations(analysis: &Analysis) -> Result<(), String> {
    let fields = |record: &str| -> Vec<(String, sico_semantics::Type)> {
        analysis
            .facts
            .iter()
            .filter(|fact| fact.kind == sico_semantics::SemanticFactKind::Field)
            .filter_map(|fact| {
                let (owner, name) = fact.name.split_once('.')?;
                let ty = fact.ty.clone()?;
                (owner == record).then_some((name.to_owned(), ty))
            })
            .collect()
    };
    let named = |name: &str| sico_semantics::Type::Named(name.to_owned());
    let bytes = named("Bytes");
    let text_list = sico_semantics::Type::Generic {
        name: "List".to_owned(),
        arguments: vec![named("Text")],
    };
    let expected: [(&str, Vec<(&str, sico_semantics::Type)>); 3] = [
        (
            "ScriptInput",
            vec![("arguments", text_list), ("stdin", bytes.clone())],
        ),
        (
            "ScriptOutput",
            vec![
                ("stdout", bytes.clone()),
                ("stderr", bytes.clone()),
                ("exit_code", named("I64")),
            ],
        ),
        (
            "ScriptError",
            vec![
                ("code", named("ScriptErrorCode")),
                ("message", named("Text")),
            ],
        ),
    ];
    for (record, expected_fields) in expected {
        let actual = fields(record);
        let expected: Vec<(String, sico_semantics::Type)> = expected_fields
            .into_iter()
            .map(|(name, ty)| (name.to_owned(), ty))
            .collect();
        if actual.is_empty() {
            return Err(format!(
                "script profile requires an explicit record {record} declaration"
            ));
        }
        if actual != expected {
            return Err(format!(
                "record {record} does not match the frozen Script ABI shape {expected:?}"
            ));
        }
    }
    let variants: Vec<&str> = analysis
        .facts
        .iter()
        .filter(|fact| fact.kind == sico_semantics::SemanticFactKind::Variant)
        .filter_map(|fact| {
            let (owner, name) = fact.name.split_once('.')?;
            (owner == "ScriptErrorCode").then_some(name)
        })
        .collect();
    if variants.is_empty() {
        return Err("script profile requires an explicit enum ScriptErrorCode declaration".into());
    }
    if variants != ["InvalidInput", "ResourceLimit", "DomainError", "Cancelled"] {
        return Err(format!(
            "enum ScriptErrorCode does not match the frozen Script ABI cases {variants:?}"
        ));
    }
    Ok(())
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

fn build_output_path(input: &str, output: Option<&String>) -> Result<PathBuf, String> {
    if let Some(output) = output {
        return Ok(PathBuf::from(output));
    }
    if input == "-" {
        return Err("sico: build from stdin requires --output".to_owned());
    }
    Ok(Path::new(input).with_extension("component.wasm"))
}

#[derive(Clone)]
struct DebugArtifactPaths {
    component: PathBuf,
    debug_map: PathBuf,
    identity: PathBuf,
}

fn debug_artifact_paths(output: &Path) -> DebugArtifactPaths {
    let sidecar = |suffix: &str| {
        let mut path = output.as_os_str().to_os_string();
        path.push(suffix);
        PathBuf::from(path)
    };
    DebugArtifactPaths {
        component: output.to_owned(),
        debug_map: sidecar(".debug-map.json"),
        identity: sidecar(".debug-identity.json"),
    }
}

fn write_debug_artifacts(
    output: &Path,
    artifact: &DebugArtifact,
) -> Result<DebugArtifactPaths, String> {
    let paths = debug_artifact_paths(output);
    for path in [&paths.component, &paths.debug_map, &paths.identity] {
        if path.exists() {
            return Err(format!(
                "sico: refusing to overwrite existing artifact {}",
                path.display()
            ));
        }
    }
    let artifacts = [
        (&paths.component, artifact.component.as_slice()),
        (&paths.debug_map, artifact.debug_map.as_slice()),
        (&paths.identity, artifact.identity.as_slice()),
    ];
    let mut installed = Vec::new();
    for (path, bytes) in artifacts {
        if let Err(error) = write_new_artifact(path, bytes) {
            for installed_path in installed {
                let _ = fs::remove_file(installed_path);
            }
            return Err(error);
        }
        installed.push(path);
    }
    Ok(paths)
}

fn remove_debug_artifacts(paths: &DebugArtifactPaths) {
    for path in [&paths.component, &paths.debug_map, &paths.identity] {
        let _ = fs::remove_file(path);
    }
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

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn lower_error(error: &CoreLowerError) -> String {
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
        CoreLowerError::InvalidIr(errors) => {
            // Compiler-defect visibility: every verifier rejection names its
            // function and stable error kind so the defect is debuggable.
            let details = errors
                .iter()
                .map(|error| format!("{}: {:?}", error.path, error.kind))
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "IR verifier rejected compiler output with {} error(s): {details}",
                errors.len()
            )
        }
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
        CodegenError::DebugArtifact(message) => format!("invalid debug artifact: {message}"),
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
    // RFC-0039 (STEP-0143): assemble the module set first; parse failures
    // (entry or imports) surface through the same frontend renderer.
    let assembled = match modules::assemble_from_bytes(source.name(), source.text().as_bytes()) {
        Ok(assembled) => assembled,
        Err(modules::AssembleError::Frontend(failure)) => {
            let modules::FrontendFailure { source, parsed } = *failure;
            return emit_frontend_failure(&source, &parsed, json_output, stdout, stderr);
        }
        Err(modules::AssembleError::Diagnostics(lines)) => {
            for line in &lines {
                let _ = writeln!(stderr, "{line}");
            }
            return EXIT_DIAGNOSTIC;
        }
        Err(modules::AssembleError::Tool(message)) => {
            let _ = writeln!(stderr, "{message}");
            return EXIT_TOOL_ERROR;
        }
    };
    let set = modules::analyze_set(&assembled);
    if !set.entry.is_success() {
        return emit_semantic_result(&assembled.entry, &set.entry, json_output, stdout, stderr);
    }
    for (name, analysis) in &set.imports {
        if analysis.is_success() {
            continue;
        }
        let source = assembled
            .imports
            .iter()
            .find(|import| &import.name == name)
            .map(|import| &import.source)
            .expect("analyzed imports come from the assembled set");
        return emit_semantic_result(source, analysis, json_output, stdout, stderr);
    }
    emit_semantic_result(&assembled.entry, &set.entry, json_output, stdout, stderr)
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

pub(crate) fn read_input_bytes(
    input: &str,
    stdin: &mut dyn Read,
) -> Result<(String, Vec<u8>), String> {
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

pub(crate) fn emit_frontend_failure(
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
        DeclarationKind::Module => "module",
        DeclarationKind::Use => "use",
    }
}
