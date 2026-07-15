use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

use crate::{OpenedPackage, is_link_like};

static RECORD_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionChoice {
    Deny,
    AllowOnce,
    AllowPersistent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PermissionPrompt {
    pub schema: String,
    pub app_id: String,
    pub app_version: String,
    pub signer_fingerprint: String,
    pub package_digest: String,
    pub capability_fingerprint: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PermissionOutcome {
    Granted(BTreeSet<String>),
    Denied(Vec<String>),
    PromptRequired(PermissionPrompt),
}

#[derive(Clone, Debug, Default)]
pub struct PermissionSession {
    grants: BTreeMap<String, BTreeSet<String>>,
}

impl PermissionSession {
    pub fn clear(&mut self) {
        self.grants.clear();
    }
}

#[derive(Clone, Debug)]
pub struct PermissionStore {
    root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PermissionRecord {
    schema: String,
    app_identity: String,
    app_id: String,
    trust_identity: String,
    created_revision: String,
    capability_fingerprint: String,
    persistent_grants: Vec<String>,
}

#[derive(Debug)]
pub enum PermissionError {
    Io(std::io::Error),
    UnsafeStore,
    DecisionSetMismatch,
    InvalidRecord(String),
    IdentityMismatch,
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "permission I/O failed: {error}"),
            Self::UnsafeStore => formatter.write_str("permission store is unsafe"),
            Self::DecisionSetMismatch => {
                formatter.write_str("permission decision set does not match requested capabilities")
            }
            Self::InvalidRecord(message) => {
                write!(formatter, "permission record invalid: {message}")
            }
            Self::IdentityMismatch => {
                formatter.write_str("permission record identity does not close")
            }
        }
    }
}

impl std::error::Error for PermissionError {}

impl From<std::io::Error> for PermissionError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl PermissionStore {
    /// Opens a direct-child, link-safe permission record store.
    ///
    /// # Errors
    ///
    /// Rejects unsafe paths and I/O failures.
    pub fn open(host_root: &Path) -> Result<Self, PermissionError> {
        let root = host_root.join("permissions");
        fs::create_dir_all(&root)?;
        reject_directory(&root)?;
        Ok(Self {
            root: root.canonicalize()?,
        })
    }

    #[must_use]
    pub fn prompt(package: &OpenedPackage) -> PermissionPrompt {
        PermissionPrompt {
            schema: "sico.desktop.permission-prompt.v0".to_owned(),
            app_id: package.metadata.app_id.clone(),
            app_version: package.metadata.app_version.clone(),
            signer_fingerprint: package.metadata.trust_identity.clone(),
            package_digest: package.metadata.revision_digest.clone(),
            capability_fingerprint: package.metadata.capability_fingerprint.clone(),
            capabilities: package
                .package
                .granted_capabilities
                .iter()
                .cloned()
                .collect(),
        }
    }

    /// Resolves durable plus session grants without prompting.
    ///
    /// # Errors
    ///
    /// Corrupt, unknown-version, non-canonical or identity-drifted records fail
    /// closed. Missing records return `PromptRequired`.
    pub fn resolve(
        &self,
        package: &OpenedPackage,
        session: &PermissionSession,
    ) -> Result<PermissionOutcome, PermissionError> {
        let requested = &package.package.granted_capabilities;
        if requested.is_empty() {
            return Ok(PermissionOutcome::Granted(BTreeSet::new()));
        }
        let key = record_key(package);
        let mut granted = session.grants.get(&key).cloned().unwrap_or_default();
        if let Some(persistent) = self.read_record(package)? {
            granted.extend(persistent);
        }
        if requested.is_subset(&granted) {
            Ok(PermissionOutcome::Granted(requested.clone()))
        } else {
            Ok(PermissionOutcome::PromptRequired(Self::prompt(package)))
        }
    }

    /// Applies one exact decision per requested capability.
    ///
    /// # Errors
    ///
    /// Rejects missing/extra decisions and unsafe/corrupt existing records.
    pub fn apply(
        &self,
        package: &OpenedPackage,
        decisions: &BTreeMap<String, PermissionChoice>,
        session: &mut PermissionSession,
    ) -> Result<PermissionOutcome, PermissionError> {
        let requested = &package.package.granted_capabilities;
        let decided: BTreeSet<_> = decisions.keys().cloned().collect();
        if decided != *requested {
            return Err(PermissionError::DecisionSetMismatch);
        }
        let denied: Vec<_> = decisions
            .iter()
            .filter(|(_, choice)| **choice == PermissionChoice::Deny)
            .map(|(capability, _)| capability.clone())
            .collect();
        if !denied.is_empty() {
            return Ok(PermissionOutcome::Denied(denied));
        }
        let once: BTreeSet<_> = decisions
            .iter()
            .filter(|(_, choice)| **choice == PermissionChoice::AllowOnce)
            .map(|(capability, _)| capability.clone())
            .collect();
        let persistent: BTreeSet<_> = decisions
            .iter()
            .filter(|(_, choice)| **choice == PermissionChoice::AllowPersistent)
            .map(|(capability, _)| capability.clone())
            .collect();
        let key = record_key(package);
        session.grants.entry(key).or_default().extend(once);
        if !persistent.is_empty() {
            self.write_record(package, &persistent)?;
        }
        Ok(PermissionOutcome::Granted(requested.clone()))
    }

