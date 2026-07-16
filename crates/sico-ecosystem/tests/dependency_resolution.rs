use std::collections::BTreeSet;

use sico_ecosystem::{
    CompatibilityProfile, DependencyError, DependencyIdentity, DependencyRequirement,
    HostCompatibility, LOCK_GRAPH_SCHEMA, PackageCandidate, RevocationPolicy,
    STANDARD_LIBRARY_SCHEMA, StandardLibraryContract, StandardLibraryModule, canonical_lock_bytes,
    dependency_lock_digest, resolve_dependencies, standard_library_digest, verify_dependency_lock,
    verify_dependency_lock_json,
};

fn digest(seed: u8) -> String {
    format!("{seed:02x}").repeat(32)
}

fn identity(registry: &str, package: &str) -> DependencyIdentity {
    DependencyIdentity {
        registry_id: registry.to_owned(),
        namespace: "example".to_owned(),
        package_name: package.to_owned(),
    }
}

fn profile() -> CompatibilityProfile {
    CompatibilityProfile {
        language_semantics: "sico-semantics-v0".to_owned(),
        compiler_version: "sico-compiler-v0".to_owned(),
        ir_schema: "sico-ir-v0".to_owned(),
        component_world: "sico-app-v0".to_owned(),
        wasi_contract: "wasi-preview2-v0".to_owned(),
        package_format: 1,
        manifest_schema: "sico-sapp-manifest-v0".to_owned(),
        publisher_policy_schema: "sico-publisher-policy-v0".to_owned(),
        release_schema: "sico-registry-release-v0".to_owned(),
        capability_contract: "sico-capability-v0".to_owned(),
        host_contract: "sico-host-v0".to_owned(),
    }
}

fn host() -> HostCompatibility {
    let profile = profile();
    HostCompatibility {
        language_semantics: BTreeSet::from([profile.language_semantics]),
        compiler_versions: BTreeSet::from([profile.compiler_version]),
        ir_schemas: BTreeSet::from([profile.ir_schema]),
        component_worlds: BTreeSet::from([profile.component_world]),
        wasi_contracts: BTreeSet::from([profile.wasi_contract]),
        package_formats: BTreeSet::from([profile.package_format]),
        manifest_schemas: BTreeSet::from([profile.manifest_schema]),
        publisher_policy_schemas: BTreeSet::from([profile.publisher_policy_schema]),
        release_schemas: BTreeSet::from([profile.release_schema]),
        capability_contracts: BTreeSet::from([profile.capability_contract]),
        host_contracts: BTreeSet::from([profile.host_contract]),
    }
}

fn standard_library() -> StandardLibraryContract {
    StandardLibraryContract {
        schema: STANDARD_LIBRARY_SCHEMA.to_owned(),
        version: "0.1.0".to_owned(),
        language_semantics: "sico-semantics-v0".to_owned(),
        component_world: "sico-app-v0".to_owned(),
        wasi_contract: "wasi-preview2-v0".to_owned(),
        capability_contract: "sico-capability-v0".to_owned(),
        modules: vec![
            StandardLibraryModule {
                name: "collections".to_owned(),
                api_sha256: digest(0x10),
            },
            StandardLibraryModule {
                name: "text".to_owned(),
                api_sha256: digest(0x11),
            },
        ],
    }
}

fn requirement(registry: &str, package: &str, version: &str) -> DependencyRequirement {
    DependencyRequirement {
        identity: identity(registry, package),
        version_requirement: version.to_owned(),
    }
}

fn candidate(
    registry: &str,
    package: &str,
    version: &str,
    seed: u8,
    dependencies: Vec<DependencyRequirement>,
    capabilities: &[&str],
) -> PackageCandidate {
    PackageCandidate {
        identity: identity(registry, package),
        publisher_policy_id: "example-policy-v0".to_owned(),
        version: version.to_owned(),
        release_record_sha256: digest(seed),
        package_sha256: digest(seed.saturating_add(1)),
        compatibility: profile(),
        dependencies,
        capabilities: capabilities
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        yanked: false,
        revoked: false,
    }
}

fn resolve(
    roots: &[DependencyRequirement],
    catalog: &[PackageCandidate],
    root_caps: &[&str],
    closure: &[&str],
) -> Result<sico_ecosystem::DependencyLockGraph, DependencyError> {
    let root_caps: BTreeSet<_> = root_caps.iter().map(|value| (*value).to_owned()).collect();
    let closure: BTreeSet<_> = closure.iter().map(|value| (*value).to_owned()).collect();
    resolve_dependencies(
        roots,
        catalog,
        &standard_library(),
        &host(),
        &root_caps,
        &closure,
        &closure,
    )
}

