use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_ecosystem::{
    ADVISORY_SCHEMA, AdvisoryRecord, CHANNEL_SCHEMA, CHECKPOINT_SCHEMA, ChannelRecord, Custody,
    DisclosureIntake, DisclosurePolicy, IdentityRule, LocalRecoveryPolicy, NAMESPACE_SCHEMA,
    NamespaceEvent, NamespaceEventKind, POLICY_SCHEMA, ProductionIdentity, PublicKeyRecord,
    PublisherPolicy, RECOVERY_AUTHORIZATION_SCHEMA, RELEASE_SCHEMA, RecoveryAuthorization,
    RecoveryDecision, RegistryAuthority, RegistryCheckpoint, ReleaseRecord, RolePolicies,
    RolePolicy, TransparencyPolicy, UPDATE_SNAPSHOT_SCHEMA, UpdateCandidate, UpdateClient,
    UpdateError, UpdateSnapshot, capability_fingerprint, local_recovery_confirmation,
    policy_digest, public_key_id, publisher_reference, sign_advisory_record_fixture,
    sign_channel_record_fixture, sign_checkpoint_fixture, sign_namespace_event_fixture,
    sign_recovery_authorization_fixture, sign_release_record_fixture, sign_update_snapshot_fixture,
    verify_advisory_record_json, verify_update_snapshot_json,
};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sha256_hex};
use wasm_encoder::{
    Component, ComponentExportKind, ComponentExportSection, ComponentImportSection,
    ComponentTypeRef, ComponentValType, PrimitiveValType,
};

const DAY: u64 = 24 * 60 * 60;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}

fn key_record(key: &SigningKey) -> PublicKeyRecord {
    PublicKeyRecord {
        key_id: public_key_id(key.verifying_key().as_bytes()),
        scheme: "ed25519".to_owned(),
        public_key: hex(key.verifying_key().as_bytes()),
    }
}

fn role(custody: Custody, threshold: u16, seeds: &[u8]) -> RolePolicy {
    let mut key_ids: Vec<_> = seeds
        .iter()
        .map(|seed| public_key_id(key(*seed).verifying_key().as_bytes()))
        .collect();
    key_ids.sort();
    RolePolicy {
        custody,
        threshold,
        key_ids,
    }
}

fn policy() -> PublisherPolicy {
    let mut keys: Vec<_> = (1..=9).map(|seed| key_record(&key(seed))).collect();
    keys.sort_by(|left, right| left.key_id.cmp(&right.key_id));
    PublisherPolicy {
        schema: POLICY_SCHEMA.to_owned(),
        version: 1,
        issued_at: 900,
        expires_at: 900 + 30 * DAY,
        identity: ProductionIdentity {
            registry_id: "registry.example".to_owned(),
            namespace: "example".to_owned(),
            package_name: "hello-sico".to_owned(),
            publisher_policy_id: "example-policy-v0".to_owned(),
        },
        keys,
        roles: RolePolicies {
            root: role(Custody::Offline, 2, &[1, 2]),
            release: role(Custody::Online, 1, &[3]),
            recovery: role(Custody::Offline, 2, &[4, 5]),
            rotation: role(Custody::Offline, 2, &[6, 7]),
            revocation: role(Custody::Offline, 2, &[8, 9]),
        },
        identity_rules: vec![IdentityRule::Key {
            key_id: public_key_id(key(3).verifying_key().as_bytes()),
        }],
        disclosure: DisclosurePolicy {
            intake: DisclosureIntake::OwnerControlledUnconfigured,
            retention_days: 30,
            redacted_fields: vec![
                "access-token".to_owned(),
                "credential".to_owned(),
                "private-key".to_owned(),
                "user-data".to_owned(),
            ],
            transparency: TransparencyPolicy::OwnerApprovalRequired,
        },
    }
}

fn authority() -> RegistryAuthority {
    let mut keys = vec![key_record(&key(50)), key_record(&key(51))];
    keys.sort_by(|left, right| left.key_id.cmp(&right.key_id));
    RegistryAuthority {
        registry_id: "registry.example".to_owned(),
        keys,
        threshold: 2,
    }
}

fn digest(seed: u8) -> String {
    format!("{seed:02x}").repeat(32)
}

