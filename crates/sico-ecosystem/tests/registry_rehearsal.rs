//! M19 gate 4 rehearsal (M19 plan §3.3): publish → discover → download →
//! verify against a local registry origin — the deployable configuration's
//! end-to-end proof with public deployment still owner-gated. Reuses the
//! STEP-0064 fixture-signing helpers' exact patterns.

use std::{fs, path::PathBuf};

use ed25519_dalek::SigningKey;
use sico_ecosystem::{
    Custody, DisclosureIntake, DisclosurePolicy, IdentityRule, LocalRegistry, NAMESPACE_SCHEMA,
    NamespaceEvent, NamespaceEventKind, POLICY_SCHEMA, ProductionIdentity, PublicKeyRecord,
    PublisherPolicy, RELEASE_SCHEMA, RegistryAuthority, ReleaseRecord, RolePolicies, RolePolicy,
    TransparencyPolicy, policy_digest, public_key_id, publisher_reference,
    sign_namespace_event_fixture, sign_release_record_fixture,
};
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sha256_hex};

const DAY: u64 = 24 * 60 * 60;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        use std::fmt::Write as _;
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

fn policy(issued_at: u64) -> PublisherPolicy {
    let base = 10_u8;
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
            package_name: "rehearsal".to_owned(),
            publisher_policy_id: "rehearsal-1".to_owned(),
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

fn signed_grant(policy: &PublisherPolicy, issued_at: u64) -> Vec<u8> {
    let event = NamespaceEvent {
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
    };
    sign_namespace_event_fixture(event, &[key(10), key(11), key(50), key(51)]).unwrap()
}

fn package() -> Vec<u8> {
    let unsigned = build_unsigned(BuildInput {
        app_id: "example.rehearsal".to_owned(),
        app_version: "1.0.0".to_owned(),
        component: wasm_encoder::Component::new().finish(),
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .unwrap();
    sico_package::sign_development(&unsigned, &[9_u8; 32]).unwrap()
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
        app_id: "example.rehearsal".to_owned(),
        app_version: "1.0.0".to_owned(),
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
        description: "M19 registry rehearsal".to_owned(),
        issued_at,
        expires_at: issued_at + DAY,
    }
}

#[test]
fn registry_rehearsal_publish_discover_download_verify() {
    let now = 1_000_u64;
    let authority = authority();
    let policy = policy(now);
    let namespace = signed_grant(&policy, now);
    let package = package();
    let record = release(&policy, &namespace, &package, now);
    let release_bytes = sign_release_record_fixture(record, &[key(12)]).unwrap();

    let root = std::env::temp_dir().join(format!("sico-m19-registry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let registry = LocalRegistry::open(&root).expect("origin opens");

    // Publish: every namespace/publisher/package check must pass.
    let release_digest = registry
        .publish_release(
            &namespace,
            &release_bytes,
            &package,
            &authority,
            &policy,
            now,
        )
        .expect("publish accepted");

    // Channel publish (the "stable" channel points at the release), then
    // discover: channel resolution returns the immutable release digest.
    let channel = sico_ecosystem::ChannelRecord {
        schema: sico_ecosystem::CHANNEL_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        channel: "stable".to_owned(),
        sequence: 1,
        release_record_sha256: release_digest.clone(),
        issued_at: now,
        expires_at: now + DAY,
    };
    registry
        .publish_channel(
            &sico_ecosystem::sign_channel_record_fixture(channel, &[key(12)]).unwrap(),
            &policy,
            now,
        )
        .expect("channel published");
    let discovered = registry
        .discover(&policy.identity, "stable", &policy, now)
        .expect("channel resolves");
    assert_eq!(discovered, release_digest);

    // Download by digest: transport re-verification.
    let downloaded = registry
        .download(&release_digest, &namespace, &authority, &policy, now)
        .expect("download verifies");

    // The downloaded bytes are the published package, digest-equal.
    assert_eq!(downloaded.record.package_sha256, sha256_hex(&package));
    assert_eq!(downloaded.package_bytes.len(), package.len());

    // Package-level verification (the consumer-side gate before authorize).
    let verified = sico_package::verify(&downloaded.package_bytes).expect("sapp verifies");
    assert_eq!(verified.manifest.app.id, "example.rehearsal");

    // Immutability: a conflicting re-publish of the same digest is refused.
    let mut tampered = package.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xFF;
    assert!(
        registry
            .publish_release(
                &namespace,
                &release_bytes,
                &tampered,
                &authority,
                &policy,
                now
            )
            .is_err()
    );

    let _ = PathBuf::from("unused");
    // Evidence: record the rehearsal shape.
    let evidence = std::path::Path::new("target/evidence/m19/registry-rehearsal.json");
    fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    fs::write(
        evidence,
        format!(
            "{{\"release_digest\":\"{release_digest}\",\"package_bytes\":{},\"steps\":[\"publish\",\"discover\",\"download\",\"verify\",\"immutability\"]}}",
            package.len()
        ),
    )
    .unwrap();
}
