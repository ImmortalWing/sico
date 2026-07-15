use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AndroidScope {
    AppPrivateStorage,
    Clock,
    SecureRandom,
    HostLog,
    Network,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidPermissionPlan {
    pub schema: String,
    pub capabilities: BTreeSet<String>,
    pub manifest_permissions: BTreeSet<String>,
    pub runtime_permissions: BTreeSet<String>,
    pub scopes: BTreeSet<AndroidScope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AndroidPermissionError {
    UnknownCapability(String),
    InvalidUriGrant,
}

impl std::fmt::Display for AndroidPermissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Android permission mapping rejected: {self:?}")
    }
}

impl std::error::Error for AndroidPermissionError {}

/// Maps an exact authorized Sico capability closure to Android platform needs.
///
/// # Errors
///
/// Unknown capability names fail closed instead of gaining a broad Android
/// permission. v0 has no dangerous runtime permission.
pub fn map_android_permissions(
    capabilities: &BTreeSet<String>,
) -> Result<AndroidPermissionPlan, AndroidPermissionError> {
    let mut manifest_permissions = BTreeSet::new();
    let runtime_permissions = BTreeSet::new();
    let mut scopes = BTreeSet::new();
    for capability in capabilities {
        match capability.as_str() {
            "storage.read-write" => {
                scopes.insert(AndroidScope::AppPrivateStorage);
            }
            "clock.read" => {
                scopes.insert(AndroidScope::Clock);
            }
            "random.read" => {
                scopes.insert(AndroidScope::SecureRandom);
            }
            "log.write" => {
                scopes.insert(AndroidScope::HostLog);
            }
            "network.connect" => {
                manifest_permissions.insert("android.permission.INTERNET".to_owned());
                scopes.insert(AndroidScope::Network);
            }
            unknown => {
                return Err(AndroidPermissionError::UnknownCapability(
                    unknown.to_owned(),
                ));
            }
        }
    }
    Ok(AndroidPermissionPlan {
        schema: "sico.android.permission-plan.v0".to_owned(),
        capabilities: capabilities.clone(),
        manifest_permissions,
        runtime_permissions,
        scopes,
    })
}

#[derive(Clone, Debug, Default)]
pub struct UriGrantSession {
    read_grants: BTreeSet<String>,
}

impl UriGrantSession {
    /// Records a temporary Host ingress grant, never a guest capability.
    ///
    /// # Errors
    ///
    /// Rejects non-content URIs and oversized identifiers.
    pub fn grant_read(&mut self, uri: &str) -> Result<(), AndroidPermissionError> {
        if uri.len() > 4_096 || !uri.starts_with("content://") {
            return Err(AndroidPermissionError::InvalidUriGrant);
        }
        self.read_grants.insert(uri.to_owned());
        Ok(())
    }

    #[must_use]
    pub fn can_read(&self, uri: &str) -> bool {
        self.read_grants.contains(uri)
    }

    pub fn clear(&mut self) {
        self.read_grants.clear();
    }
}
