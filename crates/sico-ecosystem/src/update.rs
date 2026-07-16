use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

use super::{
    ChannelRecord, MetadataSignature, NamespaceEvent, ProductionIdentity, PublisherPolicy,
    RegistryAuthority, RegistryCheckpoint, RegistryError, ReleaseRecord, RoleKind,
    ensure_signature_keys_allowed, policy_digest, sha256_hex, sign_payload, validate_identity,
    validate_lower_hex, validate_name, validate_policy, validate_signatures, validate_time,
    verify_authority_threshold, verify_channel_record_json, verify_checkpoint_json,
    verify_checkpoint_unlinked, verify_namespace_event_json, verify_release_record_json,
    verify_role_threshold,
};

pub const UPDATE_SNAPSHOT_SCHEMA: &str = "sico.update.snapshot.v0";
pub const ADVISORY_SCHEMA: &str = "sico.update.advisory.v0";
pub const RECOVERY_AUTHORIZATION_SCHEMA: &str = "sico.update.recovery-authorization.v0";
pub const TRUSTED_UPDATE_STATE_SCHEMA: &str = "sico.update.trusted-state.v0";
pub const MAX_UPDATE_RECORD_BYTES: usize = 256 * 1024;

const SNAPSHOT_DOMAIN: &[u8] = b"SICO-UPDATE-SNAPSHOT-V0\0";
const ADVISORY_DOMAIN: &[u8] = b"SICO-UPDATE-ADVISORY-V0\0";
const RECOVERY_DOMAIN: &[u8] = b"SICO-UPDATE-RECOVERY-AUTHORIZATION-V0\0";
const STATE_DOMAIN: &[u8] = b"SICO-UPDATE-TRUSTED-STATE-V0\0";
const CAPABILITY_DOMAIN: &[u8] = b"SICO-UPDATE-CAPABILITY-FINGERPRINT-V0\0";
const ONLINE_LIFETIME: u64 = 31 * 24 * 60 * 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateError {
    Registry(RegistryError),
    Policy(super::PolicyError),
    Package(String),
    Io(String),
    Invalid(&'static str),
    InvalidValue(String),
    NonCanonical,
    DigestMismatch,
    LengthMismatch,
    NotFound,
    Conflict,
    Rollback,
    Freeze,
    RecoveryAuthorization,
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registry(error) => {
                write!(formatter, "update registry verification failed: {error}")
            }
            Self::Policy(error) => write!(formatter, "update publisher policy failed: {error}"),
            Self::Package(error) => {
                write!(formatter, "update package verification failed: {error}")
            }
            Self::Io(error) => write!(formatter, "update I/O failed: {error}"),
            Self::Invalid(field) => write!(formatter, "update record is invalid: {field}"),
            Self::InvalidValue(value) => write!(formatter, "update value is invalid: {value}"),
            Self::NonCanonical => formatter.write_str("update record is not canonical"),
            Self::DigestMismatch => formatter.write_str("update digest does not match"),
            Self::LengthMismatch => formatter.write_str("update byte length does not match"),
            Self::NotFound => formatter.write_str("update artifact was not found"),
            Self::Conflict => formatter.write_str("update immutable artifact conflicts"),
            Self::Rollback => formatter.write_str("update would roll trusted state back"),
            Self::Freeze => formatter.write_str("update metadata is stale or time moved backwards"),
            Self::RecoveryAuthorization => {
                formatter.write_str("rollback lacks exact recovery authorization")
            }
        }
    }
}

impl std::error::Error for UpdateError {}

