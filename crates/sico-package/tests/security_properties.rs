use std::collections::BTreeSet;

use sico_package::{
    BuildInput, ResourceInput, RuntimeLimits, TrustPolicy, build_unsigned, sign_development,
    verify, verify_trusted,
};
use wasm_encoder::Component;

#[test]
fn signed_single_byte_mutations_never_retain_validity() {
    let unsigned = package(vec![resource("asset.bin", b"stable")]);
    let signed = sign_development(&unsigned, &[7_u8; 32]).unwrap();
    for case in 0..2_048_usize {
        let mut mutated = signed.clone();
        let index = case.wrapping_mul(1_103) % mutated.len();
        mutated[index] ^= 1 << (case % 8);
        assert!(
            verify_trusted(&mutated, &TrustPolicy::AllowUnsignedDevelopment).is_err(),
            "signed mutation {case} unexpectedly retained validity"
        );
    }
}

#[test]
fn component_mutations_are_caught_by_structure_or_digest() {
    let package = package(Vec::new());
    let verified = verify(&package).unwrap();
    let start = package
        .windows(verified.component.len())
        .position(|window| window == verified.component)
        .unwrap();
    for case in 0..1_024_usize {
        let mut mutated = package.clone();
        let offset = case % verified.component.len();
        mutated[start + offset] ^= 1 << (case % 8);
        assert!(
            verify(&mutated).is_err(),
            "Component mutation {case} unexpectedly retained validity"
        );
    }
}

#[test]
fn resource_input_order_is_canonical_for_256_permutations() {
    let resources = vec![
        resource("a.txt", b"a"),
        resource("b.txt", b"bb"),
        resource("nested/c.txt", b"ccc"),
        resource("nested/d.txt", b"dddd"),
    ];
    let expected = package(resources.clone());
    for case in 0..256_usize {
        let mut permuted = resources.clone();
        permuted.rotate_left(case % resources.len());
        if case & 1 != 0 {
            permuted.reverse();
        }
        if case & 2 != 0 {
            permuted.swap(0, 2);
        }
        assert_eq!(package(permuted), expected, "permutation {case} drifted");
    }
}

fn package(resources: Vec<ResourceInput>) -> Vec<u8> {
    build_unsigned(BuildInput {
        app_id: "dev.sico.security-property".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: Component::new().finish(),
        resources,
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .unwrap()
}

fn resource(path: &str, bytes: &[u8]) -> ResourceInput {
    ResourceInput {
        path: path.to_owned(),
        bytes: bytes.to_vec(),
    }
}

#[test]
fn explicit_empty_host_key_set_does_not_trust_a_signed_package() {
    let unsigned = package(Vec::new());
    let signed = sign_development(&unsigned, &[9_u8; 32]).unwrap();
    assert!(verify_trusted(&signed, &TrustPolicy::RequireDevelopment(BTreeSet::new())).is_err());
}
