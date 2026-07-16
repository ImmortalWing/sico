//! Fail-closed ecosystem trust contracts shared by publishing and clients.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const POLICY_SCHEMA: &str = "sico.publisher.policy.v0";
pub const POLICY_UPDATE_SCHEMA: &str = "sico.publisher.policy-update.v0";
pub const MAX_POLICY_BYTES: usize = 128 * 1024;
pub const MAX_KEYS: usize = 64;
pub const MAX_SIGNATURES: usize = 64;
pub const MAX_IDENTITY_RULES: usize = 32;
pub const MAX_POLICY_LIFETIME_SECONDS: u64 = 366 * 24 * 60 * 60;
pub const MAX_TRANSITION_LIFETIME_SECONDS: u64 = 24 * 60 * 60;

const KEY_ID_DOMAIN: &[u8] = b"SICO-PUBLISHER-KEY-ID-V0\0";
const POLICY_SIGNATURE_DOMAIN: &[u8] = b"SICO-PUBLISHER-POLICY-V0\0";
const UPDATE_SIGNATURE_DOMAIN: &[u8] = b"SICO-PUBLISHER-POLICY-UPDATE-V0\0";
const REQUIRED_REDACTIONS: [&str; 4] = ["access-token", "credential", "private-key", "user-data"];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionIdentity {
    pub registry_id: String,
    pub namespace: String,
    pub package_name: String,
    pub publisher_policy_id: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Custody {
    Offline,
    Online,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoleKind {
    Root,
    Release,
    Recovery,
    Rotation,
    Revocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicKeyRecord {
    pub key_id: String,
    pub scheme: String,
    pub public_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePolicy {
    pub custody: Custody,
    pub threshold: u16,
    pub key_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePolicies {
    pub root: RolePolicy,
    pub release: RolePolicy,
    pub recovery: RolePolicy,
    pub rotation: RolePolicy,
    pub revocation: RolePolicy,
}

impl RolePolicies {
    fn entries(&self) -> [(RoleKind, &RolePolicy); 5] {
        [
            (RoleKind::Root, &self.root),
            (RoleKind::Release, &self.release),
            (RoleKind::Recovery, &self.recovery),
            (RoleKind::Rotation, &self.rotation),
            (RoleKind::Revocation, &self.revocation),
        ]
    }

    fn get(&self, role: RoleKind) -> &RolePolicy {
        match role {
            RoleKind::Root => &self.root,
            RoleKind::Release => &self.release,
            RoleKind::Recovery => &self.recovery,
            RoleKind::Rotation => &self.rotation,
            RoleKind::Revocation => &self.revocation,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum IdentityRule {
    Key {
        key_id: String,
    },
    Oidc {
        issuer: String,
        subject: String,
        repository: String,
        workflow_ref: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PresentedIdentity {
    Key {
        key_id: String,
    },
    Oidc {
        issuer: String,
        subject: String,
        repository: String,
        workflow_ref: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisclosureIntake {
    OwnerControlledUnconfigured,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransparencyPolicy {
    OwnerApprovalRequired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisclosurePolicy {
    pub intake: DisclosureIntake,
    pub retention_days: u16,
    pub redacted_fields: Vec<String>,
    pub transparency: TransparencyPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublisherPolicy {
    pub schema: String,
    pub version: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub identity: ProductionIdentity,
    pub keys: Vec<PublicKeyRecord>,
    pub roles: RolePolicies,
    pub identity_rules: Vec<IdentityRule>,
    pub disclosure: DisclosurePolicy,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataSignature {
    pub key_id: String,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedPublisherPolicy {
    signed: PublisherPolicy,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionKind {
    RoutineRotation,
    EmergencyRevocation,
    Recovery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionReason {
    ScheduledRotation,
    SuspectedCompromise,
    CustodyLoss,
    DisasterRecovery,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyTransition {
    pub schema: String,
    pub kind: TransitionKind,
    pub reason: TransitionReason,
    pub previous_policy_sha256: String,
    pub next_policy_version: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub removed_key_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedPolicyUpdate {
    transition: PolicyTransition,
    next_policy: PublisherPolicy,
    signatures: Vec<MetadataSignature>,
}

#[derive(Serialize)]
struct UnsignedPolicyUpdate<'a> {
    transition: &'a PolicyTransition,
    next_policy: &'a PublisherPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    TooLarge,
    Json(String),
    NonCanonical,
    Invalid(&'static str),
    InvalidValue(String),
    NotYetValid,
    Expired,
    DevelopmentKeyReuse(String),
    InvalidSignature(String),
    Threshold(RoleKind),
    Transition(&'static str),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("publisher metadata exceeds the v0 limit"),
            Self::Json(message) => {
                write!(formatter, "publisher metadata JSON is invalid: {message}")
            }
            Self::NonCanonical => formatter.write_str("publisher metadata JSON is not canonical"),
            Self::Invalid(field) => write!(formatter, "publisher metadata is invalid: {field}"),
            Self::InvalidValue(value) => {
                write!(formatter, "publisher metadata value is invalid: {value}")
            }
            Self::NotYetValid => formatter.write_str("publisher metadata is not yet valid"),
            Self::Expired => formatter.write_str("publisher metadata is expired"),
            Self::DevelopmentKeyReuse(key_id) => {
                write!(
                    formatter,
                    "development key cannot be production key: {key_id}"
                )
            }
            Self::InvalidSignature(key_id) => {
                write!(formatter, "publisher signature is invalid: {key_id}")
            }
            Self::Threshold(role) => {
                write!(
                    formatter,
                    "publisher signature threshold is not met: {role:?}"
                )
            }
            Self::Transition(message) => {
                write!(formatter, "publisher transition is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// Returns the domain-separated identifier for an Ed25519 public key.
#[must_use]
pub fn public_key_id(public_key: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(KEY_ID_DOMAIN);
    hasher.update(public_key);
    encode_hex(&hasher.finalize())
}

/// Returns canonical compact JSON bytes for a publisher policy.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn canonical_policy_bytes(policy: &PublisherPolicy) -> Result<Vec<u8>, PolicyError> {
    serde_json::to_vec(policy).map_err(|error| PolicyError::Json(error.to_string()))
}

/// Returns the SHA-256 digest of canonical publisher policy bytes.
///
/// # Errors
///
/// Returns an error if policy serialization fails.
pub fn policy_digest(policy: &PublisherPolicy) -> Result<String, PolicyError> {
    Ok(sha256_hex(&canonical_policy_bytes(policy)?))
}

/// Strictly verifies a canonical initial policy and its root threshold.
///
/// # Errors
///
/// Rejects malformed, non-canonical, invalid, expired, development-key-reusing,
/// weakly signed or threshold-incomplete policies.
pub fn verify_initial_policy_json(
    bytes: &[u8],
    now: u64,
    development_keys: &BTreeSet<[u8; 32]>,
) -> Result<PublisherPolicy, PolicyError> {
    if bytes.len() > MAX_POLICY_BYTES {
        return Err(PolicyError::TooLarge);
    }
    let envelope: SignedPublisherPolicy =
        serde_json::from_slice(bytes).map_err(|error| PolicyError::Json(error.to_string()))?;
    if serde_json::to_vec(&envelope).map_err(|error| PolicyError::Json(error.to_string()))? != bytes
    {
        return Err(PolicyError::NonCanonical);
    }
    if envelope.signed.version != 1 {
        return Err(PolicyError::Invalid("initial policy version must be one"));
    }
    validate_policy(&envelope.signed, now, development_keys)?;
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(
        &envelope.signatures,
        envelope.signed.roles.root.key_ids.iter(),
    )?;
    let payload = policy_signing_bytes(&envelope.signed)?;
    verify_role_threshold(
        &envelope.signed,
        RoleKind::Root,
        &envelope.signatures,
        &payload,
    )?;
    Ok(envelope.signed)
}

/// Strictly verifies a canonical policy transition against the currently
/// trusted policy.
///
/// # Errors
///
/// Rejects rollback, skipped versions, wrong predecessor, invalid key-set
/// changes, expired transitions, development-key reuse and incomplete old/new
/// root or lifecycle-role thresholds.
pub fn verify_policy_update_json(
    current: &PublisherPolicy,
    bytes: &[u8],
    now: u64,
    development_keys: &BTreeSet<[u8; 32]>,
) -> Result<PublisherPolicy, PolicyError> {
    if bytes.len() > MAX_POLICY_BYTES {
        return Err(PolicyError::TooLarge);
    }
    validate_policy_shape(current, development_keys)?;
    let envelope: SignedPolicyUpdate =
        serde_json::from_slice(bytes).map_err(|error| PolicyError::Json(error.to_string()))?;
    if serde_json::to_vec(&envelope).map_err(|error| PolicyError::Json(error.to_string()))? != bytes
    {
        return Err(PolicyError::NonCanonical);
    }
    validate_transition(current, &envelope.transition, &envelope.next_policy, now)?;
    validate_policy(&envelope.next_policy, now, development_keys)?;
    if envelope.transition.kind != TransitionKind::Recovery {
        validate_time(
            current.issued_at,
            current.expires_at,
            now,
            MAX_POLICY_LIFETIME_SECONDS,
        )?;
    }
    validate_signatures(&envelope.signatures)?;
    let payload = update_signing_bytes(&envelope.transition, &envelope.next_policy)?;
    let transition_role = match envelope.transition.kind {
        TransitionKind::RoutineRotation => RoleKind::Rotation,
        TransitionKind::EmergencyRevocation => RoleKind::Revocation,
        TransitionKind::Recovery => RoleKind::Recovery,
    };
    ensure_signature_keys_allowed(
        &envelope.signatures,
        current
            .roles
            .root
            .key_ids
            .iter()
            .chain(&envelope.next_policy.roles.root.key_ids)
            .chain(&current.roles.get(transition_role).key_ids),
    )?;
    verify_role_threshold(current, RoleKind::Root, &envelope.signatures, &payload)?;
    verify_role_threshold(
        &envelope.next_policy,
        RoleKind::Root,
        &envelope.signatures,
        &payload,
    )?;
    verify_role_threshold(current, transition_role, &envelope.signatures, &payload)?;
    Ok(envelope.next_policy)
}

/// Matches an already cryptographically established credential claim against
/// exact signed policy fields. This does not validate certificates or logs.
///
/// # Errors
///
/// Rejects malformed presented claims.
pub fn identity_is_authorized(
    policy: &PublisherPolicy,
    claim: &PresentedIdentity,
) -> Result<bool, PolicyError> {
    validate_policy_shape(policy, &BTreeSet::new())?;
    validate_presented_identity(claim)?;
    Ok(policy
        .identity_rules
        .iter()
        .any(|rule| match (rule, claim) {
            (IdentityRule::Key { key_id: expected }, PresentedIdentity::Key { key_id }) => {
                expected == key_id
            }
            (
                IdentityRule::Oidc {
                    issuer: expected_issuer,
                    subject: expected_subject,
                    repository: expected_repository,
                    workflow_ref: expected_workflow,
                },
                PresentedIdentity::Oidc {
                    issuer,
                    subject,
                    repository,
                    workflow_ref,
                },
            ) => {
                expected_issuer == issuer
                    && expected_subject == subject
                    && expected_repository == repository
                    && expected_workflow == workflow_ref
            }
            _ => false,
        }))
}

/// Produces deterministic signed initial-policy bytes for local fixtures.
/// It performs no key storage and must not be treated as a production signing
/// ceremony.
///
/// # Errors
///
/// Returns an error for invalid policy structure or serialization.
pub fn sign_initial_policy_fixture(
    policy: PublisherPolicy,
    signing_keys: &[SigningKey],
) -> Result<Vec<u8>, PolicyError> {
    validate_policy(&policy, policy.issued_at, &BTreeSet::new())?;
    let payload = policy_signing_bytes(&policy)?;
    let signatures = sign_payload(&payload, signing_keys);
    serde_json::to_vec(&SignedPublisherPolicy {
        signed: policy,
        signatures,
    })
    .map_err(|error| PolicyError::Json(error.to_string()))
}

/// Produces deterministic signed policy-update bytes for local fixtures.
///
/// # Errors
///
/// Returns an error for invalid serialization. Verification remains mandatory.
pub fn sign_policy_update_fixture(
    transition: PolicyTransition,
    next_policy: PublisherPolicy,
    signing_keys: &[SigningKey],
) -> Result<Vec<u8>, PolicyError> {
    let payload = update_signing_bytes(&transition, &next_policy)?;
    let signatures = sign_payload(&payload, signing_keys);
    serde_json::to_vec(&SignedPolicyUpdate {
        transition,
        next_policy,
        signatures,
    })
    .map_err(|error| PolicyError::Json(error.to_string()))
}

fn validate_policy(
    policy: &PublisherPolicy,
    now: u64,
    development_keys: &BTreeSet<[u8; 32]>,
) -> Result<(), PolicyError> {
    validate_policy_shape(policy, development_keys)?;
    validate_time(
        policy.issued_at,
        policy.expires_at,
        now,
        MAX_POLICY_LIFETIME_SECONDS,
    )
}

fn validate_policy_shape(
    policy: &PublisherPolicy,
    development_keys: &BTreeSet<[u8; 32]>,
) -> Result<(), PolicyError> {
    if policy.schema != POLICY_SCHEMA {
        return Err(PolicyError::Invalid("policy schema"));
    }
    if policy.version == 0 {
        return Err(PolicyError::Invalid("policy version"));
    }
    validate_lifetime(
        policy.issued_at,
        policy.expires_at,
        MAX_POLICY_LIFETIME_SECONDS,
    )?;
    validate_identity(&policy.identity)?;
    if policy.keys.is_empty() || policy.keys.len() > MAX_KEYS {
        return Err(PolicyError::Invalid("key count"));
    }
    let mut key_map = BTreeMap::new();
    let mut previous_key_id: Option<&str> = None;
    for record in &policy.keys {
        validate_lower_hex(&record.key_id, 64, "key id")?;
        validate_lower_hex(&record.public_key, 64, "public key")?;
        if record.scheme != "ed25519" {
            return Err(PolicyError::Invalid("key scheme"));
        }
        if previous_key_id.is_some_and(|previous| previous >= record.key_id.as_str()) {
            return Err(PolicyError::Invalid("keys must be sorted and unique"));
        }
        previous_key_id = Some(&record.key_id);
        let key_bytes = decode_hex::<32>(&record.public_key, "public key")?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| PolicyError::Invalid("Ed25519 public key"))?;
        if verifying_key.is_weak() {
            return Err(PolicyError::Invalid("weak Ed25519 public key"));
        }
        if public_key_id(&key_bytes) != record.key_id {
            return Err(PolicyError::Invalid("key id digest"));
        }
        if development_keys.contains(&key_bytes) {
            return Err(PolicyError::DevelopmentKeyReuse(record.key_id.clone()));
        }
        key_map.insert(record.key_id.clone(), key_bytes);
    }

    let mut role_keys = BTreeSet::new();
    for (role, role_policy) in policy.roles.entries() {
        validate_role(role, role_policy, &key_map)?;
        for key_id in &role_policy.key_ids {
            if !role_keys.insert(key_id.clone()) {
                return Err(PolicyError::Invalid("one key cannot serve multiple roles"));
            }
        }
    }

    if policy.identity_rules.is_empty() || policy.identity_rules.len() > MAX_IDENTITY_RULES {
        return Err(PolicyError::Invalid("identity rule count"));
    }
    ensure_sorted_unique(&policy.identity_rules, "identity rules")?;
    let mut referenced_keys = role_keys;
    for rule in &policy.identity_rules {
        match rule {
            IdentityRule::Key { key_id } => {
                validate_lower_hex(key_id, 64, "identity key id")?;
                if !key_map.contains_key(key_id) {
                    return Err(PolicyError::Invalid("identity key is unknown"));
                }
                referenced_keys.insert(key_id.clone());
            }
            IdentityRule::Oidc {
                issuer,
                subject,
                repository,
                workflow_ref,
            } => validate_oidc(issuer, subject, repository, workflow_ref)?,
        }
    }
    if referenced_keys.len() != key_map.len() {
        return Err(PolicyError::Invalid("unused public key"));
    }
    validate_disclosure(&policy.disclosure)
}

fn validate_identity(identity: &ProductionIdentity) -> Result<(), PolicyError> {
    validate_name(&identity.registry_id, 64, "registry id")?;
    validate_name(&identity.namespace, 128, "namespace")?;
    validate_name(&identity.package_name, 64, "package name")?;
    validate_name(&identity.publisher_policy_id, 64, "publisher policy id")
}

fn validate_role(
    role: RoleKind,
    policy: &RolePolicy,
    keys: &BTreeMap<String, [u8; 32]>,
) -> Result<(), PolicyError> {
    let expected_custody = if role == RoleKind::Release {
        Custody::Online
    } else {
        Custody::Offline
    };
    if policy.custody != expected_custody {
        return Err(PolicyError::Invalid("role custody"));
    }
    if policy.key_ids.is_empty() || usize::from(policy.threshold) > policy.key_ids.len() {
        return Err(PolicyError::Invalid("role threshold"));
    }
    if role != RoleKind::Release && policy.threshold < 2 {
        return Err(PolicyError::Invalid(
            "offline role threshold must be at least two",
        ));
    }
    if role == RoleKind::Release && policy.threshold == 0 {
        return Err(PolicyError::Invalid("release role threshold"));
    }
    ensure_sorted_unique(&policy.key_ids, "role key ids")?;
    if policy
        .key_ids
        .iter()
        .any(|key_id| !keys.contains_key(key_id))
    {
        return Err(PolicyError::Invalid("role key is unknown"));
    }
    Ok(())
}

fn validate_disclosure(policy: &DisclosurePolicy) -> Result<(), PolicyError> {
    if !(1..=3_650).contains(&policy.retention_days) {
        return Err(PolicyError::Invalid("disclosure retention"));
    }
    let expected = REQUIRED_REDACTIONS.map(str::to_owned).to_vec();
    if policy.redacted_fields != expected {
        return Err(PolicyError::Invalid("disclosure redaction classes"));
    }
    Ok(())
}

fn validate_transition(
    current: &PublisherPolicy,
    transition: &PolicyTransition,
    next: &PublisherPolicy,
    now: u64,
) -> Result<(), PolicyError> {
    if transition.schema != POLICY_UPDATE_SCHEMA {
        return Err(PolicyError::Transition("schema"));
    }
    validate_time(
        transition.issued_at,
        transition.expires_at,
        now,
        MAX_TRANSITION_LIFETIME_SECONDS,
    )?;
    if next.version
        != current
            .version
            .checked_add(1)
            .ok_or(PolicyError::Transition("version overflow"))?
        || transition.next_policy_version != next.version
    {
        return Err(PolicyError::Transition(
            "next version must increment by one",
        ));
    }
    if current.identity != next.identity {
        return Err(PolicyError::Transition("production identity changed"));
    }
    if transition.previous_policy_sha256 != policy_digest(current)? {
        return Err(PolicyError::Transition("previous policy digest"));
    }
    if next.issued_at < current.issued_at || next.issued_at < transition.issued_at {
        return Err(PolicyError::Transition("non-monotonic issue time"));
    }
    ensure_sorted_unique(&transition.removed_key_ids, "removed key ids")?;
    let current_ids: BTreeSet<_> = current.keys.iter().map(|key| key.key_id.clone()).collect();
    let next_ids: BTreeSet<_> = next.keys.iter().map(|key| key.key_id.clone()).collect();
    let removed: Vec<_> = current_ids.difference(&next_ids).cloned().collect();
    let added: Vec<_> = next_ids.difference(&current_ids).cloned().collect();
    if removed.is_empty() || added.is_empty() || transition.removed_key_ids != removed {
        return Err(PolicyError::Transition("key-set difference"));
    }
    match transition.kind {
        TransitionKind::RoutineRotation => {
            if transition.reason != TransitionReason::ScheduledRotation {
                return Err(PolicyError::Transition("rotation reason"));
            }
        }
        TransitionKind::EmergencyRevocation => {
            if transition.reason != TransitionReason::SuspectedCompromise {
                return Err(PolicyError::Transition("revocation reason"));
            }
        }
        TransitionKind::Recovery => {
            if !matches!(
                transition.reason,
                TransitionReason::CustodyLoss | TransitionReason::DisasterRecovery
            ) {
                return Err(PolicyError::Transition("recovery reason"));
            }
            let old_root: BTreeSet<_> = current.roles.root.key_ids.iter().collect();
            let new_root: BTreeSet<_> = next.roles.root.key_ids.iter().collect();
            if old_root == new_root {
                return Err(PolicyError::Transition("recovery must replace root keys"));
            }
        }
    }
    Ok(())
}

fn validate_presented_identity(claim: &PresentedIdentity) -> Result<(), PolicyError> {
    match claim {
        PresentedIdentity::Key { key_id } => validate_lower_hex(key_id, 64, "claim key id"),
        PresentedIdentity::Oidc {
            issuer,
            subject,
            repository,
            workflow_ref,
        } => validate_oidc(issuer, subject, repository, workflow_ref),
    }
}

fn validate_oidc(
    issuer: &str,
    subject: &str,
    repository: &str,
    workflow_ref: &str,
) -> Result<(), PolicyError> {
    validate_visible_ascii(issuer, 256, "OIDC issuer")?;
    validate_visible_ascii(subject, 512, "OIDC subject")?;
    validate_visible_ascii(repository, 256, "OIDC repository")?;
    validate_visible_ascii(workflow_ref, 512, "OIDC workflow ref")?;
    if !issuer.starts_with("https://")
        || issuer.contains('*')
        || subject.contains('*')
        || repository.contains('*')
        || workflow_ref.contains('*')
    {
        return Err(PolicyError::Invalid("OIDC exact claim"));
    }
    Ok(())
}

fn validate_time(
    issued_at: u64,
    expires_at: u64,
    now: u64,
    max_lifetime: u64,
) -> Result<(), PolicyError> {
    validate_lifetime(issued_at, expires_at, max_lifetime)?;
    if now < issued_at {
        return Err(PolicyError::NotYetValid);
    }
    if now >= expires_at {
        return Err(PolicyError::Expired);
    }
    Ok(())
}

fn validate_lifetime(
    issued_at: u64,
    expires_at: u64,
    max_lifetime: u64,
) -> Result<(), PolicyError> {
    if issued_at == 0
        || expires_at <= issued_at
        || expires_at.saturating_sub(issued_at) > max_lifetime
    {
        return Err(PolicyError::Invalid("metadata lifetime"));
    }
    Ok(())
}

fn verify_role_threshold(
    policy: &PublisherPolicy,
    role: RoleKind,
    signatures: &[MetadataSignature],
    payload: &[u8],
) -> Result<(), PolicyError> {
    let role_policy = policy.roles.get(role);
    let keys: BTreeMap<_, _> = policy.keys.iter().map(|key| (&key.key_id, key)).collect();
    let mut verified = 0_u16;
    for key_id in &role_policy.key_ids {
        let Some(signature) = signatures
            .iter()
            .find(|signature| &signature.key_id == key_id)
        else {
            continue;
        };
        let key = keys
            .get(key_id)
            .ok_or(PolicyError::Invalid("threshold key is unknown"))?;
        let public_key = decode_hex::<32>(&key.public_key, "public key")?;
        let verifying_key = VerifyingKey::from_bytes(&public_key)
            .map_err(|_| PolicyError::Invalid("Ed25519 public key"))?;
        let signature_bytes = decode_hex::<64>(&signature.signature, "signature")?;
        let signature_value = Signature::from_bytes(&signature_bytes);
        verifying_key
            .verify_strict(payload, &signature_value)
            .map_err(|_| PolicyError::InvalidSignature(key_id.clone()))?;
        verified = verified.saturating_add(1);
    }
    if verified < role_policy.threshold {
        return Err(PolicyError::Threshold(role));
    }
    Ok(())
}

fn validate_signatures(signatures: &[MetadataSignature]) -> Result<(), PolicyError> {
    if signatures.is_empty() || signatures.len() > MAX_SIGNATURES {
        return Err(PolicyError::Invalid("signature count"));
    }
    ensure_sorted_unique(signatures, "signatures")?;
    for signature in signatures {
        validate_lower_hex(&signature.key_id, 64, "signature key id")?;
        validate_lower_hex(&signature.signature, 128, "signature")?;
    }
    Ok(())
}

fn ensure_signature_keys_allowed<'a>(
    signatures: &[MetadataSignature],
    allowed: impl IntoIterator<Item = &'a String>,
) -> Result<(), PolicyError> {
    let allowed: BTreeSet<_> = allowed.into_iter().collect();
    if signatures
        .iter()
        .any(|signature| !allowed.contains(&signature.key_id))
    {
        return Err(PolicyError::Invalid(
            "signature key is not authorized for this record",
        ));
    }
    Ok(())
}

fn policy_signing_bytes(policy: &PublisherPolicy) -> Result<Vec<u8>, PolicyError> {
    let canonical = canonical_policy_bytes(policy)?;
    let mut payload = Vec::with_capacity(POLICY_SIGNATURE_DOMAIN.len() + canonical.len());
    payload.extend_from_slice(POLICY_SIGNATURE_DOMAIN);
    payload.extend_from_slice(&canonical);
    Ok(payload)
}

fn update_signing_bytes(
    transition: &PolicyTransition,
    next_policy: &PublisherPolicy,
) -> Result<Vec<u8>, PolicyError> {
    let canonical = serde_json::to_vec(&UnsignedPolicyUpdate {
        transition,
        next_policy,
    })
    .map_err(|error| PolicyError::Json(error.to_string()))?;
    let mut payload = Vec::with_capacity(UPDATE_SIGNATURE_DOMAIN.len() + canonical.len());
    payload.extend_from_slice(UPDATE_SIGNATURE_DOMAIN);
    payload.extend_from_slice(&canonical);
    Ok(payload)
}

fn sign_payload(payload: &[u8], signing_keys: &[SigningKey]) -> Vec<MetadataSignature> {
    let mut signatures: Vec<_> = signing_keys
        .iter()
        .map(|key| MetadataSignature {
            key_id: public_key_id(key.verifying_key().as_bytes()),
            signature: encode_hex(&key.sign(payload).to_bytes()),
        })
        .collect();
    signatures.sort();
    signatures
}

fn validate_name(value: &str, max: usize, field: &'static str) -> Result<(), PolicyError> {
    if value.is_empty()
        || value.len() > max
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        || !value.as_bytes()[0].is_ascii_alphanumeric()
        || !value.as_bytes()[value.len() - 1].is_ascii_alphanumeric()
    {
        return Err(PolicyError::Invalid(field));
    }
    Ok(())
}

fn validate_visible_ascii(value: &str, max: usize, field: &'static str) -> Result<(), PolicyError> {
    if value.is_empty()
        || value.len() > max
        || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
    {
        return Err(PolicyError::Invalid(field));
    }
    Ok(())
}

fn validate_lower_hex(value: &str, length: usize, field: &'static str) -> Result<(), PolicyError> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PolicyError::Invalid(field));
    }
    Ok(())
}

fn ensure_sorted_unique<T: Ord>(values: &[T], field: &'static str) -> Result<(), PolicyError> {
    if values.windows(2).any(|window| window[0] >= window[1]) {
        return Err(PolicyError::Invalid(field));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn decode_hex<const N: usize>(text: &str, field: &'static str) -> Result<[u8; N], PolicyError> {
    validate_lower_hex(text, N * 2, field)?;
    let mut output = [0_u8; N];
    for (index, output_byte) in output.iter_mut().enumerate() {
        let pair = &text.as_bytes()[index * 2..index * 2 + 2];
        *output_byte = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(output)
}

const fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0,
    }
}
