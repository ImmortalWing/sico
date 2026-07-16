use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use ed25519_dalek::SigningKey;
use sico_ecosystem::{
    CHANNEL_SCHEMA, CHECKPOINT_SCHEMA, ChannelRecord, Custody, DisclosureIntake, DisclosurePolicy,
    IdentityRule, LocalRegistry, NAMESPACE_SCHEMA, NamespaceEvent, NamespaceEventKind,
    POLICY_SCHEMA, ProductionIdentity, PublicKeyRecord, PublisherPolicy, RELEASE_SCHEMA,
    RegistryAuthority, RegistryCheckpoint, RegistryError, ReleaseRecord, RolePolicies, RolePolicy,
    TransparencyPolicy, policy_digest, public_key_id, publisher_reference,
    sign_channel_record_fixture, sign_checkpoint_fixture, sign_namespace_event_fixture,
    sign_release_record_fixture, verify_channel_record_json, verify_checkpoint_json,
    verify_namespace_event_json, verify_release_record_json,
};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sha256_hex};
use wasm_encoder::Component;

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

fn policy(base: u8, package_name: &str, issued_at: u64) -> PublisherPolicy {
    let mut keys: Vec<_> = (base..base + 9)
        .map(|seed| key_record(&key(seed)))
        .collect();
    keys.sort_by(|left, right| left.key_id.cmp(&right.key_id));
    PublisherPolicy {
        schema: POLICY_SCHEMA.to_owned(),
        version: 1,
        issued_at,
        expires_at: issued_at + 30 * DAY,
        identity: ProductionIdentity {
            registry_id: "registry.example".to_owned(),
            namespace: "example".to_owned(),
            package_name: package_name.to_owned(),
            publisher_policy_id: format!("example-{base}"),
        },
        keys,
        roles: RolePolicies {
            root: role(Custody::Offline, 2, &[base, base + 1]),
            release: role(Custody::Online, 1, &[base + 2]),
            recovery: role(Custody::Offline, 2, &[base + 3, base + 4]),
            rotation: role(Custody::Offline, 2, &[base + 5, base + 6]),
            revocation: role(Custody::Offline, 2, &[base + 7, base + 8]),
        },
        identity_rules: vec![IdentityRule::Key {
            key_id: public_key_id(key(base + 2).verifying_key().as_bytes()),
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

fn audit(seed: u8) -> String {
    format!("{seed:02x}").repeat(32)
}

fn grant(policy: &PublisherPolicy, issued_at: u64) -> NamespaceEvent {
    NamespaceEvent {
        schema: NAMESPACE_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        namespace: "example".to_owned(),
        sequence: 1,
        issued_at,
        expires_at: issued_at + DAY,
        kind: NamespaceEventKind::Grant,
        previous_event_sha256: None,
        previous_owner: None,
        next_owner: Some(publisher_reference(policy).unwrap()),
        reuse_after: None,
        audit_ref: audit(0xa1),
    }
}

fn signed_grant(policy: &PublisherPolicy, issued_at: u64) -> Vec<u8> {
    sign_namespace_event_fixture(
        grant(policy, issued_at),
        &[key(1), key(2), key(50), key(51)],
    )
    .unwrap()
}

fn transfer_event(
    first: &PublisherPolicy,
    second: &PublisherPolicy,
    previous_bytes: &[u8],
) -> NamespaceEvent {
    NamespaceEvent {
        schema: NAMESPACE_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        namespace: "example".to_owned(),
        sequence: 2,
        issued_at: 2_000,
        expires_at: 2_000 + DAY,
        kind: NamespaceEventKind::Transfer,
        previous_event_sha256: Some(sha256_hex(previous_bytes)),
        previous_owner: Some(publisher_reference(first).unwrap()),
        next_owner: Some(publisher_reference(second).unwrap()),
        reuse_after: None,
        audit_ref: audit(0xa2),
    }
}

fn tombstone_event(policy: &PublisherPolicy, previous_bytes: &[u8]) -> NamespaceEvent {
    NamespaceEvent {
        schema: NAMESPACE_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        namespace: "example".to_owned(),
        sequence: 3,
        issued_at: 3_000,
        expires_at: 3_000 + DAY,
        kind: NamespaceEventKind::Tombstone,
        previous_event_sha256: Some(sha256_hex(previous_bytes)),
        previous_owner: Some(publisher_reference(policy).unwrap()),
        next_owner: None,
        reuse_after: Some(5_000),
        audit_ref: audit(0xa3),
    }
}

fn package() -> Vec<u8> {
    build_unsigned(BuildInput {
        app_id: "dev.sico.answer".to_owned(),
        app_version: "1.2.3".to_owned(),
        component: Component::new().finish(),
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .unwrap()
}

fn release(
    policy: &PublisherPolicy,
    namespace_bytes: &[u8],
    package: &[u8],
    issued_at: u64,
) -> ReleaseRecord {
    ReleaseRecord {
        schema: RELEASE_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        publisher_policy_version: policy.version,
        publisher_policy_sha256: policy_digest(policy).unwrap(),
        namespace_event_sha256: sha256_hex(namespace_bytes),
        app_id: "dev.sico.answer".to_owned(),
        app_version: "1.2.3".to_owned(),
        package_sha256: sha256_hex(package),
        package_bytes: package.len() as u64,
        package_format: sico_package::FORMAT_VERSION,
        manifest_schema: "sico.sapp.manifest.v0".to_owned(),
        language_semantics: "sico-semantics-v0".to_owned(),
        component_world: "sico-app-v0".to_owned(),
        wasi_contract: "wasi-preview2-v0".to_owned(),
        capability_contract: "sico-capability-v0".to_owned(),
        dependency_lock_sha256: audit(0xb1),
        minimum_host_contract: "sico-host-v0".to_owned(),
        description: "A literal <script>alert('inert')</script> description.".to_owned(),
        issued_at,
        expires_at: issued_at + DAY,
    }
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-step-0064-{name}-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn namespace_grant_transfer_tombstone_and_delayed_reuse_are_chained() {
    let authority = authority();
    let first = policy(1, "hello-sico", 900);
    let second = policy(11, "hello-sico", 900);
    let third = policy(21, "hello-sico", 900);
    let grant_bytes = signed_grant(&first, 1_000);
    let grant_event =
        verify_namespace_event_json(&grant_bytes, &authority, None, None, Some(&first), 1_001)
            .unwrap();

    let transfer = transfer_event(&first, &second, &grant_bytes);
    let transfer_bytes = sign_namespace_event_fixture(
        transfer,
        &[key(1), key(2), key(11), key(12), key(50), key(51)],
    )
    .unwrap();
    let transfer_event = verify_namespace_event_json(
        &transfer_bytes,
        &authority,
        Some((&grant_event, &sha256_hex(&grant_bytes))),
        Some(&first),
        Some(&second),
        2_001,
    )
    .unwrap();

    let tombstone = tombstone_event(&second, &transfer_bytes);
    let tombstone_bytes =
        sign_namespace_event_fixture(tombstone, &[key(11), key(12), key(50), key(51)]).unwrap();
    let tombstone_event = verify_namespace_event_json(
        &tombstone_bytes,
        &authority,
        Some((&transfer_event, &sha256_hex(&transfer_bytes))),
        Some(&second),
        None,
        3_001,
    )
    .unwrap();

    let mut reuse = grant(&third, 4_999);
    reuse.sequence = 4;
    reuse.previous_event_sha256 = Some(sha256_hex(&tombstone_bytes));
    let early =
        sign_namespace_event_fixture(reuse.clone(), &[key(21), key(22), key(50), key(51)]).unwrap();
    assert!(
        verify_namespace_event_json(
            &early,
            &authority,
            Some((&tombstone_event, &sha256_hex(&tombstone_bytes))),
            None,
            Some(&third),
            4_999,
        )
        .is_err()
    );

    reuse.issued_at = 5_000;
    reuse.expires_at = 5_000 + DAY;
    let reuse_bytes =
        sign_namespace_event_fixture(reuse, &[key(21), key(22), key(50), key(51)]).unwrap();
    assert!(
        verify_namespace_event_json(
            &reuse_bytes,
            &authority,
            Some((&tombstone_event, &sha256_hex(&tombstone_bytes))),
            None,
            Some(&third),
            5_001,
        )
        .is_ok()
    );
    assert!(
        verify_namespace_event_json(
            &reuse_bytes,
            &authority,
            Some((&tombstone_event, &audit(0xff))),
            None,
            Some(&third),
            5_001,
        )
        .is_err()
    );
}

#[test]
fn publish_discover_download_reverify_and_immutable_channel_history() {
    let root = temp_root("roundtrip");
    let registry = LocalRegistry::open(&root).unwrap();
    let policy = policy(1, "hello-sico", 900);
    let authority = authority();
    let namespace_bytes = signed_grant(&policy, 1_000);
    let package = package();
    let release = release(&policy, &namespace_bytes, &package, 1_100);
    let release_bytes = sign_release_record_fixture(release.clone(), &[key(3)]).unwrap();
    let digest = registry
        .publish_release(
            &namespace_bytes,
            &release_bytes,
            &package,
            &authority,
            &policy,
            1_101,
        )
        .unwrap();
    let channel = ChannelRecord {
        schema: CHANNEL_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        channel: "stable".to_owned(),
        sequence: 1,
        release_record_sha256: digest.clone(),
        issued_at: 1_200,
        expires_at: 1_200 + DAY,
    };
    let channel_bytes = sign_channel_record_fixture(channel.clone(), &[key(3)]).unwrap();
    println!(
        "REGISTRY_FIXTURE_METRICS package={} namespace={} release={} channel={}",
        package.len(),
        namespace_bytes.len(),
        release_bytes.len(),
        channel_bytes.len()
    );
    registry
        .publish_channel(&channel_bytes, &policy, 1_201)
        .unwrap();
    assert_eq!(
        registry
            .discover(&policy.identity, "stable", &policy, 1_201)
            .unwrap(),
        digest
    );
    let downloaded = registry
        .download(&digest, &namespace_bytes, &authority, &policy, 1_201)
        .unwrap();
    assert_eq!(downloaded.package_bytes, package);
    assert!(downloaded.record.description.contains("<script>"));

    assert_eq!(
        registry.publish_channel(&channel_bytes, &policy, 1_201),
        Err(RegistryError::Invalid("channel sequence"))
    );
    assert!(
        root.join("channels/example/hello-sico/stable/1.json")
            .is_file()
    );

    fs::write(
        root.join(format!("blobs/sha256/{}", release.package_sha256)),
        b"tampered",
    )
    .unwrap();
    assert_eq!(
        registry.download(&digest, &namespace_bytes, &authority, &policy, 1_201),
        Err(RegistryError::LengthMismatch)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn chained_checkpoints_are_append_only_and_support_more_than_two_generations() {
    let authority = authority();
    let first = RegistryCheckpoint {
        schema: CHECKPOINT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        sequence: 1,
        previous_checkpoint_sha256: None,
        namespace_event_digests: vec![audit(1)],
        release_record_digests: vec![audit(2)],
        channel_record_digests: vec![],
        issued_at: 1_000,
        expires_at: 1_500,
    };
    let first_bytes = sign_checkpoint_fixture(first, &[key(50), key(51)]).unwrap();
    verify_checkpoint_json(&first_bytes, &authority, None, 1_001).unwrap();

    let second = RegistryCheckpoint {
        schema: CHECKPOINT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        sequence: 2,
        previous_checkpoint_sha256: Some(sha256_hex(&first_bytes)),
        namespace_event_digests: vec![audit(1), audit(3)],
        release_record_digests: vec![audit(2)],
        channel_record_digests: vec![audit(4)],
        issued_at: 1_400,
        expires_at: 1_900,
    };
    let second_bytes = sign_checkpoint_fixture(second, &[key(50), key(51)]).unwrap();
    verify_checkpoint_json(&second_bytes, &authority, Some(&first_bytes), 1_401).unwrap();

    let third = RegistryCheckpoint {
        schema: CHECKPOINT_SCHEMA.to_owned(),
        registry_id: "registry.example".to_owned(),
        sequence: 3,
        previous_checkpoint_sha256: Some(sha256_hex(&second_bytes)),
        namespace_event_digests: vec![audit(1), audit(3)],
        release_record_digests: vec![audit(2), audit(5)],
        channel_record_digests: vec![audit(4)],
        issued_at: 2_000,
        expires_at: 2_500,
    };
    let third_bytes = sign_checkpoint_fixture(third.clone(), &[key(50), key(51)]).unwrap();
    verify_checkpoint_json(&third_bytes, &authority, Some(&second_bytes), 2_001).unwrap();

    let mut removed = third;
    removed.namespace_event_digests = vec![audit(3)];
    let removed_bytes = sign_checkpoint_fixture(removed, &[key(50), key(51)]).unwrap();
    assert!(
        verify_checkpoint_json(&removed_bytes, &authority, Some(&second_bytes), 2_001).is_err()
    );
}

#[test]
fn signatures_canonical_schema_identity_and_mutations_fail_closed() {
    let policy = policy(1, "hello-sico", 900);
    let namespace_bytes = signed_grant(&policy, 1_000);
    let package = package();
    let release = release(&policy, &namespace_bytes, &package, 1_100);
    let bytes = sign_release_record_fixture(release.clone(), &[key(3)]).unwrap();
    verify_release_record_json(&bytes, &policy, 1_101).unwrap();

    let mut rejected = 0;
    for index in 0..bytes.len() {
        let mut mutation = bytes.clone();
        mutation[index] ^= 1;
        if verify_release_record_json(&mutation, &policy, 1_101).is_err() {
            rejected += 1;
        }
    }
    assert_eq!(rejected, bytes.len());

    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["signed"]["unknown"] = serde_json::json!(true);
    let unknown = serde_json::to_vec(&value).unwrap();
    assert!(verify_release_record_json(&unknown, &policy, 1_101).is_err());

    let mut wrong_schema = release.clone();
    wrong_schema.schema = "sico.registry.release.v99".to_owned();
    let wrong_schema = sign_release_record_fixture(wrong_schema, &[key(3)]).unwrap();
    assert!(verify_release_record_json(&wrong_schema, &policy, 1_101).is_err());

    let mut wrong_identity = release;
    wrong_identity.identity.namespace = "attacker".to_owned();
    let wrong_identity = sign_release_record_fixture(wrong_identity, &[key(3)]).unwrap();
    assert!(verify_release_record_json(&wrong_identity, &policy, 1_101).is_err());
    println!(
        "REGISTRY_MUTATION_METRICS release_bytes={} rejected={rejected}",
        bytes.len()
    );
}

#[test]
fn semver_build_metadata_is_accepted_but_noncanonical_versions_are_rejected() {
    let policy = policy(1, "hello-sico", 900);
    let namespace_bytes = signed_grant(&policy, 1_000);
    let package = package();
    let mut record = release(&policy, &namespace_bytes, &package, 1_100);
    record.app_version = "1.2.3-alpha-beta.1+build.007".to_owned();
    let valid = sign_release_record_fixture(record.clone(), &[key(3)]).unwrap();
    verify_release_record_json(&valid, &policy, 1_101).unwrap();
    for invalid in ["v1.2.3", "1.2", "01.2.3", "1.2.3-01", "1.2.3+"] {
        record.app_version = invalid.to_owned();
        let bytes = sign_release_record_fixture(record.clone(), &[key(3)]).unwrap();
        assert!(verify_release_record_json(&bytes, &policy, 1_101).is_err());
    }
}

#[test]
fn registry_and_publisher_keys_must_be_separate() {
    let policy = policy(1, "hello-sico", 900);
    let mut overlapping = authority();
    overlapping.keys[0] = key_record(&key(1));
    overlapping
        .keys
        .sort_by(|left, right| left.key_id.cmp(&right.key_id));
    let bytes = signed_grant(&policy, 1_000);
    assert!(
        verify_namespace_event_json(&bytes, &overlapping, None, None, Some(&policy), 1_001,)
            .is_err()
    );

    let mut one_of_two = authority();
    one_of_two.threshold = 1;
    assert!(
        verify_namespace_event_json(&bytes, &one_of_two, None, None, Some(&policy), 1_001,)
            .is_err()
    );
}

#[test]
fn channel_record_requires_exact_release_role_and_current_policy() {
    let policy = policy(1, "hello-sico", 900);
    let record = ChannelRecord {
        schema: CHANNEL_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        channel: "stable".to_owned(),
        sequence: 1,
        release_record_sha256: audit(7),
        issued_at: 1_100,
        expires_at: 1_100 + DAY,
    };
    let wrong_role = sign_channel_record_fixture(record.clone(), &[key(1)]).unwrap();
    assert!(verify_channel_record_json(&wrong_role, &policy, 1_101).is_err());
    let valid = sign_channel_record_fixture(record, &[key(3)]).unwrap();
    verify_channel_record_json(&valid, &policy, 1_101).unwrap();
    assert!(verify_channel_record_json(&valid, &policy, policy.expires_at).is_err());
}

#[test]
fn policy_fixture_remains_structurally_valid_for_registry_use() {
    let policy = policy(1, "hello-sico", 900);
    let namespace = signed_grant(&policy, 1_000);
    let verified =
        verify_namespace_event_json(&namespace, &authority(), None, None, Some(&policy), 1_001)
            .unwrap();
    assert_eq!(
        verified.next_owner,
        Some(publisher_reference(&policy).unwrap())
    );
    assert_eq!(
        BTreeSet::from([verified.namespace]),
        BTreeSet::from(["example".to_owned()])
    );
}
