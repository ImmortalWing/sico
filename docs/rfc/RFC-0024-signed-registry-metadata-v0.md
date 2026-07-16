# RFC-0024: Signed registry metadata and local transport v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: ecosystem registry metadata v0
> - supersedes: -
> - superseded-by: -

## Summary

Define canonical signed namespace, immutable release, monotonic channel and append-only checkpoint records. Registry transport remains untrusted: clients verify authority, publisher policy, byte length, digest and the strict `.sapp` format locally.

## Problem

A registry account, first upload, mutable tag or successful download does not prove namespace ownership, publisher authorization, immutable package identity or a consistent publication history. Transport metadata can be altered by a registry, mirror, cache or local filesystem fault.

## Goals and non-goals

Goals: deterministic bounded records; explicit namespace ownership; visible transfer and delayed reuse; registry/publisher role separation; signed release discovery; content-addressed bytes; freshness and rollback checks; append-only checkpoint evidence; strict post-download package verification.

Non-goals: public registry protocol, legal namespace policy, public transparency service, network authentication, billing, production key custody, secure update activation or dependency resolution.

## Record semantics

All records are compact canonical UTF-8 JSON with fixed schema strings, unknown-field rejection, bounded lowercase ASCII identifiers, lowercase SHA-256 digests and Ed25519 domain-separated signatures. Registry authority keys are pinned independently from `PublisherPolicy`; a key cannot serve both authority sets.

`sico.registry.namespace-event.v0` is a monotonic digest chain. A genesis grant requires the registry threshold and the new publisher root threshold. Transfer additionally retains the previous and next exact policy references and requires both publisher root thresholds. Tombstone retains the previous owner and a future `reuse_after`; a later grant is valid only at or after that time. Every event has a bounded audit digest.

`sico.registry.release.v0` is immutable and release-role signed. It binds the exact publisher policy, namespace event, `.sapp` SHA-256 and length, app ID/version and all twelve RFC-0022 compatibility surfaces represented by package format, manifest schema, language semantics, Component world, WASI, capability, dependency lock and minimum Host fields. SemVer 2.0.0 pre-release and build metadata are accepted; non-canonical core and numeric pre-release identifiers fail closed.

`sico.registry.channel.v0` is a signed discovery event, not an artifact identity. Each package/channel stores immutable sequence-numbered records and advances by exactly one. Resolution yields only a release-record digest.

`sico.registry.checkpoint.v0` is registry-threshold signed and digest-chained. Its sorted unique namespace, release and channel inclusion sets are append-only. Historical checkpoint signature verification does not require unexpired historical metadata, but the newly trusted checkpoint must be current and its issuance cannot precede its predecessor.

## Client and local transport workflow

Publish verifies the current publisher policy, namespace grant, release signature, package length/digest and strict `.sapp` parser before writing. Blobs and release records use digest paths and create-new immutable writes. Channel history uses immutable sequence files, avoiding platform-dependent replacement semantics.

Discovery verifies the highest valid signed channel event. Download fetches release metadata and blob by digest, rechecks namespace ownership, policy reference, byte length and SHA-256, reruns `sico_package::verify`, and compares manifest app ID/version. Filesystem paths and transport metadata never add authority.

The design follows TUF's version/expiry, rollback, mix-and-match and consistent-snapshot principles while remaining a narrower Sico v0 record set. It follows OCI's distinction between mutable tags and digest-addressed content and its requirement that clients verify returned content. Public Rekor-style inclusion proofs are not claimed; local signed checkpoints are independently cacheable evidence only.

## Positive and negative cases

Positive: root/authority namespace grant; dual-owner transfer; tombstone and delayed reuse; signed immutable release; exact channel progression; three-generation append-only checkpoint; inert markup-looking description; exact download roundtrip.

Negative: first-upload inference; Unicode/case impersonation; wrong predecessor digest; missing owner threshold; immediate reuse; authority/publisher key overlap; stale policy; wrong role; channel rollback/gap; missing release; altered length/digest/package; checkpoint fork/removal; unknown schema/field; non-canonical JSON or SemVer; signed-byte mutation.

## Security and privacy

Descriptions are returned as bounded text and are never interpreted as HTML, shell or template input. Errors do not echo package content, credentials, keys, host paths or user data. Test signing helpers and deterministic seeds are fixtures, not a production ceremony. Trusted time is caller supplied. A public namespace, transparency identity or external service requires owner authorization and a separate operational review.

## Alternatives

- Trust registry TLS/account: rejected because transport compromise would become publisher authority.
- Treat a channel as identity: rejected because mutable names permit rollback and substitution.
- Replace channel files atomically: rejected for v0 because replacement behavior differs on Windows; immutable sequence events are simpler and auditable.
- Adopt a complete TUF repository: deferred to STEP-0065 update activation; this step freezes the narrower publish/discovery contract.
- Claim local checkpoints as public transparency: rejected because no independent public witness exists.

## Validation and acceptance criteria

Acceptance requires all ten assigned threat cases, the 32-case machine corpus, namespace lifecycle, immutable local publish/discovery/download, at least three checkpoint generations, every one-byte release-record mutation rejected, strict package re-verification, full workspace regression and zero external side effects.

## Links

- [STEP-0064](../steps/STEP-0064-signed-local-registry.md)
- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0022](./RFC-0022-ecosystem-compatibility-contract-v0.md)
- [RFC-0023](./RFC-0023-production-publisher-policy-v0.md)
- [TUF specification](https://theupdateframework.github.io/specification/latest/)
- [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md)
- [Semantic Versioning 2.0.0](https://semver.org/)
- [Sigstore Rekor overview](https://docs.sigstore.dev/logging/overview/)
