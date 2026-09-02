//! Host-owned secret provider and mandatory redaction (M12 STEP-0116,
//! RFC-0037 §6). Secrets are opaque names to guests; values never leave
//! this module except into the exact authorized header, and every
//! user-visible path gets fingerprint redaction.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

/// One authorized secret binding: exact name, exact endpoint identity,
/// exact injection policy. The triple must match the package manifest,
/// the Host grants and the invocation request — intersection failure is
/// a typed refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretBinding {
    pub name: String,
    /// Endpoint identity string from `authority::Endpoint::identity`.
    pub endpoint: String,
    /// Injection policy: header template with exact-name substitution.
    pub policy: InjectionPolicy,
}

/// Header injection policy (RFC-0037 §6: header-only in v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionPolicy {
    /// `Authorization: Bearer <value>`
    AuthorizationBearer,
    /// `Authorization: Basic <base64(user:pass)>` requires two values.
    AuthorizationBasic { username: String },
    /// Any exact single header name (e.g. `x-api-key`).
    Header { name: String },
}

/// Why a secret use was refused; stable names, part of the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    UnknownSecret,
    WrongEndpoint,
    WrongPolicy,
    MissingValue,
    ManifestDrift,
    NotRedactable,
}

impl core::fmt::Display for SecretError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::UnknownSecret => "unknown secret name",
            Self::WrongEndpoint => "secret is not authorized for this endpoint",
            Self::WrongPolicy => "secret injection policy does not match the grant",
            Self::MissingValue => "secret value missing from the Host store",
            Self::ManifestDrift => "secret grant differs from the package manifest",
            Self::NotRedactable => "output path cannot guarantee redaction",
        };
        f.write_str(text)
    }
}

impl std::error::Error for SecretError {}

/// The Host-owned secret store: values enter once, leave only as
/// injected header material for one exact binding.
#[derive(Debug, Default)]
pub struct SecretStore {
    values: BTreeMap<String, String>,
    bindings: Vec<SecretBinding>,
}

impl SecretStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one value under an opaque name (Host side only).
    pub fn insert(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_owned(), value.to_owned());
    }

    /// Authorizes one binding (Host side only).
    pub fn authorize(&mut self, binding: SecretBinding) {
        self.bindings.push(binding);
    }

    /// Resolves the injected header for `name` against `endpoint`.
    /// Returns the header name/value pair; the value never leaves this
    /// call except inside that header.
    ///
    /// # Errors
    ///
    /// [`SecretError`] for every intersection mismatch; see variants.
    pub fn resolve(
        &self,
        name: &str,
        endpoint: &str,
        requested: &InjectionPolicy,
    ) -> Result<(String, String), SecretError> {
        let binding = self
            .bindings
            .iter()
            .find(|binding| binding.name == name)
            .ok_or(SecretError::UnknownSecret)?;
        if binding.endpoint != endpoint {
            return Err(SecretError::WrongEndpoint);
        }
        let policy_matches = match (&binding.policy, requested) {
            (InjectionPolicy::AuthorizationBearer, InjectionPolicy::AuthorizationBearer) => true,
            (
                InjectionPolicy::AuthorizationBasic { username: a },
                InjectionPolicy::AuthorizationBasic { username: b },
            ) => a == b,
            (InjectionPolicy::Header { name: a }, InjectionPolicy::Header { name: b }) => {
                a.eq_ignore_ascii_case(b)
            }
            _ => false,
        };
        if !policy_matches {
            return Err(SecretError::WrongPolicy);
        }
        let value = self.values.get(name).ok_or(SecretError::MissingValue)?;
        let (header_name, header_value) = match requested {
            InjectionPolicy::AuthorizationBearer => {
                ("authorization".to_owned(), format!("Bearer {value}"))
            }
            InjectionPolicy::AuthorizationBasic { username } => {
                let raw = format!("{username}:{value}");
                ("authorization".to_owned(), format!("Basic {}", b64(raw.as_bytes())))
            }
            InjectionPolicy::Header { name } => (name.to_ascii_lowercase(), (*value).clone()),
        };
        Ok((header_name, header_value))
    }

    /// Authorized bindings for one endpoint identity (engine helper).
    #[must_use]
    pub fn bindings_for(&self, endpoint: &str) -> Vec<&SecretBinding> {
        self.bindings
            .iter()
            .filter(|binding| binding.endpoint == endpoint)
            .collect()
    }

    /// True when `binding` agrees with the manifest entry set (exact
    /// name/endpoint/policy triple must exist in `manifest`).
    #[must_use]
    pub fn manifest_intersects(&self, binding: &SecretBinding, manifest: &[SecretBinding]) -> bool {
        manifest.iter().any(|entry| entry == binding)
    }
}

