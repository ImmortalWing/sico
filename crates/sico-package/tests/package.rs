use std::collections::BTreeSet;

use ed25519_dalek::SigningKey;
use sico_codegen_wasm::compile_component;
use sico_ir::lower_core;
use sico_package::{
    BuildInput, PackageError, ResourceInput, RuntimeLimits, TrustPolicy, TrustStatus, authorize,
    build_unsigned, capabilities_for_imports, sign_development, verify, verify_trusted,
};
use sico_source::{SourceFile, SourceId};
use wasm_encoder::{
    Component, ComponentExportKind, ComponentExportSection, ComponentImportSection,
    ComponentTypeRef, ComponentValType, PrimitiveValType,
};

fn component() -> Vec<u8> {
    let source = SourceFile::from_text(
        SourceId::new(1),
        "answer.sico",
        "function main() returns Int:\n  return 42\nend function\n",
    )
    .unwrap();
    compile_component(&lower_core(&source).unwrap()).unwrap()
}

fn input() -> BuildInput {
    BuildInput {
        app_id: "dev.sico.answer".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: component(),
        resources: vec![ResourceInput {
            path: "messages/hello.txt".to_owned(),
            bytes: b"hello\n".to_vec(),
        }],
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    }
}

#[test]
fn build_is_byte_identical_and_verified() {
    let first = build_unsigned(input()).unwrap();
    let second = build_unsigned(input()).unwrap();
    assert_eq!(first, second);
    let package = verify(&first).unwrap();
    assert_eq!(package.manifest.app.id, "dev.sico.answer");
    assert_eq!(package.component_imports, Vec::<String>::new());
    assert_eq!(
        package.resources["resources/messages/hello.txt"],
        b"hello\n"
    );
    assert!(package.signature.is_none());
    assert_eq!(package.unsigned_bytes(), first);
}

#[test]
fn tamper_truncation_trailing_and_version_are_rejected() {
    let package = build_unsigned(input()).unwrap();
    let mut tampered = package.clone();
    *tampered.last_mut().unwrap() ^= 1;
    assert!(matches!(
        verify(&tampered),
        Err(PackageError::DigestMismatch(_))
    ));
    assert!(matches!(
        verify(&package[..package.len() - 1]),
        Err(PackageError::Truncated)
    ));
    let mut trailing = package.clone();
    trailing.push(0);
    assert_eq!(verify(&trailing).unwrap_err(), PackageError::TrailingBytes);
    let mut version = package;
    version[8..10].copy_from_slice(&2_u16.to_le_bytes());
    assert_eq!(
        verify(&version).unwrap_err(),
        PackageError::UnsupportedVersion(2)
    );
}

#[test]
fn resource_paths_and_sizes_are_bounded() {
    for path in ["../secret", "/absolute", "a\\b", "CON", "a:stream"] {
        let mut build = input();
        build.resources[0].path = path.to_owned();
        assert!(matches!(
            build_unsigned(build),
            Err(PackageError::InvalidPath(_))
        ));
    }
    let mut duplicate = input();
    duplicate.resources.push(ResourceInput {
        path: "MESSAGES/HELLO.TXT".to_owned(),
        bytes: vec![1],
    });
    assert!(matches!(
        build_unsigned(duplicate),
        Err(PackageError::DuplicatePath(_))
    ));
}

#[test]
fn unknown_manifest_field_is_rejected_before_component() {
    let package = build_unsigned(input()).unwrap();
    let manifest_length =
        usize::try_from(u64::from_le_bytes(package[16..24].try_into().unwrap())).unwrap();
    let manifest_start = 24 + "manifest.json".len();
    let manifest_end = manifest_start + manifest_length;
    let manifest = std::str::from_utf8(&package[manifest_start..manifest_end]).unwrap();
    let mutated = manifest.replacen('{', "{\"unknown\":true,", 1);
    let mut forged = package[..16].to_vec();
    forged.extend_from_slice(&(mutated.len() as u64).to_le_bytes());
    forged.extend_from_slice(b"manifest.json");
    forged.extend_from_slice(mutated.as_bytes());
    forged.extend_from_slice(&package[manifest_end..]);
    assert!(matches!(verify(&forged), Err(PackageError::Manifest(_))));
}