fn importing_component(name: &str) -> Vec<u8> {
    let mut imports = ComponentImportSection::new();
    imports.import(
        name,
        ComponentTypeRef::Value(ComponentValType::Primitive(PrimitiveValType::Bool)),
    );
    let mut component = Component::new();
    component.section(&imports);
    let mut exports = ComponentExportSection::new();
    exports.export("capability-probe", ComponentExportKind::Value, 0, None);
    component.section(&exports);
    component.finish()
}

fn package(version: &str, clock: bool) -> Vec<u8> {
    let capabilities = if clock {
        vec!["clock.read".to_owned()]
    } else {
        Vec::new()
    };
    build_unsigned(BuildInput {
        app_id: "dev.sico.answer".to_owned(),
        app_version: version.to_owned(),
        component: if clock {
            importing_component("wasi:clocks/monotonic-clock@0.2.0")
        } else {
            Component::new().finish()
        },
        resources: Vec::new(),
        source_effects: capabilities.clone(),
        capabilities,
        limits: RuntimeLimits::default(),
    })
    .unwrap()
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-step-0065-{name}-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[derive(Clone)]
struct OwnedCandidate {
    snapshot: Vec<u8>,
    checkpoint: Vec<u8>,
    namespace: Vec<u8>,
    channel: Vec<u8>,
    release: Vec<u8>,
    package: Vec<u8>,
    advisory: Option<Vec<u8>>,
}

impl OwnedCandidate {
    fn borrowed(&self) -> UpdateCandidate<'_> {
        UpdateCandidate {
            snapshot_bytes: &self.snapshot,
            checkpoint_bytes: &self.checkpoint,
            namespace_bytes: &self.namespace,
            channel_bytes: &self.channel,
            release_bytes: &self.release,
            package_bytes: &self.package,
            advisory_bytes: self.advisory.as_deref(),
        }
    }
}

struct Fixture {
    policy: PublisherPolicy,
    authority: RegistryAuthority,
    first: OwnedCandidate,
    second: OwnedCandidate,
    second_snapshot: UpdateSnapshot,
    advisory: AdvisoryRecord,
}

