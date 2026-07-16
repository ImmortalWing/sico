//! Application packaging, inspection, trust authorization, and Runtime launch.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};
use serde_json::{Value, json};
use sico_package::{
    AuthorizedPackage, BuildInput, RuntimeLimits, TrustPolicy, TrustStatus, authorize,
    build_unsigned, sha256_hex, sign_development, verify, verify_trusted,
};
use sico_runtime::{AppStorage, FaultClass, HostLimits, prepare_storage, run_authorized_package};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_TOOL_ERROR: i32 = 2;
pub const EXIT_DOMAIN_ERROR: i32 = 3;
pub const EXIT_CANCELLED: i32 = 4;
pub const EXIT_TIMEOUT: i32 = 5;
pub const EXIT_RUNTIME_FAULT: i32 = 6;

static ARTIFACT_ID: AtomicU64 = AtomicU64::new(0);

/// Runs the application CLI with injectable output streams.
pub fn run<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32
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
        Some(("pack", command)) => run_pack(command, stdout, stderr),
        Some(("run", command)) => run_package(command, stdout, stderr),
        Some(("inspect", command)) => run_inspect(command, stdout, stderr),
        _ => EXIT_TOOL_ERROR,
    }
}

fn command() -> Command {
    Command::new("sico-app")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Sico application packaging and Runtime launcher")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("pack")
                .about("Build a deterministic .sapp from a WebAssembly Component")
                .arg(component_arg())
                .arg(output_arg())
                .arg(app_id_arg())
                .arg(app_version_arg())
                .arg(sign_key_arg()),
        )
        .subcommand(
            Command::new("run")
                .about("Verify, authorize, and run a .sapp")
                .arg(package_arg())
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
                ),
        )
        .subcommand(
            Command::new("inspect")
                .about("Verify and inspect a .sapp without executing it")
                .arg(package_arg())
                .arg(trusted_key_arg())
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Emit sico.sapp.inspect.v0 JSON")
                        .action(ArgAction::SetTrue),
                ),
        )
}

fn component_arg() -> Arg {
    Arg::new("component")
        .value_name("COMPONENT")
        .help("Compiled WebAssembly Component")
        .required(true)
}

fn package_arg() -> Arg {
    Arg::new("package")
        .value_name("PACKAGE.sapp")
        .help("Sico application package")
        .required(true)
}

fn output_arg() -> Arg {
    Arg::new("output")
        .short('o')
        .long("output")
        .value_name("PACKAGE.sapp")
        .help("Output path (default: component path with .sapp extension)")
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

fn run_pack(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let input = Path::new(matches.get_one::<String>("component").unwrap());
    let component = match fs::read(input) {
        Ok(component) => component,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico-app: cannot read Component {}: {error}",
                input.display()
            );
            return EXIT_TOOL_ERROR;
        }
    };
    let unsigned = match build_unsigned(BuildInput {
        app_id: matches.get_one::<String>("app-id").unwrap().clone(),
        app_version: matches.get_one::<String>("app-version").unwrap().clone(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    }) {
        Ok(package) => package,
        Err(error) => {
            let _ = writeln!(stderr, "sico-app: cannot build package: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let package = match matches.get_one::<String>("sign-key") {
        Some(path) => {
            let seed = match read_hex_key::<32>(Path::new(path), "development signing seed") {
                Ok(seed) => seed,
                Err(message) => {
                    let _ = writeln!(stderr, "{message}");
                    return EXIT_TOOL_ERROR;
                }
            };
            match sign_development(&unsigned, &seed) {
                Ok(package) => package,
                Err(error) => {
                    let _ = writeln!(stderr, "sico-app: cannot sign package: {error}");
                    return EXIT_TOOL_ERROR;
                }
            }
        }
        None => unsigned,
    };
    let output = matches
        .get_one::<String>("output")
        .map_or_else(|| input.with_extension("sapp"), PathBuf::from);
    match write_new_artifact(&output, &package) {
        Ok(()) if writeln!(stdout, "packed {}", output.display()).is_ok() => EXIT_SUCCESS,
        Ok(()) => {
            let _ = fs::remove_file(output);
            EXIT_TOOL_ERROR
        }
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            EXIT_TOOL_ERROR
        }
    }
}

fn run_package(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    if matches
        .get_many::<String>("args")
        .is_some_and(|mut arguments| arguments.next().is_some())
    {
        let _ = writeln!(
            stderr,
            "sico-app: scalar package entry main() does not accept application arguments"
        );
        return EXIT_TOOL_ERROR;
    }
    let path = matches.get_one::<String>("package").unwrap();
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "sico-app: cannot read package {path}: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let policy = match trust_policy(matches) {
        Ok(policy) => policy,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return EXIT_TOOL_ERROR;
        }
    };
    let trusted = match verify_trusted(&bytes, &policy) {
        Ok(trusted) => trusted,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sico-app: package trust verification failed: {error}"
            );
            return EXIT_TOOL_ERROR;
        }
    };
    let grants: BTreeSet<String> = matches
        .get_many::<String>("grant")
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let authorized = match authorize(trusted, &grants) {
        Ok(package) => package,
        Err(error) => {
            let _ = writeln!(stderr, "sico-app: package authorization failed: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let limits = HostLimits::default();
    let storage = match storage_for_run(matches, &authorized, &limits, stderr) {
        Ok(storage) => storage,
        Err(exit) => return exit,
    };
    let runtime = matches
        .get_one::<String>("runtime")
        .map(OsString::from)
        .or_else(|| std::env::var_os("SICO_WASMTIME"))
        .unwrap_or_else(|| OsString::from("wasmtime"));
    match run_authorized_package(&runtime, &authorized, storage.as_ref(), &limits) {
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
                "sico-app: cannot start Runtime {}: {} ({:?})",
                runtime.to_string_lossy(),
                error,
                error.fault_class()
            );
            EXIT_TOOL_ERROR
        }
    }
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
        let _ = writeln!(
            stderr,
            "sico-app: storage.read-write requires --storage-root"
        );
        EXIT_TOOL_ERROR
    })?;
    prepare_storage(Path::new(root), package, limits.storage_bytes)
        .map(Some)
        .map_err(|error| {
            let _ = writeln!(stderr, "sico-app: cannot prepare isolated storage: {error}");
            EXIT_TOOL_ERROR
        })
}

