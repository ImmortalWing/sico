//! M24 gate-4 provenance — **install-path enforcement** (STEP-0578, RFC-0051
//! Amendment A1.3 load-time half). Unlike the primitive-level
//! `provenance_fixture.rs` (which deliberately does not touch the package
//! carry path), this exercises the `.sapp` install path: a package carrying
//! the frozen provenance manifest + payload is built, then `verify()` must
//! enforce the A1.2 name→payload binding and the A1.3 budget ⊆ package-ceiling
//! subset fail-closed.
//!
//! Three states are pinned here (the load-time half of the task's tri-state):
//! 1. budget ⊆ ceiling → `verify()` passes;
//! 2. budget > ceiling (limit+1) → distinct typed `Manifest` refusal;
//! 3. a provenance package with no sibling payload / a binding mismatch →
//!    distinct typed refusal.
//!
//! The run-time enforcement half (budget → `RunnerLimits` narrowing, memory /
//! wall / fuel cancellation) is the runner slice in
//! `runner/sico-runner/tests/provenance_runtime.rs`; it is deliberately not
//! claimed here.

use sico_codegen_wasm::compile_component;
use sico_ir::lower_core;
use sico_package::{
    BuildInput, PackageError, ResourceInput, RuntimeLimits, build_unsigned, verify,
};
use sico_source::{SourceFile, SourceId};

/// A frozen 256-byte fixture model (bytes `0x00..=0xFF`).
const MODEL_BYTES: &[u8] = include_bytes!("fixtures/provenance/model-fixture.bin");
/// Its provenance manifest, carrying the genuine SHA-256 and size of the file
/// above as committed constants (budget 1 MiB / 5000 ms / 100000 hostcalls).
const MODEL_MANIFEST: &[u8] = include_bytes!("fixtures/provenance/model-fixture.provenance.json");

/// A trivial, valid v0 command Component (same shape the other package tests
/// build), so the `.sapp` carries a real runnable artifact.
fn component() -> Vec<u8> {
    let source = SourceFile::from_text(
        SourceId::new(1),
        "answer.sico",
        "function main() returns Int:\n  return 42\nend function\n",
    )
    .unwrap();
    compile_component(&lower_core(&source).unwrap()).unwrap()
}

/// Builds an unsigned `.sapp` carrying the provenance manifest + payload.
fn package_with_provenance(manifest: &[u8]) -> Vec<u8> {
    build_unsigned(BuildInput {
        app_id: "dev.sico.fixture".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: component(),
        resources: vec![
            ResourceInput {
                path: "model-fixture".to_owned(),
                bytes: MODEL_BYTES.to_vec(),
            },
            ResourceInput {
                path: "model-fixture.provenance.json".to_owned(),
                bytes: manifest.to_vec(),
            },
        ],
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("the fixture package builds")
}

#[test]
fn a_provenance_package_with_budget_subset_of_ceiling_installs() {
    // State 1: the frozen budget (1 MiB / 5000 ms / 100000) is a subset of the
    // default package ceiling (64 MiB / 5000 ms / 100000) → install passes.
    let package = package_with_provenance(MODEL_MANIFEST);
    let verified = verify(&package).expect("budget ⊆ ceiling must install");
    assert_eq!(verified.manifest.app.id, "dev.sico.fixture");
    assert!(verified.resources.contains_key("resources/model-fixture"));
    assert!(
        verified
            .resources
            .contains_key("resources/model-fixture.provenance.json")
    );
}

#[test]
fn a_provenance_budget_over_ceiling_is_a_distinct_typed_refusal() {
    // State 2 + limit+1: each mapped budget, at exactly ceiling+1, is a
    // distinct fail-closed `Manifest` refusal. Equality is allowed (tested
    // above). A budget above the package ceiling must never install.
    let package = RuntimeLimits::default();

    for (needle, replacement, field) in [
        (
            "\"max_memory_bytes\": 1048576",
            format!("\"max_memory_bytes\": {}", package.memory_bytes + 1),
            "max_memory_bytes",
        ),
        (
            "\"max_wall_ms\": 5000",
            format!("\"max_wall_ms\": {}", package.timeout_ms + 1),
            "max_wall_ms",
        ),
        (
            "\"max_hostcalls\": 100000",
            format!("\"max_hostcalls\": {}", package.hostcall_fuel + 1),
            "max_hostcalls",
        ),
    ] {
        let over = std::str::from_utf8(MODEL_MANIFEST)
            .expect("manifest is utf-8")
            .replace(needle, &replacement);
        let package = package_with_provenance(over.as_bytes());
        match verify(&package) {
            Err(PackageError::Manifest(message)) => assert!(
                message.contains(field),
                "{field} must be named in the refusal, got: {message}"
            ),
            other => panic!("budget {field} = ceiling+1 must be a Manifest refusal, got {other:?}"),
        }
    }
}

#[test]
fn a_provenance_manifest_without_its_sibling_payload_refuses() {
    // A1.2: the manifest declares a payload path that carries no resource →
    // distinct `Manifest` refusal (fail-closed, never a silent skip).
    let mut manifest = MODEL_MANIFEST.to_vec();
    let text = std::str::from_utf8(&manifest)
        .expect("manifest is utf-8")
        .replace("\"name\": \"model-fixture\"", "\"name\": \"ghost-fixture\"");
    manifest = text.into_bytes();
    // The manifest now declares `resources/ghost-fixture`, which is absent;
    // the resource it sits in is still `model-fixture.provenance.json` (which
    // also mismatches the A1.1 path) — the sibling absence is the first check.
    let package = package_with_provenance(&manifest);
    assert!(matches!(verify(&package), Err(PackageError::Manifest(_))));
}

#[test]
fn a_package_without_provenance_installs_unchanged() {
    // Provenance is opt-in per A1.1: a package carrying no `.provenance.json`
    // resource is untouched by the install-time check.
    let package = build_unsigned(BuildInput {
        app_id: "dev.sico.plain".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: component(),
        resources: vec![ResourceInput {
            path: "messages/hello.txt".to_owned(),
            bytes: b"hello\n".to_vec(),
        }],
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("plain package builds");
    let verified = verify(&package).expect("no provenance → unchanged install");
    assert_eq!(verified.manifest.app.id, "dev.sico.plain");
    assert_eq!(
        verified.resources["resources/messages/hello.txt"],
        b"hello\n"
    );
}