fn release(
    policy: &PublisherPolicy,
    namespace: &[u8],
    package: &[u8],
    version: &str,
    issued_at: u64,
) -> ReleaseRecord {
    ReleaseRecord {
        schema: RELEASE_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        publisher_policy_version: policy.version,
        publisher_policy_sha256: policy_digest(policy).unwrap(),
        namespace_event_sha256: sha256_hex(namespace),
        app_id: "dev.sico.answer".to_owned(),
        app_version: version.to_owned(),
        package_sha256: sha256_hex(package),
        package_bytes: package.len() as u64,
        package_format: sico_package::FORMAT_VERSION,
        manifest_schema: "sico.sapp.manifest.v0".to_owned(),
        language_semantics: "sico-semantics-v0".to_owned(),
        component_world: "sico-app-v0".to_owned(),
        wasi_contract: "wasi-preview2-v0".to_owned(),
        capability_contract: "sico-capability-v0".to_owned(),
        dependency_lock_sha256: digest(0xd1),
        minimum_host_contract: "sico-host-v0".to_owned(),
        description: format!("Release {version}"),
        issued_at,
        expires_at: issued_at + DAY,
    }
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

#[allow(clippy::too_many_lines)]
fn fixture() -> Fixture {
    let policy = policy();
    let authority = authority();
    let namespace_event = NamespaceEvent {
        schema: NAMESPACE_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        namespace: "example".to_owned(),
        sequence: 1,
        issued_at: 1_000,
        expires_at: 1_000 + DAY,
        kind: NamespaceEventKind::Grant,
        previous_event_sha256: None,
        previous_owner: None,
        next_owner: Some(publisher_reference(&policy).unwrap()),
        reuse_after: None,
        audit_ref: digest(0xa1),
    };
    let namespace =
        sign_namespace_event_fixture(namespace_event, &[key(1), key(2), key(50), key(51)]).unwrap();
    let namespace_digest = sha256_hex(&namespace);

    let package1 = package("1.0.0", false);
    let release1 = release(&policy, &namespace, &package1, "1.0.0", 1_100);
    let release1_bytes = sign_release_record_fixture(release1, &[key(3)]).unwrap();
    let release1_digest = sha256_hex(&release1_bytes);
    let channel1 = ChannelRecord {
        schema: CHANNEL_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        channel: "stable".to_owned(),
        sequence: 1,
        release_record_sha256: release1_digest.clone(),
        issued_at: 1_150,
        expires_at: 1_150 + DAY,
    };
    let channel1_bytes = sign_channel_record_fixture(channel1, &[key(3)]).unwrap();
    let channel1_digest = sha256_hex(&channel1_bytes);
    let checkpoint1 = RegistryCheckpoint {
        schema: CHECKPOINT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        sequence: 1,
        previous_checkpoint_sha256: None,
        namespace_event_digests: vec![namespace_digest.clone()],
        release_record_digests: vec![release1_digest.clone()],
        channel_record_digests: vec![channel1_digest.clone()],
        issued_at: 1_200,
        expires_at: 1_200 + DAY,
    };
    let checkpoint1_bytes = sign_checkpoint_fixture(checkpoint1, &[key(50), key(51)]).unwrap();
    let snapshot1 = UpdateSnapshot {
        schema: UPDATE_SNAPSHOT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        identity: policy.identity.clone(),
        snapshot_sequence: 1,
        checkpoint_sequence: 1,
        channel_sequence: 1,
        release_sequence: 1,
        publisher_policy_version: policy.version,
        publisher_policy_sha256: policy_digest(&policy).unwrap(),
        namespace_event_sha256: namespace_digest.clone(),
        checkpoint_sha256: sha256_hex(&checkpoint1_bytes),
        channel_record_sha256: channel1_digest.clone(),
        release_record_sha256: release1_digest.clone(),
        package_sha256: sha256_hex(&package1),
        package_bytes: package1.len() as u64,
        capability_fingerprint: capability_fingerprint(&[]).unwrap(),
        advisory_sequence: 0,
        advisory_record_sha256: None,
        issued_at: 1_250,
        expires_at: 1_250 + DAY,
    };
    let snapshot1_bytes = sign_update_snapshot_fixture(snapshot1, &[key(50), key(51)]).unwrap();

    let package2 = package("1.1.0", true);
    let release2 = release(&policy, &namespace, &package2, "1.1.0", 2_100);
    let release2_bytes = sign_release_record_fixture(release2, &[key(3)]).unwrap();
    let release2_digest = sha256_hex(&release2_bytes);
    let channel2 = ChannelRecord {
        schema: CHANNEL_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        channel: "stable".to_owned(),
        sequence: 2,
        release_record_sha256: release2_digest.clone(),
        issued_at: 2_150,
        expires_at: 2_150 + DAY,
    };
    let channel2_bytes = sign_channel_record_fixture(channel2, &[key(3)]).unwrap();
    let channel2_digest = sha256_hex(&channel2_bytes);
    let checkpoint2 = RegistryCheckpoint {
        schema: CHECKPOINT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        sequence: 2,
        previous_checkpoint_sha256: Some(sha256_hex(&checkpoint1_bytes)),
        namespace_event_digests: vec![namespace_digest.clone()],
        release_record_digests: sorted(vec![release1_digest.clone(), release2_digest.clone()]),
        channel_record_digests: sorted(vec![channel1_digest, channel2_digest.clone()]),
        issued_at: 2_200,
        expires_at: 2_200 + DAY,
    };
    let checkpoint2_bytes = sign_checkpoint_fixture(checkpoint2, &[key(50), key(51)]).unwrap();
    let advisory = AdvisoryRecord {
        schema: ADVISORY_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        sequence: 1,
        previous_advisory_sha256: None,
        advisory_id: "sico-2026-0001".to_owned(),
        affected_semver: ">=1.1.0,<1.1.1".to_owned(),
        exact_release_record_digests: vec![release2_digest.clone()],
        exact_package_digests: vec![sha256_hex(&package2)],
        severity: "high".to_owned(),
        description: "Security advisory; render as inert text only.".to_owned(),
        audit_ref: digest(0xad),
        issued_at: 2_225,
    };
    let advisory_bytes = sign_advisory_record_fixture(advisory.clone(), &[key(8), key(9)]).unwrap();
    let snapshot2 = UpdateSnapshot {
        schema: UPDATE_SNAPSHOT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        identity: policy.identity.clone(),
        snapshot_sequence: 2,
        checkpoint_sequence: 2,
        channel_sequence: 2,
        release_sequence: 2,
        publisher_policy_version: policy.version,
        publisher_policy_sha256: policy_digest(&policy).unwrap(),
        namespace_event_sha256: namespace_digest,
        checkpoint_sha256: sha256_hex(&checkpoint2_bytes),
        channel_record_sha256: channel2_digest,
        release_record_sha256: release2_digest,
        package_sha256: sha256_hex(&package2),
        package_bytes: package2.len() as u64,
        capability_fingerprint: capability_fingerprint(&["clock.read".to_owned()]).unwrap(),
        advisory_sequence: 1,
        advisory_record_sha256: Some(sha256_hex(&advisory_bytes)),
        issued_at: 2_250,
        expires_at: 2_250 + DAY,
    };
    let snapshot2_bytes =
        sign_update_snapshot_fixture(snapshot2.clone(), &[key(50), key(51)]).unwrap();
    Fixture {
        policy,
        authority,
        first: OwnedCandidate {
            snapshot: snapshot1_bytes,
            checkpoint: checkpoint1_bytes,
            namespace: namespace.clone(),
            channel: channel1_bytes,
            release: release1_bytes,
            package: package1,
            advisory: None,
        },
        second: OwnedCandidate {
            snapshot: snapshot2_bytes,
            checkpoint: checkpoint2_bytes,
            namespace,
            channel: channel2_bytes,
            release: release2_bytes,
            package: package2,
            advisory: Some(advisory_bytes),
        },
        second_snapshot: snapshot2,
        advisory,
    }
}

fn apply_both(root: &Path, fixture: &Fixture) -> (UpdateClient, sico_ecosystem::UpdateOutcome) {
    let client = UpdateClient::open(root).unwrap();
    client
        .apply_update(
            fixture.first.borrowed(),
            &fixture.authority,
            &fixture.policy,
            1_300,
        )
        .unwrap();
    let second = client
        .apply_update(
            fixture.second.borrowed(),
            &fixture.authority,
            &fixture.policy,
            2_300,
        )
        .unwrap();
    (client, second)
}

#[test]
fn two_updates_persist_monotonic_state_and_reconfirm_capabilities() {
    let fixture = fixture();
    let root = temp_root("roundtrip");
    let client = UpdateClient::open(&root).unwrap();
    let first = client
        .apply_update(
            fixture.first.borrowed(),
            &fixture.authority,
            &fixture.policy,
            1_300,
        )
        .unwrap();
    assert!(first.permissions_must_be_reconfirmed);
    let second = client
        .apply_update(
            fixture.second.borrowed(),
            &fixture.authority,
            &fixture.policy,
            2_300,
        )
        .unwrap();
    assert!(second.permissions_must_be_reconfirmed);
    assert_eq!(second.state.activation_sequence, 2);
    assert_eq!(second.state.checkpoint_sequence, 2);
    assert_eq!(second.state.advisory_sequence, 1);
    assert_eq!(
        UpdateClient::open(&root).unwrap().current_state().unwrap(),
        Some(second.state)
    );
    assert!(root.join("states/1.json").is_file());
    assert!(root.join("states/2.json").is_file());
    println!(
        "UPDATE_FIXTURE_METRICS package1={} package2={} state1={} state2={}",
        fixture.first.package.len(),
        fixture.second.package.len(),
        fs::metadata(root.join("states/1.json")).unwrap().len(),
        fs::metadata(root.join("states/2.json")).unwrap().len()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn replay_and_mix_and_match_are_rejected_before_activation() {
    let fixture = fixture();
    let root = temp_root("replay");
    let (client, current) = apply_both(&root, &fixture);
    assert_eq!(
        client.apply_update(
            fixture.first.borrowed(),
            &fixture.authority,
            &fixture.policy,
            2_200,
        ),
        Err(UpdateError::Freeze)
    );
    assert!(
        client
            .apply_update(
                fixture.first.borrowed(),
                &fixture.authority,
                &fixture.policy,
                2_400,
            )
            .is_err()
    );
    let mixed = UpdateCandidate {
        release_bytes: &fixture.first.release,
        package_bytes: &fixture.first.package,
        ..fixture.second.borrowed()
    };
    assert!(
        client
            .apply_update(mixed, &fixture.authority, &fixture.policy, 2_400)
            .is_err()
    );
    assert_eq!(client.current_state().unwrap(), Some(current.state));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_partial_candidate_preserves_last_known_good_and_can_retry() {
    let fixture = fixture();
    let root = temp_root("partial");
    let client = UpdateClient::open(&root).unwrap();
    let first = client
        .apply_update(
            fixture.first.borrowed(),
            &fixture.authority,
            &fixture.policy,
            1_300,
        )
        .unwrap();
    let mut corrupt = fixture.second.package.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    let candidate = UpdateCandidate {
        package_bytes: &corrupt,
        ..fixture.second.borrowed()
    };
    assert!(
        client
            .apply_update(candidate, &fixture.authority, &fixture.policy, 2_300)
            .is_err()
    );
    assert_eq!(client.current_state().unwrap(), Some(first.state));
    let second = client
        .apply_update(
            fixture.second.borrowed(),
            &fixture.authority,
            &fixture.policy,
            2_300,
        )
        .unwrap();
    assert_eq!(second.state.activation_sequence, 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn signed_recovery_retains_trust_head_and_rechecks_permissions() {
    let fixture = fixture();
    let root = temp_root("signed-recovery");
    let (client, current) = apply_both(&root, &fixture);
    let first: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("states/1.json")).unwrap()).unwrap();
    let first_state: sico_ecosystem::TrustedUpdateState =
        serde_json::from_value(first["state"].clone()).unwrap();
    let authorization = RecoveryAuthorization {
        schema: RECOVERY_AUTHORIZATION_SCHEMA.to_owned(),
        identity: fixture.policy.identity.clone(),
        activation_sequence: 3,
        from_package_sha256: current.state.package_sha256.clone(),
        to_release_record_sha256: first_state.release_record_sha256.clone(),
        to_package_sha256: first_state.package_sha256.clone(),
        to_capability_fingerprint: first_state.capability_fingerprint.clone(),
        reason: "compromised-release".to_owned(),
        audit_ref: digest(0xee),
        issued_at: 3_000,
        expires_at: 3_000 + DAY,
    };
    let wrong = sign_recovery_authorization_fixture(authorization.clone(), &[key(3)]).unwrap();
    assert!(
        client
            .recover(
                &first_state.package_sha256,
                RecoveryDecision::Signed(&wrong),
                &fixture.policy,
                3_001,
            )
            .is_err()
    );
    let mut cross_identity = authorization.clone();
    cross_identity.identity.package_name = "attacker".to_owned();
    let cross_identity =
        sign_recovery_authorization_fixture(cross_identity, &[key(4), key(5)]).unwrap();
    assert!(
        client
            .recover(
                &first_state.package_sha256,
                RecoveryDecision::Signed(&cross_identity),
                &fixture.policy,
                3_001,
            )
            .is_err()
    );
    let signed = sign_recovery_authorization_fixture(authorization, &[key(4), key(5)]).unwrap();
    let recovered = client
        .recover(
            &first_state.package_sha256,
            RecoveryDecision::Signed(&signed),
            &fixture.policy,
            3_001,
        )
        .unwrap();
    assert!(recovered.permissions_must_be_reconfirmed);
    assert!(recovered.state.recovery_activation);
    assert_eq!(
        recovered.state.snapshot_sequence,
        current.state.snapshot_sequence
    );
    assert_eq!(recovered.state.package_sha256, first_state.package_sha256);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_local_recovery_requires_exact_confirmation() {
    let fixture = fixture();
    let root = temp_root("local-recovery");
    let (client, current) = apply_both(&root, &fixture);
    let first: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("states/1.json")).unwrap()).unwrap();
    let first_state: sico_ecosystem::TrustedUpdateState =
        serde_json::from_value(first["state"].clone()).unwrap();
    let mut local = LocalRecoveryPolicy {
        identity: fixture.policy.identity.clone(),
        from_package_sha256: current.state.package_sha256.clone(),
        to_package_sha256: first_state.package_sha256.clone(),
        confirmation_sha256: digest(0xff),
        expires_at: 3_100,
    };
    assert!(
        client
            .recover(
                &first_state.package_sha256,
                RecoveryDecision::ExplicitLocal(&local),
                &fixture.policy,
                3_000,
            )
            .is_err()
    );
    local.confirmation_sha256 =
        local_recovery_confirmation(&current.state.package_sha256, &first_state.package_sha256)
            .unwrap();
    assert!(
        client
            .recover(
                &first_state.package_sha256,
                RecoveryDecision::ExplicitLocal(&local),
                &fixture.policy,
                3_000,
            )
            .is_ok()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn advisory_requires_append_only_exact_artifact_identity() {
    let fixture = fixture();
    verify_advisory_record_json(
        fixture.second.advisory.as_ref().unwrap(),
        None,
        &fixture.policy,
        2_300,
    )
    .unwrap();
    let mut ambiguous = fixture.advisory;
    ambiguous.exact_package_digests.clear();
    let ambiguous = sign_advisory_record_fixture(ambiguous, &[key(8), key(9)]).unwrap();
    assert!(verify_advisory_record_json(&ambiguous, None, &fixture.policy, 2_300).is_err());

    let mut fork = verify_advisory_record_json(
        fixture.second.advisory.as_ref().unwrap(),
        None,
        &fixture.policy,
        2_300,
    )
    .unwrap();
    fork.sequence = 2;
    fork.previous_advisory_sha256 = Some(digest(0xfe));
    fork.issued_at = 2_301;
    let fork = sign_advisory_record_fixture(fork, &[key(8), key(9)]).unwrap();
    assert!(
        verify_advisory_record_json(
            &fork,
            fixture.second.advisory.as_deref(),
            &fixture.policy,
            2_302,
        )
        .is_err()
    );
}

#[test]
fn snapshot_expiry_unknown_fields_and_every_byte_mutation_fail_closed() {
    let fixture = fixture();
    verify_update_snapshot_json(
        &fixture.second.snapshot,
        &fixture.authority,
        &fixture.policy,
        2_300,
    )
    .unwrap();
    let mut expired = fixture.second_snapshot;
    expired.expires_at = 2_300;
    let expired = sign_update_snapshot_fixture(expired, &[key(50), key(51)]).unwrap();
    assert!(
        verify_update_snapshot_json(&expired, &fixture.authority, &fixture.policy, 2_300).is_err()
    );

    let mut value: serde_json::Value = serde_json::from_slice(&fixture.second.snapshot).unwrap();
    value["signed"]["unknown"] = serde_json::json!(true);
    assert!(
        verify_update_snapshot_json(
            &serde_json::to_vec(&value).unwrap(),
            &fixture.authority,
            &fixture.policy,
            2_300,
        )
        .is_err()
    );

    let mut rejected = 0;
    for index in 0..fixture.second.snapshot.len() {
        let mut mutation = fixture.second.snapshot.clone();
        mutation[index] ^= 1;
        if verify_update_snapshot_json(&mutation, &fixture.authority, &fixture.policy, 2_300)
            .is_err()
        {
            rejected += 1;
        }
    }
    assert_eq!(rejected, fixture.second.snapshot.len());
    println!(
        "UPDATE_MUTATION_METRICS snapshot_bytes={} rejected={rejected}",
        fixture.second.snapshot.len()
    );
}

#[test]
fn tampered_trusted_state_and_missing_revision_fail_closed() {
    let fixture = fixture();
    let root = temp_root("state-tamper");
    let client = UpdateClient::open(&root).unwrap();
    let first = client
        .apply_update(
            fixture.first.borrowed(),
            &fixture.authority,
            &fixture.policy,
            1_300,
        )
        .unwrap();
    fs::remove_file(
        root.join("revisions/sha256")
            .join(&first.state.package_sha256),
    )
    .unwrap();
    assert!(client.current_state().is_err());
    fs::remove_dir_all(&root).unwrap();

    let root = temp_root("state-checksum");
    let client = UpdateClient::open(&root).unwrap();
    client
        .apply_update(
            fixture.first.borrowed(),
            &fixture.authority,
            &fixture.policy,
            1_300,
        )
        .unwrap();
    let path = root.join("states/1.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["checksum_sha256"] = serde_json::json!(digest(0));
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(client.current_state().is_err());
    fs::remove_dir_all(root).unwrap();
}