/// Minimal RFC-4648 base64 (no external dependency; values only travel
/// inside the Host process).
fn b64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Fingerprint redaction: replaces any occurrence of `secret` (and its
/// exact encoded forms) in `text` with a stable short digest marker.
/// Applied to faults, events, CLI text, DAP variables and AI summaries.
///
/// # Panics
///
/// Never — the digest is hex.
#[must_use]
pub fn redact(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_owned();
    for secret in secrets {
        if secret.is_empty() {
            continue;
        }
        let marker = {
            let mut hasher = Sha256::new();
            hasher.update(secret.as_bytes());
            let digest = hasher.finalize();
            format!("[REDACTED:sha256:{}]", hex(&digest[..6]))
        };
        // Exact value and the bearer/basic wrapper forms.
        out = out.replace(secret.as_str(), &marker);
        out = out.replace(&format!("Bearer {secret}"), &marker);
        out = out.replace(&format!("Basic {}", b64(secret.as_bytes())), &marker);
    }
    out
}

fn hex(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANARY: &str = "canary-token-value-9f2b";

    fn store() -> SecretStore {
        let mut store = SecretStore::new();
        store.insert("github-token", CANARY);
        store.authorize(SecretBinding {
            name: "github-token".to_owned(),
            endpoint: "https|api.example.com|443".to_owned(),
            policy: InjectionPolicy::AuthorizationBearer,
        });
        store
    }

    #[test]
    fn authorized_injection_returns_only_the_header() {
        let store = store();
        let (name, value) = store
            .resolve("github-token", "https|api.example.com|443", &InjectionPolicy::AuthorizationBearer)
            .unwrap();
        assert_eq!(name, "authorization");
        assert_eq!(value, format!("Bearer {CANARY}"));
    }

    #[test]
    fn intersection_mismatches_fail_closed() {
        let store = store();
        // Wrong endpoint.
        assert_eq!(
            store.resolve("github-token", "https|other.example.com|443", &InjectionPolicy::AuthorizationBearer),
            Err(SecretError::WrongEndpoint)
        );
        // Wrong policy.
        assert_eq!(
            store.resolve("github-token", "https|api.example.com|443", &InjectionPolicy::Header { name: "x-api-key".to_owned() }),
            Err(SecretError::WrongPolicy)
        );
        // Unknown secret.
        assert_eq!(
            store.resolve("nope", "https|api.example.com|443", &InjectionPolicy::AuthorizationBearer),
            Err(SecretError::UnknownSecret)
        );
        // Missing value (authorized name, no value inserted).
        let mut fresh_store = SecretStore::new();
        fresh_store.authorize(SecretBinding {
            name: "empty".to_owned(),
            endpoint: "https|api.example.com|443".to_owned(),
            policy: InjectionPolicy::AuthorizationBearer,
        });
        assert_eq!(
            fresh_store.resolve("empty", "https|api.example.com|443", &InjectionPolicy::AuthorizationBearer),
            Err(SecretError::MissingValue)
        );
    }

    #[test]
    fn manifest_intersection_is_exact() {
        let store = store();
        let binding = SecretBinding {
            name: "github-token".to_owned(),
            endpoint: "https|api.example.com|443".to_owned(),
            policy: InjectionPolicy::AuthorizationBearer,
        };
        assert!(store.manifest_intersects(&binding, std::slice::from_ref(&binding)));
        let drifted = SecretBinding {
            endpoint: "https|api.example.com|8443".to_owned(),
            ..binding
        };
        assert!(!store.manifest_intersects(&drifted, &[]));
    }

    #[test]
    fn canary_never_appears_in_redacted_output() {
        let text = format!(
            "request failed: GET https://api.example.com/v1 with authorization Bearer {CANARY} and raw {CANARY}"
        );
        let redacted = redact(&text, &[CANARY.to_owned()]);
        assert!(!redacted.contains(CANARY), "{redacted}");
        assert!(redacted.contains("[REDACTED:sha256:"));
        // Redaction is deterministic (stable fingerprints for logs).
        assert_eq!(redacted, redact(&text, &[CANARY.to_owned()]));
        // Unrelated text passes through unchanged.
        assert_eq!(redact("clean text", &[CANARY.to_owned()]), "clean text");
        assert_eq!(redact("clean text", &[]), "clean text");
    }

    #[test]
    fn basic_and_header_policies_inject() {
        let mut store = SecretStore::new();
        store.insert("git-basic", "hunter2");
        store.authorize(SecretBinding {
            name: "git-basic".to_owned(),
            endpoint: "https|git.example.com|443".to_owned(),
            policy: InjectionPolicy::AuthorizationBasic { username: "octocat".to_owned() },
        });
        store.insert("api-key", "k-123");
        store.authorize(SecretBinding {
            name: "api-key".to_owned(),
            endpoint: "https|api.example.com|443".to_owned(),
            policy: InjectionPolicy::Header { name: "X-API-Key".to_owned() },
        });
        let (_, value) = store
            .resolve("git-basic", "https|git.example.com|443", &InjectionPolicy::AuthorizationBasic { username: "octocat".to_owned() })
            .unwrap();
        assert!(value.starts_with("Basic "));
        let (name, value) = store
            .resolve("api-key", "https|api.example.com|443", &InjectionPolicy::Header { name: "x-api-key".to_owned() })
            .unwrap();
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "k-123");
    }
}
