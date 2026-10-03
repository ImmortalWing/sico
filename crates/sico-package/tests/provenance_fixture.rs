//! M24 gate-4 provenance — **frozen fixture-model corpus** (RFC-0051 §3.1/§3.2/
//! §3.5 at internal-fixture level). Unlike the in-crate unit tests (which build
//! a manifest from an inline byte string), these exercise the already-shipped
//! `provenance.rs` primitives against a **committed, fixed** payload + manifest
//! pair, so the §3.1 evidence is reproducible against real asset bytes rather
//! than a value recomputed inside the test.
//!
//! Scope discipline: this is the schema/verification **primitive** layer only.
//! It does NOT touch the package carry path, the manifest→payload resource
//! binding, or budget/cancel enforcement — those are the (unaccepted)
//! RFC-0051-A1 wiring and a runner slice, deliberately not exercised here.

use sico_package::PackageError;
use sico_package::provenance::{AssetKind, ProvenanceManifest};

/// A frozen 256-byte fixture model (bytes `0x00..=0xFF`).
const MODEL_BYTES: &[u8] = include_bytes!("fixtures/provenance/model-fixture.bin");
/// Its provenance manifest, carrying the genuine SHA-256 and size of the file
/// above as committed constants.
const MODEL_MANIFEST: &[u8] = include_bytes!("fixtures/provenance/model-fixture.provenance.json");
/// A manifest carrying an unknown field — a §3.5 malicious-asset probe.
const UNKNOWN_FIELD_MANIFEST: &[u8] =
    include_bytes!("fixtures/provenance/malformed-unknown-field.provenance.json");

/// The digest the frozen manifest declares, restated here as an independent
/// constant so a silent edit to either fixture is caught by two tests
/// (`verify_asset` re-hashes the payload; the literal pins the manifest).
const DECLARED_SHA256: &str = "40aff2e9d2d8922e47afd4648e6967497158785fbd1da870e7110266bf944880";

#[test]
fn fixture_corpus_is_self_consistent() {
    // The two committed fixtures must agree; if either drifts this fails.
    assert_eq!(MODEL_BYTES.len(), 256);
    let parsed = ProvenanceManifest::parse(MODEL_MANIFEST).expect("frozen manifest parses");
    assert_eq!(parsed.asset.sha256, DECLARED_SHA256);
    assert_eq!(
        usize::try_from(parsed.asset.size).expect("size fits usize"),
        MODEL_BYTES.len()
    );
    assert_eq!(parsed.asset.kind, AssetKind::Model);
    assert_eq!(parsed.asset.name, "model-fixture");
}

#[test]
fn digest_verified_load_over_frozen_fixture() {
    // RFC-0051 §3.1, on real committed bytes.
    let manifest = ProvenanceManifest::parse(MODEL_MANIFEST).expect("valid manifest");
    manifest
        .verify_asset(MODEL_BYTES)
        .expect("frozen payload matches its declared digest and size");
}

#[test]
fn truncated_payload_over_fixture_is_distinct_refusal() {
    // RFC-0051 §3.2: a byte-for-byte short payload is `Truncated`, not a
    // digest mismatch.
    let manifest = ProvenanceManifest::parse(MODEL_MANIFEST).expect("valid manifest");
    let truncated = &MODEL_BYTES[..MODEL_BYTES.len() - 1];
    assert!(matches!(
        manifest.verify_asset(truncated),
        Err(PackageError::Truncated)
    ));
}

#[test]
fn oversized_payload_limit_plus_one_over_fixture_is_distinct_refusal() {
    // RFC-0051 §3.2: exactly one byte over the declared size is `LengthMismatch`.
    let manifest = ProvenanceManifest::parse(MODEL_MANIFEST).expect("valid manifest");
    let mut oversized = MODEL_BYTES.to_vec();
    oversized.push(0x00);
    assert_eq!(oversized.len(), MODEL_BYTES.len() + 1);
    assert!(matches!(
        manifest.verify_asset(&oversized),
        Err(PackageError::LengthMismatch(_))
    ));
}

#[test]
fn bad_digest_same_size_over_fixture_is_distinct_refusal() {
    // RFC-0051 §3.5 "signature-confused bytes": a same-length payload whose
    // digest differs is `DigestMismatch` — the third, distinct refusal.
    let manifest = ProvenanceManifest::parse(MODEL_MANIFEST).expect("valid manifest");
    let mut confused = MODEL_BYTES.to_vec();
    confused[0] ^= 0xff;
    assert_eq!(confused.len(), MODEL_BYTES.len());
    assert!(matches!(
        manifest.verify_asset(&confused),
        Err(PackageError::DigestMismatch(_))
    ));
}

#[test]
fn unknown_field_manifest_over_fixture_refuses() {
    // RFC-0051 §3.5 structural: the manifest is closed.
    assert!(matches!(
        ProvenanceManifest::parse(UNKNOWN_FIELD_MANIFEST),
        Err(PackageError::Manifest(_))
    ));
}
