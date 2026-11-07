//! M24 gate-4 provenance — **budget mount point** (STEP-0578, RFC-0051
//! Amendment A1.3 runtime half). The load-time subset check
//! (`verify_provenance_in_package` → `check_budgets_within`) is exercised in
//! `provenance_install.rs`; this file pins the other half of the bridge: the
//! `VerifiedPackage::provenance_budgets` extraction that a run-time caller
//! (the app layer, or a runner test) uses to obtain the operative asset
//! budget from a verified package.
//!
//! States pinned here:
//! 1. a package carrying the frozen manifest exposes exactly its budget;
//! 2. a package with no provenance manifest exposes `None` (opt-in, A1.1);
//! 3. a package carrying two provenance manifests is a fail-closed refusal
//!    (the operative budget is ambiguous; never a silent pick).

use sico_codegen_wasm::compile_component;
use sico_ir::lower_core;
use sico_package::{
    BuildInput, PackageError, ResourceInput, RuntimeLimits, build_unsigned, verify,
};
use sico_source::{SourceFile, SourceId};

/// The frozen 256-byte fixture model (bytes `0x00..=0xFF`).
const MODEL_BYTES: &[u8] = include_bytes!("fixtures/provenance/model-fixture.bin");
/// Its provenance manifest, carrying the genuine SHA-256 and size as committed
/// constants (budget 1 MiB / 5000 ms / 100000 hostcalls).
const MODEL_MANIFEST: &[u8] = include_bytes!("fixtures/provenance/model-fixture.provenance.json");

/// A trivial, valid v0 command Component so the `.sapp` carries a runnable
/// artifact (the same shape the other package tests build).
fn component() -> Vec<u8> {
    let source = SourceFile::from_text(
        SourceId::new(1),
        "answer.sico",
        "function main() returns Int:\n  return 42\nend function\n",
    )
    .unwrap();
    compile_component(&lower_core(&source).unwrap()).unwrap()
}

/// Builds an unsigned `.sapp` carrying the given resource entries.
fn package_with(resources: Vec<ResourceInput>) -> Vec<u8> {
    build_unsigned(BuildInput {
        app_id: "dev.sico.fixture".to_owned(),
        app_version: "0.1.0".to_owned(),
        component: component(),
        resources,
        source_effects: Vec::new(),
        capabilities: Vec::new(),
        limits: RuntimeLimits::default(),
    })
    .expect("the fixture package builds")
}

fn fixture_resources() -> Vec<ResourceInput> {
    vec![
        ResourceInput {
            path: "model-fixture".to_owned(),
            bytes: MODEL_BYTES.to_vec(),
        },
        ResourceInput {
            path: "model-fixture.provenance.json".to_owned(),
            bytes: MODEL_MANIFEST.to_vec(),
        },
    ]
}

#[test]
fn a_verified_package_exposes_its_frozen_budget() {
    // State 1: `verify` passes (subset holds) and the mount point returns
    // exactly the frozen budget the manifest declares.
    let package = package_with(fixture_resources());
    let verified = verify(&package).expect("fixture package installs");
    let budgets = verified.provenance_budgets().expect("budget extraction");
    let budgets = budgets.expect("the package carries a provenance budget");
    assert_eq!(budgets.max_memory_bytes, 1_048_576);
    assert_eq!(budgets.max_wall_ms, 5_000);
    assert_eq!(budgets.max_hostcalls, 100_000);
}

#[test]
fn a_package_without_provenance_yields_no_budget() {
    // State 2: provenance is opt-in — no `.provenance.json` resource means the
    // mount point is `None`, and the caller keeps its own run-time ceiling.
    let package = package_with(vec![ResourceInput {
        path: "messages/hello.txt".to_owned(),
        bytes: b"hello\n".to_vec(),
    }]);
    let verified = verify(&package).expect("plain package installs");
    assert!(
        verified
            .provenance_budgets()
            .expect("no-manifest is not an error")
            .is_none()
    );
}

#[test]
fn two_provenance_manifests_are_an_ambiguous_budget_refusal() {
    // State 3: two manifests each declare a valid asset + budget, but the
    // operative run budget is ambiguous → fail-closed `Manifest` refusal.
    // This never silently picks one manifest.
    let second_name = "model-fixture-2";
    let second_manifest = std::str::from_utf8(MODEL_MANIFEST)
        .expect("manifest is utf-8")
        .replace(
            "\"name\": \"model-fixture\"",
            &format!("\"name\": \"{second_name}\""),
        );
    let resources = vec![
        ResourceInput {
            path: "model-fixture".to_owned(),
            bytes: MODEL_BYTES.to_vec(),
        },
        ResourceInput {
            path: "model-fixture.provenance.json".to_owned(),
            bytes: MODEL_MANIFEST.to_vec(),
        },
        ResourceInput {
            path: second_name.to_owned(),
            bytes: MODEL_BYTES.to_vec(),
        },
        ResourceInput {
            path: format!("{second_name}.provenance.json"),
            bytes: second_manifest.into_bytes(),
        },
    ];
    let package = package_with(resources);
    let verified = verify(&package).expect("both manifests are individually valid");
    assert!(matches!(
        verified.provenance_budgets(),
        Err(PackageError::Manifest(message)) if message.contains("more than one provenance manifest")
    ));
}
