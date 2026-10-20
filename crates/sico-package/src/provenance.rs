//! M17 gate-4 model/package provenance manifest (RFC-0051 §2/§3, accepted
//! 2026-09-30). A **closed** asset manifest carried inside a package that
//! lets a fixture model satisfy the provenance gate end to end.
//!
//! This module implements the manifest *schema + load-time verification*
//! half of RFC-0051 §3 — properties 1 (digest-verified load), 2 (bad digest
//! / truncated / oversized each a distinct typed refusal, no partial load)
//! and the structural half of 5 (unknown fields, missing required fields,
//! malformed digest form, path-like names, unknown `kind` all refused).
//! It deliberately reuses the existing M7 digest discipline
//! (the crate-root `validate_digest` / [`crate::sha256_hex`]) so no second
//! trust path is introduced (RFC-0051 §2).
//!
//! The budget **ceiling check** (RFC-0051 Amendment A1.3, accepted by owner
//! re-adjudication 2026-10-01) is a load-time property and lives here: an
//! asset budget that exceeds its mapped package limit is a distinct typed
//! refusal, and [`ProvenanceManifest::verify_binding`] performs A1.2's
//! name→payload digest/size cross-check. The *run-time* enforcement of budgets
//! (executing a fixture-model guest under the mapped limits) and cancellation
//! mid-inference (properties 3-runtime and 4) belong to the runner wiring and
//! are intentionally **not** implemented or claimed here.

use serde::{Deserialize, Serialize};

use crate::{PackageError, RuntimeLimits, sha256_hex, validate_digest};

/// RFC-0051 §2 closed asset-kind set (v0: fixture models and lookup tables).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    /// A fixture model asset.
    Model,
    /// A lookup table asset.
    Table,
}

/// RFC-0051 §2 `provenance` block. All fields required; closed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Where the bytes came from (URL or `"in-repo generated"`).
    pub origin: String,
    /// SPDX identifier or `"in-repo"`.
    pub license: String,
    /// ISO-8601 date.
    pub created: String,
    /// Tool/family that produced it (`"hand-authored fixture"` for v0).
    pub generator: String,
}

/// RFC-0051 §2 `budgets` block. All three required; each enforced by a
/// typed refusal at run time (the run-time enforcement is the runner's, not
/// this schema's).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// Peak memory ceiling in bytes.
    pub max_memory_bytes: u64,
    /// Wall-clock ceiling in milliseconds.
    pub max_wall_ms: u64,
    /// Host-call count ceiling.
    pub max_hostcalls: u64,
}

/// RFC-0051 §2 `asset` record. Closed: unknown fields refuse.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    /// Package-unique name; path-like values refuse at load.
    pub name: String,
    /// The closed v0 asset kind.
    pub kind: AssetKind,
    /// Media type, e.g. `"application/x-sico-fixture-model"`.
    pub media: String,
    /// Lowercase 64-hex digest (M7 discipline, no parallel trust path).
    pub sha256: String,
    /// Byte size; must match the payload exactly.
    pub size: u64,
    /// Provenance metadata block.
    pub provenance: Provenance,
    /// Budget ceiling block.
    pub budgets: Budgets,
    /// The package function names allowed to load this asset.
    pub interface: String,
}

/// The RFC-0051 `sico.m17.provenance.v0` manifest root.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceManifest {
    /// The single declared asset.
    pub asset: Asset,
}

