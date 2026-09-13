//! Minimal selected-Runtime boundary for compiler-produced Components.

#![forbid(unsafe_code)]

pub mod canonical;

use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component as PathComponent, Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use sico_package::{
    AuthorizedPackage, SCRIPT_ARGS_CAPABILITY, SCRIPT_STDIO_CAPABILITY, TrustStatus, sha256_hex,
};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum RuntimeError {
    TemporaryArtifact(std::io::Error),
    Launch(std::io::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostLimits {
    pub fuel: u64,
    pub timeout_ms: u64,
    pub memory_bytes: u64,
    pub table_elements: u32,
    pub instances: u32,
    pub tables: u32,
    pub memories: u32,
    pub wasi_resources: u32,
    pub hostcall_fuel: u64,
    pub random_bytes: u64,
    pub body_bytes: u64,
    pub storage_bytes: u64,
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            fuel: 10_000_000,
            timeout_ms: 10_000,
            memory_bytes: 128 * 1024 * 1024,
            table_elements: 100_000,
            instances: 32,
            tables: 32,
            memories: 16,
            wasi_resources: 256,
            hostcall_fuel: 1_000_000,
            random_bytes: 1024 * 1024,
            body_bytes: 4 * 1024 * 1024,
            storage_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveLimits {
    pub fuel: u64,
    pub timeout_ms: u64,
    pub memory_bytes: u64,
    pub table_elements: u32,
    pub instances: u32,
    pub tables: u32,
    pub memories: u32,
    pub wasi_resources: u32,
    pub hostcall_fuel: u64,
    pub random_bytes: u64,
    pub body_bytes: u64,
    pub storage_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultClass {
    DomainError,
    CapabilityDenied,
    Cancelled,
    Timeout,
    ResourceLimit,
    Trap,
    HostFatal,
}

#[derive(Debug)]
pub struct PackageRuntimeOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub fault: Option<FaultClass>,
    pub limits: EffectiveLimits,
}

#[derive(Debug)]
pub enum PackageRuntimeError {
    TemporaryArtifact(std::io::Error),
    Launch(std::io::Error),
    Storage(StorageError),
}

impl std::fmt::Display for PackageRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TemporaryArtifact(error) => {
                write!(formatter, "temporary Component failed: {error}")
            }
            Self::Launch(error) => write!(formatter, "Runtime launch failed: {error}"),
            Self::Storage(error) => write!(formatter, "Runtime storage failed: {error}"),
        }
    }
}

impl std::error::Error for PackageRuntimeError {}

impl PackageRuntimeError {
    /// Classifies Runtime-owned setup and launch failures without exposing
    /// guest stderr as host state.
    #[must_use]
    pub const fn fault_class(&self) -> FaultClass {
        FaultClass::HostFatal
    }
}

#[derive(Debug)]
pub struct RuntimeOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppStorage {
    base: PathBuf,
    root: PathBuf,
    quota_bytes: u64,
}

impl AppStorage {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub const fn quota_bytes(&self) -> u64 {
        self.quota_bytes
    }

    /// Audits the current tree without following symbolic links.
    ///
    /// # Errors
    ///
    /// Rejects links, special files, paths outside the canonical app root, I/O
    /// failures, and total file bytes above the configured quota.
    pub fn audit(&self) -> Result<StorageUsage, StorageError> {
        let canonical = self.root.canonicalize().map_err(StorageError::Io)?;
        if !canonical.starts_with(&self.base) {
            return Err(StorageError::EscapedRoot);
        }
        let mut usage = StorageUsage::default();
        audit_directory(&canonical, &mut usage)?;
        if usage.bytes > self.quota_bytes {
            return Err(StorageError::QuotaExceeded {
                used: usage.bytes,
                quota: self.quota_bytes,
            });
        }
        Ok(usage)
    }

