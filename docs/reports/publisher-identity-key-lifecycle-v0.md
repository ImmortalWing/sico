# Publisher identity and key lifecycle v0 review

> - status: complete
> - date: 2026-07-16
> - related-step: STEP-0063
> - environment: Windows x86_64 host; Rust 1.97.0 stable override; offline Cargo dependencies

## 1. Question

Can Sico establish a deterministic production publisher-policy trust layer with separated roles, exact identities and recoverable key transitions without creating a real production credential or trusting a registry?

## 2. Method

Implemented `sico-ecosystem` with canonical compact JSON, Ed25519 domain-separated signatures, SHA-256 key/policy IDs, bounded records and strict unknown-field rejection. A deterministic fixture uses nine initial test-only keys across root, release, recovery, rotation and revocation roles. The chain performs release-key rotation, emergency release-key revocation and root recovery.

Tests cover the 28-case machine-readable corpus and the five STEP-0063 ecosystem threats. A deterministic loop flips one bit in 2,048 signed initial-policy inputs and requires every mutation to fail verification.

## 3. Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
& "$HOME/.cargo/bin/cargo.exe" test --offline -p sico-ecosystem
& "$HOME/.cargo/bin/cargo.exe" test --offline -p sico-ecosystem rotation_revocation_and_recovery_form_a_monotonic_chain -- --nocapture
& .\tools\validate-step-0063.ps1
```

## 4. Raw evidence

- [`publisher_lifecycle.rs`](../../crates/sico-ecosystem/tests/publisher_lifecycle.rs)
- [`publisher-key-lifecycle-cases.json`](../../tests/ecosystem/publisher-key-lifecycle-cases.json)
- [`threat-matrix.json`](../../tests/ecosystem/threat-matrix.json)

Test seeds are deterministic fixtures and are never serialized into metadata. No production key, access token, OIDC token, registry account or public operation is present.

## 5. Results

| Result | Value | Evidence label |
|---|---:|---|
| Rust tests | 8 passed | verified |
| signed-byte mutations | 2,048/2,048 rejected | verified |
| machine-readable lifecycle cases | 28 | verified |
| assigned ecosystem threats | 5 | verified |
| initial signed policy | 3,737 bytes | measured |
| routine rotation update | 4,537 bytes | measured |
| emergency revocation update | 4,543 bytes | measured |
| root recovery update | 5,032 bytes | measured |
| external side effects | 0 | verified by scope/repository review |

## 6. Interpretation

The local trust primitive closes ECO-PUBLISHER-001–004 and ECO-DISCLOSURE-001 at fixture/implementation level. Development keys are excluded by an explicit deny set. Offline roles require independent 2-of-N keys; release is a distinct online role. Updates require old-root, new-root and transition-specific thresholds. Exact OIDC claims are policy inputs only; this step does not validate a live certificate or transparency proof.

## 7. Limitations

- Custodians, threshold people/devices, HSM/KMS and legal publisher identity are not selected.
- Unix time is supplied by the caller; trusted-time/freeze policy belongs to registry/update clients.
- Only Ed25519 is accepted in v0; algorithm migration needs a new schema.
- No public transparency, registry, namespace or disclosure channel is configured.
- Fixture signing helpers are not a production ceremony or key-management implementation.

## 8. Decision impact

RFC-0023 can be accepted for local ecosystem metadata v0. STEP-0064 may build signed registry release/snapshot records on `PublisherPolicy`, while retaining local `.sapp` verification and owner-controlled external decisions.

## 9. Links

- [STEP-0063](../steps/STEP-0063-production-publisher-identity-key-lifecycle.md)
- [RFC-0023](../rfc/RFC-0023-production-publisher-policy-v0.md)
- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