impl ProvenanceManifest {
    /// Parses and structurally validates a closed provenance manifest.
    ///
    /// Refusals (RFC-0051 §2/§3.5): unknown fields, missing required fields
    /// and an unknown `kind` fail through `serde` into
    /// [`PackageError::Manifest`]; a malformed digest, an empty/`..`/path-like
    /// name, an empty interface, or a zero `size` are the same typed error.
    ///
    /// # Errors
    ///
    /// [`PackageError::Manifest`] for any schema or structural violation.
    pub fn parse(bytes: &[u8]) -> Result<Self, PackageError> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|error| PackageError::Manifest(format!("provenance manifest: {error}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), PackageError> {
        let asset = &self.asset;
        validate_digest(&asset.sha256)
            .map_err(|error| PackageError::Manifest(format!("asset sha256: {error}")))?;
        if asset.name.trim().is_empty() {
            return Err(PackageError::Manifest(
                "asset name must not be empty".to_owned(),
            ));
        }
        // RFC-0051 §3.5: path-like names are a malicious-asset probe; refuse
        // before any loader could treat the name as a path.
        if asset.name.contains('/')
            || asset.name.contains('\\')
            || asset.name.contains("..")
            || asset.name.contains(':')
        {
            return Err(PackageError::Manifest(
                "asset name must not be path-like".to_owned(),
            ));
        }
        if asset.interface.trim().is_empty() {
            return Err(PackageError::Manifest(
                "asset interface must not be empty".to_owned(),
            ));
        }
        if asset.size == 0 {
            return Err(PackageError::Manifest(
                "asset size must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }

    /// Verifies a payload against the declared `size` and `sha256`
    /// (RFC-0051 §3.2, "no partial load"). The two size violations are
    /// distinct typed refusals and precede digest comparison: an oversized
    /// payload is [`PackageError::LengthMismatch`], a truncated one is
    /// [`PackageError::Truncated`], and a byte-for-byte size match with a
    /// wrong digest is [`PackageError::DigestMismatch`].
    ///
    /// # Errors
    ///
    /// The matching typed refusal; `Ok(())` only on an exact length and
    /// digest match.
    pub fn verify_asset(&self, payload: &[u8]) -> Result<(), PackageError> {
        let declared = self.asset.size;
        let actual = u64::try_from(payload.len())
            .map_err(|_| PackageError::LengthMismatch("asset size overflows u64".to_owned()))?;
        if actual > declared {
            return Err(PackageError::LengthMismatch(format!(
                "payload {actual} bytes exceeds declared size {declared}"
            )));
        }
        if actual < declared {
            return Err(PackageError::Truncated);
        }
        let computed = sha256_hex(payload);
        if computed != self.asset.sha256 {
            return Err(PackageError::DigestMismatch(format!(
                "asset digest mismatch: declared {}, computed {computed}",
                self.asset.sha256
            )));
        }
        Ok(())
    }

    /// RFC-0051 Amendment A1.1 (accepted 2026-10-01): the deterministic M7
    /// resource path the manifest itself occupies, `resources/<name>.provenance.json`.
    /// [`validate`] already rejects any path-like `name`, so this construction
    /// cannot escape the `resources/` directory.
    #[must_use]
    pub fn provenance_resource_path(&self) -> String {
        format!("resources/{}.provenance.json", self.asset.name)
    }

    /// RFC-0051 Amendment A1.1 (accepted): the sibling resource path of the
    /// payload the manifest describes, `resources/<name>`.
    #[must_use]
    pub fn payload_resource_path(&self) -> String {
        format!("resources/{}", self.asset.name)
    }

    /// RFC-0051 Amendment A1.2 (accepted): the name→payload binding
    /// cross-check. The sibling resource's **stored M7 artifact digest and byte
    /// length** must equal the manifest's declared `sha256`/`size`; any
    /// disagreement is the RFC-0051 §3.5 "manifest/payload mismatch" typed
    /// refusal (a [`PackageError::DigestMismatch`], distinct from the
    /// payload-content `Truncated`/`LengthMismatch` that [`verify_asset`]
    /// returns). This is a metadata-layer check against the install-time M7
    /// resource table; it introduces no second trust path.
    ///
    /// # Errors
    ///
    /// [`PackageError::DigestMismatch`] when the stored digest or size does not
    /// match the manifest declaration; `Ok(())` on an exact cross-check.
    pub fn verify_binding(
        &self,
        stored_sha256: &str,
        stored_bytes: u64,
    ) -> Result<(), PackageError> {
        if stored_bytes != self.asset.size {
            return Err(PackageError::DigestMismatch(format!(
                "manifest/payload mismatch: declared size {} but resource {} carries {stored_bytes} bytes",
                self.asset.size,
                self.payload_resource_path()
            )));
        }
        if stored_sha256 != self.asset.sha256 {
            return Err(PackageError::DigestMismatch(format!(
                "manifest/payload mismatch: declared digest {} but resource {} carries {stored_sha256}",
                self.asset.sha256,
                self.payload_resource_path()
            )));
        }
        Ok(())
    }

    /// RFC-0051 Amendment A1.3 (accepted): the asset budget must be a **subset**
    /// of the package's own [`RuntimeLimits`], mapped one-to-one onto the
    /// existing limit kinds (no new field, no parallel limit path):
    ///
    /// ```text
    /// max_memory_bytes → RuntimeLimits::memory_bytes
    /// max_wall_ms      → RuntimeLimits::timeout_ms
    /// max_hostcalls    → RuntimeLimits::hostcall_fuel
    /// ```
    ///
    /// A budget that **exceeds** its mapped ceiling (limit+1) is a distinct
    /// fail-closed load refusal ([`PackageError::Manifest`]); equality is
    /// allowed. This never widens the package limit (AGENTS.md §helpers narrow,
    /// never widen) — the operative run ceiling for the asset is the asset
    /// budget, which is by construction `≤` the package ceiling.
    ///
    /// # Errors
    ///
    /// [`PackageError::Manifest`] naming the first over-ceiling budget;
    /// `Ok(())` when all three budgets are within the package limits.
    pub fn check_budgets_within(&self, package: &RuntimeLimits) -> Result<(), PackageError> {
        let budgets = &self.asset.budgets;
        if budgets.max_memory_bytes > package.memory_bytes {
            return Err(PackageError::Manifest(format!(
                "asset budget max_memory_bytes {} exceeds package ceiling {}",
                budgets.max_memory_bytes, package.memory_bytes
            )));
        }
        if budgets.max_wall_ms > package.timeout_ms {
            return Err(PackageError::Manifest(format!(
                "asset budget max_wall_ms {} exceeds package ceiling {}",
                budgets.max_wall_ms, package.timeout_ms
            )));
        }
        if budgets.max_hostcalls > package.hostcall_fuel {
            return Err(PackageError::Manifest(format!(
                "asset budget max_hostcalls {} exceeds package ceiling {}",
                budgets.max_hostcalls, package.hostcall_fuel
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal, valid fixture manifest plus a payload matching its digest.
    fn valid_fixture() -> (String, Vec<u8>) {
        let payload = b"model-bytes-0123456789".to_vec();
        let digest = sha256_hex(&payload);
        let manifest = format!(
            r#"{{
              "asset": {{
                "name": "demo-fixture",
                "kind": "model",
                "media": "application/x-sico-fixture-model",
                "sha256": "{digest}",
                "size": {},
                "provenance": {{
                  "origin": "in-repo generated",
                  "license": "in-repo",
                  "created": "2026-09-30",
                  "generator": "hand-authored fixture"
                }},
                "budgets": {{
                  "max_memory_bytes": 1048576,
                  "max_wall_ms": 5000,
                  "max_hostcalls": 100000
                }},
                "interface": "load-model"
              }}
            }}"#,
            payload.len()
        );
        (manifest, payload)
    }

    #[test]
    fn digest_verified_load_succeeds() {
        // RFC-0051 §3.1.
        let (manifest, payload) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).expect("valid manifest");
        assert_eq!(parsed.asset.kind, AssetKind::Model);
        parsed.verify_asset(&payload).expect("payload matches");
    }

    #[test]
    fn unknown_field_refuses() {
        let (mut manifest, _) = valid_fixture();
        manifest = manifest.replace(
            "\"interface\": \"load-model\"",
            "\"interface\": \"load-model\", \"surprise\": 1",
        );
        assert!(matches!(
            ProvenanceManifest::parse(manifest.as_bytes()),
            Err(PackageError::Manifest(_))
        ));
    }

    #[test]
    fn missing_required_field_refuses() {
        let (manifest, _) = valid_fixture();
        let stripped = manifest.replace("\"license\": \"in-repo\",", "");
        assert!(matches!(
            ProvenanceManifest::parse(stripped.as_bytes()),
            Err(PackageError::Manifest(_))
        ));
    }

    #[test]
    fn unknown_kind_variant_refuses() {
        let (manifest, _) = valid_fixture();
        let swapped = manifest.replace("\"kind\": \"model\"", "\"kind\": \"quantum\"");
        assert!(ProvenanceManifest::parse(swapped.as_bytes()).is_err());
    }

    #[test]
    fn malformed_digest_refuses() {
        let (manifest, _) = valid_fixture();
        let bad = manifest.replace(
            &sha256_hex(b"model-bytes-0123456789".as_slice()),
            "not-64-hex-chars-but-short",
        );
        assert!(matches!(
            ProvenanceManifest::parse(bad.as_bytes()),
            Err(PackageError::Manifest(_))
        ));
    }

    #[test]
    fn path_like_name_refuses() {
        for name in ["../evil", "a/b", "c:\\d", "C:evil"] {
            let (manifest, _) = valid_fixture();
            let swapped = manifest.replace(
                "\"name\": \"demo-fixture\"",
                &format!("\"name\": \"{name}\""),
            );
            assert!(
                matches!(
                    ProvenanceManifest::parse(swapped.as_bytes()),
                    Err(PackageError::Manifest(_))
                ),
                "path-like name {name:?} must refuse"
            );
        }
    }

    #[test]
    fn truncated_payload_is_distinct_refusal() {
        // RFC-0051 §3.2: truncated vs oversized vs bad digest are distinct.
        let (manifest, payload) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        let short = &payload[..payload.len() - 1];
        assert!(matches!(
            parsed.verify_asset(short),
            Err(PackageError::Truncated)
        ));
    }

    #[test]
    fn oversized_payload_limit_plus_one_is_distinct_refusal() {
        let (manifest, payload) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        let mut long = payload.clone();
        long.push(0); // size + 1
        assert!(matches!(
            parsed.verify_asset(&long),
            Err(PackageError::LengthMismatch(_))
        ));
    }

    #[test]
    fn bad_digest_same_length_is_distinct_refusal() {
        // Same declared size, wrong bytes → digest mismatch, not a size error.
        let (manifest, payload) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        let mut same_len = payload.clone();
        let last = same_len.len() - 1;
        same_len[last] ^= 0xff; // keeps length, changes digest
        assert_eq!(same_len.len(), payload.len());
        assert!(matches!(
            parsed.verify_asset(&same_len),
            Err(PackageError::DigestMismatch(_))
        ));
    }

    #[test]
    fn round_trip_serialization_is_stable() {
        let (manifest, _) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        let re = serde_json::to_vec(&parsed).expect("serialize");
        let again = ProvenanceManifest::parse(&re).expect("reparse");
        assert_eq!(parsed, again);
    }

    #[test]
    fn a11_carry_paths_are_deterministic_and_scoped() {
        // RFC-0051 A1.1: manifest and payload sit as name-keyed siblings under
        // the M7 `resources/` prefix; a path-like name is refused earlier by
        // validate(), so the derived path can never escape that directory.
        let (manifest, _) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        assert_eq!(
            parsed.provenance_resource_path(),
            "resources/demo-fixture.provenance.json"
        );
        assert_eq!(parsed.payload_resource_path(), "resources/demo-fixture");
    }

    #[test]
    fn a12_binding_cross_check_accepts_a_matching_sibling() {
        // The stored M7 digest + size of the sibling resource equal the
        // manifest declaration → Ok.
        let payload = b"model-bytes-0123456789".to_vec();
        let (manifest, _) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        parsed
            .verify_binding(&sha256_hex(&payload), payload.len() as u64)
            .expect("binding cross-check passes");
    }

    #[test]
    fn a12_binding_digest_mismatch_is_a_distinct_refusal() {
        let (manifest, payload) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        // Right size, wrong stored digest → the §3.5 manifest/payload mismatch.
        let forged = sha256_hex(&[0u8; 1]);
        assert!(matches!(
            parsed.verify_binding(&forged, payload.len() as u64),
            Err(PackageError::DigestMismatch(_))
        ));
    }

    #[test]
    fn a12_binding_size_mismatch_is_a_distinct_refusal() {
        // The declared digest is presented but the resource's stored length
        // disagrees → also the manifest/payload mismatch (not Truncated, which
        // is a payload-content read result).
        let (manifest, payload) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        assert!(matches!(
            parsed.verify_binding(&parsed.asset.sha256, payload.len() as u64 + 1),
            Err(PackageError::DigestMismatch(_))
        ));
    }

    #[test]
    fn a13_budget_subset_accepts_equality_and_within() {
        // valid_fixture budgets are within (memory) or exactly at (wall 5000,
        // hostcalls 100000) the package default ceilings → subset holds.
        let (manifest, _) = valid_fixture();
        let parsed = ProvenanceManifest::parse(manifest.as_bytes()).unwrap();
        parsed
            .check_budgets_within(&RuntimeLimits::default())
            .expect("budgets are a subset of the package ceiling");
    }

    #[test]
    fn a13_budget_over_ceiling_is_limit_plus_one_refusal() {
        // Each of the three mapped budgets, at ceiling+1, is a distinct
        // fail-closed load refusal; equality is permitted (tested above).
        let (manifest, _) = valid_fixture();
        let package = RuntimeLimits::default();

        let over_memory = manifest.replace(
            "\"max_memory_bytes\": 1048576",
            &format!("\"max_memory_bytes\": {}", package.memory_bytes + 1),
        );
        let parsed = ProvenanceManifest::parse(over_memory.as_bytes()).unwrap();
        assert!(matches!(
            parsed.check_budgets_within(&package),
            Err(PackageError::Manifest(_))
        ));

        let over_wall = manifest.replace(
            "\"max_wall_ms\": 5000",
            &format!("\"max_wall_ms\": {}", package.timeout_ms + 1),
        );
        let parsed = ProvenanceManifest::parse(over_wall.as_bytes()).unwrap();
        assert!(matches!(
            parsed.check_budgets_within(&package),
            Err(PackageError::Manifest(_))
        ));

        let over_hostcalls = manifest.replace(
            "\"max_hostcalls\": 100000",
            &format!("\"max_hostcalls\": {}", package.hostcall_fuel + 1),
        );
        let parsed = ProvenanceManifest::parse(over_hostcalls.as_bytes()).unwrap();
        assert!(matches!(
            parsed.check_budgets_within(&package),
            Err(PackageError::Manifest(_))
        ));
    }
}