    /// Resolves a host-management path beneath the app root.
    ///
    /// Guest paths are resolved by Wasmtime's preopened capability directory;
    /// this method is for Runtime-owned cleanup and tests.
    ///
    /// # Errors
    ///
    /// Rejects empty, absolute, parent, prefix, current-dir, colon, NUL, or
    /// symlink-containing paths.
    pub fn resolve(&self, relative: &Path) -> Result<PathBuf, StorageError> {
        if relative.as_os_str().is_empty() {
            return Err(StorageError::InvalidPath);
        }
        let text = relative.to_string_lossy();
        if text.contains([':', '\0']) {
            return Err(StorageError::InvalidPath);
        }
        let mut result = self.root.clone();
        for component in relative.components() {
            match component {
                PathComponent::Normal(segment) => {
                    result.push(segment);
                    if let Ok(metadata) = fs::symlink_metadata(&result)
                        && is_link_like(&metadata)
                    {
                        return Err(StorageError::Symlink);
                    }
                }
                _ => return Err(StorageError::InvalidPath),
            }
        }
        if !result.starts_with(&self.root) {
            return Err(StorageError::EscapedRoot);
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StorageUsage {
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
}

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    InvalidPath,
    Symlink,
    SpecialFile,
    EscapedRoot,
    InvalidQuota,
    QuotaExceeded { used: u64, quota: u64 },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "storage I/O failed: {error}"),
            Self::InvalidPath => formatter.write_str("storage path is invalid"),
            Self::Symlink => formatter.write_str("storage symbolic link is not allowed"),
            Self::SpecialFile => formatter.write_str("storage special file is not allowed"),
            Self::EscapedRoot => formatter.write_str("storage root escaped its app namespace"),
            Self::InvalidQuota => formatter.write_str("storage quota is invalid"),
            Self::QuotaExceeded { used, quota } => {
                write!(
                    formatter,
                    "storage quota exceeded: used={used} quota={quota}"
                )
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// Creates or verifies a stable per-app storage root.
///
/// # Errors
///
/// Rejects zero quota, symlink roots, non-directory roots, cross-root
/// canonicalization, existing over-quota content, and I/O failures.
pub fn prepare_storage(
    base: &Path,
    package: &AuthorizedPackage,
    quota_bytes: u64,
) -> Result<AppStorage, StorageError> {
    if quota_bytes == 0 {
        return Err(StorageError::InvalidQuota);
    }
    fs::create_dir_all(base).map_err(StorageError::Io)?;
    reject_link(base)?;
    let canonical_base = base.canonicalize().map_err(StorageError::Io)?;
    let identity = storage_identity(package);
    let root = canonical_base.join(identity);
    fs::create_dir_all(&root).map_err(StorageError::Io)?;
    reject_link(&root)?;
    let canonical_root = root.canonicalize().map_err(StorageError::Io)?;
    if canonical_root.parent() != Some(canonical_base.as_path()) {
        return Err(StorageError::EscapedRoot);
    }
    let storage = AppStorage {
        base: canonical_base,
        root: canonical_root,
        quota_bytes,
    };
    storage.audit()?;
    Ok(storage)
}

/// Returns only the WASI flags authorized for this package.
///
/// # Errors
///
/// Rejects a storage capability without a prepared root and capabilities for
/// which the selected Wasmtime CLI has no explicit v0 adapter.
pub fn wasi_arguments(
    package: &AuthorizedPackage,
    storage: Option<&AppStorage>,
) -> Result<Vec<String>, StorageError> {
    let granted = &package.granted_capabilities;
    let mut arguments = vec!["-S".to_owned(), "cli=n".to_owned()];
    if granted.contains("clock.read")
        || granted.contains("random.read")
        || granted.contains(SCRIPT_ARGS_CAPABILITY)
        || granted.contains(SCRIPT_STDIO_CAPABILITY)
    {
        // Script entries execute as wasi:cli command components: the
        // composed command imports the environment/stdin/stdout/stderr/exit
        // interfaces, so the script capabilities require the CLI world.
        "cli=y".clone_into(&mut arguments[1]);
    }
    if granted.contains("storage.read-write") {
        let root = storage.ok_or(StorageError::InvalidPath)?;
        "cli=y".clone_into(&mut arguments[1]);
        arguments.push("--dir".to_owned());
        arguments.push(format!("{}::/data", root.root.display()));
    }
    if granted.contains("network.connect") {
        "cli=y".clone_into(&mut arguments[1]);
        arguments.extend([
            "-S".to_owned(),
            "tcp=y".to_owned(),
            "-S".to_owned(),
            "udp=n".to_owned(),
            "-S".to_owned(),
            "allow-ip-name-lookup=n".to_owned(),
        ]);
    }
    if granted.contains("log.write") {
        return Err(StorageError::InvalidPath);
    }
    Ok(arguments)
}

#[must_use]
pub fn effective_limits(package: &AuthorizedPackage, host: &HostLimits) -> EffectiveLimits {
    let requested = &package.trusted.package.manifest.limits;
    EffectiveLimits {
        fuel: requested.fuel.min(host.fuel),
        timeout_ms: requested.timeout_ms.min(host.timeout_ms),
        memory_bytes: requested.memory_bytes.min(host.memory_bytes),
        table_elements: requested.table_elements.min(host.table_elements),
        instances: requested.instances.min(host.instances),
        tables: requested.tables.min(host.tables),
        memories: requested.memories.min(host.memories),
        wasi_resources: requested.wasi_resources.min(host.wasi_resources),
        hostcall_fuel: requested.hostcall_fuel.min(host.hostcall_fuel),
        random_bytes: requested.random_bytes.min(host.random_bytes),
        body_bytes: requested.body_bytes.min(host.body_bytes),
        storage_bytes: host.storage_bytes,
    }
}

/// Executes an authorized package under explicit Wasmtime/WASI ceilings.
///
/// # Errors
///
/// Returns only host-fatal setup/launch/storage errors. Guest timeout, fuel,
/// resource-limit and ordinary traps are returned as classified output.
pub fn run_authorized_package(
    runtime: &OsStr,
    package: &AuthorizedPackage,
    storage: Option<&AppStorage>,
    host_limits: &HostLimits,
) -> Result<PackageRuntimeOutput, PackageRuntimeError> {
    let limits = effective_limits(package, host_limits);
    if let Some(storage) = storage {
        storage.audit().map_err(PackageRuntimeError::Storage)?;
    }
    let path = temporary_path();
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(PackageRuntimeError::TemporaryArtifact)?;
    if let Err(error) = file
        .write_all(&package.trusted.package.component)
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(&path);
        return Err(PackageRuntimeError::TemporaryArtifact(error));
    }
    drop(file);

    let mut command = Command::new(runtime);
    command
        .arg("run")
        .arg("--codegen")
        .arg("cache=n")
        .arg("-W")
        .arg(format!("fuel={}", limits.fuel))
        .arg("-W")
        .arg(format!("timeout={}ms", limits.timeout_ms))
        .arg("-W")
        .arg(format!("max-memory-size={}", limits.memory_bytes))
        .arg("-W")
        .arg(format!("max-table-elements={}", limits.table_elements))
        .arg("-W")
        .arg(format!("max-instances={}", limits.instances))
        .arg("-W")
        .arg(format!("max-tables={}", limits.tables))
        .arg("-W")
        .arg(format!("max-memories={}", limits.memories))
        .arg("-W")
        .arg("trap-on-grow-failure=y")
        .arg("-S")
        .arg(format!("max-resources={}", limits.wasi_resources))
        .arg("-S")
        .arg(format!("hostcall-fuel={}", limits.hostcall_fuel))
        .arg("-S")
        .arg(format!("max-random-size={}", limits.random_bytes))
        .arg("-S")
        .arg(format!(
            "http-outgoing-body-chunk-size={}",
            limits.body_bytes
        ));
    for argument in wasi_arguments(package, storage).map_err(PackageRuntimeError::Storage)? {
        command.arg(argument);
    }
    if package.trusted.package.manifest.script.is_some() {
        // Script entry (v1 manifest): the component is a wasi:cli command
        // exporting `run`; the runtime executes it directly and stdin
        // feeds the ScriptInput channel. `--invoke` must stay
        // scalar-only — a command component has no `main`.
        command.stdin(Stdio::inherit());
    } else {
        command.arg("--invoke").arg("main()");
    }
    command.arg(&path);
    let result = command.output().map_err(PackageRuntimeError::Launch);
    let _ = fs::remove_file(path);
    let output = result?;
    if let Some(storage) = storage {
        storage.audit().map_err(PackageRuntimeError::Storage)?;
    }
    let fault = classify_fault(output.status.success(), &output.stderr);
    Ok(PackageRuntimeOutput {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
        fault,
        limits,
    })
}

fn classify_fault(success: bool, stderr: &[u8]) -> Option<FaultClass> {
    if success {
        return None;
    }
    let message = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if message.contains("timeout") || message.contains("interrupt") || message.contains("epoch") {
        Some(FaultClass::Timeout)
    } else if message.contains("fuel")
        || message.contains("resource limit")
        || message.contains("memory limit")
        || message.contains("table limit")
        || message.contains("failed to grow")
    {
        Some(FaultClass::ResourceLimit)
    } else {
        Some(FaultClass::Trap)
    }
}

fn storage_identity(package: &AuthorizedPackage) -> String {
    let trust = match &package.trusted.trust {
        TrustStatus::UnsignedDevelopment => "unsigned-development",
        TrustStatus::Development { public_key } => public_key,
    };
    let mut identity = b"SICO-STORAGE-ID-V0\0".to_vec();
    identity.extend_from_slice(package.trusted.package.manifest.app.id.as_bytes());
    identity.push(0);
    identity.extend_from_slice(trust.as_bytes());
    sha256_hex(&identity)
}

fn reject_link(path: &Path) -> Result<(), StorageError> {
    let metadata = fs::symlink_metadata(path).map_err(StorageError::Io)?;
    if is_link_like(&metadata) {
        Err(StorageError::Symlink)
    } else if !metadata.is_dir() {
        Err(StorageError::SpecialFile)
    } else {
        Ok(())
    }
}

fn audit_directory(path: &Path, usage: &mut StorageUsage) -> Result<(), StorageError> {
    usage.directories = usage
        .directories
        .checked_add(1)
        .ok_or(StorageError::InvalidQuota)?;
    for entry in fs::read_dir(path).map_err(StorageError::Io)? {
        let entry = entry.map_err(StorageError::Io)?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(StorageError::Io)?;
        if is_link_like(&metadata) {
            return Err(StorageError::Symlink);
        }
        if metadata.is_dir() {
            audit_directory(&entry.path(), usage)?;
        } else if metadata.is_file() {
            usage.files = usage
                .files
                .checked_add(1)
                .ok_or(StorageError::InvalidQuota)?;
            usage.bytes = usage
                .bytes
                .checked_add(metadata.len())
                .ok_or(StorageError::InvalidQuota)?;
        } else {
            return Err(StorageError::SpecialFile);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_link_like(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// Executes a compiler-produced Component with the selected Wasmtime CLI.
///
/// The Component is written to a process-unique temporary file and removed
/// before this function returns. Runtime stdout, stderr, and status are kept
/// separate so the CLI can preserve its public channel contract.
///
/// # Errors
///
/// Returns an error when the temporary artifact cannot be created or the
/// Runtime process cannot be launched. A Runtime non-zero exit is returned as
/// a normal [`RuntimeOutput`] and is classified by the caller.
pub fn run_component(
    runtime: &OsStr,
    component: &[u8],
    invocation: &str,
) -> Result<RuntimeOutput, RuntimeError> {
    let path = temporary_path();
    run_component_at(runtime, component, invocation, &path)
}

fn run_component_at(
    runtime: &OsStr,
    component: &[u8],
    invocation: &str,
    path: &Path,
) -> Result<RuntimeOutput, RuntimeError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(RuntimeError::TemporaryArtifact)?;
    if let Err(error) = file.write_all(component).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(RuntimeError::TemporaryArtifact(error));
    }
    drop(file);

    let result = Command::new(runtime)
        .arg("run")
        .arg("--codegen")
        .arg("cache=n")
        .arg("--invoke")
        .arg(invocation)
        .arg(path)
        .output()
        .map_err(RuntimeError::Launch);
    let _ = fs::remove_file(path);
    result.map(|output| RuntimeOutput {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn temporary_path() -> PathBuf {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "sico-runtime-{}-{id}.component.wasm",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use sico_codegen_wasm::compile_component;
    use sico_ir::lower_core;
    use sico_package::{
        BuildInput, RuntimeLimits, TrustPolicy, authorize, build_unsigned, verify_trusted,
    };
    use sico_source::{SourceFile, SourceId};
    use wasm_encoder::{
        BlockType, CodeSection, ComponentBuilder, ComponentExportKind, ExportKind, ExportSection,
        Function, FunctionSection, Instruction, Module, ModuleArg, TypeSection,
    };

    use super::{
        FaultClass, HostLimits, RuntimeError, StorageError, effective_limits, prepare_storage,
        run_authorized_package, run_component_at, wasi_arguments,
    };

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn launch_failure_still_removes_temporary_component() {
        let path = test_directory().join("launch-failure.component.wasm");
        let error = run_component_at(
            OsStr::new("sico-runtime-command-that-does-not-exist"),
            b"component",
            "main()",
            &path,
        )
        .unwrap_err();
        assert!(matches!(error, RuntimeError::Launch(_)));
        assert!(!path.exists());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn app_storage_isolated_paths_and_quota_are_enforced() {
        let base = test_directory();
        let first = authorized("dev.sico.first");
        let second = authorized("dev.sico.second");
        let first_storage = prepare_storage(&base, &first, 4).unwrap();
        let second_storage = prepare_storage(&base, &second, 4).unwrap();
        assert_ne!(first_storage.root(), second_storage.root());
        assert_eq!(
            first_storage.root().parent(),
            second_storage.root().parent()
        );
        for path in [
            Path::new("../second"),
            Path::new("/absolute"),
            Path::new("C:escape"),
        ] {
            assert!(matches!(
                first_storage.resolve(path),
                Err(StorageError::InvalidPath)
            ));
        }
        fs::write(
            first_storage.resolve(Path::new("data.bin")).unwrap(),
            b"12345",
        )
        .unwrap();
        assert!(matches!(
            first_storage.audit(),
            Err(StorageError::QuotaExceeded { used: 5, quota: 4 })
        ));
        assert_eq!(second_storage.audit().unwrap().bytes, 0);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn wasi_flags_have_no_ambient_storage_or_network() {
        let package = authorized("dev.sico.flags");
        assert_eq!(wasi_arguments(&package, None).unwrap(), ["-S", "cli=n"]);
    }

    #[test]
    fn storage_links_and_reparse_points_are_rejected() {
        let base = test_directory();
        let package = authorized("dev.sico.links");
        let storage = prepare_storage(&base, &package, 1024).unwrap();
        let target = base.join("outside");
        fs::create_dir(&target).unwrap();
        let link = storage.root().join("escape");
        create_directory_link(&target, &link);
        assert!(matches!(storage.audit(), Err(StorageError::Symlink)));
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn manifest_limits_can_only_reduce_host_ceilings() {
        let package = authorized("dev.sico.limits");
        let host = HostLimits {
            fuel: 2_000_000,
            timeout_ms: 250,
            memory_bytes: 32 * 1024 * 1024,
            ..HostLimits::default()
        };
        let limits = effective_limits(&package, &host);
        assert_eq!(limits.fuel, 1_000_000);
        assert_eq!(limits.timeout_ms, 250);
        assert_eq!(limits.memory_bytes, 32 * 1024 * 1024);
        assert_eq!(limits.storage_bytes, host.storage_bytes);
    }

    #[test]
    fn real_wasmtime_limits_trap_loop_and_host_survives() {
        let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") else {
            return;
        };
        let requested = RuntimeLimits {
            fuel: 10_000,
            timeout_ms: 1_000,
            ..RuntimeLimits::default()
        };
        let malicious = authorized_component("dev.sico.loop", infinite_component(), requested);
        let outcome =
            run_authorized_package(&runtime, &malicious, None, &HostLimits::default()).unwrap();
        assert!(!outcome.status.success());
        assert!(matches!(
            outcome.fault,
            Some(FaultClass::ResourceLimit | FaultClass::Timeout)
        ));

        let healthy = authorized("dev.sico.after-loop");
        let outcome =
            run_authorized_package(&runtime, &healthy, None, &HostLimits::default()).unwrap();
        assert!(outcome.status.success());
        assert_eq!(outcome.fault, None);
    }

    fn authorized(app_id: &str) -> sico_package::AuthorizedPackage {
        let source = SourceFile::from_text(
            SourceId::new(99),
            "main.sico",
            "function main() returns Unit:\n  return Unit\nend function\n",
        )
        .unwrap();
        let component = compile_component(&lower_core(&source).unwrap()).unwrap();
        authorized_component(app_id, component, RuntimeLimits::default())
    }

    fn authorized_component(
        app_id: &str,
        component: Vec<u8>,
        limits: RuntimeLimits,
    ) -> sico_package::AuthorizedPackage {
        let package = build_unsigned(BuildInput {
            app_id: app_id.to_owned(),
            app_version: "0.1.0".to_owned(),
            component,
            resources: Vec::new(),
            source_effects: Vec::new(),
            capabilities: Vec::new(),
            limits,
        })
        .unwrap();
        let trusted = verify_trusted(&package, &TrustPolicy::AllowUnsignedDevelopment).unwrap();
        authorize(trusted, &BTreeSet::new()).unwrap()
    }

    fn infinite_component() -> Vec<u8> {
        let mut types = TypeSection::new();
        types.ty().function([], []);
        let mut functions = FunctionSection::new();
        functions.function(0);
        let mut exports = ExportSection::new();
        exports.export("main", ExportKind::Func, 0);
        let mut body = Function::new([]);
        body.instruction(&Instruction::Loop(BlockType::Empty));
        body.instruction(&Instruction::Br(0));
        body.instruction(&Instruction::End);
        body.instruction(&Instruction::End);
        let mut code = CodeSection::new();
        code.function(&body);
        let mut core_module = Module::new();
        core_module.section(&types);
        core_module.section(&functions);
        core_module.section(&exports);
        core_module.section(&code);

        let mut builder = ComponentBuilder::default();
        let module = builder.core_module_raw(Some("loop-core"), &core_module.finish());
        let instance = builder.core_instantiate(
            Some("loop-core"),
            module,
            std::iter::empty::<(&str, ModuleArg)>(),
        );
        let function = builder.core_alias_export(Some("main"), instance, "main", ExportKind::Func);
        let (ty, mut function_type) = builder.type_function(Some("main"));
        function_type.params(std::iter::empty::<(&str, wasm_encoder::ComponentValType)>());
        function_type.result(None);
        let lifted = builder.lift_func(Some("main"), function, ty, []);
        builder.export("main", ComponentExportKind::Func, lifted, None);
        builder.finish()
    }

    fn test_directory() -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sico-runtime-storage-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[cfg(windows)]
    fn create_directory_link(target: &Path, link: &Path) {
        let status = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[cfg(not(windows))]
    fn create_directory_link(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
}
