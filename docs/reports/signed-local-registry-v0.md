# Signed local registry v0 review

> - status: complete
> - date: 2026-07-16
> - related-step: STEP-0064
> - environment: Windows x86_64 host; Rust 1.97.0 stable GNU toolchain; local filesystem transport

## 1. Question

Can Sico prove signed namespace ownership, immutable release discovery and strict download verification while treating the registry transport as untrusted and avoiding every public operation?

## 2. Method

Extended `sico-ecosystem` with four canonical metadata schemas and a local content-addressed store. Deterministic test-only authority and publisher keys exercise grant, transfer, tombstone, delayed reuse, release publication, immutable channel history, three checkpoint generations and download-time package re-verification.

The eight Rust tests cover the 32-case machine corpus and ten STEP-0064 threats. A mutation loop flips one bit at each byte position of a 1,231-byte signed release record and requires all 1,231 variants to fail.

## 3. Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
& "$HOME/.cargo/bin/cargo.exe" test --offline -p sico-ecosystem --test signed_registry -- --nocapture
& "$HOME/.cargo/bin/cargo.exe" clippy --offline -p sico-ecosystem --all-targets -- -D warnings
& .\tools\validate-step-0064.ps1
```

## 4. Raw evidence

- [`registry.rs`](../../crates/sico-ecosystem/src/registry.rs)
- [`signed_registry.rs`](../../crates/sico-ecosystem/tests/signed_registry.rs)
- [`signed-registry-cases.json`](../../tests/ecosystem/signed-registry-cases.json)
- [`threat-matrix.json`](../../tests/ecosystem/threat-matrix.json)

All filesystem writes are confined to unique process-scoped temporary directories and removed by the test. No network request, namespace allocation, registry account, token, public log or production credential is used.

## 5. Results

| Result | Value | Evidence label |
|---|---:|---|
| Rust registry tests | 8 passed | verified |
| signed release mutations | 1,231/1,231 rejected | verified |
| machine-readable registry cases | 32 | verified |
| assigned ecosystem threats | 10 | verified |
| strict `.sapp` fixture | 590 bytes | measured |
| signed namespace grant | 1,470 bytes | measured |
| signed release record | 1,231 bytes | measured |
| signed channel event | 573 bytes | measured |
| checkpoint generations | 3 | verified |
| external side effects | 0 | verified by scope and test-root review |

## 6. Interpretation

The implementation closes ECO-NAMESPACE-001–004, ECO-REGISTRY-001–004, ECO-DISCLOSURE-004 and ECO-COMPAT-002 at the local implementation level. Namespace authority is explicit rather than inferred from package ID. Mutable discovery cannot replace immutable identity. A mirror or filesystem can cause denial of service but cannot create trusted bytes without the required signatures and digests.

The append-only checkpoint chain makes cached views comparable, but it is not a public transparency log: no external witness, gossip protocol or inclusion service was contacted. Markup-looking descriptions are preserved as inert text only.

## 7. Limitations

- Registry authority policy and keys are deterministic fixtures; real governance and custody remain owner decisions.
- Current `LocalRegistry::publish_release` consumes a genesis grant. Persisting and selecting the latest full namespace history is a future service concern; lifecycle verification itself is implemented and tested.
- Trusted Unix time is supplied by the caller.
- Local discovery scans immutable channel events. Network pagination, caching and denial-of-service controls are not implemented.
- Checkpoints contain signed inclusion sets but no Merkle proof or independent witness.
- Secure update selection/activation, rollback persistence and recovery belong to STEP-0065.

## 8. Decision impact

RFC-0024 is accepted for local registry metadata v0. STEP-0065 may consume immutable release records and checkpoints to implement fail-closed update activation without introducing a public service.

## 9. Links

- [STEP-0064](../steps/STEP-0064-signed-local-registry.md)
- [RFC-0024](../rfc/RFC-0024-signed-registry-metadata-v0.md)
- [RFC-0023](../rfc/RFC-0023-production-publisher-policy-v0.md)
- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
