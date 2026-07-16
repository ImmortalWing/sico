# STEP-0063: Production publisher identity and key lifecycle

> - status: complete
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Implement an offline-verifiable publisher policy and key lifecycle that separates production from development trust, enforces role thresholds, matches exact identity claims and proves rotation, revocation and recovery without creating real production credentials or external side effects.

## 2. Context and evidence

ADR-0006 accepts role-separated signed metadata over content-addressed packages. RFC-0022 freezes the production identity tuple and fail-closed compatibility surfaces. Five threat-matrix cases are assigned to this step: four publisher threats and one disclosure/privacy threat.

Official references reviewed on 2026-07-16: TUF specification 1.0.34 (root thresholds, expiry, rollback and dual-authorized root rotation) and Sigstore certificate/policy documentation (exact issuer, subject and workflow/repository claims).

## 3. Scope

Included: canonical policy/update schemas; Ed25519 key identifiers and signatures; offline/online role custody; thresholds; expiry/version checks; development-key exclusion; exact key/OIDC claim matching; routine rotation, emergency revocation and recovery transitions; disclosure redaction/retention policy; deterministic local fixtures, negative corpus and validator.

Excluded: legal publisher identity, real private keys, HSM/KMS, public transparency, registry/network operations, production signing ceremony, namespace allocation and public disclosure channel.

## 4. Options and decision

A single production key is rejected because compromise authorizes every operation and recovery is ambiguous. Directly embedding a full TUF repository is deferred because registry/update records belong to STEP-0064/0065. The selected v0 is a narrow Sico publisher-policy layer using TUF-style version/expiry/threshold/dual-root invariants and exact optional OIDC claims.

The crate will be `sico-ecosystem` so later registry, update and dependency modules reuse one canonical trust implementation. Test-only signing functions accept in-memory fixture keys but never serialize seeds or provide key storage.

## 5. Plan

1. Freeze RFC-0023 concrete policy/update JSON and limits.
2. Implement strict validation, key identity, canonical signing and threshold verification.
3. Implement transition verification for rotation, revocation and recovery.
4. Add exact claim matching and disclosure policy validation.
5. Add deterministic fixtures, mutation/negative tests and machine-readable coverage.
6. Validate the workspace, accept the RFC, update status/indexes and commit.

## 6. Changes

- Added the `sico-ecosystem` workspace crate.
- Implemented canonical policy/update JSON, key IDs, Ed25519 signature domains, bounds and strict parsing.
- Enforced independent root/release/recovery/rotation/revocation roles, custody and thresholds.
- Enforced initial root trust, development-key exclusion and exact key/OIDC claims.
- Implemented monotonic routine rotation, emergency revocation and root recovery with old/new root plus lifecycle-role authorization.
- Added 28 machine-readable cases, 8 Rust tests, 2,048 signed-byte mutations, RFC-0023 and a review report.

## 7. Validation

```powershell
& .\tools\validate-step-0063.ps1
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
git diff --check
```

Result: `STEP_0063_OK tests=8 mutations=2048 cases=28 threats=5 roles=5 rotation=verified revocation=verified recovery=verified external_side_effects=0 next=STEP-0064`.

## 8. Metrics

Eight tests pass. All 2,048 deterministic single-byte signed-policy mutations are rejected. Fixture sizes are 3,737 bytes initial, 4,537 rotation, 4,543 revocation and 5,032 recovery. These are local non-SLA measurements.

## 9. Risks and follow-ups

Threshold/custody defaults remain a software policy baseline, not an assertion about real people or hardware. Owner choices are still required before production use. Trusted time, public transparency and certificate validation remain later/external boundaries. Registry records consume this policy in STEP-0064.

## 10. Audit links

- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0022](../rfc/RFC-0022-ecosystem-compatibility-contract-v0.md)
- [RFC-0023](../rfc/RFC-0023-production-publisher-policy-v0.md)
- [threat matrix](../../tests/ecosystem/threat-matrix.json)
- [review report](../reports/publisher-identity-key-lifecycle-v0.md)
- [fixture cases](../../tests/ecosystem/publisher-key-lifecycle-cases.json)
