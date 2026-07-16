use std::collections::BTreeSet;

use ed25519_dalek::SigningKey;
use sico_ecosystem::{
    Custody, DisclosureIntake, DisclosurePolicy, IdentityRule, POLICY_SCHEMA, POLICY_UPDATE_SCHEMA,
    PolicyError, PolicyTransition, PresentedIdentity, ProductionIdentity, PublicKeyRecord,
    PublisherPolicy, RolePolicies, RolePolicy, TransitionKind, TransitionReason,
    TransparencyPolicy, canonical_policy_bytes, identity_is_authorized, policy_digest,
    public_key_id, sign_initial_policy_fixture, sign_policy_update_fixture,
    verify_initial_policy_json, verify_policy_update_json,
};

const DAY: u64 = 24 * 60 * 60;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn key_record(key: &SigningKey) -> PublicKeyRecord {
    PublicKeyRecord {
        key_id: public_key_id(key.verifying_key().as_bytes()),
        scheme: "ed25519".to_owned(),
        public_key: hex(key.verifying_key().as_bytes()),
    }
}

fn role(custody: Custody, threshold: u16, keys: &[SigningKey]) -> RolePolicy {
    let mut key_ids: Vec<_> = keys
        .iter()
        .map(|key| public_key_id(key.verifying_key().as_bytes()))
        .collect();
    key_ids.sort();
    RolePolicy {
        custody,
        threshold,
        key_ids,
    }
}

struct RoleKeys {
    root: Vec<SigningKey>,
    release: Vec<SigningKey>,
    recovery: Vec<SigningKey>,
    rotation: Vec<SigningKey>,
    revocation: Vec<SigningKey>,
}

impl RoleKeys {
    fn initial() -> Self {
        Self {
            root: vec![key(1), key(2)],
            release: vec![key(3)],
            recovery: vec![key(4), key(5)],
            rotation: vec![key(6), key(7)],
            revocation: vec![key(8), key(9)],
        }
    }

    fn all(&self) -> Vec<&SigningKey> {
        self.root
            .iter()
            .chain(&self.release)
            .chain(&self.recovery)
            .chain(&self.rotation)
            .chain(&self.revocation)
            .collect()
    }
}

