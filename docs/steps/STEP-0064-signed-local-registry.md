# STEP-0064: Signed local registry publish, discovery and download

> - status: complete
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Implement a deterministic local-filesystem registry that treats transport as untrusted, requires signed namespace/release/discovery/checkpoint records, stores `.sapp` by digest and repeats strict local verification after download.

## 2. Context and evidence

STEP-0063 provides publisher policy, role thresholds and exact production identity. Ten threat-matrix cases are assigned here: four namespace, four registry, one inert-text and one unknown-schema case.

## 3. Scope

Included: pinned local registry authority; canonical namespace grant/transfer/tombstone; signed release and channel discovery; chained checkpoints and inclusion evidence; content-addressed blob store; immutable local publish; length/digest/strict `.sapp` reverify; negative/mutation corpus.

Excluded: public domain, service account, legal namespace grant, real production key, HTTP/OCI transport, public transparency service, billing, rate limiting and external publish.

## 4. Options and decision

An in-memory mock would not prove filesystem corruption, atomic writes or download re-verification. A local filesystem transport is selected because it exercises the complete trust path while remaining reversible and side-effect-free outside test directories. Registry authority remains a pinned test policy and never replaces publisher authorization.

## 5. Plan

1. Freeze registry record schemas and bounds. — complete
2. Implement namespace lifecycle with registry and publisher authorization. — complete
3. Implement immutable release/channel/checkpoint records. — complete
4. Implement content-addressed local publish/discover/download. — complete
5. Re-run strict package verification and identity/version checks on download. — complete
6. Add security corpus, report, validator and workspace regression. — complete

## 6. Changes

- Added `registry.rs` with canonical namespace, release, channel and checkpoint envelopes, four signature domains and strict time/schema/identity/bounds checks.
- Required independent pinned registry and publisher authority sets. Grant requires registry plus new-root thresholds; transfer requires registry plus old/new roots; tombstone requires registry plus current root.
- Bound every non-genesis namespace event to the exact signed predecessor digest. Tombstones enforce a future reuse time and a later grant cannot bypass it.
- Added release-role signed immutable metadata with exact publisher-policy, namespace-event, package length/digest, app and compatibility bindings.
- Stored blobs and release records by SHA-256. Channel discovery is an immutable exact-sequence history, avoiding cross-platform mutable replacement behavior.
- Added append-only sorted inclusion sets and digest-chained checkpoints. Historical checkpoint signatures remain verifiable after historical expiry while the newly trusted checkpoint must be current.
- Added local publish/discover/download. Download repeats release/namespace verification, length and digest checks, strict `.sapp` parsing and manifest ID/version comparison.
- Accepted [RFC-0024](../rfc/RFC-0024-signed-registry-metadata-v0.md), added the [32-case corpus](../../tests/ecosystem/signed-registry-cases.json), [review report](../reports/signed-local-registry-v0.md) and validator.

## 7. Validation

The focused registry suite passed 8/8 tests. Every one-bit mutation at all 1,231 byte positions of a signed release record was rejected. Namespace grant/transfer/tombstone/reuse, three checkpoint generations, immutable channel progression, corrupted transport bytes, inert text, unknown fields/schema and SemVer cases passed.

Full Rust formatting, Clippy with warnings denied, workspace tests, local-link validation, `git diff --check` and `validate-step-0064.ps1` are required immediately before the step commit.

Result: `STEP_0064_OK tests=8 mutations=1231 cases=32 threats=10 namespace=verified releases=verified checkpoints=3 downloads=reverified external_side_effects=0 next=STEP-0065`.

## 8. Metrics

| Metric | Result |
|---|---:|
| registry tests | 8 passed |
| release mutations | 1,231/1,231 rejected |
| machine cases | 32 |
| assigned threats | 10 |
| `.sapp` fixture | 590 bytes |
| signed namespace grant | 1,470 bytes |
| signed release | 1,231 bytes |
| signed channel event | 573 bytes |
| checkpoint generations | 3 |
| external operations | 0 |

## 9. Risks and follow-ups

Local authority fixtures do not allocate a public namespace or prove public transparency. The local service path consumes a genesis grant; hosted full-history selection, network availability and governance remain service concerns. Checkpoints are signed cacheable evidence, not a witnessed Merkle transparency log. Secure update activation, locally persisted anti-rollback state and recovery remain STEP-0065.

## 10. Audit links

- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0022](../rfc/RFC-0022-ecosystem-compatibility-contract-v0.md)
- [RFC-0023](../rfc/RFC-0023-production-publisher-policy-v0.md)
- [RFC-0024](../rfc/RFC-0024-signed-registry-metadata-v0.md)
- [signed registry review](../reports/signed-local-registry-v0.md)
