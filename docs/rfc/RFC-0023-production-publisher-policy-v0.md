# RFC-0023: Production publisher policy and key lifecycle v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: ecosystem publisher metadata v0
> - supersedes: -
> - superseded-by: -

## Summary

Define canonical, threshold-signed publisher policy and policy-transition records that remain distinct from `.sapp` development signatures and support fail-closed rotation, revocation and offline recovery.

## Problem

Development signing proves only local M4 trust. A production system needs a stable publisher policy identity, multiple separated authorities, expiry, monotonic versions, exact workload identity matching and recovery from compromised keys. A registry account or one online key cannot supply those guarantees.

## Goals and non-goals

Goals: deterministic bytes; bounded records; Ed25519 verification; root/release/recovery/rotation/revocation separation; dual root authorization; exact identity claims; development-key exclusion; audit-safe disclosure policy.

Non-goals: private-key storage, legal identity, HSM/KMS, public registry, transparency service, namespace governance, release/update records or changing `.sapp` v0.

## Semantics

`sico.publisher.policy.v0` contains version and validity interval, the RFC-0022 identity tuple, sorted public keys, five role policies, exact credential rules and disclosure handling. Each role has sorted unique key IDs, an explicit threshold and `offline`/`online` custody. Root, recovery, rotation and revocation are offline and require at least 2-of-N; release is online and requires at least 1-of-N. One key cannot serve two authority roles.

Key ID is lowercase SHA-256 over `SICO-PUBLISHER-KEY-ID-V0\0 || raw-ed25519-public-key`. Initial policy version is 1 and is root-threshold signed over `SICO-PUBLISHER-POLICY-V0\0 || canonical-policy-json`. Any public key matching a caller-supplied development-key deny set is rejected.

`sico.publisher.policy-update.v0` binds transition kind, previous policy digest, exact next version, validity interval, removed key IDs and reason code to the complete next policy. It is signed over a distinct update domain. Every update requires old-root and new-root thresholds. Routine rotation additionally requires the old rotation threshold, emergency revocation the old revocation threshold, and recovery the old recovery threshold. Versions advance by exactly one and removed keys exactly match the key-set difference.

Identity rules are either an exact key ID or exact OIDC issuer/subject/repository/workflow reference. Matching is byte-for-byte after bounded canonical validation; wildcard, substring and registry-account inference are forbidden. This RFC validates claims but does not validate an external certificate or transparency proof.

## Positive and negative cases

Positive: root-signed initial policy; exact key/OIDC claim; release-key rotation; compromised release-key revocation; offline root recovery signed by old/new roots plus recovery role.

Negative: development-key reuse; one-key offline role; duplicated signature counting; expired/future/non-canonical policy; version skip/rollback; wrong previous digest; incomplete old/new root threshold; approximate OIDC claim; revoked key retained; unknown field/schema/algorithm/transition.

## Compatibility

Unknown schema, algorithm, role, custody or transition fails closed. v0 field order and signature domains are immutable. Changes require a new schema/RFC and an explicitly authorized transition path.

## Security and privacy

Private seeds, access tokens, host paths and user data are forbidden in metadata/logs. Disclosure policy requires a bounded retention period and the fixed redaction classes `access-token`, `credential`, `private-key`, `user-data`. Public transparency and identity disclosure remain `owner-approval-required`.

## Alternatives

- One online publisher key: rejected due to total compromise and weak recovery.
- Registry account as identity: rejected because provider control is not cryptographic publisher authority.
- Adopt all TUF record types now: deferred; publisher policy is the trust input for STEP-0064/0065 rather than a replacement for their records.
- Accept any valid OIDC certificate: rejected; issuer, subject, repository and workflow must all match signed policy.

## Validation and acceptance criteria

Acceptance requires strict canonical roundtrip; deterministic signed fixtures; all five assigned threat cases; rotation/revocation/recovery chains; at least 2,048 signed-byte mutations; bounds/unknown-field/expiry/rollback/threshold/claim negatives; full workspace regression; no real credential or external operation.

## Links

- [STEP-0063](../steps/STEP-0063-production-publisher-identity-key-lifecycle.md)
- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0022](./RFC-0022-ecosystem-compatibility-contract-v0.md)
- [TUF specification 1.0.34](https://theupdateframework.github.io/specification/v1.0.34/)
- [Sigstore certificate issuing](https://docs.sigstore.dev/certificate_authority/certificate-issuing-overview/)
