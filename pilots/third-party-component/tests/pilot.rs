use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use sico_ai_tools::{REQUEST_SCHEMA, execute_value};
use sico_codegen_wasm::compile_component;
use sico_ecosystem::{
    CHANNEL_SCHEMA, CHECKPOINT_SCHEMA, ChannelRecord, Custody, DisclosureIntake, DisclosurePolicy,
    IdentityRule, LocalRegistry, NAMESPACE_SCHEMA, NamespaceEvent, NamespaceEventKind,
    POLICY_SCHEMA, ProductionIdentity, PublicKeyRecord, PublisherPolicy, RELEASE_SCHEMA,
    RegistryAuthority, RegistryCheckpoint, RegistryError, ReleaseRecord, RolePolicies, RolePolicy,
    TransparencyPolicy, policy_digest, public_key_id, publisher_reference,
    sign_channel_record_fixture, sign_checkpoint_fixture, sign_namespace_event_fixture,
    sign_release_record_fixture, verify_checkpoint_json,
};
use sico_host_core::{HostError, HostStore, InstallOptions, UiModel};
use sico_ir::lower_core;
use sico_language_server::LanguageServer;
use sico_package::{BuildInput, RuntimeLimits, build_unsigned, sha256_hex, sign_development};
use sico_runtime::{HostLimits, run_authorized_package};
use sico_source::{SourceFile, SourceId};
use sico_third_party_pilot::PILOT_SCHEMA;

const DAY: u64 = 24 * 60 * 60;
const APP_ID: &str = "dev.sico.third-party-pilot";
const PACKAGE_NAME: &str = "third-party-pilot";
const DEV_SEED: [u8; 32] = [90; 32];
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct ReleaseFixture {
    version: &'static str,
    package: Vec<u8>,
    release: Vec<u8>,
    release_digest: String,
    channel: Vec<u8>,
}

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
    let mut key_ids = seeds
        .iter()
        .map(|seed| public_key_id(key(*seed).verifying_key().as_bytes()))
        .collect::<Vec<_>>();
    key_ids.sort();
    RolePolicy {
        custody,
        threshold,
        key_ids,
    }
}