fn policy(version: u64, issued_at: u64, role_keys: &RoleKeys) -> PublisherPolicy {
    let mut keys: Vec<_> = role_keys.all().into_iter().map(key_record).collect();
    keys.sort_by(|left, right| left.key_id.cmp(&right.key_id));
    let mut identity_rules = vec![
        IdentityRule::Key {
            key_id: public_key_id(role_keys.release[0].verifying_key().as_bytes()),
        },
        IdentityRule::Oidc {
            issuer: "https://token.actions.githubusercontent.com".to_owned(),
            subject: "repo:example/sico-app:ref:refs/heads/main".to_owned(),
            repository: "example/sico-app".to_owned(),
            workflow_ref: "example/sico-app/.github/workflows/release.yml@refs/heads/main"
                .to_owned(),
        },
    ];
    identity_rules.sort();
    PublisherPolicy {
        schema: POLICY_SCHEMA.to_owned(),
        version,
        issued_at,
        expires_at: issued_at + 30 * DAY,
        identity: ProductionIdentity {
            registry_id: "registry.example".to_owned(),
            namespace: "example".to_owned(),
            package_name: "hello-sico".to_owned(),
            publisher_policy_id: "example-policy-v0".to_owned(),
        },
        keys,
        roles: RolePolicies {
            root: role(Custody::Offline, 2, &role_keys.root),
            release: role(Custody::Online, 1, &role_keys.release),
            recovery: role(Custody::Offline, 2, &role_keys.recovery),
            rotation: role(Custody::Offline, 2, &role_keys.rotation),
            revocation: role(Custody::Offline, 2, &role_keys.revocation),
        },
        identity_rules,
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

fn transition(
    current: &PublisherPolicy,
    next: &PublisherPolicy,
    kind: TransitionKind,
    reason: TransitionReason,
) -> PolicyTransition {
    let current_ids: BTreeSet<_> = current.keys.iter().map(|key| key.key_id.clone()).collect();
    let next_ids: BTreeSet<_> = next.keys.iter().map(|key| key.key_id.clone()).collect();
    PolicyTransition {
        schema: POLICY_UPDATE_SCHEMA.to_owned(),
        kind,
        reason,
        previous_policy_sha256: policy_digest(current).unwrap(),
        next_policy_version: next.version,
        issued_at: next.issued_at,
        expires_at: next.issued_at + DAY,
        removed_key_ids: current_ids.difference(&next_ids).cloned().collect(),
    }
}

fn signing_set(seeds: &[u8]) -> Vec<SigningKey> {
    seeds.iter().copied().map(key).collect()
}

#[test]
fn initial_policy_and_exact_identity_claims_are_verified() {
    let roles = RoleKeys::initial();
    let expected = policy(1, 1_000, &roles);
    let bytes = sign_initial_policy_fixture(expected.clone(), &signing_set(&[1, 2])).unwrap();
    let verified = verify_initial_policy_json(&bytes, 1_001, &BTreeSet::new()).unwrap();
    assert_eq!(verified, expected);
    assert_eq!(
        canonical_policy_bytes(&verified).unwrap(),
        canonical_policy_bytes(&expected).unwrap()
    );

    let key_claim = PresentedIdentity::Key {
        key_id: public_key_id(key(3).verifying_key().as_bytes()),
    };
    assert!(identity_is_authorized(&verified, &key_claim).unwrap());
    let oidc = PresentedIdentity::Oidc {
        issuer: "https://token.actions.githubusercontent.com".to_owned(),
        subject: "repo:example/sico-app:ref:refs/heads/main".to_owned(),
        repository: "example/sico-app".to_owned(),
        workflow_ref: "example/sico-app/.github/workflows/release.yml@refs/heads/main".to_owned(),
    };
    assert!(identity_is_authorized(&verified, &oidc).unwrap());
    let wrong_workflow = PresentedIdentity::Oidc {
        issuer: "https://token.actions.githubusercontent.com".to_owned(),
        subject: "repo:example/sico-app:ref:refs/heads/main".to_owned(),
        repository: "example/sico-app".to_owned(),
        workflow_ref: "example/sico-app/.github/workflows/other.yml@refs/heads/main".to_owned(),
    };
    assert!(!identity_is_authorized(&verified, &wrong_workflow).unwrap());
}

#[test]
fn rotation_revocation_and_recovery_form_a_monotonic_chain() {
    let initial_roles = RoleKeys::initial();
    let initial = policy(1, 1_000, &initial_roles);
    let initial_bytes =
        sign_initial_policy_fixture(initial.clone(), &signing_set(&[1, 2])).unwrap();
    let current = verify_initial_policy_json(&initial_bytes, 1_001, &BTreeSet::new()).unwrap();

    let rotated_roles = RoleKeys {
        release: vec![key(10)],
        ..RoleKeys::initial()
    };
    let rotated = policy(2, 2_000, &rotated_roles);
    let rotate = transition(
        &current,
        &rotated,
        TransitionKind::RoutineRotation,
        TransitionReason::ScheduledRotation,
    );
    let rotate_bytes =
        sign_policy_update_fixture(rotate, rotated.clone(), &signing_set(&[1, 2, 6, 7])).unwrap();
    let current =
        verify_policy_update_json(&current, &rotate_bytes, 2_001, &BTreeSet::new()).unwrap();
    assert_eq!(current, rotated);

    let revoked_roles = RoleKeys {
        release: vec![key(11)],
        ..RoleKeys::initial()
    };
    let revoked = policy(3, 3_000, &revoked_roles);
    let revoke = transition(
        &current,
        &revoked,
        TransitionKind::EmergencyRevocation,
        TransitionReason::SuspectedCompromise,
    );
    let revoked_id = public_key_id(key(10).verifying_key().as_bytes());
    assert_eq!(revoke.removed_key_ids, vec![revoked_id.clone()]);
    let revoke_bytes =
        sign_policy_update_fixture(revoke, revoked.clone(), &signing_set(&[1, 2, 8, 9])).unwrap();
    let current =
        verify_policy_update_json(&current, &revoke_bytes, 3_001, &BTreeSet::new()).unwrap();
    assert!(current.keys.iter().all(|key| key.key_id != revoked_id));

    let recovered_roles = RoleKeys {
        root: vec![key(12), key(13)],
        release: vec![key(11)],
        recovery: vec![key(4), key(5)],
        rotation: vec![key(6), key(7)],
        revocation: vec![key(8), key(9)],
    };
    let recovered = policy(4, 4_000, &recovered_roles);
    let recover = transition(
        &current,
        &recovered,
        TransitionKind::Recovery,
        TransitionReason::CustodyLoss,
    );
    let recover_bytes = sign_policy_update_fixture(
        recover,
        recovered.clone(),
        &signing_set(&[1, 2, 4, 5, 12, 13]),
    )
    .unwrap();
    let current =
        verify_policy_update_json(&current, &recover_bytes, 4_001, &BTreeSet::new()).unwrap();
    assert_eq!(current, recovered);
    println!(
        "PUBLISHER_FIXTURE_METRICS initial={} rotation={} revocation={} recovery={}",
        initial_bytes.len(),
        rotate_bytes.len(),
        revoke_bytes.len(),
        recover_bytes.len()
    );
}

#[test]
fn development_keys_threshold_duplicates_and_time_fail_closed() {
    let roles = RoleKeys::initial();
    let policy = policy(1, 1_000, &roles);
    let bytes = sign_initial_policy_fixture(policy.clone(), &signing_set(&[1, 2])).unwrap();

    let development_keys = BTreeSet::from([*key(3).verifying_key().as_bytes()]);
    assert!(matches!(
        verify_initial_policy_json(&bytes, 1_001, &development_keys),
        Err(PolicyError::DevelopmentKeyReuse(_))
    ));
    assert_eq!(
        verify_initial_policy_json(&bytes, policy.expires_at, &BTreeSet::new()),
        Err(PolicyError::Expired)
    );
    assert_eq!(
        verify_initial_policy_json(&bytes, 999, &BTreeSet::new()),
        Err(PolicyError::NotYetValid)
    );

    let one_signature = sign_initial_policy_fixture(policy.clone(), &signing_set(&[1])).unwrap();
    assert_eq!(
        verify_initial_policy_json(&one_signature, 1_001, &BTreeSet::new()),
        Err(PolicyError::Threshold(sico_ecosystem::RoleKind::Root))
    );

    let mut envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let duplicate = envelope["signatures"][0].clone();
    envelope["signatures"]
        .as_array_mut()
        .unwrap()
        .insert(1, duplicate);
    let duplicate_bytes = serde_json::to_vec(&envelope).unwrap();
    assert!(verify_initial_policy_json(&duplicate_bytes, 1_001, &BTreeSet::new()).is_err());

    let mut spaced = vec![b' '];
    spaced.extend_from_slice(&bytes);
    assert_eq!(
        verify_initial_policy_json(&spaced, 1_001, &BTreeSet::new()),
        Err(PolicyError::NonCanonical)
    );
}

#[test]
fn transition_predecessor_roles_and_key_difference_are_enforced() {
    let roles = RoleKeys::initial();
    let current = policy(1, 1_000, &roles);
    let next_roles = RoleKeys {
        release: vec![key(10)],
        ..RoleKeys::initial()
    };
    let next = policy(2, 2_000, &next_roles);
    let mut change = transition(
        &current,
        &next,
        TransitionKind::RoutineRotation,
        TransitionReason::ScheduledRotation,
    );

    change.previous_policy_sha256 = "00".repeat(32);
    let bytes =
        sign_policy_update_fixture(change, next.clone(), &signing_set(&[1, 2, 6, 7])).unwrap();
    assert_eq!(
        verify_policy_update_json(&current, &bytes, 2_001, &BTreeSet::new()),
        Err(PolicyError::Transition("previous policy digest"))
    );

    let mut change = transition(
        &current,
        &next,
        TransitionKind::RoutineRotation,
        TransitionReason::ScheduledRotation,
    );
    change.removed_key_ids.clear();
    let bytes =
        sign_policy_update_fixture(change, next.clone(), &signing_set(&[1, 2, 6, 7])).unwrap();
    assert_eq!(
        verify_policy_update_json(&current, &bytes, 2_001, &BTreeSet::new()),
        Err(PolicyError::Transition("key-set difference"))
    );

    let change = transition(
        &current,
        &next,
        TransitionKind::RoutineRotation,
        TransitionReason::ScheduledRotation,
    );
    let missing_rotation = sign_policy_update_fixture(change, next, &signing_set(&[1, 2])).unwrap();
    assert_eq!(
        verify_policy_update_json(&current, &missing_rotation, 2_001, &BTreeSet::new()),
        Err(PolicyError::Threshold(sico_ecosystem::RoleKind::Rotation))
    );
}

#[test]
fn exact_claims_reject_wildcards_and_partial_matches() {
    let roles = RoleKeys::initial();
    let policy = policy(1, 1_000, &roles);
    let wildcard = PresentedIdentity::Oidc {
        issuer: "https://token.actions.githubusercontent.com".to_owned(),
        subject: "repo:example/*".to_owned(),
        repository: "example/sico-app".to_owned(),
        workflow_ref: "example/sico-app/.github/workflows/release.yml@refs/heads/main".to_owned(),
    };
    assert_eq!(
        identity_is_authorized(&policy, &wildcard),
        Err(PolicyError::Invalid("OIDC exact claim"))
    );
    let partial = PresentedIdentity::Oidc {
        issuer: "https://token.actions.githubusercontent.com".to_owned(),
        subject: "repo:example/sico-app".to_owned(),
        repository: "example/sico-app".to_owned(),
        workflow_ref: "release.yml".to_owned(),
    };
    assert!(!identity_is_authorized(&policy, &partial).unwrap());
}

#[test]
fn role_key_algorithm_disclosure_and_unknown_fields_are_strict() {
    let roles = RoleKeys::initial();
    let baseline = policy(1, 1_000, &roles);

    let mut bad = baseline.clone();
    bad.roles.recovery.threshold = 1;
    assert!(sign_initial_policy_fixture(bad, &signing_set(&[1, 2])).is_err());

    let mut bad = baseline.clone();
    bad.roles.rotation.custody = Custody::Online;
    assert!(sign_initial_policy_fixture(bad, &signing_set(&[1, 2])).is_err());

    let mut bad = baseline.clone();
    bad.roles.release.key_ids = bad.roles.root.key_ids.clone();
    assert!(sign_initial_policy_fixture(bad, &signing_set(&[1, 2])).is_err());

    let mut bad = baseline.clone();
    bad.keys[0].scheme = "unknown".to_owned();
    assert!(sign_initial_policy_fixture(bad, &signing_set(&[1, 2])).is_err());

    let mut bad = baseline.clone();
    bad.disclosure.redacted_fields.remove(2);
    assert!(sign_initial_policy_fixture(bad, &signing_set(&[1, 2])).is_err());

    let bytes = sign_initial_policy_fixture(baseline, &signing_set(&[1, 2])).unwrap();
    let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    unknown["signed"]["unexpected"] = serde_json::Value::Bool(true);
    let unknown = serde_json::to_vec(&unknown).unwrap();
    assert!(verify_initial_policy_json(&unknown, 1_001, &BTreeSet::new()).is_err());
}

#[test]
fn version_revocation_and_recovery_shape_cannot_be_weakened() {
    let roles = RoleKeys::initial();
    let current = policy(1, 1_000, &roles);
    let next_roles = RoleKeys {
        release: vec![key(10)],
        ..RoleKeys::initial()
    };

    let skipped = policy(3, 2_000, &next_roles);
    let skipped_transition = transition(
        &current,
        &skipped,
        TransitionKind::RoutineRotation,
        TransitionReason::ScheduledRotation,
    );
    let bytes =
        sign_policy_update_fixture(skipped_transition, skipped, &signing_set(&[1, 2, 6, 7]))
            .unwrap();
    assert_eq!(
        verify_policy_update_json(&current, &bytes, 2_001, &BTreeSet::new()),
        Err(PolicyError::Transition(
            "next version must increment by one"
        ))
    );

    let next = policy(2, 2_000, &next_roles);
    let mut retained = transition(
        &current,
        &next,
        TransitionKind::EmergencyRevocation,
        TransitionReason::SuspectedCompromise,
    );
    retained.removed_key_ids = vec![public_key_id(key(11).verifying_key().as_bytes())];
    let bytes =
        sign_policy_update_fixture(retained, next.clone(), &signing_set(&[1, 2, 8, 9])).unwrap();
    assert_eq!(
        verify_policy_update_json(&current, &bytes, 2_001, &BTreeSet::new()),
        Err(PolicyError::Transition("key-set difference"))
    );

    let no_root_recovery = transition(
        &current,
        &next,
        TransitionKind::Recovery,
        TransitionReason::CustodyLoss,
    );
    let bytes =
        sign_policy_update_fixture(no_root_recovery, next, &signing_set(&[1, 2, 4, 5])).unwrap();
    assert_eq!(
        verify_policy_update_json(&current, &bytes, 2_001, &BTreeSet::new()),
        Err(PolicyError::Transition("recovery must replace root keys"))
    );
}

#[test]
fn deterministic_initial_fixture_rejects_2048_signed_byte_mutations() {
    let roles = RoleKeys::initial();
    let policy = policy(1, 1_000, &roles);
    let first = sign_initial_policy_fixture(policy.clone(), &signing_set(&[1, 2])).unwrap();
    let second = sign_initial_policy_fixture(policy, &signing_set(&[1, 2])).unwrap();
    assert_eq!(first, second);
    for case in 0..2_048_usize {
        let mut mutated = first.clone();
        let index = case.wrapping_mul(7_919) % mutated.len();
        mutated[index] ^= 1 << (case % 7);
        assert!(
            verify_initial_policy_json(&mutated, 1_001, &BTreeSet::new()).is_err(),
            "mutation {case} unexpectedly verified"
        );
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