    /// Removes every persistent decision for one immutable app identity.
    ///
    /// # Errors
    ///
    /// Rejects malformed identities, link/reparse targets and I/O failures.
    pub fn remove_app(&self, app_identity: &str) -> Result<bool, PermissionError> {
        if app_identity.len() != 64
            || !app_identity
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(PermissionError::IdentityMismatch);
        }
        let directory = self.root.join(app_identity);
        if !directory.exists() {
            return Ok(false);
        }
        reject_directory(&directory)?;
        fs::remove_dir_all(directory)?;
        Ok(true)
    }

    fn write_record(
        &self,
        package: &OpenedPackage,
        persistent: &BTreeSet<String>,
    ) -> Result<(), PermissionError> {
        let directory = self.root.join(&package.metadata.app_identity);
        fs::create_dir_all(&directory)?;
        reject_directory(&directory)?;
        let record = PermissionRecord {
            schema: "sico.desktop.permission.v0".to_owned(),
            app_identity: package.metadata.app_identity.clone(),
            app_id: package.metadata.app_id.clone(),
            trust_identity: package.metadata.trust_identity.clone(),
            created_revision: package.metadata.revision_digest.clone(),
            capability_fingerprint: package.metadata.capability_fingerprint.clone(),
            persistent_grants: persistent.iter().cloned().collect(),
        };
        let bytes = serde_json::to_vec(&record)
            .map_err(|error| PermissionError::InvalidRecord(error.to_string()))?;
        let path = record_path(&directory, package);
        if path.exists() {
            let existing = fs::read(&path)?;
            if existing == bytes {
                return Ok(());
            }
            return Err(PermissionError::IdentityMismatch);
        }
        let temporary = directory.join(format!(
            ".permission-{}-{}.tmp",
            std::process::id(),
            RECORD_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            return Err(PermissionError::Io(error));
        }
        drop(file);
        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(()),
            Err(_) if path.exists() && fs::read(path)? == bytes => {
                let _ = fs::remove_file(temporary);
                Ok(())
            }
            Err(error) => {
                let _ = fs::remove_file(temporary);
                Err(PermissionError::Io(error))
            }
        }
    }

    fn read_record(
        &self,
        package: &OpenedPackage,
    ) -> Result<Option<BTreeSet<String>>, PermissionError> {
        let directory = self.root.join(&package.metadata.app_identity);
        if !directory.exists() {
            return Ok(None);
        }
        reject_directory(&directory)?;
        let path = record_path(&directory, package);
        if !path.exists() {
            return Ok(None);
        }
        reject_file(&path)?;
        let bytes = fs::read(path)?;
        let record: PermissionRecord = serde_json::from_slice(&bytes)
            .map_err(|error| PermissionError::InvalidRecord(error.to_string()))?;
        let canonical = serde_json::to_vec(&record)
            .map_err(|error| PermissionError::InvalidRecord(error.to_string()))?;
        if canonical != bytes
            || record.schema != "sico.desktop.permission.v0"
            || record.app_identity != package.metadata.app_identity
            || record.app_id != package.metadata.app_id
            || record.trust_identity != package.metadata.trust_identity
            || record.capability_fingerprint != package.metadata.capability_fingerprint
        {
            return Err(PermissionError::IdentityMismatch);
        }
        let grants: BTreeSet<_> = record.persistent_grants.into_iter().collect();
        if !grants.is_subset(&package.package.granted_capabilities) {
            return Err(PermissionError::IdentityMismatch);
        }
        Ok(Some(grants))
    }
}

fn record_key(package: &OpenedPackage) -> String {
    format!(
        "{}:{}",
        package.metadata.app_identity, package.metadata.capability_fingerprint
    )
}

fn record_path(directory: &Path, package: &OpenedPackage) -> PathBuf {
    directory.join(format!("{}.json", package.metadata.capability_fingerprint))
}

fn reject_directory(path: &Path) -> Result<(), PermissionError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_like(&metadata) || !metadata.is_dir() {
        Err(PermissionError::UnsafeStore)
    } else {
        Ok(())
    }
}

fn reject_file(path: &Path) -> Result<(), PermissionError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_like(&metadata) || !metadata.is_file() {
        Err(PermissionError::UnsafeStore)
    } else {
        Ok(())
    }
}