#[test]
fn highest_compatible_versions_resolve_to_byte_identical_lock() {
    let roots = vec![requirement("registry.example", "alpha", "^1.0.0")];
    let beta = candidate(
        "registry.example",
        "beta",
        "2.1.0",
        20,
        Vec::new(),
        &["random.read"],
    );
    let alpha1 = candidate(
        "registry.example",
        "alpha",
        "1.0.0",
        30,
        vec![requirement("registry.example", "beta", "~2.1.0")],
        &["clock.read"],
    );
    let alpha2 = candidate(
        "registry.example",
        "alpha",
        "1.2.0",
        40,
        vec![requirement("registry.example", "beta", "~2.1.0")],
        &["clock.read"],
    );
    let catalog = vec![alpha1, beta, alpha2];
    let lock = resolve(
        &roots,
        &catalog,
        &["log.write"],
        &["clock.read", "log.write", "random.read"],
    )
    .unwrap();
    assert_eq!(lock.schema, LOCK_GRAPH_SCHEMA);
    assert_eq!(lock.nodes[0].version, "1.2.0");
    assert_eq!(lock.nodes[1].version, "2.1.0");
    let mut reversed = catalog;
    reversed.reverse();
    let second = resolve(
        &roots,
        &reversed,
        &["log.write"],
        &["clock.read", "log.write", "random.read"],
    )
    .unwrap();
    assert_eq!(
        canonical_lock_bytes(&lock).unwrap(),
        canonical_lock_bytes(&second).unwrap()
    );
    assert_eq!(
        lock.standard_library_sha256,
        standard_library_digest(&standard_library()).unwrap()
    );
}

#[test]
fn exact_source_identity_prevents_dependency_confusion() {
    let roots = vec![requirement("registry.example", "alpha", "^1.0.0")];
    let trusted = candidate("registry.example", "alpha", "1.1.0", 20, Vec::new(), &[]);
    let attacker = candidate("evil.example", "alpha", "9.9.9", 30, Vec::new(), &[]);
    let lock = resolve(&roots, &[attacker, trusted.clone()], &[], &[]).unwrap();
    assert_eq!(lock.nodes[0].identity, trusted.identity);
    assert_eq!(lock.nodes[0].version, "1.1.0");
}

#[test]
fn equal_precedence_builds_are_ambiguous_and_prereleases_are_opt_in() {
    let roots = vec![requirement("registry.example", "alpha", "^1.0.0")];
    let first = candidate(
        "registry.example",
        "alpha",
        "1.2.0+one",
        20,
        Vec::new(),
        &[],
    );
    let second = candidate(
        "registry.example",
        "alpha",
        "1.2.0+two",
        30,
        Vec::new(),
        &[],
    );
    assert!(matches!(
        resolve(&roots, &[first, second], &[], &[]),
        Err(DependencyError::Ambiguous(_))
    ));
    let prerelease = candidate(
        "registry.example",
        "alpha",
        "1.3.0-beta.1",
        40,
        Vec::new(),
        &[],
    );
    let stable = candidate("registry.example", "alpha", "1.2.0", 50, Vec::new(), &[]);
    let lock = resolve(&roots, &[prerelease, stable], &[], &[]).unwrap();
    assert_eq!(lock.nodes[0].version, "1.2.0");
}

#[test]
fn cycles_and_conflicting_transitive_constraints_fail_closed() {
    let alpha_req = requirement("registry.example", "alpha", "=1.0.0");
    let beta_req = requirement("registry.example", "beta", "=1.0.0");
    let alpha = candidate(
        "registry.example",
        "alpha",
        "1.0.0",
        20,
        vec![beta_req.clone()],
        &[],
    );
    let beta = candidate(
        "registry.example",
        "beta",
        "1.0.0",
        30,
        vec![alpha_req.clone()],
        &[],
    );
    assert!(matches!(
        resolve(std::slice::from_ref(&alpha_req), &[alpha, beta], &[], &[]),
        Err(DependencyError::Cycle(_))
    ));

    let alpha1 = candidate("registry.example", "alpha", "1.0.0", 40, Vec::new(), &[]);
    let alpha2 = candidate("registry.example", "alpha", "2.0.0", 50, Vec::new(), &[]);
    let beta = candidate(
        "registry.example",
        "beta",
        "1.0.0",
        60,
        vec![requirement("registry.example", "alpha", "=2.0.0")],
        &[],
    );
    assert!(matches!(
        resolve(&[alpha_req, beta_req], &[alpha1, alpha2, beta], &[], &[]),
        Err(DependencyError::Conflict(_))
    ));
}