fn policy() -> PublisherPolicy {
    let mut keys = (1..=9)
        .map(|seed| key_record(&key(seed)))
        .collect::<Vec<_>>();
    keys.sort_by(|left, right| left.key_id.cmp(&right.key_id));
    PublisherPolicy {
        schema: POLICY_SCHEMA.to_owned(),
        version: 1,
        issued_at: 900,
        expires_at: 900 + 30 * DAY,
        identity: ProductionIdentity {
            registry_id: "registry.pilot.invalid".to_owned(),
            namespace: "clean-room".to_owned(),
            package_name: PACKAGE_NAME.to_owned(),
            publisher_policy_id: "pilot-fixture-policy".to_owned(),
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
        registry_id: "registry.pilot.invalid".to_owned(),
        keys,
        threshold: 2,
    }
}

fn namespace(policy: &PublisherPolicy) -> Vec<u8> {
    let event = NamespaceEvent {
        schema: NAMESPACE_SCHEMA.to_owned(),
        registry_id: policy.identity.registry_id.clone(),
        namespace: policy.identity.namespace.clone(),
        sequence: 1,
        issued_at: 1_000,
        expires_at: 1_000 + DAY,
        kind: NamespaceEventKind::Grant,
        previous_event_sha256: None,
        previous_owner: None,
        next_owner: Some(publisher_reference(policy).unwrap()),
        reuse_after: None,
        audit_ref: "a1".repeat(32),
    };
    sign_namespace_event_fixture(event, &[key(1), key(2), key(50), key(51)]).unwrap()
}

fn build_package(source: &str, version: &str) -> Vec<u8> {
    let source = SourceFile::from_text(SourceId::new(1), "pilot.sico", source).unwrap();
    let component = compile_component(&lower_core(&source).unwrap()).unwrap();
    let unsigned = build_unsigned(BuildInput {
        app_id: APP_ID.to_owned(),
        app_version: version.to_owned(),
        component,
        resources: Vec::new(),
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .unwrap();
    sign_development(&unsigned, &DEV_SEED).unwrap()
}

fn release_fixture(
    policy: &PublisherPolicy,
    namespace: &[u8],
    source: &str,
    version: &'static str,
    sequence: u64,
) -> ReleaseFixture {
    let package = build_package(source, version);
    let issued_at = 1_100 + sequence * 100;
    let record = ReleaseRecord {
        schema: RELEASE_SCHEMA.to_owned(),
        identity: policy.identity.clone(),
        publisher_policy_version: policy.version,
        publisher_policy_sha256: policy_digest(policy).unwrap(),
        namespace_event_sha256: sha256_hex(namespace),
        app_id: APP_ID.to_owned(),
        app_version: version.to_owned(),
        package_sha256: sha256_hex(&package),
        package_bytes: u64::try_from(package.len()).unwrap(),
        package_format: sico_package::FORMAT_VERSION,
        manifest_schema: "sico.sapp.manifest.v0".to_owned(),
        language_semantics: "sico-semantics-v0".to_owned(),
        component_world: "sico-app-v0".to_owned(),
        wasi_contract: "wasi-preview2-v0".to_owned(),
        capability_contract: "sico-capability-v0".to_owned(),
        dependency_lock_sha256: "d1".repeat(32),
        minimum_host_contract: "sico-host-v0".to_owned(),
        description: format!("Clean-room pilot release {version}"),
        issued_at,
        expires_at: issued_at + DAY,
    };
    let release = sign_release_record_fixture(record, &[key(3)]).unwrap();
    let release_digest = sha256_hex(&release);
    let channel = sign_channel_record_fixture(
        ChannelRecord {
            schema: CHANNEL_SCHEMA.to_owned(),
            identity: policy.identity.clone(),
            channel: "stable".to_owned(),
            sequence,
            release_record_sha256: release_digest.clone(),
            issued_at: issued_at + 10,
            expires_at: issued_at + DAY,
        },
        &[key(3)],
    )
    .unwrap();
    ReleaseFixture {
        version,
        package,
        release,
        release_digest,
        channel,
    }
}

fn lsp_message(id: u64, method: &str, params: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

fn ai_inspect(source: &str) -> Value {
    execute_value(json!({
        "schema": REQUEST_SCHEMA,
        "protocol_version": 0,
        "request_id": "pilot-inspect",
        "operation": "inspect",
        "budget": {
            "max_files": 1,
            "max_input_bytes": 65536,
            "max_symbols": 64,
            "max_diagnostics": 32,
            "max_response_bytes": 65536
        },
        "input": { "files": [{ "uri": "file:///pilot/app.sico", "text": source }] }
    }))
}

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-third-party-pilot-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn public_editor_ai_and_ui_contracts_accept_the_clean_room_assets() {
    let source = include_str!("../app-v1.sico");
    let mut server = LanguageServer::default();
    let initialized = server.process(lsp_message(1, "initialize", &json!({})));
    assert_eq!(
        initialized[0]["result"]["capabilities"]["positionEncoding"],
        "utf-16"
    );
    let opened = server.process(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": { "textDocument": {
            "uri": "file:///pilot/app.sico", "languageId": "sico", "version": 1, "text": source
        }}
    }));
    assert_eq!(opened[0]["params"]["diagnostics"], json!([]));
    let symbols = server.process(lsp_message(
        2,
        "textDocument/documentSymbol",
        &json!({ "textDocument": { "uri": "file:///pilot/app.sico" } }),
    ));
    assert_eq!(symbols[0]["result"].as_array().unwrap().len(), 1);

    let inspection = ai_inspect(source);
    assert_eq!(inspection["ok"], true);
    assert_eq!(inspection["result"]["diagnostics"], json!([]));
    assert_eq!(inspection["result"]["symbols"].as_array().unwrap().len(), 1);

    let ui: UiModel = serde_json::from_str(include_str!("../ui.json")).unwrap();
    let plan = ui.validate().unwrap();
    assert_eq!(plan.nodes.len(), 3);
}

