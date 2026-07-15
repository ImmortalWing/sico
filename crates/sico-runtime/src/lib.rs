//! Minimal selected-Runtime boundary for compiler-produced Components.

#![forbid(unsafe_code)]

use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component as PathComponent, Path, PathBuf},
    process::{Command, ExitStatus},
    sync::atomic::{AtomicU64, Ordering},
};

use sico_package::{AuthorizedPackage, TrustStatus, sha256_hex};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum RuntimeError {
    TemporaryArtifact(std::io::Error),
    Launch(std::io::Error),
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
    if granted.contains("clock.read") || granted.contains("random.read") {
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
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(RuntimeError::TemporaryArtifact)?;
    if let Err(error) = file.write_all(component).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&path);
        return Err(RuntimeError::TemporaryArtifact(error));
    }
    drop(file);

    let result = Command::new(runtime)
        .arg("run")
        .arg("--codegen")
        .arg("cache=n")
        .arg("--invoke")
        .arg(invocation)
        .arg(&path)
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

    use super::{RuntimeError, StorageError, prepare_storage, run_component, wasi_arguments};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn launch_failure_still_removes_temporary_component() {
        let before = temporary_components();
        let error = run_component(
            OsStr::new("sico-runtime-command-that-does-not-exist"),
            b"component",
            "main()",
        )
        .unwrap_err();
        assert!(matches!(error, RuntimeError::Launch(_)));
        assert_eq!(temporary_components(), before);
    }

    fn temporary_components() -> Vec<std::path::PathBuf> {
        let prefix = format!("sico-runtime-{}-", std::process::id());
        let mut paths: Vec<_> = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| {
                        name.starts_with(&prefix) && name.ends_with(".component.wasm")
                    })
            })
            .collect();
        paths.sort();
        paths
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

    fn authorized(app_id: &str) -> sico_package::AuthorizedPackage {
        let source = SourceFile::from_text(
            SourceId::new(99),
            "main.sico",
            "function main() returns Unit:\n  return Unit\nend function\n",
        )
        .unwrap();
        let component = compile_component(&lower_core(&source).unwrap()).unwrap();
        let package = build_unsigned(BuildInput {
            app_id: app_id.to_owned(),
            app_version: "0.1.0".to_owned(),
            component,
            resources: Vec::new(),
            source_effects: Vec::new(),
            capabilities: Vec::new(),
            limits: RuntimeLimits::default(),
        })
        .unwrap();
        let trusted = verify_trusted(&package, &TrustPolicy::AllowUnsignedDevelopment).unwrap();
        authorize(trusted, &BTreeSet::new()).unwrap()
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