fn run_inspect(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let path = matches.get_one::<String>("package").unwrap();
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "sico-app: cannot read package {path}: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let structural = match verify(&bytes) {
        Ok(package) => package,
        Err(error) => {
            let _ = writeln!(stderr, "sico-app: package verification failed: {error}");
            return EXIT_TOOL_ERROR;
        }
    };
    let trusted_key = matches.get_one::<String>("trusted-key");
    let policy = match inspect_policy(trusted_key) {
        Ok(policy) => policy,
        Err(message) => {
            let _ = writeln!(stderr, "{message}");
            return EXIT_TOOL_ERROR;
        }
    };
    let trusted = match verify_trusted(&bytes, &policy) {
        Ok(trusted) => trusted,
        Err(error) => {
            let _ = writeln!(stderr, "sico-app: signature verification failed: {error}");
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

fn trust_policy(matches: &ArgMatches) -> Result<TrustPolicy, String> {
    if let Some(path) = matches.get_one::<String>("trusted-key") {
        let key = read_hex_key::<32>(Path::new(path), "trusted public key")?;
        Ok(TrustPolicy::RequireDevelopment(BTreeSet::from([key])))
    } else if matches.get_flag("allow-unsigned-dev") {
        Ok(TrustPolicy::AllowUnsignedDevelopment)
    } else {
        Err(
            "sico-app: package run requires --trusted-key or explicit --allow-unsigned-dev"
                .to_owned(),
        )
    }
}

fn inspect_policy(trusted_key: Option<&String>) -> Result<TrustPolicy, String> {
    trusted_key.map_or(Ok(TrustPolicy::AllowUnsignedDevelopment), |path| {
        read_hex_key::<32>(Path::new(path), "trusted public key")
            .map(|key| TrustPolicy::RequireDevelopment(BTreeSet::from([key])))
    })
}

fn read_hex_key<const N: usize>(path: &Path, label: &str) -> Result<[u8; N], String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("sico-app: cannot read {label} {}: {error}", path.display()))?;
    let text = text.trim();
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "sico-app: {label} must contain exactly {} lowercase hex characters",
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

fn write_new_artifact(output: &Path, bytes: &[u8]) -> Result<(), String> {
    if output.exists() {
        return Err(format!(
            "sico-app: refusing to overwrite existing artifact {}",
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
        .unwrap_or("app.sapp");
    let temporary = parent.join(format!(
        ".{file_name}.sico-app-{}-{}.tmp",
        std::process::id(),
        ARTIFACT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| {
            format!(
                "sico-app: cannot create temporary artifact {}: {error}",
                temporary.display()
            )
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "sico-app: cannot write temporary artifact {}: {error}",
            temporary.display()
        ));
    }
    drop(file);
    fs::rename(&temporary, output).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!(
            "sico-app: cannot install artifact {}: {error}",
            output.display()
        )
    })
}