#[test]
#[allow(clippy::too_many_lines)]
fn signed_release_drill_closes_registry_host_runtime_and_security_boundaries() {
    let started = Instant::now();
    let policy = policy();
    let authority = authority();
    let namespace = namespace(&policy);
    let build_started = Instant::now();
    let first = release_fixture(
        &policy,
        &namespace,
        include_str!("../app-v1.sico"),
        "1.0.0",
        1,
    );
    let second = release_fixture(
        &policy,
        &namespace,
        include_str!("../app-v2.sico"),
        "1.1.0",
        2,
    );
    let build_ms = build_started.elapsed().as_millis();
    assert_ne!(first.package, second.package);

    let root = temp_root();
    let registry_started = Instant::now();
    let registry = LocalRegistry::open(&root.join("registry")).unwrap();
    let first_digest = registry
        .publish_release(
            &namespace,
            &first.release,
            &first.package,
            &authority,
            &policy,
            1_201,
        )
        .unwrap();
    assert_eq!(first_digest, first.release_digest);
    registry
        .publish_channel(&first.channel, &policy, 1_211)
        .unwrap();
    let second_digest = registry
        .publish_release(
            &namespace,
            &second.release,
            &second.package,
            &authority,
            &policy,
            1_301,
        )
        .unwrap();
    registry
        .publish_channel(&second.channel, &policy, 1_311)
        .unwrap();
    assert_eq!(
        registry
            .discover(&policy.identity, "stable", &policy, 1_312)
            .unwrap(),
        second_digest
    );
    let downloaded = registry
        .download(&second_digest, &namespace, &authority, &policy, 1_312)
        .unwrap();
    assert_eq!(downloaded.package_bytes, second.package);

    let first_checkpoint = sign_checkpoint_fixture(
        RegistryCheckpoint {
            schema: CHECKPOINT_SCHEMA.to_owned(),
            registry_id: authority.registry_id.clone(),
            sequence: 1,
            previous_checkpoint_sha256: None,
            namespace_event_digests: vec![sha256_hex(&namespace)],
            release_record_digests: vec![first_digest.clone()],
            channel_record_digests: vec![sha256_hex(&first.channel)],
            issued_at: 1_220,
            expires_at: 1_220 + DAY,
        },
        &[key(50), key(51)],
    )
    .unwrap();
    verify_checkpoint_json(&first_checkpoint, &authority, None, 1_221).unwrap();
    let mut releases = vec![first_digest, second_digest];
    releases.sort();
    let mut channels = vec![sha256_hex(&first.channel), sha256_hex(&second.channel)];
    channels.sort();
    let second_checkpoint = sign_checkpoint_fixture(
        RegistryCheckpoint {
            schema: CHECKPOINT_SCHEMA.to_owned(),
            registry_id: authority.registry_id.clone(),
            sequence: 2,
            previous_checkpoint_sha256: Some(sha256_hex(&first_checkpoint)),
            namespace_event_digests: vec![sha256_hex(&namespace)],
            release_record_digests: releases,
            channel_record_digests: channels,
            issued_at: 1_320,
            expires_at: 1_320 + DAY,
        },
        &[key(50), key(51)],
    )
    .unwrap();
    verify_checkpoint_json(
        &second_checkpoint,
        &authority,
        Some(&first_checkpoint),
        1_321,
    )
    .unwrap();
    let registry_ms = registry_started.elapsed().as_millis();

    let host_started = Instant::now();
    let store = HostStore::open(&root.join("host")).unwrap();
    let trusted = BTreeSet::from([*SigningKey::from_bytes(&DEV_SEED).verifying_key().as_bytes()]);
    let first_installed = store
        .install_bytes(&first.package, &trusted, InstallOptions::default())
        .unwrap();
    let second_installed = store
        .install_bytes(
            &downloaded.package_bytes,
            &trusted,
            InstallOptions::default(),
        )
        .unwrap();
    assert_eq!(first_installed.app_identity, second_installed.app_identity);
    assert_eq!(second_installed.app_version, second.version);
    let opened = store
        .open_installed(
            &second_installed.app_identity,
            &second_installed.revision_digest,
            &trusted,
            &BTreeSet::new(),
        )
        .unwrap();

    let mut security_rejections = 0;
    let mut tampered = downloaded.package_bytes.clone();
    *tampered.last_mut().unwrap() ^= 1;
    security_rejections += usize::from(
        store
            .install_bytes(&tampered, &trusted, InstallOptions::default())
            .is_err(),
    );
    security_rejections += usize::from(matches!(
        store.install_bytes(&first.package, &trusted, InstallOptions::default()),
        Err(HostError::Downgrade { .. })
    ));
    security_rejections += usize::from(
        registry
            .publish_channel(&first.channel, &policy, 1_400)
            .is_err(),
    );
    let mut bad_checkpoint = second_checkpoint.clone();
    *bad_checkpoint.last_mut().unwrap() ^= 1;
    security_rejections += usize::from(
        verify_checkpoint_json(&bad_checkpoint, &authority, Some(&first_checkpoint), 1_321)
            .is_err(),
    );
    assert_eq!(security_rejections, 4);
    assert_eq!(
        registry.publish_channel(&second.channel, &policy, 1_400),
        Err(RegistryError::Invalid("channel sequence"))
    );
    let host_ms = host_started.elapsed().as_millis();

    let runtime_started = Instant::now();
    let runtime_verified = if let Some(runtime) = std::env::var_os("SICO_TEST_WASMTIME") {
        let output =
            run_authorized_package(&runtime, &opened.package, None, &HostLimits::default())
                .unwrap();
        assert!(output.status.success(), "{:?}", output.fault);
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "42");
        true
    } else {
        false
    };
    let runtime_ms = runtime_started.elapsed().as_millis();
    println!(
        "PILOT_RELEASE_METRICS releases=2 result=42 runtime={runtime_verified} security_rejections={security_rejections} build_ms={build_ms} registry_ms={registry_ms} host_ms={host_ms} runtime_ms={runtime_ms} total_ms={}",
        started.elapsed().as_millis()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pilot_manifest_never_upgrades_local_fixtures_to_external_evidence() {
    let manifest: Value = serde_json::from_str(include_str!("../pilot.json")).unwrap();
    assert_eq!(manifest["schema"], PILOT_SCHEMA);
    assert_eq!(manifest["evidence_level"], "repository-authored-clean-room");
    assert_eq!(manifest["expected_result"], 42);
    assert_eq!(manifest["local_stages"].as_array().unwrap().len(), 11);
    assert!(
        manifest["external_evidence"]
            .as_object()
            .unwrap()
            .values()
            .all(|status| status != "complete")
    );
}

#[test]
fn public_assets_are_standalone_and_do_not_reference_repository_examples() {
    for source in [
        include_str!("../app-v1.sico"),
        include_str!("../app-v2.sico"),
    ] {
        assert!(!source.contains("examples/"));
        assert!(source.contains("function main() returns Int:"));
    }
    let readme = include_str!("../README.md");
    assert!(readme.contains("must not be relabeled as third-party evidence"));
}