#[test]
fn transitive_capability_closure_and_host_grants_are_exact() {
    let roots = vec![requirement("registry.example", "alpha", "=1.0.0")];
    let alpha = candidate(
        "registry.example",
        "alpha",
        "1.0.0",
        20,
        Vec::new(),
        &["network.connect"],
    );
    assert_eq!(
        resolve(&roots, std::slice::from_ref(&alpha), &[], &[]),
        Err(DependencyError::CapabilityClosure)
    );
    let declared = BTreeSet::from(["network.connect".to_owned()]);
    let denied = resolve_dependencies(
        &roots,
        &[alpha],
        &standard_library(),
        &host(),
        &BTreeSet::new(),
        &declared,
        &BTreeSet::new(),
    );
    assert_eq!(
        denied,
        Err(DependencyError::HostDenied(vec![
            "network.connect".to_owned()
        ]))
    );
}

#[test]
fn yank_affects_new_resolution_while_revocation_is_explicit_for_existing_lock() {
    let roots = vec![requirement("registry.example", "alpha", "^1.0.0")];
    let selected = candidate("registry.example", "alpha", "1.1.0", 20, Vec::new(), &[]);
    let mut yanked_newer = candidate("registry.example", "alpha", "1.2.0", 30, Vec::new(), &[]);
    yanked_newer.yanked = true;
    let lock = resolve(&roots, &[selected.clone(), yanked_newer], &[], &[]).unwrap();
    assert_eq!(lock.nodes[0].version, "1.1.0");

    let mut yanked_locked = selected.clone();
    yanked_locked.yanked = true;
    verify_dependency_lock(
        &lock,
        std::slice::from_ref(&yanked_locked),
        &standard_library(),
        &host(),
        &BTreeSet::new(),
        &RevocationPolicy::Deny,
    )
    .unwrap();
    yanked_locked.revoked = true;
    assert!(matches!(
        verify_dependency_lock(
            &lock,
            std::slice::from_ref(&yanked_locked),
            &standard_library(),
            &host(),
            &BTreeSet::new(),
            &RevocationPolicy::Deny,
        ),
        Err(DependencyError::Revoked(_))
    ));
    verify_dependency_lock(
        &lock,
        std::slice::from_ref(&yanked_locked),
        &standard_library(),
        &host(),
        &BTreeSet::new(),
        &RevocationPolicy::AllowExact(BTreeSet::from([yanked_locked.package_sha256.clone()])),
    )
    .unwrap();
}

#[test]
fn every_compatibility_surface_is_independent_and_unknown_host_contract_rejects() {
    let roots = vec![requirement("registry.example", "alpha", "=1.0.0")];
    let mut incompatible = candidate("registry.example", "alpha", "1.0.0", 20, Vec::new(), &[]);
    incompatible.compatibility.component_world = "unknown-world-v9".to_owned();
    assert!(matches!(
        resolve(&roots, &[incompatible], &[], &[]),
        Err(DependencyError::NotFound(_))
    ));
    assert_ne!(profile().language_semantics, profile().compiler_version);
    assert_ne!(profile().component_world, profile().wasi_contract);
    assert_ne!(profile().capability_contract, profile().host_contract);
}

#[test]
fn canonical_semver_lock_schema_and_digest_mutations_fail_closed() {
    let roots = vec![requirement("registry.example", "alpha", ">=1.0.0,<2.0.0")];
    for invalid in ["01.0.0", "1.0", "1.0.0-01", "v1.0.0", "1.0.0+"] {
        let invalid = candidate("registry.example", "alpha", invalid, 20, Vec::new(), &[]);
        assert!(resolve(&roots, &[invalid], &[], &[]).is_err());
    }
    let valid = candidate(
        "registry.example",
        "alpha",
        "1.2.3-alpha-beta.1",
        30,
        Vec::new(),
        &[],
    );
    let prerelease_root = vec![requirement(
        "registry.example",
        "alpha",
        ">=1.2.3-alpha.1,<2.0.0",
    )];
    let lock = resolve(&prerelease_root, &[valid], &[], &[]).unwrap();
    let bytes = canonical_lock_bytes(&lock).unwrap();
    let lock_digest = dependency_lock_digest(&lock).unwrap();
    assert_eq!(
        verify_dependency_lock_json(&bytes, &lock_digest).unwrap(),
        lock
    );
    let mut rejected = 0;
    for index in 0..bytes.len() {
        let mut mutation = bytes.clone();
        mutation[index] ^= 1;
        if verify_dependency_lock_json(&mutation, &lock_digest).is_err() {
            rejected += 1;
        }
    }
    assert_eq!(rejected, bytes.len());
    println!(
        "DEPENDENCY_MUTATION_METRICS lock_bytes={} rejected={rejected}",
        bytes.len()
    );
}
