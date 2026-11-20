//! STEP-0497: the app path's enforced guest memory ceiling must cover the
//! guest linear memory every script component declares.
//!
//! The ceiling is applied as wasmtime's `-W max-memory-size`. A ceiling
//! *below* the declared minimum makes wasmtime unable to satisfy the
//! minimum at instantiation, so every script launch on this path fails with
//! "forcing trap when growing memory to … bytes" — a trap at launch, not a
//! lowering failure, and not a host-memory problem: it reproduces on every
//! machine.
//!
//! The effective ceiling is the minimum of the package limit and the Host
//! limit, so *both* ends of that pair have to clear the declaration. These
//! guards are pure constant comparisons plus one package-level refusal, so
//! they run on every machine and need no runtime binary (the launch-seam
//! test in `sico-app-cli` needs `SICO_TEST_WASMTIME` and skips without it).

use std::collections::BTreeSet;

use sico_codegen_wasm::{SCRIPT_MEMORY_BYTES, compile_component};
use sico_ir::lower_core;
use sico_package::{
    AuthorizedPackage, BuildInput, BuildScriptInput, ResourceInput, RuntimeLimits, TrustStatus,
    TrustedPackage, build_script_v1, build_unsigned, verify,
};
use sico_runtime::{HostLimits, PackageRuntimeError, effective_memory_limits};
use sico_source::{SourceFile, SourceId};

#[test]
fn the_app_path_host_ceiling_covers_the_declared_component_memory() {
    let ceiling = HostLimits::default().memory_bytes;
    assert!(
        ceiling >= SCRIPT_MEMORY_BYTES,
        "the app path enforces a {ceiling} B guest memory ceiling but a script component declares \
         {SCRIPT_MEMORY_BYTES} B: wasmtime cannot satisfy the declared minimum and every script \
         instantiation traps"
    );
}

#[test]
fn the_package_layer_grants_the_declared_component_memory() {
    let granted = sico_package::script_profile_limits().memory_bytes;
    assert!(
        granted >= SCRIPT_MEMORY_BYTES,
        "a script package grants {granted} B of guest memory but its component declares \
         {SCRIPT_MEMORY_BYTES} B: the package limit is the binding constraint whenever it is \
         the tighter of the two"
    );
}

#[test]
fn the_default_ceilings_keep_a_script_package_launchable() {
    let package = authorized_script_package();
    let limits = effective_memory_limits(&package, &HostLimits::default()).unwrap();
    assert!(limits.memory_bytes >= SCRIPT_MEMORY_BYTES);
}

#[test]
fn a_provenance_budget_narrows_the_effective_memory_ceiling() {
    // RFC-0051 Amendment A2 (I3): a package carrying a provenance asset budget
    // applies it as a pure narrowing of the effective limits on the app run
    // path. The budget's memory ceiling (1 MiB here, below the 64 MiB package
    // ceiling and below the Script floor — this is a non-Script package, so no
    // floor binds) becomes the operative ceiling, which `run_authorized_package`
    // passes to wasmtime as `-W max-memory-size`.
    let payload = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sico-package/tests/fixtures/provenance/model-fixture.bin"
    ));
    let manifest = std::str::from_utf8(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sico-package/tests/fixtures/provenance/model-fixture.provenance.json"
    )))
    .expect("manifest is utf-8")
    .replace("\"max_memory_bytes\": 2214592512", "\"max_memory_bytes\": 1048576");
    let source = SourceFile::from_text(
        SourceId::new(7),
        "main.sico",
        "function main() returns Unit:\n  return Unit\nend function\n",
    )
    .unwrap();
    let component = compile_component(&lower_core(&source).unwrap()).unwrap();
    let package = build_unsigned(BuildInput {
        app_id: "dev.sico.budget.narrow".to_owned(),
        app_version: "0.1.0".to_owned(),
        component,
        resources: vec![
            ResourceInput {
                path: "model-fixture".to_owned(),
                bytes: payload.to_vec(),
            },
            ResourceInput {
                path: "model-fixture.provenance.json".to_owned(),
                bytes: manifest.into_bytes(),
            },
        ],
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("the budget package builds");
    let verified = verify(&package).expect("budget ⊆ ceiling: the package installs");
    let authorized = AuthorizedPackage {
        trusted: TrustedPackage {
            package: verified,
            trust: TrustStatus::UnsignedDevelopment,
        },
        granted_capabilities: BTreeSet::new(),
    };
    let limits = effective_memory_limits(&authorized, &HostLimits::default()).unwrap();
    assert_eq!(limits.memory_bytes, 1_048_576);
}

#[test]
fn a_ceiling_below_the_declared_component_memory_is_refused_with_a_typed_error() {
    let package = authorized_script_package();
    let host = HostLimits {
        memory_bytes: 64 * 1024 * 1024,
        ..HostLimits::default()
    };
    match effective_memory_limits(&package, &host) {
        Err(PackageRuntimeError::MemoryCeilingBelowDeclared {
            effective_bytes,
            declared_bytes,
        }) => {
            assert_eq!(effective_bytes, 64 * 1024 * 1024);
            assert_eq!(declared_bytes, SCRIPT_MEMORY_BYTES);
        }
        other => panic!("expected a typed memory ceiling refusal, got {other:?}"),
    }
}

/// A Script package built with the profile limits, authorized far enough for
/// the Runtime to read its manifest.
fn authorized_script_package() -> AuthorizedPackage {
    let source = SourceFile::from_text(
        SourceId::new(7),
        "main.sico",
        "function main() returns Unit:\n  return Unit\nend function\n",
    )
    .unwrap();
    let component = compile_component(&lower_core(&source).unwrap()).unwrap();
    let package = build_script_v1(BuildScriptInput {
        app_id: "dev.sico.script.ceiling".to_owned(),
        app_version: "0.1.0".to_owned(),
        component,
        resources: Vec::new(),
        semantics: "sico.ir.v0".to_owned(),
        wit_sha256: "a".repeat(64),
        adapter_sha256: "b".repeat(64),
        limits: sico_package::script_profile_limits(),
    })
    .unwrap();
    let verified = verify(&package).unwrap();
    AuthorizedPackage {
        trusted: TrustedPackage {
            package: verified,
            trust: TrustStatus::UnsignedDevelopment,
        },
        granted_capabilities: BTreeSet::new(),
    }
}