#[test]
fn development_signatures_are_deterministic_and_policy_checked() {
    let unsigned = build_unsigned(input()).unwrap();
    let seed = [7_u8; 32];
    let signed = sign_development(&unsigned, &seed).unwrap();
    assert_eq!(signed, sign_development(&unsigned, &seed).unwrap());

    let public_key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    let trusted = verify_trusted(
        &signed,
        &TrustPolicy::RequireDevelopment(BTreeSet::from([public_key])),
    )
    .unwrap();
    assert!(matches!(trusted.trust, TrustStatus::Development { .. }));

    let other_key = SigningKey::from_bytes(&[8_u8; 32])
        .verifying_key()
        .to_bytes();
    assert_eq!(
        verify_trusted(
            &signed,
            &TrustPolicy::RequireDevelopment(BTreeSet::from([other_key]))
        )
        .unwrap_err(),
        PackageError::UntrustedKey
    );
    assert_eq!(
        verify_trusted(
            &unsigned,
            &TrustPolicy::RequireDevelopment(BTreeSet::from([public_key]))
        )
        .unwrap_err(),
        PackageError::MissingSignature
    );
    assert_eq!(
        verify_trusted(&unsigned, &TrustPolicy::AllowUnsignedDevelopment)
            .unwrap()
            .trust,
        TrustStatus::UnsignedDevelopment
    );
}

#[test]
fn signature_replay_across_version_is_rejected() {
    let seed = [11_u8; 32];
    let first = sign_development(&build_unsigned(input()).unwrap(), &seed).unwrap();
    let mut second_input = input();
    second_input.app_version = "0.1.1".to_owned();
    let second = sign_development(&build_unsigned(second_input).unwrap(), &seed).unwrap();
    let marker = b"{\"schema\":\"sico.sapp.signature.v0\"";
    let first_start = find(&first, marker);
    let second_start = find(&second, marker);
    assert_eq!(first.len() - first_start, second.len() - second_start);
    let mut replayed = first;
    replayed[first_start..].copy_from_slice(&second[second_start..]);

    let public_key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    assert!(matches!(
        verify_trusted(
            &replayed,
            &TrustPolicy::RequireDevelopment(BTreeSet::from([public_key]))
        ),
        Err(PackageError::InvalidSignature(_))
    ));
}

fn find(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap()
}

#[test]
fn capability_closure_and_host_intersection_are_enforced() {
    let import = "wasi:clocks/monotonic-clock@0.2.0";
    let mut build = input();
    build.component = importing_component(import);
    build.source_effects = vec!["clock.read".to_owned()];
    build.capabilities = vec!["clock.read".to_owned()];
    let seed = [19_u8; 32];
    let public_key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    let signed = sign_development(&build_unsigned(build).unwrap(), &seed).unwrap();
    let trusted = verify_trusted(
        &signed,
        &TrustPolicy::RequireDevelopment(BTreeSet::from([public_key])),
    )
    .unwrap();
    assert_eq!(trusted.package.component_imports, vec![import]);
    assert_eq!(
        authorize(trusted.clone(), &BTreeSet::new()).unwrap_err(),
        PackageError::HostDenied(vec!["clock.read".to_owned()])
    );
    let grants = BTreeSet::from(["clock.read".to_owned(), "random.read".to_owned()]);
    let authorized = authorize(trusted, &grants).unwrap();
    assert_eq!(
        authorized.granted_capabilities,
        BTreeSet::from(["clock.read".to_owned()])
    );
}

#[test]
fn capability_mismatch_and_unknown_import_are_default_denied() {
    assert!(matches!(
        capabilities_for_imports(&["evil:ambient/root@1.0.0".to_owned()]),
        Err(PackageError::UnknownCapability(_))
    ));

    let seed = [23_u8; 32];
    let public_key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    let mut mismatch = input();
    mismatch.source_effects = vec!["clock.read".to_owned()];
    mismatch.capabilities = vec!["clock.read".to_owned()];
    let signed = sign_development(&build_unsigned(mismatch).unwrap(), &seed).unwrap();
    let trusted = verify_trusted(
        &signed,
        &TrustPolicy::RequireDevelopment(BTreeSet::from([public_key])),
    )
    .unwrap();
    assert_eq!(
        authorize(trusted, &BTreeSet::from(["clock.read".to_owned()])).unwrap_err(),
        PackageError::ImportManifestMismatch
    );
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
