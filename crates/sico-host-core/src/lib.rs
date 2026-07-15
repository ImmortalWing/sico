//! Shared fail-closed install/open pipeline for Sico Desktop Host.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use sico_package::{
    AuthorizedPackage, PackageError, TrustPolicy, TrustStatus, authorize, sha256_hex,
    verify_trusted,
};

mod lifecycle;
mod permission;
mod ui;

pub use lifecycle::{
    CommandSpec, GuestState, LaunchOutcome, LifecycleError, OpenRequest, ProcessSupervisor,
    TerminalOutcome,
};
pub use permission::{
    PermissionChoice, PermissionError, PermissionOutcome, PermissionPrompt, PermissionSession,
    PermissionStore,
};
pub use ui::{
    EventGate, MAX_QUEUED_UI_EVENTS, MAX_TEXT_BYTES, MAX_UI_EVENTS_PER_SECOND, RenderNode,
    RenderPlan, UiError, UiEvent, UiEventKind, UiModel, UiNode, UiNodeKind,
};

static STAGING_ID: AtomicU64 = AtomicU64::new(0);
const APP_ID_DOMAIN: &[u8] = b"SICO-DESKTOP-APP-ID-V0\0";
const CAPABILITY_DOMAIN: &[u8] = b"SICO-DESKTOP-CAPABILITY-V0\0";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InstallOptions {
    pub allow_downgrade: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledRevision {
    pub schema: String,
    pub app_identity: String,
    pub app_id: String,
    pub app_version: String,
    pub trust_identity: String,
    pub revision_digest: String,
    pub capability_fingerprint: String,
    pub package_bytes: u64,
}

#[derive(Debug)]
pub struct OpenedPackage {
    pub metadata: InstalledRevision,
    pub package_path: PathBuf,
    pub package: AuthorizedPackage,
}

#[derive(Clone, Debug)]
pub struct HostStore {
    root: PathBuf,
}

#[derive(Debug)]
pub enum HostError {
    Io(std::io::Error),
    Package(PackageError),
    InvalidStore,
    InvalidPackagePath,
    UnsignedPersistentInstall,
    Metadata(String),
    IdentityMismatch,
    DigestMismatch,
    Downgrade {
        installed: String,
        requested: String,
    },
    AtomicInstall,
}

impl std::fmt::Display for HostError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "Desktop Host I/O failed: {error}"),
            Self::Package(error) => write!(formatter, "package verification failed: {error}"),
            Self::InvalidStore => formatter.write_str("Desktop Host store is not a safe directory"),
            Self::InvalidPackagePath => {
                formatter.write_str("Desktop Host input must be a .sapp file")
            }
            Self::UnsignedPersistentInstall => {
                formatter.write_str("unsigned development package cannot be installed persistently")
            }
            Self::Metadata(message) => {
                write!(formatter, "installed metadata is invalid: {message}")
            }
            Self::IdentityMismatch => formatter.write_str("installed app identity does not close"),
            Self::DigestMismatch => formatter.write_str("installed package digest does not match"),
            Self::Downgrade {
                installed,
                requested,
            } => write!(
                formatter,
                "package downgrade refused: installed={installed} requested={requested}"
            ),
            Self::AtomicInstall => formatter.write_str("atomic revision install failed"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<std::io::Error> for HostError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PackageError> for HostError {
    fn from(error: PackageError) -> Self {
        Self::Package(error)
    }
}

impl HostStore {
    /// Creates or opens a symlink/reparse-safe Host store.
    ///
    /// # Errors
    ///
    /// Rejects unsafe roots and I/O failures.
    pub fn open(root: &Path) -> Result<Self, HostError> {
        fs::create_dir_all(root)?;
        reject_link_or_file(root)?;
        let root = root.canonicalize()?;
        let apps = root.join("apps");
        fs::create_dir_all(&apps)?;
        reject_link_or_file(&apps)?;
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Installs a locally trusted signed package by exact digest.
    ///
    /// # Errors
    ///
    /// Rejects non-`.sapp`, unsigned/untrusted/malformed packages, capability
    /// closure failures, unsafe stores, downgrades and partial installs.
    pub fn install_path(
        &self,
        source: &Path,
        trusted_keys: &BTreeSet<[u8; 32]>,
        options: InstallOptions,
    ) -> Result<InstalledRevision, HostError> {
        if !source
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sapp"))
        {
            return Err(HostError::InvalidPackagePath);
        }
        let bytes = fs::read(source)?;
        self.install_bytes(&bytes, trusted_keys, options)
    }

    /// Installs already-captured bytes. The bytes are never reread from an
    /// attacker-controlled source path.
    ///
    /// # Errors
    ///
    /// Same contract as [`Self::install_path`].
    pub fn install_bytes(
        &self,
        bytes: &[u8],
        trusted_keys: &BTreeSet<[u8; 32]>,
        options: InstallOptions,
    ) -> Result<InstalledRevision, HostError> {
        let trusted = verify_trusted(
            bytes,
            &TrustPolicy::RequireDevelopment(trusted_keys.clone()),
        )?;
        let public_key = match &trusted.trust {
            TrustStatus::Development { public_key } => public_key.clone(),
            TrustStatus::UnsignedDevelopment => {
                return Err(HostError::UnsignedPersistentInstall);
            }
        };
        let requested: BTreeSet<_> = trusted
            .package
            .manifest
            .capabilities
            .iter()
            .cloned()
            .collect();
        let authorized = authorize(trusted, &requested)?;
        let manifest = &authorized.trusted.package.manifest;
        let app_identity = app_identity_key(&manifest.app.id, &public_key);
        let revision_digest = sha256_hex(bytes);
        let capability_fingerprint = capability_fingerprint(&requested);
        let package_bytes = u64::try_from(bytes.len())
            .map_err(|_| HostError::Metadata("package length exceeds u64".to_owned()))?;
        let metadata = InstalledRevision {
            schema: "sico.desktop.installed-revision.v0".to_owned(),
            app_identity: app_identity.clone(),
            app_id: manifest.app.id.clone(),
            app_version: manifest.app.version.clone(),
            trust_identity: public_key,
            revision_digest: revision_digest.clone(),
            capability_fingerprint,
            package_bytes,
        };
        self.refuse_downgrade(&metadata, options)?;
        self.atomic_install(&metadata, bytes)?;
        self.verify_installed_metadata(&app_identity, &revision_digest)
    }

    /// Reopens an installed immutable revision and repeats every trust,
    /// digest, identity and capability-closure gate.
    ///
    /// # Errors
    ///
    /// Rejects missing/tampered metadata or package bytes, trust drift,
    /// identity mismatch, and denied/unknown capabilities.
    pub fn open_installed(
        &self,
        app_identity: &str,
        revision_digest: &str,
        trusted_keys: &BTreeSet<[u8; 32]>,
        host_grants: &BTreeSet<String>,
    ) -> Result<OpenedPackage, HostError> {
        validate_digest(app_identity)?;
        validate_digest(revision_digest)?;
        let metadata = self.verify_installed_metadata(app_identity, revision_digest)?;
        let package_path = self
            .revision_path(app_identity, revision_digest)
            .join("app.sapp");
        reject_regular_file(&package_path)?;
        let bytes = fs::read(&package_path)?;
        let package_bytes = u64::try_from(bytes.len())
            .map_err(|_| HostError::Metadata("package length exceeds u64".to_owned()))?;
        if sha256_hex(&bytes) != revision_digest || package_bytes != metadata.package_bytes {
            return Err(HostError::DigestMismatch);
        }
        let trusted = verify_trusted(
            &bytes,
            &TrustPolicy::RequireDevelopment(trusted_keys.clone()),
        )?;
        let TrustStatus::Development { public_key } = &trusted.trust else {
            return Err(HostError::UnsignedPersistentInstall);
        };
        if app_identity_key(&trusted.package.manifest.app.id, public_key) != app_identity
            || trusted.package.manifest.app.id != metadata.app_id
            || trusted.package.manifest.app.version != metadata.app_version
            || *public_key != metadata.trust_identity
        {
            return Err(HostError::IdentityMismatch);
        }
        let package = authorize(trusted, host_grants)?;
        if capability_fingerprint(&package.granted_capabilities) != metadata.capability_fingerprint
        {
            return Err(HostError::IdentityMismatch);
        }
        Ok(OpenedPackage {
            metadata,
            package_path,
            package,
        })
    }

    /// Removes every installed revision for one immutable app identity.
    ///
    /// # Errors
    ///
    /// Rejects invalid identities, link/reparse targets and I/O failures.
    pub fn uninstall(&self, app_identity: &str) -> Result<bool, HostError> {
        validate_digest(app_identity)?;
        let app = self.root.join("apps").join(app_identity);
        if !app.exists() {
            return Ok(false);
        }
        reject_link_or_file(&app)?;
        if app.parent() != Some(self.root.join("apps").as_path()) {
            return Err(HostError::InvalidStore);
        }
        fs::remove_dir_all(app)?;
        Ok(true)
    }

    fn atomic_install(
        &self,
        metadata: &InstalledRevision,
        package: &[u8],
    ) -> Result<(), HostError> {
        let revisions = self
            .root
            .join("apps")
            .join(&metadata.app_identity)
            .join("revisions");
        fs::create_dir_all(&revisions)?;
        reject_link_or_file(&revisions)?;
        let target = revisions.join(&metadata.revision_digest);
        if target.exists() {
            self.verify_existing_bytes(metadata, package)?;
            return Ok(());
        }
        let staging = revisions.join(format!(
            ".staging-{}-{}",
            std::process::id(),
            STAGING_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staging)?;
        let result = (|| {
            write_new_file(&staging.join("app.sapp"), package)?;
            let metadata_bytes = serde_json::to_vec(metadata)
                .map_err(|error| HostError::Metadata(error.to_string()))?;
            write_new_file(&staging.join("metadata.json"), &metadata_bytes)?;
            match fs::rename(&staging, &target) {
                Ok(()) => Ok(()),
                Err(_error) if target.exists() => {
                    self.verify_existing_bytes(metadata, package)?;
                    let _ = fs::remove_dir_all(&staging);
                    Ok(())
                }
                Err(_) => Err(HostError::AtomicInstall),
            }
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    fn verify_existing_bytes(
        &self,
        metadata: &InstalledRevision,
        package: &[u8],
    ) -> Result<(), HostError> {
        let existing =
            self.verify_installed_metadata(&metadata.app_identity, &metadata.revision_digest)?;
        if existing != *metadata {
            return Err(HostError::IdentityMismatch);
        }
        let bytes = fs::read(
            self.revision_path(&metadata.app_identity, &metadata.revision_digest)
                .join("app.sapp"),
        )?;
        if bytes != package {
            return Err(HostError::DigestMismatch);
        }
        Ok(())
    }

    fn verify_installed_metadata(
        &self,
        app_identity: &str,
        revision_digest: &str,
    ) -> Result<InstalledRevision, HostError> {
        validate_digest(app_identity)?;
        validate_digest(revision_digest)?;
        let revision = self.revision_path(app_identity, revision_digest);
        reject_link_or_file(&revision)?;
        let path = revision.join("metadata.json");
        reject_regular_file(&path)?;
        let bytes = fs::read(path)?;
        let metadata: InstalledRevision = serde_json::from_slice(&bytes)
            .map_err(|error| HostError::Metadata(error.to_string()))?;
        let canonical = serde_json::to_vec(&metadata)
            .map_err(|error| HostError::Metadata(error.to_string()))?;
        if bytes != canonical
            || metadata.schema != "sico.desktop.installed-revision.v0"
            || metadata.app_identity != app_identity
            || metadata.revision_digest != revision_digest
        {
            return Err(HostError::IdentityMismatch);
        }
        Ok(metadata)
    }

    fn refuse_downgrade(
        &self,
        requested: &InstalledRevision,
        options: InstallOptions,
    ) -> Result<(), HostError> {
        if options.allow_downgrade {
            return Ok(());
        }
        let revisions = self
            .root
            .join("apps")
            .join(&requested.app_identity)
            .join("revisions");
        let Ok(entries) = fs::read_dir(revisions) else {
            return Ok(());
        };
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let Some(digest) = name.to_str().filter(|name| is_digest(name)) else {
                continue;
            };
            let installed = self.verify_installed_metadata(&requested.app_identity, digest)?;
            if version_less(&requested.app_version, &installed.app_version) {
                return Err(HostError::Downgrade {
                    installed: installed.app_version,
                    requested: requested.app_version.clone(),
                });
            }
        }
        Ok(())
    }

    fn revision_path(&self, app_identity: &str, revision_digest: &str) -> PathBuf {
        self.root
            .join("apps")
            .join(app_identity)
            .join("revisions")
            .join(revision_digest)
    }
}

#[must_use]
pub fn app_identity_key(app_id: &str, trust_identity: &str) -> String {
    let mut material = APP_ID_DOMAIN.to_vec();
    material.extend_from_slice(app_id.as_bytes());
    material.push(0);
    material.extend_from_slice(trust_identity.as_bytes());
    sha256_hex(&material)
}

#[must_use]
pub fn capability_fingerprint(capabilities: &BTreeSet<String>) -> String {
    let mut material = CAPABILITY_DOMAIN.to_vec();
    for capability in capabilities {
        material.extend_from_slice(capability.as_bytes());
        material.push(0);
    }
    sha256_hex(&material)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), HostError> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), HostError> {
    if is_digest(digest) {
        Ok(())
    } else {
        Err(HostError::IdentityMismatch)
    }
}

fn is_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn version_less(requested: &str, installed: &str) -> bool {
    let parse = |version: &str| {
        version
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
    };
    matches!((parse(requested), parse(installed)), (Some(left), Some(right)) if left < right)
}

fn reject_regular_file(path: &Path) -> Result<(), HostError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_like(&metadata) || !metadata.is_file() {
        Err(HostError::InvalidStore)
    } else {
        Ok(())
    }
}

fn reject_link_or_file(path: &Path) -> Result<(), HostError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_like(&metadata) || !metadata.is_dir() {
        Err(HostError::InvalidStore)
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn is_link_like(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
fn is_link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
