use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::{
    MetadataSignature, PolicyError, ProductionIdentity, PublicKeyRecord, PublisherPolicy, RoleKind,
    decode_hex, ensure_signature_keys_allowed, policy_digest, public_key_id, sha256_hex,
    sign_payload, validate_lower_hex, validate_name, validate_policy, validate_signatures,
    validate_time, verify_role_threshold,
};

pub const NAMESPACE_SCHEMA: &str = "sico.registry.namespace-event.v0";
pub const RELEASE_SCHEMA: &str = "sico.registry.release.v0";
pub const CHANNEL_SCHEMA: &str = "sico.registry.channel.v0";
pub const CHECKPOINT_SCHEMA: &str = "sico.registry.checkpoint.v0";
pub const MAX_REGISTRY_RECORD_BYTES: usize = 256 * 1024;

const NAMESPACE_DOMAIN: &[u8] = b"SICO-REGISTRY-NAMESPACE-EVENT-V0\0";
const RELEASE_DOMAIN: &[u8] = b"SICO-REGISTRY-RELEASE-V0\0";
const CHANNEL_DOMAIN: &[u8] = b"SICO-REGISTRY-CHANNEL-V0\0";
const CHECKPOINT_DOMAIN: &[u8] = b"SICO-REGISTRY-CHECKPOINT-V0\0";
const RECORD_LIFETIME: u64 = 31 * 24 * 60 * 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    Policy(PolicyError),
    Package(String),
    Io(String),
    Invalid(&'static str),
    InvalidValue(String),
    NonCanonical,
    DigestMismatch,
    LengthMismatch,
    NotFound,
    Conflict,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(error) => {
                write!(formatter, "registry policy verification failed: {error}")
            }
            Self::Package(error) => {
                write!(formatter, "registry package verification failed: {error}")
            }
            Self::Io(error) => write!(formatter, "registry I/O failed: {error}"),
            Self::Invalid(field) => write!(formatter, "registry record is invalid: {field}"),
            Self::InvalidValue(value) => write!(formatter, "registry value is invalid: {value}"),
            Self::NonCanonical => formatter.write_str("registry record is not canonical"),
            Self::DigestMismatch => formatter.write_str("registry content digest does not match"),
            Self::LengthMismatch => formatter.write_str("registry content length does not match"),
            Self::NotFound => formatter.write_str("registry content was not found"),
            Self::Conflict => formatter.write_str("registry immutable content conflicts"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<PolicyError> for RegistryError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryAuthority {
    pub registry_id: String,
    pub keys: Vec<PublicKeyRecord>,
    pub threshold: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublisherReference {
    pub identity: ProductionIdentity,
    pub policy_version: u64,
    pub policy_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NamespaceEventKind {
    Grant,
    Transfer,
    Tombstone,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamespaceEvent {
    pub schema: String,
    pub registry_id: String,
    pub namespace: String,
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub kind: NamespaceEventKind,
    pub previous_event_sha256: Option<String>,
    pub previous_owner: Option<PublisherReference>,
    pub next_owner: Option<PublisherReference>,
    pub reuse_after: Option<u64>,
    pub audit_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedNamespaceEvent {
    signed: NamespaceEvent,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseRecord {
    pub schema: String,
    pub identity: ProductionIdentity,
    pub publisher_policy_version: u64,
    pub publisher_policy_sha256: String,
    pub namespace_event_sha256: String,
    pub app_id: String,
    pub app_version: String,
    pub package_sha256: String,
    pub package_bytes: u64,
    pub package_format: u16,
    pub manifest_schema: String,
    pub language_semantics: String,
    pub component_world: String,
    pub wasi_contract: String,
    pub capability_contract: String,
    pub dependency_lock_sha256: String,
    pub minimum_host_contract: String,
    pub description: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedReleaseRecord {
    signed: ReleaseRecord,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelRecord {
    pub schema: String,
    pub identity: ProductionIdentity,
    pub channel: String,
    pub sequence: u64,
    pub release_record_sha256: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedChannelRecord {
    signed: ChannelRecord,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryCheckpoint {
    pub schema: String,
    pub registry_id: String,
    pub sequence: u64,
    pub previous_checkpoint_sha256: Option<String>,
    pub namespace_event_digests: Vec<String>,
    pub release_record_digests: Vec<String>,
    pub channel_record_digests: Vec<String>,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedCheckpoint {
    signed: RegistryCheckpoint,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadedRelease {
    pub record: ReleaseRecord,
    pub package_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct LocalRegistry {
    root: PathBuf,
}

impl LocalRegistry {
    /// Creates or opens a local untrusted transport root.
    ///
    /// # Errors
    ///
    /// Returns an error when the root cannot be created.
    pub fn open(root: &Path) -> Result<Self, RegistryError> {
        fs::create_dir_all(root).map_err(|error| io_error(&error))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Publishes immutable release metadata and package bytes after every
    /// namespace, publisher and package check succeeds.
    ///
    /// # Errors
    ///
    /// Rejects untrusted namespace events, release signatures, identity drift,
    /// malformed packages, digest/length mismatch and immutable conflicts.
    pub fn publish_release(
        &self,
        namespace_bytes: &[u8],
        release_bytes: &[u8],
        package_bytes: &[u8],
        authority: &RegistryAuthority,
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<String, RegistryError> {
        let namespace =
            verify_namespace_event_json(namespace_bytes, authority, None, None, Some(policy), now)?;
        if namespace.kind != NamespaceEventKind::Grant
            || namespace.next_owner.as_ref() != Some(&publisher_reference(policy)?)
        {
            return Err(RegistryError::Invalid("namespace owner"));
        }
        let release = verify_release_record_json(release_bytes, policy, now)?;
        if release.namespace_event_sha256 != sha256_hex(namespace_bytes)
            || release.identity != policy.identity
        {
            return Err(RegistryError::Invalid("release namespace binding"));
        }
        verify_package_against_release(package_bytes, &release)?;
        write_immutable(&self.blob_path(&release.package_sha256), package_bytes)?;
        let release_digest = sha256_hex(release_bytes);
        write_immutable(&self.release_path(&release_digest), release_bytes)?;
        write_immutable(
            &self.namespace_path(&namespace.namespace, namespace.sequence),
            namespace_bytes,
        )?;
        Ok(release_digest)
    }

    /// Publishes an immutable signed channel event with monotonic sequence.
    ///
    /// # Errors
    ///
    /// Rejects unsigned, stale, identity-drifted or missing release pointers.
    pub fn publish_channel(
        &self,
        bytes: &[u8],
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<(), RegistryError> {
        let channel = verify_channel_record_json(bytes, policy, now)?;
        if !self.release_path(&channel.release_record_sha256).is_file() {
            return Err(RegistryError::NotFound);
        }
        let previous = self.latest_channel(&channel.identity, &channel.channel, policy, now)?;
        let expected = previous.as_ref().map_or(Ok(1), |(record, _)| {
            record
                .sequence
                .checked_add(1)
                .ok_or(RegistryError::Invalid("channel sequence overflow"))
        })?;
        if channel.sequence != expected {
            return Err(RegistryError::Invalid("channel sequence"));
        }
        write_immutable(
            &self.channel_path(&channel.identity, &channel.channel, channel.sequence),
            bytes,
        )
    }

    /// Resolves a signed channel to one immutable release record digest.
    ///
    /// # Errors
    ///
    /// Rejects missing, stale or invalid channel metadata.
    pub fn discover(
        &self,
        identity: &ProductionIdentity,
        channel: &str,
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<String, RegistryError> {
        let (record, _) = self
            .latest_channel(identity, channel, policy, now)?
            .ok_or(RegistryError::NotFound)?;
        if &record.identity != identity {
            return Err(RegistryError::Invalid("discovery identity"));
        }
        Ok(record.release_record_sha256)
    }

    /// Downloads by immutable record digest and repeats release, namespace,
    /// byte-length, digest and strict `.sapp` verification locally.
    ///
    /// # Errors
    ///
    /// Rejects any missing or altered transport object.
    pub fn download(
        &self,
        release_digest: &str,
        namespace_bytes: &[u8],
        authority: &RegistryAuthority,
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<DownloadedRelease, RegistryError> {
        validate_lower_hex(release_digest, 64, "release digest")?;
        let release_bytes =
            fs::read(self.release_path(release_digest)).map_err(|_| RegistryError::NotFound)?;
        if sha256_hex(&release_bytes) != release_digest {
            return Err(RegistryError::DigestMismatch);
        }
        let namespace =
            verify_namespace_event_json(namespace_bytes, authority, None, None, Some(policy), now)?;
        let record = verify_release_record_json(&release_bytes, policy, now)?;
        if record.namespace_event_sha256 != sha256_hex(namespace_bytes)
            || namespace.next_owner.as_ref() != Some(&publisher_reference(policy)?)
        {
            return Err(RegistryError::Invalid("download namespace binding"));
        }
        let package_bytes = fs::read(self.blob_path(&record.package_sha256))
            .map_err(|_| RegistryError::NotFound)?;
        verify_package_against_release(&package_bytes, &record)?;
        Ok(DownloadedRelease {
            record,
            package_bytes,
        })
    }

    fn blob_path(&self, digest: &str) -> PathBuf {
        self.root.join("blobs/sha256").join(digest)
    }

    fn release_path(&self, digest: &str) -> PathBuf {
        self.root
            .join("records/releases")
            .join(format!("{digest}.json"))
    }

    fn namespace_path(&self, namespace: &str, sequence: u64) -> PathBuf {
        self.root
            .join("records/namespaces")
            .join(namespace)
            .join(format!("{sequence}.json"))
    }

    fn channel_dir(&self, identity: &ProductionIdentity, channel: &str) -> PathBuf {
        self.root
            .join("channels")
            .join(&identity.namespace)
            .join(&identity.package_name)
            .join(channel)
    }

    fn channel_path(&self, identity: &ProductionIdentity, channel: &str, sequence: u64) -> PathBuf {
        self.channel_dir(identity, channel)
            .join(format!("{sequence}.json"))
    }

    fn latest_channel(
        &self,
        identity: &ProductionIdentity,
        channel: &str,
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<Option<(ChannelRecord, Vec<u8>)>, RegistryError> {
        let directory = self.channel_dir(identity, channel);
        if !directory.is_dir() {
            return Ok(None);
        }
        let mut latest: Option<(ChannelRecord, Vec<u8>)> = None;
        for entry in fs::read_dir(directory).map_err(|error| io_error(&error))? {
            let entry = entry.map_err(|error| io_error(&error))?;
            let path = entry.path();
            let Some(sequence) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<u64>().ok())
            else {
                continue;
            };
            let Ok(bytes) = fs::read(path) else {
                continue;
            };
            let Ok(record) = verify_channel_record_json(&bytes, policy, now) else {
                continue;
            };
            if record.sequence != sequence
                || &record.identity != identity
                || record.channel != channel
            {
                continue;
            }
            if latest
                .as_ref()
                .is_none_or(|(current, _)| record.sequence > current.sequence)
            {
                latest = Some((record, bytes));
            }
        }
        Ok(latest)
    }
}

/// Builds a reference to an exact publisher policy.
///
/// # Errors
///
/// Returns an error if canonical policy serialization fails.
pub fn publisher_reference(policy: &PublisherPolicy) -> Result<PublisherReference, RegistryError> {
    Ok(PublisherReference {
        identity: policy.identity.clone(),
        policy_version: policy.version,
        policy_sha256: policy_digest(policy)?,
    })
}

/// Signs a namespace event with deterministic local fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_namespace_event_fixture(
    event: NamespaceEvent,
    keys: &[SigningKey],
) -> Result<Vec<u8>, RegistryError> {
    sign_envelope(event, keys, NAMESPACE_DOMAIN, |signed, signatures| {
        SignedNamespaceEvent { signed, signatures }
    })
}

/// Strictly verifies a namespace grant, transfer or tombstone.
///
/// # Errors
///
/// Rejects invalid canonical form, authority, chain, policy or signatures.
pub fn verify_namespace_event_json(
    bytes: &[u8],
    authority: &RegistryAuthority,
    previous_event: Option<(&NamespaceEvent, &str)>,
    previous_policy: Option<&PublisherPolicy>,
    next_policy: Option<&PublisherPolicy>,
    now: u64,
) -> Result<NamespaceEvent, RegistryError> {
    let envelope: SignedNamespaceEvent = parse_canonical(bytes)?;
    validate_authority(authority)?;
    validate_namespace_event(
        &envelope.signed,
        authority,
        previous_event,
        previous_policy,
        next_policy,
        now,
    )?;
    validate_signatures(&envelope.signatures)?;
    let payload = signing_bytes(NAMESPACE_DOMAIN, &envelope.signed)?;
    verify_authority_threshold(authority, &envelope.signatures, &payload)?;
    let mut allowed: Vec<&String> = authority.keys.iter().map(|key| &key.key_id).collect();
    match envelope.signed.kind {
        NamespaceEventKind::Grant => {
            let policy = next_policy.ok_or(RegistryError::Invalid("grant owner policy"))?;
            verify_role_threshold(policy, RoleKind::Root, &envelope.signatures, &payload)?;
            allowed.extend(&policy.roles.root.key_ids);
        }
        NamespaceEventKind::Transfer => {
            let old = previous_policy.ok_or(RegistryError::Invalid("transfer previous policy"))?;
            let new = next_policy.ok_or(RegistryError::Invalid("transfer next policy"))?;
            verify_role_threshold(old, RoleKind::Root, &envelope.signatures, &payload)?;
            verify_role_threshold(new, RoleKind::Root, &envelope.signatures, &payload)?;
            allowed.extend(&old.roles.root.key_ids);
            allowed.extend(&new.roles.root.key_ids);
        }
        NamespaceEventKind::Tombstone => {
            let old = previous_policy.ok_or(RegistryError::Invalid("tombstone owner policy"))?;
            verify_role_threshold(old, RoleKind::Root, &envelope.signatures, &payload)?;
            allowed.extend(&old.roles.root.key_ids);
        }
    }
    ensure_signature_keys_allowed(&envelope.signatures, allowed)?;
    Ok(envelope.signed)
}

/// Signs a release record with local fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_release_record_fixture(
    record: ReleaseRecord,
    keys: &[SigningKey],
) -> Result<Vec<u8>, RegistryError> {
    sign_envelope(record, keys, RELEASE_DOMAIN, |signed, signatures| {
        SignedReleaseRecord { signed, signatures }
    })
}

/// Verifies a canonical immutable release record against a publisher policy.
///
/// # Errors
///
/// Rejects invalid policy, metadata, time, identity or release signatures.
pub fn verify_release_record_json(
    bytes: &[u8],
    policy: &PublisherPolicy,
    now: u64,
) -> Result<ReleaseRecord, RegistryError> {
    validate_policy(policy, now, &std::collections::BTreeSet::new())?;
    let envelope: SignedReleaseRecord = parse_canonical(bytes)?;
    validate_release(&envelope.signed, policy, now)?;
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(&envelope.signatures, policy.roles.release.key_ids.iter())?;
    let payload = signing_bytes(RELEASE_DOMAIN, &envelope.signed)?;
    verify_role_threshold(policy, RoleKind::Release, &envelope.signatures, &payload)?;
    Ok(envelope.signed)
}

/// Signs a mutable channel record with local fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_channel_record_fixture(
    record: ChannelRecord,
    keys: &[SigningKey],
) -> Result<Vec<u8>, RegistryError> {
    sign_envelope(record, keys, CHANNEL_DOMAIN, |signed, signatures| {
        SignedChannelRecord { signed, signatures }
    })
}

/// Verifies a signed channel pointer.
///
/// # Errors
///
/// Rejects invalid policy, metadata, time, identity or release signatures.
pub fn verify_channel_record_json(
    bytes: &[u8],
    policy: &PublisherPolicy,
    now: u64,
) -> Result<ChannelRecord, RegistryError> {
    validate_policy(policy, now, &std::collections::BTreeSet::new())?;
    let envelope: SignedChannelRecord = parse_canonical(bytes)?;
    let record = &envelope.signed;
    if record.schema != CHANNEL_SCHEMA || record.identity != policy.identity || record.sequence == 0
    {
        return Err(RegistryError::Invalid("channel identity/schema/sequence"));
    }
    validate_name(&record.channel, 32, "channel")?;
    validate_lower_hex(&record.release_record_sha256, 64, "release record digest")?;
    validate_time(record.issued_at, record.expires_at, now, RECORD_LIFETIME)?;
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(&envelope.signatures, policy.roles.release.key_ids.iter())?;
    let payload = signing_bytes(CHANNEL_DOMAIN, record)?;
    verify_role_threshold(policy, RoleKind::Release, &envelope.signatures, &payload)?;
    Ok(envelope.signed)
}

/// Signs a checkpoint with local registry-authority fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_checkpoint_fixture(
    checkpoint: RegistryCheckpoint,
    keys: &[SigningKey],
) -> Result<Vec<u8>, RegistryError> {
    sign_envelope(checkpoint, keys, CHECKPOINT_DOMAIN, |signed, signatures| {
        SignedCheckpoint { signed, signatures }
    })
}

/// Verifies a chained signed checkpoint and its sorted inclusion sets.
///
/// # Errors
///
/// Rejects invalid authority, signature, time, chain or append-only history.
pub fn verify_checkpoint_json(
    bytes: &[u8],
    authority: &RegistryAuthority,
    previous: Option<&[u8]>,
    now: u64,
) -> Result<RegistryCheckpoint, RegistryError> {
    let checkpoint = verify_checkpoint_unlinked(bytes, authority, Some(now))?;
    match previous {
        Some(previous_bytes) => {
            let previous_checkpoint = verify_checkpoint_unlinked(previous_bytes, authority, None)?;
            let expected_sequence = previous_checkpoint
                .sequence
                .checked_add(1)
                .ok_or(RegistryError::Invalid("checkpoint sequence overflow"))?;
            if checkpoint.sequence != expected_sequence
                || checkpoint.previous_checkpoint_sha256.as_deref()
                    != Some(&sha256_hex(previous_bytes))
                || checkpoint.issued_at < previous_checkpoint.issued_at
                || !sorted_subset(
                    &previous_checkpoint.namespace_event_digests,
                    &checkpoint.namespace_event_digests,
                )
                || !sorted_subset(
                    &previous_checkpoint.release_record_digests,
                    &checkpoint.release_record_digests,
                )
                || !sorted_subset(
                    &previous_checkpoint.channel_record_digests,
                    &checkpoint.channel_record_digests,
                )
            {
                return Err(RegistryError::Invalid("checkpoint chain"));
            }
        }
        None => {
            if checkpoint.sequence != 1 || checkpoint.previous_checkpoint_sha256.is_some() {
                return Err(RegistryError::Invalid("checkpoint genesis"));
            }
        }
    }
    Ok(checkpoint)
}

pub(crate) fn verify_checkpoint_unlinked(
    bytes: &[u8],
    authority: &RegistryAuthority,
    verification_time: Option<u64>,
) -> Result<RegistryCheckpoint, RegistryError> {
    let envelope: SignedCheckpoint = parse_canonical(bytes)?;
    let checkpoint = &envelope.signed;
    validate_authority(authority)?;
    if checkpoint.schema != CHECKPOINT_SCHEMA
        || checkpoint.registry_id != authority.registry_id
        || checkpoint.sequence == 0
    {
        return Err(RegistryError::Invalid(
            "checkpoint identity/schema/sequence",
        ));
    }
    validate_time(
        checkpoint.issued_at,
        checkpoint.expires_at,
        verification_time.unwrap_or(checkpoint.issued_at),
        RECORD_LIFETIME,
    )?;
    validate_digest_set(&checkpoint.namespace_event_digests)?;
    validate_digest_set(&checkpoint.release_record_digests)?;
    validate_digest_set(&checkpoint.channel_record_digests)?;
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(
        &envelope.signatures,
        authority.keys.iter().map(|key| &key.key_id),
    )?;
    let payload = signing_bytes(CHECKPOINT_DOMAIN, checkpoint)?;
    verify_authority_threshold(authority, &envelope.signatures, &payload)?;
    Ok(envelope.signed)
}

fn sorted_subset(older: &[String], newer: &[String]) -> bool {
    let mut newer = newer.iter();
    let mut candidate = newer.next();
    for old in older {
        loop {
            match candidate {
                Some(value) if value < old => candidate = newer.next(),
                Some(value) if value == old => {
                    candidate = newer.next();
                    break;
                }
                _ => return false,
            }
        }
    }
    true
}

pub(crate) fn validate_authority(authority: &RegistryAuthority) -> Result<(), RegistryError> {
    validate_name(&authority.registry_id, 64, "registry id")?;
    if authority.keys.is_empty()
        || authority.keys.len() > 16
        || authority.threshold < 2
        || usize::from(authority.threshold) > authority.keys.len()
    {
        return Err(RegistryError::Invalid("registry authority threshold"));
    }
    let mut previous: Option<&str> = None;
    for key in &authority.keys {
        if previous.is_some_and(|value| value >= key.key_id.as_str()) {
            return Err(RegistryError::Invalid("registry authority keys"));
        }
        previous = Some(&key.key_id);
        if key.scheme != "ed25519" {
            return Err(RegistryError::Invalid("registry authority scheme"));
        }
        let bytes = decode_hex::<32>(&key.public_key, "registry public key")?;
        let verifying_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| RegistryError::Invalid("registry public key"))?;
        if verifying_key.is_weak() {
            return Err(RegistryError::Invalid("weak registry public key"));
        }
        if public_key_id(&bytes) != key.key_id {
            return Err(RegistryError::Invalid("registry authority key id"));
        }
    }
    Ok(())
}

fn validate_namespace_event(
    event: &NamespaceEvent,
    authority: &RegistryAuthority,
    previous: Option<(&NamespaceEvent, &str)>,
    previous_policy: Option<&PublisherPolicy>,
    next_policy: Option<&PublisherPolicy>,
    now: u64,
) -> Result<(), RegistryError> {
    if event.schema != NAMESPACE_SCHEMA
        || event.registry_id != authority.registry_id
        || event.sequence == 0
    {
        return Err(RegistryError::Invalid(
            "namespace event identity/schema/sequence",
        ));
    }
    validate_name(&event.namespace, 128, "namespace")?;
    validate_time(event.issued_at, event.expires_at, now, RECORD_LIFETIME)?;
    validate_lower_hex(&event.audit_ref, 64, "namespace audit ref")?;
    if let Some(digest) = &event.previous_event_sha256 {
        validate_lower_hex(digest, 64, "previous namespace event digest")?;
    }
    match previous {
        Some((previous_event, previous_digest)) => {
            validate_lower_hex(previous_digest, 64, "previous namespace event digest")?;
            let expected_sequence = previous_event
                .sequence
                .checked_add(1)
                .ok_or(RegistryError::Invalid("namespace sequence overflow"))?;
            if event.sequence != expected_sequence
                || event.registry_id != previous_event.registry_id
                || event.namespace != previous_event.namespace
                || event.previous_event_sha256.as_deref() != Some(previous_digest)
                || event.issued_at < previous_event.issued_at
            {
                return Err(RegistryError::Invalid("namespace event sequence"));
            }
        }
        None if event.sequence != 1 || event.previous_event_sha256.is_some() => {
            return Err(RegistryError::Invalid("namespace genesis"));
        }
        None => {}
    }
    match event.kind {
        NamespaceEventKind::Grant => {
            let next = next_policy.ok_or(RegistryError::Invalid("grant policy"))?;
            validate_publisher_for_namespace(next, authority, &event.namespace, event.issued_at)?;
            if event.previous_owner.is_some()
                || event.next_owner.as_ref() != Some(&publisher_reference(next)?)
                || event.reuse_after.is_some()
            {
                return Err(RegistryError::Invalid("namespace grant shape"));
            }
            if let Some((previous_event, _)) = previous
                && (previous_event.kind != NamespaceEventKind::Tombstone
                    || previous_event.next_owner.is_some()
                    || previous_event
                        .reuse_after
                        .is_none_or(|reuse| event.issued_at < reuse))
            {
                return Err(RegistryError::Invalid("namespace reuse policy"));
            }
        }
        NamespaceEventKind::Transfer => {
            let old = previous_policy.ok_or(RegistryError::Invalid("old owner policy"))?;
            let new = next_policy.ok_or(RegistryError::Invalid("new owner policy"))?;
            validate_publisher_for_namespace(old, authority, &event.namespace, event.issued_at)?;
            validate_publisher_for_namespace(new, authority, &event.namespace, event.issued_at)?;
            let old_reference = publisher_reference(old)?;
            if event.previous_owner.as_ref() != Some(&old_reference)
                || event.next_owner.as_ref() != Some(&publisher_reference(new)?)
                || event.reuse_after.is_some()
                || old.identity == new.identity
                || previous
                    .is_none_or(|(prior, _)| prior.next_owner.as_ref() != Some(&old_reference))
            {
                return Err(RegistryError::Invalid("namespace transfer shape"));
            }
        }
        NamespaceEventKind::Tombstone => {
            let old = previous_policy.ok_or(RegistryError::Invalid("tombstone policy"))?;
            validate_publisher_for_namespace(old, authority, &event.namespace, event.issued_at)?;
            let old_reference = publisher_reference(old)?;
            if event.previous_owner.as_ref() != Some(&old_reference)
                || event.next_owner.is_some()
                || event
                    .reuse_after
                    .is_none_or(|reuse| reuse <= event.issued_at)
                || previous
                    .is_none_or(|(prior, _)| prior.next_owner.as_ref() != Some(&old_reference))
            {
                return Err(RegistryError::Invalid("namespace tombstone shape"));
            }
        }
    }
    Ok(())
}

fn validate_publisher_for_namespace(
    policy: &PublisherPolicy,
    authority: &RegistryAuthority,
    namespace: &str,
    at: u64,
) -> Result<(), RegistryError> {
    validate_policy(policy, at, &std::collections::BTreeSet::new())?;
    if policy.identity.registry_id != authority.registry_id
        || policy.identity.namespace != namespace
    {
        return Err(RegistryError::Invalid("publisher namespace identity"));
    }
    if authority.keys.iter().any(|authority_key| {
        policy
            .keys
            .iter()
            .any(|publisher_key| publisher_key.key_id == authority_key.key_id)
    }) {
        return Err(RegistryError::Invalid("registry/publisher key separation"));
    }
    Ok(())
}

fn validate_release(
    record: &ReleaseRecord,
    policy: &PublisherPolicy,
    now: u64,
) -> Result<(), RegistryError> {
    if record.schema != RELEASE_SCHEMA
        || record.identity != policy.identity
        || record.publisher_policy_version != policy.version
        || record.publisher_policy_sha256 != policy_digest(policy)?
    {
        return Err(RegistryError::Invalid("release publisher binding"));
    }
    validate_time(record.issued_at, record.expires_at, now, RECORD_LIFETIME)?;
    validate_lower_hex(&record.namespace_event_sha256, 64, "namespace event digest")?;
    validate_lower_hex(&record.package_sha256, 64, "package digest")?;
    validate_lower_hex(&record.dependency_lock_sha256, 64, "dependency lock digest")?;
    if record.package_bytes == 0 || record.package_bytes > sico_package::MAX_PACKAGE_BYTES as u64 {
        return Err(RegistryError::Invalid("release package bytes"));
    }
    if record.package_format != sico_package::FORMAT_VERSION
        || record.manifest_schema != "sico.sapp.manifest.v0"
    {
        return Err(RegistryError::Invalid("release package contract"));
    }
    validate_semver(&record.app_version)?;
    validate_inert_text(&record.app_id, 128, "app id")?;
    validate_inert_text(&record.description, 4_096, "description")?;
    for (value, field) in [
        (&record.language_semantics, "language semantics"),
        (&record.component_world, "component world"),
        (&record.wasi_contract, "WASI contract"),
        (&record.capability_contract, "capability contract"),
        (&record.minimum_host_contract, "minimum host contract"),
    ] {
        validate_name(value, 64, field)?;
    }
    Ok(())
}

fn verify_package_against_release(
    package_bytes: &[u8],
    record: &ReleaseRecord,
) -> Result<(), RegistryError> {
    if package_bytes.len() as u64 != record.package_bytes {
        return Err(RegistryError::LengthMismatch);
    }
    if sha256_hex(package_bytes) != record.package_sha256 {
        return Err(RegistryError::DigestMismatch);
    }
    let package = sico_package::verify(package_bytes)
        .map_err(|error| RegistryError::Package(error.to_string()))?;
    if package.manifest.app.id != record.app_id
        || package.manifest.app.version != record.app_version
    {
        return Err(RegistryError::Invalid("package manifest release identity"));
    }
    Ok(())
}

fn validate_semver(value: &str) -> Result<(), RegistryError> {
    if value.is_empty() || value.len() > 64 || value.starts_with('v') {
        return Err(RegistryError::Invalid("canonical SemVer"));
    }
    let mut build_split = value.split('+');
    let version = build_split.next().unwrap_or_default();
    let build = build_split.next();
    if build_split.next().is_some()
        || build.is_some_and(|identifiers| !valid_identifiers(identifiers, false))
    {
        return Err(RegistryError::Invalid("canonical SemVer"));
    }
    let (core, prerelease) = version
        .split_once('-')
        .map_or((version, None), |(core, identifiers)| {
            (core, Some(identifiers))
        });
    if prerelease.is_some_and(|identifiers| !valid_identifiers(identifiers, true)) {
        return Err(RegistryError::Invalid("canonical SemVer"));
    }
    let parts: Vec<_> = core.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(RegistryError::Invalid("canonical SemVer"));
    }
    Ok(())
}

fn valid_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!reject_numeric_leading_zero
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || part.len() == 1
                    || !part.starts_with('0'))
        })
}

fn validate_inert_text(value: &str, max: usize, field: &'static str) -> Result<(), RegistryError> {
    if value.is_empty()
        || value.len() > max
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Err(RegistryError::Invalid(field));
    }
    Ok(())
}

fn validate_digest_set(values: &[String]) -> Result<(), RegistryError> {
    if values.len() > 4_096 || values.windows(2).any(|window| window[0] >= window[1]) {
        return Err(RegistryError::Invalid("checkpoint digest set"));
    }
    for value in values {
        validate_lower_hex(value, 64, "checkpoint digest")?;
    }
    Ok(())
}

pub(crate) fn verify_authority_threshold(
    authority: &RegistryAuthority,
    signatures: &[MetadataSignature],
    payload: &[u8],
) -> Result<(), RegistryError> {
    let mut verified = 0_u16;
    for record in &authority.keys {
        let Some(signature) = signatures.iter().find(|item| item.key_id == record.key_id) else {
            continue;
        };
        let public = decode_hex::<32>(&record.public_key, "registry public key")?;
        let verifying_key = VerifyingKey::from_bytes(&public)
            .map_err(|_| RegistryError::Invalid("registry public key"))?;
        let signature_bytes = decode_hex::<64>(&signature.signature, "registry signature")?;
        verifying_key
            .verify_strict(payload, &Signature::from_bytes(&signature_bytes))
            .map_err(|_| RegistryError::Invalid("registry signature"))?;
        verified = verified.saturating_add(1);
    }
    if verified < authority.threshold {
        return Err(RegistryError::Invalid("registry authority threshold"));
    }
    Ok(())
}

fn sign_envelope<T, E>(
    signed: T,
    keys: &[SigningKey],
    domain: &[u8],
    make: impl FnOnce(T, Vec<MetadataSignature>) -> E,
) -> Result<Vec<u8>, RegistryError>
where
    T: Serialize,
    E: Serialize,
{
    let payload = signing_bytes(domain, &signed)?;
    let signatures = sign_payload(&payload, keys);
    serde_json::to_vec(&make(signed, signatures))
        .map_err(|error| RegistryError::InvalidValue(error.to_string()))
}

fn signing_bytes<T: Serialize>(domain: &[u8], signed: &T) -> Result<Vec<u8>, RegistryError> {
    let canonical = serde_json::to_vec(signed)
        .map_err(|error| RegistryError::InvalidValue(error.to_string()))?;
    let mut payload = Vec::with_capacity(domain.len() + canonical.len());
    payload.extend_from_slice(domain);
    payload.extend_from_slice(&canonical);
    Ok(payload)
}

fn parse_canonical<T>(bytes: &[u8]) -> Result<T, RegistryError>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    if bytes.len() > MAX_REGISTRY_RECORD_BYTES {
        return Err(RegistryError::Invalid("record size"));
    }
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| RegistryError::InvalidValue(error.to_string()))?;
    if serde_json::to_vec(&value).map_err(|error| RegistryError::InvalidValue(error.to_string()))?
        != bytes
    {
        return Err(RegistryError::NonCanonical);
    }
    Ok(value)
}

fn write_immutable(path: &Path, bytes: &[u8]) -> Result<(), RegistryError> {
    if path.is_file() {
        let existing = fs::read(path).map_err(|error| io_error(&error))?;
        return if existing == bytes {
            Ok(())
        } else {
            Err(RegistryError::Conflict)
        };
    }
    write_new(path, bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), RegistryError> {
    let parent = path
        .parent()
        .ok_or(RegistryError::Invalid("registry path"))?;
    fs::create_dir_all(parent).map_err(|error| io_error(&error))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error(&error))?;
    file.write_all(bytes).map_err(|error| io_error(&error))?;
    file.sync_all().map_err(|error| io_error(&error))
}

fn io_error(error: &std::io::Error) -> RegistryError {
    RegistryError::Io(error.to_string())
}