impl From<RegistryError> for UpdateError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<super::PolicyError> for UpdateError {
    fn from(error: super::PolicyError) -> Self {
        Self::Policy(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateSnapshot {
    pub schema: String,
    pub registry_id: String,
    pub identity: ProductionIdentity,
    pub snapshot_sequence: u64,
    pub checkpoint_sequence: u64,
    pub channel_sequence: u64,
    pub release_sequence: u64,
    pub publisher_policy_version: u64,
    pub publisher_policy_sha256: String,
    pub namespace_event_sha256: String,
    pub checkpoint_sha256: String,
    pub channel_record_sha256: String,
    pub release_record_sha256: String,
    pub package_sha256: String,
    pub package_bytes: u64,
    pub capability_fingerprint: String,
    pub advisory_sequence: u64,
    pub advisory_record_sha256: Option<String>,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedUpdateSnapshot {
    signed: UpdateSnapshot,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryRecord {
    pub schema: String,
    pub identity: ProductionIdentity,
    pub sequence: u64,
    pub previous_advisory_sha256: Option<String>,
    pub advisory_id: String,
    pub affected_semver: String,
    pub exact_release_record_digests: Vec<String>,
    pub exact_package_digests: Vec<String>,
    pub severity: String,
    pub description: String,
    pub audit_ref: String,
    pub issued_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedAdvisoryRecord {
    signed: AdvisoryRecord,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryAuthorization {
    pub schema: String,
    pub identity: ProductionIdentity,
    pub activation_sequence: u64,
    pub from_package_sha256: String,
    pub to_release_record_sha256: String,
    pub to_package_sha256: String,
    pub to_capability_fingerprint: String,
    pub reason: String,
    pub audit_ref: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedRecoveryAuthorization {
    signed: RecoveryAuthorization,
    signatures: Vec<MetadataSignature>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalRecoveryPolicy {
    pub identity: ProductionIdentity,
    pub from_package_sha256: String,
    pub to_package_sha256: String,
    pub confirmation_sha256: String,
    pub expires_at: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum RecoveryDecision<'a> {
    Signed(&'a [u8]),
    ExplicitLocal(&'a LocalRecoveryPolicy),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedUpdateState {
    pub schema: String,
    pub identity: ProductionIdentity,
    pub activation_sequence: u64,
    pub publisher_policy_version: u64,
    pub publisher_policy_sha256: String,
    pub checkpoint_sequence: u64,
    pub checkpoint_sha256: String,
    pub snapshot_sequence: u64,
    pub channel_sequence: u64,
    pub release_sequence: u64,
    pub release_record_sha256: String,
    pub package_sha256: String,
    pub package_bytes: u64,
    pub app_id: String,
    pub app_version: String,
    pub capability_fingerprint: String,
    pub advisory_sequence: u64,
    pub advisory_record_sha256: Option<String>,
    pub last_trusted_time: u64,
    pub recovery_activation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedStateEnvelope {
    state: TrustedUpdateState,
    checksum_sha256: String,
}

#[derive(Clone, Copy, Debug)]
pub struct UpdateCandidate<'a> {
    pub snapshot_bytes: &'a [u8],
    pub checkpoint_bytes: &'a [u8],
    pub namespace_bytes: &'a [u8],
    pub channel_bytes: &'a [u8],
    pub release_bytes: &'a [u8],
    pub package_bytes: &'a [u8],
    pub advisory_bytes: Option<&'a [u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateOutcome {
    pub state: TrustedUpdateState,
    pub permissions_must_be_reconfirmed: bool,
}

#[derive(Clone, Debug)]
pub struct UpdateClient {
    root: PathBuf,
}

impl UpdateClient {
    /// Opens a local update-state root without trusting transport content.
    ///
    /// # Errors
    ///
    /// Returns an error when the root cannot be created.
    pub fn open(root: &Path) -> Result<Self, UpdateError> {
        fs::create_dir_all(root).map_err(|error| io_error(&error))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Loads and verifies the complete contiguous trust history.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-contiguous, regressing or corrupt local state.
    pub fn current_state(&self) -> Result<Option<TrustedUpdateState>, UpdateError> {
        Ok(self.load_states()?.last().cloned())
    }

    /// Stages, verifies and logically atomically activates one signed update.
    ///
    /// # Errors
    ///
    /// Rejects stale, mixed, unsigned, corrupt, incompatible or regressing input.
    pub fn apply_update(
        &self,
        candidate: UpdateCandidate<'_>,
        authority: &RegistryAuthority,
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<UpdateOutcome, UpdateError> {
        let current = self.current_state()?;
        if current
            .as_ref()
            .is_some_and(|state| now < state.last_trusted_time)
        {
            return Err(UpdateError::Freeze);
        }
        let snapshot =
            verify_update_snapshot_json(candidate.snapshot_bytes, authority, policy, now)?;
        let checkpoint = self.verify_candidate_checkpoint(
            candidate.checkpoint_bytes,
            authority,
            current.as_ref(),
            now,
        )?;
        let namespace = verify_namespace_event_json(
            candidate.namespace_bytes,
            authority,
            None,
            None,
            Some(policy),
            now,
        )?;
        let channel = verify_channel_record_json(candidate.channel_bytes, policy, now)?;
        let release = verify_release_record_json(candidate.release_bytes, policy, now)?;
        let advisory = self.verify_candidate_advisory(
            &snapshot,
            candidate.advisory_bytes,
            policy,
            current.as_ref(),
            now,
        )?;
        validate_transaction(
            &snapshot,
            &checkpoint,
            &namespace,
            &channel,
            &release,
            candidate,
            policy,
        )?;
        validate_monotonic_update(&snapshot, current.as_ref())?;

        let staged_path = self.stage_path(&snapshot);
        write_staging(&staged_path, candidate.package_bytes)?;
        let staged_bytes = fs::read(&staged_path).map_err(|error| io_error(&error))?;
        let (app_id, app_version, capability_fingerprint) =
            verify_package(&staged_bytes, &release, &snapshot.capability_fingerprint)?;
        if current.as_ref().is_some_and(|state| state.app_id != app_id) {
            return Err(UpdateError::Invalid("application id drift"));
        }
        let permissions_must_be_reconfirmed = current
            .as_ref()
            .is_none_or(|state| state.capability_fingerprint != capability_fingerprint);

        self.persist_candidate(&snapshot, candidate, &staged_bytes)?;

        let activation_sequence = current.as_ref().map_or(Ok(1), |state| {
            checked_next(state.activation_sequence, "activation sequence")
        })?;
        let trusted_state = TrustedUpdateState {
            schema: TRUSTED_UPDATE_STATE_SCHEMA.to_owned(),
            identity: snapshot.identity,
            activation_sequence,
            publisher_policy_version: snapshot.publisher_policy_version,
            publisher_policy_sha256: snapshot.publisher_policy_sha256,
            checkpoint_sequence: snapshot.checkpoint_sequence,
            checkpoint_sha256: snapshot.checkpoint_sha256,
            snapshot_sequence: snapshot.snapshot_sequence,
            channel_sequence: snapshot.channel_sequence,
            release_sequence: snapshot.release_sequence,
            release_record_sha256: snapshot.release_record_sha256,
            package_sha256: snapshot.package_sha256,
            package_bytes: snapshot.package_bytes,
            app_id,
            app_version,
            capability_fingerprint,
            advisory_sequence: advisory.as_ref().map_or_else(
                || current.as_ref().map_or(0, |state| state.advisory_sequence),
                |record| record.sequence,
            ),
            advisory_record_sha256: snapshot.advisory_record_sha256,
            last_trusted_time: now,
            recovery_activation: false,
        };
        self.commit_state(&trusted_state)?;
        let _ = fs::remove_file(staged_path);
        Ok(UpdateOutcome {
            state: trusted_state,
            permissions_must_be_reconfirmed,
        })
    }

    /// Activates a retained revision under exact signed or explicit local recovery policy.
    ///
    /// # Errors
    ///
    /// Rejects absent revisions, identity drift, stale policy or inexact authorization.
    pub fn recover(
        &self,
        target_package_sha256: &str,
        decision: RecoveryDecision<'_>,
        policy: &PublisherPolicy,
        now: u64,
    ) -> Result<UpdateOutcome, UpdateError> {
        validate_lower_hex(target_package_sha256, 64, "target package digest")?;
        validate_policy(policy, now, &BTreeSet::new())?;
        let states = self.load_states()?;
        let current = states.last().ok_or(UpdateError::NotFound)?;
        if now < current.last_trusted_time
            || policy.identity != current.identity
            || policy.version != current.publisher_policy_version
            || policy_digest(policy)? != current.publisher_policy_sha256
        {
            return Err(UpdateError::Freeze);
        }
        let target = states
            .iter()
            .rev()
            .find(|state| {
                state.identity == current.identity && state.package_sha256 == target_package_sha256
            })
            .ok_or(UpdateError::NotFound)?;
        if target.package_sha256 == current.package_sha256 {
            return Err(UpdateError::RecoveryAuthorization);
        }
        if target.app_id != current.app_id {
            return Err(UpdateError::RecoveryAuthorization);
        }
        let next_activation = checked_next(current.activation_sequence, "activation sequence")?;
        match decision {
            RecoveryDecision::Signed(bytes) => verify_recovery_authorization_json(
                bytes,
                policy,
                current,
                target,
                next_activation,
                now,
            )?,
            RecoveryDecision::ExplicitLocal(local) => {
                verify_local_recovery(local, current, target, now)?;
            }
        }
        let recovered = TrustedUpdateState {
            schema: TRUSTED_UPDATE_STATE_SCHEMA.to_owned(),
            identity: current.identity.clone(),
            activation_sequence: next_activation,
            publisher_policy_version: current.publisher_policy_version,
            publisher_policy_sha256: current.publisher_policy_sha256.clone(),
            checkpoint_sequence: current.checkpoint_sequence,
            checkpoint_sha256: current.checkpoint_sha256.clone(),
            snapshot_sequence: current.snapshot_sequence,
            channel_sequence: current.channel_sequence,
            release_sequence: current.release_sequence,
            release_record_sha256: target.release_record_sha256.clone(),
            package_sha256: target.package_sha256.clone(),
            package_bytes: target.package_bytes,
            app_id: target.app_id.clone(),
            app_version: target.app_version.clone(),
            capability_fingerprint: target.capability_fingerprint.clone(),
            advisory_sequence: current.advisory_sequence,
            advisory_record_sha256: current.advisory_record_sha256.clone(),
            last_trusted_time: now,
            recovery_activation: true,
        };
        self.commit_state(&recovered)?;
        Ok(UpdateOutcome {
            permissions_must_be_reconfirmed: current.capability_fingerprint
                != recovered.capability_fingerprint,
            state: recovered,
        })
    }

    fn verify_candidate_checkpoint(
        &self,
        bytes: &[u8],
        authority: &RegistryAuthority,
        current: Option<&TrustedUpdateState>,
        now: u64,
    ) -> Result<RegistryCheckpoint, UpdateError> {
        match current {
            Some(state) if sha256_hex(bytes) == state.checkpoint_sha256 => {
                let stored = fs::read(self.metadata_path("checkpoints", &state.checkpoint_sha256))
                    .map_err(|_| UpdateError::NotFound)?;
                if stored != bytes {
                    return Err(UpdateError::DigestMismatch);
                }
                parse_trusted_checkpoint(bytes, authority, state, now)
            }
            Some(state) => {
                let previous =
                    fs::read(self.metadata_path("checkpoints", &state.checkpoint_sha256))
                        .map_err(|_| UpdateError::NotFound)?;
                Ok(verify_checkpoint_json(
                    bytes,
                    authority,
                    Some(&previous),
                    now,
                )?)
            }
            None => Ok(verify_checkpoint_json(bytes, authority, None, now)?),
        }
    }

    fn persist_candidate(
        &self,
        snapshot: &UpdateSnapshot,
        candidate: UpdateCandidate<'_>,
        staged_bytes: &[u8],
    ) -> Result<(), UpdateError> {
        write_immutable(&self.revision_path(&snapshot.package_sha256), staged_bytes)?;
        for (kind, digest, bytes) in [
            (
                "checkpoints",
                snapshot.checkpoint_sha256.as_str(),
                candidate.checkpoint_bytes,
            ),
            (
                "namespaces",
                snapshot.namespace_event_sha256.as_str(),
                candidate.namespace_bytes,
            ),
            (
                "channels",
                snapshot.channel_record_sha256.as_str(),
                candidate.channel_bytes,
            ),
            (
                "releases",
                snapshot.release_record_sha256.as_str(),
                candidate.release_bytes,
            ),
        ] {
            write_immutable(&self.metadata_path(kind, digest), bytes)?;
        }
        write_immutable(
            &self.metadata_path("snapshots", &sha256_hex(candidate.snapshot_bytes)),
            candidate.snapshot_bytes,
        )?;
        if let (Some(bytes), Some(digest)) =
            (candidate.advisory_bytes, &snapshot.advisory_record_sha256)
        {
            write_immutable(&self.metadata_path("advisories", digest), bytes)?;
        }
        Ok(())
    }

    fn verify_candidate_advisory(
        &self,
        snapshot: &UpdateSnapshot,
        bytes: Option<&[u8]>,
        policy: &PublisherPolicy,
        current: Option<&TrustedUpdateState>,
        now: u64,
    ) -> Result<Option<AdvisoryRecord>, UpdateError> {
        let current_sequence = current.map_or(0, |state| state.advisory_sequence);
        let current_digest = current.and_then(|state| state.advisory_record_sha256.as_deref());
        if snapshot.advisory_sequence == current_sequence
            && snapshot.advisory_record_sha256.as_deref() == current_digest
        {
            if bytes.is_some() {
                return Err(UpdateError::Invalid("unexpected advisory bytes"));
            }
            return Ok(None);
        }
        if snapshot.advisory_sequence != checked_next(current_sequence, "advisory sequence")?
            || bytes.is_none()
            || snapshot.advisory_record_sha256.as_deref() != bytes.map(sha256_hex).as_deref()
        {
            return Err(UpdateError::Rollback);
        }
        let previous_bytes = current_digest
            .map(|digest| fs::read(self.metadata_path("advisories", digest)))
            .transpose()
            .map_err(|_| UpdateError::NotFound)?;
        let advisory = verify_advisory_record_json(
            bytes.ok_or(UpdateError::NotFound)?,
            previous_bytes.as_deref(),
            policy,
            now,
        )?;
        Ok(Some(advisory))
    }

    fn load_states(&self) -> Result<Vec<TrustedUpdateState>, UpdateError> {
        let directory = self.root.join("states");
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(directory).map_err(|error| io_error(&error))? {
            let entry = entry.map_err(|error| io_error(&error))?;
            let path = entry.path();
            let sequence = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or(UpdateError::Invalid("trusted state filename"))?;
            let bytes = fs::read(path).map_err(|error| io_error(&error))?;
            let envelope: TrustedStateEnvelope = parse_canonical(&bytes)?;
            if envelope.state.activation_sequence != sequence
                || state_checksum(&envelope.state)? != envelope.checksum_sha256
            {
                return Err(UpdateError::Invalid("trusted state checksum/sequence"));
            }
            validate_state_shape(&envelope.state)?;
            entries.push(envelope.state);
        }
        entries.sort_by_key(|state| state.activation_sequence);
        for (index, state) in entries.iter().enumerate() {
            let expected = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(UpdateError::Invalid("trusted state count"))?;
            if state.activation_sequence != expected {
                return Err(UpdateError::Invalid("trusted state continuity"));
            }
            if let Some(previous) = index.checked_sub(1).and_then(|value| entries.get(value)) {
                validate_state_transition(previous, state)?;
            }
            verify_retained_revision(&self.revision_path(&state.package_sha256), state)?;
        }
        Ok(entries)
    }

    fn commit_state(&self, state: &TrustedUpdateState) -> Result<(), UpdateError> {
        let envelope = TrustedStateEnvelope {
            state: state.clone(),
            checksum_sha256: state_checksum(state)?,
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|error| UpdateError::InvalidValue(error.to_string()))?;
        write_new(
            &self
                .root
                .join("states")
                .join(format!("{}.json", state.activation_sequence)),
            &bytes,
        )
    }

    fn revision_path(&self, digest: &str) -> PathBuf {
        self.root.join("revisions/sha256").join(digest)
    }

    fn metadata_path(&self, kind: &str, digest: &str) -> PathBuf {
        self.root
            .join("metadata")
            .join(kind)
            .join(format!("{digest}.json"))
    }

    fn stage_path(&self, snapshot: &UpdateSnapshot) -> PathBuf {
        self.root.join("staging").join(format!(
            "{}-{}.sapp.partial",
            snapshot.snapshot_sequence, snapshot.package_sha256
        ))
    }
}

/// Signs an update snapshot with deterministic registry-authority fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_update_snapshot_fixture(
    snapshot: UpdateSnapshot,
    keys: &[SigningKey],
) -> Result<Vec<u8>, UpdateError> {
    sign_envelope(snapshot, keys, SNAPSHOT_DOMAIN, |signed, signatures| {
        SignedUpdateSnapshot { signed, signatures }
    })
}

/// Verifies an exact fresh update snapshot against registry and publisher trust.
///
/// # Errors
///
/// Rejects invalid schemas, identities, bounds, policy bindings or signatures.
pub fn verify_update_snapshot_json(
    bytes: &[u8],
    authority: &RegistryAuthority,
    policy: &PublisherPolicy,
    now: u64,
) -> Result<UpdateSnapshot, UpdateError> {
    super::validate_authority(authority)?;
    validate_policy(policy, now, &BTreeSet::new())?;
    let envelope: SignedUpdateSnapshot = parse_canonical(bytes)?;
    let snapshot = &envelope.signed;
    if snapshot.schema != UPDATE_SNAPSHOT_SCHEMA
        || snapshot.registry_id != authority.registry_id
        || snapshot.identity != policy.identity
        || snapshot.identity.registry_id != authority.registry_id
        || snapshot.snapshot_sequence == 0
        || snapshot.checkpoint_sequence == 0
        || snapshot.channel_sequence == 0
        || snapshot.release_sequence == 0
        || snapshot.publisher_policy_version != policy.version
        || snapshot.publisher_policy_sha256 != policy_digest(policy)?
        || snapshot.package_bytes == 0
    {
        return Err(UpdateError::Invalid("update snapshot identity/sequence"));
    }
    validate_time(
        snapshot.issued_at,
        snapshot.expires_at,
        now,
        ONLINE_LIFETIME,
    )?;
    for (value, field) in [
        (&snapshot.publisher_policy_sha256, "snapshot policy digest"),
        (
            &snapshot.namespace_event_sha256,
            "snapshot namespace digest",
        ),
        (&snapshot.checkpoint_sha256, "snapshot checkpoint digest"),
        (&snapshot.channel_record_sha256, "snapshot channel digest"),
        (&snapshot.release_record_sha256, "snapshot release digest"),
        (&snapshot.package_sha256, "snapshot package digest"),
        (
            &snapshot.capability_fingerprint,
            "snapshot capability fingerprint",
        ),
    ] {
        validate_lower_hex(value, 64, field)?;
    }
    match (snapshot.advisory_sequence, &snapshot.advisory_record_sha256) {
        (0, None) => {}
        (0, Some(_)) | (_, None) => return Err(UpdateError::Invalid("snapshot advisory binding")),
        (_, Some(digest)) => validate_lower_hex(digest, 64, "snapshot advisory digest")?,
    }
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(
        &envelope.signatures,
        authority.keys.iter().map(|key| &key.key_id),
    )?;
    let payload = signing_bytes(SNAPSHOT_DOMAIN, snapshot)?;
    verify_authority_threshold(authority, &envelope.signatures, &payload)?;
    Ok(envelope.signed)
}

/// Signs an append-only security advisory with fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_advisory_record_fixture(
    advisory: AdvisoryRecord,
    keys: &[SigningKey],
) -> Result<Vec<u8>, UpdateError> {
    sign_envelope(advisory, keys, ADVISORY_DOMAIN, |signed, signatures| {
        SignedAdvisoryRecord { signed, signatures }
    })
}

/// Verifies a signed advisory and its exact append-only predecessor.
///
/// # Errors
///
/// Rejects ambiguous affected artifacts, invalid chain data or missing revocation threshold.
pub fn verify_advisory_record_json(
    bytes: &[u8],
    previous_bytes: Option<&[u8]>,
    policy: &PublisherPolicy,
    now: u64,
) -> Result<AdvisoryRecord, UpdateError> {
    validate_policy(policy, now, &BTreeSet::new())?;
    let envelope: SignedAdvisoryRecord = parse_canonical(bytes)?;
    let advisory = &envelope.signed;
    if advisory.schema != ADVISORY_SCHEMA
        || advisory.identity != policy.identity
        || advisory.sequence == 0
        || advisory.issued_at == 0
        || advisory.issued_at > now
    {
        return Err(UpdateError::Invalid("advisory identity/time/sequence"));
    }
    validate_name(&advisory.advisory_id, 64, "advisory id")?;
    validate_name(&advisory.severity, 16, "advisory severity")?;
    validate_inert_text(&advisory.affected_semver, 128, "affected SemVer")?;
    validate_inert_text(&advisory.description, 4_096, "advisory description")?;
    validate_lower_hex(&advisory.audit_ref, 64, "advisory audit ref")?;
    validate_digest_set(
        &advisory.exact_release_record_digests,
        "advisory release digests",
    )?;
    validate_digest_set(&advisory.exact_package_digests, "advisory package digests")?;
    if advisory.exact_release_record_digests.is_empty() || advisory.exact_package_digests.is_empty()
    {
        return Err(UpdateError::Invalid("advisory exact affected artifacts"));
    }
    match previous_bytes {
        Some(previous) => {
            let prior: SignedAdvisoryRecord = parse_canonical(previous)?;
            let expected = checked_next(prior.signed.sequence, "advisory sequence")?;
            if advisory.sequence != expected
                || advisory.previous_advisory_sha256.as_deref() != Some(&sha256_hex(previous))
                || advisory.issued_at < prior.signed.issued_at
            {
                return Err(UpdateError::Invalid("advisory chain"));
            }
        }
        None => {
            if advisory.sequence != 1 || advisory.previous_advisory_sha256.is_some() {
                return Err(UpdateError::Invalid("advisory genesis"));
            }
        }
    }
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(&envelope.signatures, policy.roles.revocation.key_ids.iter())?;
    let payload = signing_bytes(ADVISORY_DOMAIN, advisory)?;
    verify_role_threshold(policy, RoleKind::Revocation, &envelope.signatures, &payload)?;
    Ok(envelope.signed)
}

/// Signs exact retained-revision recovery authorization with fixture keys.
///
/// # Errors
///
/// Returns an error when canonical serialization fails.
pub fn sign_recovery_authorization_fixture(
    authorization: RecoveryAuthorization,
    keys: &[SigningKey],
) -> Result<Vec<u8>, UpdateError> {
    sign_envelope(
        authorization,
        keys,
        RECOVERY_DOMAIN,
        |signed, signatures| SignedRecoveryAuthorization { signed, signatures },
    )
}

fn verify_recovery_authorization_json(
    bytes: &[u8],
    policy: &PublisherPolicy,
    current: &TrustedUpdateState,
    target: &TrustedUpdateState,
    expected_activation: u64,
    now: u64,
) -> Result<(), UpdateError> {
    let envelope: SignedRecoveryAuthorization = parse_canonical(bytes)?;
    let authorization = &envelope.signed;
    if authorization.schema != RECOVERY_AUTHORIZATION_SCHEMA
        || authorization.identity != current.identity
        || authorization.identity != target.identity
        || authorization.identity != policy.identity
        || authorization.activation_sequence != expected_activation
        || authorization.from_package_sha256 != current.package_sha256
        || authorization.to_release_record_sha256 != target.release_record_sha256
        || authorization.to_package_sha256 != target.package_sha256
        || authorization.to_capability_fingerprint != target.capability_fingerprint
    {
        return Err(UpdateError::RecoveryAuthorization);
    }
    validate_time(
        authorization.issued_at,
        authorization.expires_at,
        now,
        ONLINE_LIFETIME,
    )?;
    validate_name(&authorization.reason, 64, "recovery reason")?;
    validate_lower_hex(&authorization.audit_ref, 64, "recovery audit ref")?;
    validate_signatures(&envelope.signatures)?;
    ensure_signature_keys_allowed(&envelope.signatures, policy.roles.recovery.key_ids.iter())?;
    let payload = signing_bytes(RECOVERY_DOMAIN, authorization)?;
    verify_role_threshold(policy, RoleKind::Recovery, &envelope.signatures, &payload)?;
    Ok(())
}

fn verify_local_recovery(
    local: &LocalRecoveryPolicy,
    current: &TrustedUpdateState,
    target: &TrustedUpdateState,
    now: u64,
) -> Result<(), UpdateError> {
    let expected_confirmation =
        local_recovery_confirmation(&current.package_sha256, &target.package_sha256)?;
    if now >= local.expires_at
        || local.identity != current.identity
        || local.identity != target.identity
        || local.from_package_sha256 != current.package_sha256
        || local.to_package_sha256 != target.package_sha256
        || local.confirmation_sha256 != expected_confirmation
    {
        return Err(UpdateError::RecoveryAuthorization);
    }
    Ok(())
}

/// Computes the exact confirmation digest for an explicit local recovery decision.
///
/// # Errors
///
/// Rejects malformed source or target package digests.
pub fn local_recovery_confirmation(from: &str, to: &str) -> Result<String, UpdateError> {
    validate_lower_hex(from, 64, "local recovery source digest")?;
    validate_lower_hex(to, 64, "local recovery target digest")?;
    Ok(sha256_hex(
        [
            b"SICO-LOCAL-RECOVERY-CONFIRMATION-V0\0".as_slice(),
            from.as_bytes(),
            to.as_bytes(),
        ]
        .concat()
        .as_slice(),
    ))
}

fn validate_transaction(
    snapshot: &UpdateSnapshot,
    checkpoint: &RegistryCheckpoint,
    namespace: &NamespaceEvent,
    channel: &ChannelRecord,
    release: &ReleaseRecord,
    candidate: UpdateCandidate<'_>,
    policy: &PublisherPolicy,
) -> Result<(), UpdateError> {
    let namespace_digest = sha256_hex(candidate.namespace_bytes);
    let checkpoint_digest = sha256_hex(candidate.checkpoint_bytes);
    let channel_digest = sha256_hex(candidate.channel_bytes);
    let release_digest = sha256_hex(candidate.release_bytes);
    if snapshot.identity != policy.identity
        || snapshot.namespace_event_sha256 != namespace_digest
        || snapshot.checkpoint_sha256 != checkpoint_digest
        || snapshot.channel_record_sha256 != channel_digest
        || snapshot.release_record_sha256 != release_digest
        || snapshot.package_sha256 != release.package_sha256
        || snapshot.package_bytes != release.package_bytes
        || snapshot.checkpoint_sequence != checkpoint.sequence
        || snapshot.channel_sequence != channel.sequence
        || namespace.next_owner.as_ref() != Some(&super::publisher_reference(policy)?)
        || release.namespace_event_sha256 != namespace_digest
        || channel.release_record_sha256 != release_digest
        || !checkpoint
            .namespace_event_digests
            .contains(&namespace_digest)
        || !checkpoint.release_record_digests.contains(&release_digest)
        || !checkpoint.channel_record_digests.contains(&channel_digest)
    {
        return Err(UpdateError::Invalid("mixed update transaction"));
    }
    Ok(())
}

fn validate_monotonic_update(
    snapshot: &UpdateSnapshot,
    current: Option<&TrustedUpdateState>,
) -> Result<(), UpdateError> {
    let Some(current) = current else {
        if snapshot.snapshot_sequence != 1
            || snapshot.release_sequence != 1
            || snapshot.channel_sequence != 1
            || snapshot.checkpoint_sequence != 1
        {
            return Err(UpdateError::Rollback);
        }
        return Ok(());
    };
    if snapshot.identity != current.identity
        || snapshot.publisher_policy_version != current.publisher_policy_version
        || snapshot.snapshot_sequence <= current.snapshot_sequence
        || snapshot.checkpoint_sequence < current.checkpoint_sequence
        || snapshot.channel_sequence < current.channel_sequence
        || snapshot.release_sequence < current.release_sequence
        || snapshot.publisher_policy_sha256 != current.publisher_policy_sha256
        || (snapshot.checkpoint_sequence == current.checkpoint_sequence
            && snapshot.checkpoint_sha256 != current.checkpoint_sha256)
        || (snapshot.release_sequence == current.release_sequence
            && (snapshot.release_record_sha256 != current.release_record_sha256
                || snapshot.package_sha256 != current.package_sha256))
        || snapshot.advisory_sequence < current.advisory_sequence
    {
        return Err(UpdateError::Rollback);
    }
    Ok(())
}

fn parse_trusted_checkpoint(
    bytes: &[u8],
    authority: &RegistryAuthority,
    state: &TrustedUpdateState,
    now: u64,
) -> Result<RegistryCheckpoint, UpdateError> {
    let parsed = verify_checkpoint_unlinked(bytes, authority, Some(now))?;
    if parsed.sequence != state.checkpoint_sequence || sha256_hex(bytes) != state.checkpoint_sha256
    {
        return Err(UpdateError::Rollback);
    }
    Ok(parsed)
}

fn verify_package(
    bytes: &[u8],
    release: &ReleaseRecord,
    expected_capability_fingerprint: &str,
) -> Result<(String, String, String), UpdateError> {
    if bytes.len() as u64 != release.package_bytes {
        return Err(UpdateError::LengthMismatch);
    }
    if sha256_hex(bytes) != release.package_sha256 {
        return Err(UpdateError::DigestMismatch);
    }
    let package =
        sico_package::verify(bytes).map_err(|error| UpdateError::Package(error.to_string()))?;
    if package.manifest.app.id != release.app_id
        || package.manifest.app.version != release.app_version
    {
        return Err(UpdateError::Invalid("package release identity"));
    }
    let imported = sico_package::capabilities_for_imports(&package.component_imports)
        .map_err(|error| UpdateError::Package(error.to_string()))?;
    let requested: BTreeSet<_> = package.manifest.capabilities.iter().cloned().collect();
    if package.manifest.source_effects != package.manifest.capabilities || imported != requested {
        return Err(UpdateError::Invalid("package capability closure"));
    }
    let capability_fingerprint = capability_fingerprint(&package.manifest.capabilities)?;
    if capability_fingerprint != expected_capability_fingerprint {
        return Err(UpdateError::Invalid("capability fingerprint"));
    }
    Ok((
        package.manifest.app.id,
        package.manifest.app.version,
        capability_fingerprint,
    ))
}

/// Computes the canonical capability fingerprint used for permission re-evaluation.
///
/// # Errors
///
/// Returns an error when the capability list cannot be serialized.
pub fn capability_fingerprint(capabilities: &[String]) -> Result<String, UpdateError> {
    let bytes = serde_json::to_vec(capabilities)
        .map_err(|error| UpdateError::InvalidValue(error.to_string()))?;
    Ok(sha256_hex(&[CAPABILITY_DOMAIN, &bytes].concat()))
}

fn validate_state_shape(state: &TrustedUpdateState) -> Result<(), UpdateError> {
    validate_identity(&state.identity)?;
    validate_name(&state.app_id, 128, "state application id")?;
    if state.schema != TRUSTED_UPDATE_STATE_SCHEMA
        || state.activation_sequence == 0
        || state.publisher_policy_version == 0
        || state.checkpoint_sequence == 0
        || state.snapshot_sequence == 0
        || state.channel_sequence == 0
        || state.release_sequence == 0
        || state.package_bytes == 0
        || state.last_trusted_time == 0
    {
        return Err(UpdateError::Invalid("trusted state shape"));
    }
    for (value, field) in [
        (&state.publisher_policy_sha256, "state policy digest"),
        (&state.checkpoint_sha256, "state checkpoint digest"),
        (&state.release_record_sha256, "state release digest"),
        (&state.package_sha256, "state package digest"),
        (
            &state.capability_fingerprint,
            "state capability fingerprint",
        ),
    ] {
        validate_lower_hex(value, 64, field)?;
    }
    match (state.advisory_sequence, &state.advisory_record_sha256) {
        (0, None) => {}
        (0, Some(_)) | (_, None) => return Err(UpdateError::Invalid("state advisory binding")),
        (_, Some(digest)) => validate_lower_hex(digest, 64, "state advisory digest")?,
    }
    Ok(())
}

fn validate_state_transition(
    previous: &TrustedUpdateState,
    current: &TrustedUpdateState,
) -> Result<(), UpdateError> {
    if current.activation_sequence != checked_next(previous.activation_sequence, "state sequence")?
        || current.identity != previous.identity
        || current.app_id != previous.app_id
        || current.publisher_policy_version < previous.publisher_policy_version
        || current.checkpoint_sequence < previous.checkpoint_sequence
        || current.snapshot_sequence < previous.snapshot_sequence
        || current.channel_sequence < previous.channel_sequence
        || current.release_sequence < previous.release_sequence
        || current.advisory_sequence < previous.advisory_sequence
        || current.last_trusted_time < previous.last_trusted_time
        || (current.publisher_policy_version == previous.publisher_policy_version
            && current.publisher_policy_sha256 != previous.publisher_policy_sha256)
        || (current.checkpoint_sequence == previous.checkpoint_sequence
            && current.checkpoint_sha256 != previous.checkpoint_sha256)
        || (!current.recovery_activation && current.snapshot_sequence == previous.snapshot_sequence)
    {
        return Err(UpdateError::Rollback);
    }
    Ok(())
}

fn verify_retained_revision(path: &Path, state: &TrustedUpdateState) -> Result<(), UpdateError> {
    let bytes = fs::read(path).map_err(|_| UpdateError::NotFound)?;
    if bytes.len() as u64 != state.package_bytes {
        return Err(UpdateError::LengthMismatch);
    }
    if sha256_hex(&bytes) != state.package_sha256 {
        return Err(UpdateError::DigestMismatch);
    }
    let package =
        sico_package::verify(&bytes).map_err(|error| UpdateError::Package(error.to_string()))?;
    if package.manifest.app.id != state.app_id
        || package.manifest.app.version != state.app_version
        || capability_fingerprint(&package.manifest.capabilities)? != state.capability_fingerprint
    {
        return Err(UpdateError::Invalid("retained revision metadata"));
    }
    Ok(())
}

fn validate_digest_set(values: &[String], field: &'static str) -> Result<(), UpdateError> {
    if values.len() > 4_096 || values.windows(2).any(|window| window[0] >= window[1]) {
        return Err(UpdateError::Invalid(field));
    }
    for value in values {
        validate_lower_hex(value, 64, field)?;
    }
    Ok(())
}

fn validate_inert_text(value: &str, max: usize, field: &'static str) -> Result<(), UpdateError> {
    if value.is_empty()
        || value.len() > max
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Err(UpdateError::Invalid(field));
    }
    Ok(())
}

fn state_checksum(state: &TrustedUpdateState) -> Result<String, UpdateError> {
    let bytes =
        serde_json::to_vec(state).map_err(|error| UpdateError::InvalidValue(error.to_string()))?;
    Ok(sha256_hex(&[STATE_DOMAIN, &bytes].concat()))
}

fn checked_next(value: u64, field: &'static str) -> Result<u64, UpdateError> {
    value.checked_add(1).ok_or(UpdateError::Invalid(field))
}

fn sign_envelope<T, E>(
    signed: T,
    keys: &[SigningKey],
    domain: &[u8],
    make: impl FnOnce(T, Vec<MetadataSignature>) -> E,
) -> Result<Vec<u8>, UpdateError>
where
    T: Serialize,
    E: Serialize,
{
    let payload = signing_bytes(domain, &signed)?;
    let signatures = sign_payload(&payload, keys);
    serde_json::to_vec(&make(signed, signatures))
        .map_err(|error| UpdateError::InvalidValue(error.to_string()))
}

fn signing_bytes<T: Serialize>(domain: &[u8], signed: &T) -> Result<Vec<u8>, UpdateError> {
    let canonical =
        serde_json::to_vec(signed).map_err(|error| UpdateError::InvalidValue(error.to_string()))?;
    Ok([domain, &canonical].concat())
}

fn parse_canonical<T>(bytes: &[u8]) -> Result<T, UpdateError>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    if bytes.len() > MAX_UPDATE_RECORD_BYTES {
        return Err(UpdateError::Invalid("update record size"));
    }
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| UpdateError::InvalidValue(error.to_string()))?;
    if serde_json::to_vec(&value).map_err(|error| UpdateError::InvalidValue(error.to_string()))?
        != bytes
    {
        return Err(UpdateError::NonCanonical);
    }
    Ok(value)
}

fn write_immutable(path: &Path, bytes: &[u8]) -> Result<(), UpdateError> {
    if path.is_file() {
        let existing = fs::read(path).map_err(|error| io_error(&error))?;
        return if existing == bytes {
            Ok(())
        } else {
            Err(UpdateError::Conflict)
        };
    }
    write_new(path, bytes)
}

fn write_staging(path: &Path, bytes: &[u8]) -> Result<(), UpdateError> {
    if path.is_file() {
        let existing = fs::read(path).map_err(|error| io_error(&error))?;
        if existing == bytes {
            return Ok(());
        }
        fs::remove_file(path).map_err(|error| io_error(&error))?;
    }
    write_new(path, bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), UpdateError> {
    let parent = path.parent().ok_or(UpdateError::Invalid("update path"))?;
    fs::create_dir_all(parent).map_err(|error| io_error(&error))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error(&error))?;
    file.write_all(bytes).map_err(|error| io_error(&error))?;
    file.sync_all().map_err(|error| io_error(&error))
}

fn io_error(error: &std::io::Error) -> UpdateError {
    UpdateError::Io(error.to_string())
}
