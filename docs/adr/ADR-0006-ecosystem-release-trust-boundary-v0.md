# ADR-0006: Ecosystem and release trust boundary v0

> - status: accepted
> - date: 2026-07-16
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -

## Context

M4 established canonical `.sapp` bytes, strict local verification and development signatures. M5 established immutable installed revisions and app identity derived from app id plus signer. Those controls do not establish a production publisher, namespace ownership, registry freshness, revocation or secure update policy. An untrusted registry can replay, omit, fork or substitute metadata even when individual blobs are content-addressed.

M6 remains blocked on unavailable Android tooling and runners. Repository-owner direction on 2026-07-16 defers Android and HarmonyOS/OpenHarmony tracks while allowing platform-independent ecosystem work to proceed. This decision does not mark M6 complete and does not permit mobile support claims.

Relevant external baselines are the [TUF specification v1.0.35](https://theupdateframework.github.io/specification/v1.0.35/), [Sigstore identity and transparency model](https://docs.sigstore.dev/), [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md) and [Semantic Versioning 2.0.0](https://semver.org/).

## Decision drivers

- registry compromise must not authorize code;
- signing-key compromise must be recoverable without promoting development keys;
- namespace, publisher, artifact and release identity must not collapse into one mutable string;
- offline inspection and last-known-good recovery must remain possible;
- compatibility decisions must be deterministic and fail closed;
- early ecosystem design must remain independent of Android and Harmony platform adapters.

## Considered options

### Make the registry the trust root

Simple to operate, but a registry or account compromise can replace packages, rewrite history or seize names. TLS protects a connection, not repository history or publisher authorization. Rejected.

### Trust one long-lived publisher key embedded in every package

Improves artifact integrity but has weak recovery, rotation and role separation. It also tempts reuse of M4 development keys. Rejected as the production model.

### Signed role-separated metadata over content-addressed artifacts

The registry transports immutable blobs and signed records. Offline/root, online release, recovery, rotation and revocation roles are distinct; thresholds and identity claims are policy. Clients retain trusted version state and verify digest, release policy and `.sapp` locally. Accepted.

Sigstore-style identity certificates and transparency proofs remain an allowed publisher credential, not the only root model. A future implementation must pin issuer and subject/workflow policy and must not accept “present in a log” as sufficient authorization.

## Decision

1. Registry, mirror, CDN, tag, channel and search results are untrusted discovery inputs.
2. Package blobs are immutable by SHA-256 digest and byte length. After download, clients repeat RFC-0015 structural/hash/Component checks, production publisher verification and capability closure locally.
3. Production trust uses signed, versioned role metadata with explicit root, release, recovery, rotation and revocation authority. Development keys from RFC-0016 are never production roots.
4. Namespace grants and transfers are signed registry governance events. A `.sapp` `app.id`, DNS-like spelling, first upload or display name does not prove namespace ownership.
5. Release selection uses signed immutable records and monotonic trusted state. Snapshot/release versions cannot decrease; online metadata expires; one snapshot binds a consistent repository view.
6. Activation is staged and atomic. The last verified revision remains recoverable until the new revision passes every gate. Automatic rollback is not an exception to signature, identity or capability checks.
7. Transparency/inclusion evidence is required for public publication design, but it supplements rather than replaces publisher authorization and local verification.
8. Android and Harmony platform delivery are deferred tracks. STEP-0062–0068 may proceed where their evidence is platform-neutral; STEP-0069 and the final M7 exit remain blocked on declared platform pilots, including the unresolved M6 gate unless a later accepted roadmap decision changes product support.

## Consequences

Positive: registry compromise alone cannot authorize execution; content can be mirrored safely; key recovery and namespace transfer have explicit audit surfaces; desktop tools can advance while mobile runners are unavailable.

Cost: production publication needs multiple signed record types, retained client state, expiry handling, checkpoint/inclusion evidence and owner decisions about legal publisher identity, key custody and public infrastructure. Offline clients need an explicit stale-metadata policy rather than silently accepting old state.

## Validation

- [`threat-matrix.json`](../../tests/ecosystem/threat-matrix.json) enumerates 32 publisher, namespace, registry, update, dependency, disclosure, rollback and compatibility threats.
- [`compatibility-matrix.json`](../../tests/ecosystem/compatibility-matrix.json) keeps 12 version/trust surfaces separate.
- [`validate-step-0062.ps1`](../../tools/validate-step-0062.ps1) checks coverage, unique identities, accepted documents and the retained mobile blocker.
- No production key, registry service, public namespace or live signing was created in STEP-0062.

## Revisit conditions

- production publisher identity or key-custody owner decision;
- a selected transparency or registry implementation cannot provide offline-verifiable evidence;
- SHA-256 or Ed25519 deprecation requires a new algorithm transition contract;
- Android or Harmony product scope changes the final supported-platform exit gate.

## Links

- [`STEP-0062`](../steps/STEP-0062-ecosystem-release-contract.md)
- [`RFC-0022`](../rfc/RFC-0022-ecosystem-compatibility-contract-v0.md)
- [`RFC-0015`](../rfc/RFC-0015-sapp-package-format-v0.md)
- [`RFC-0016`](../rfc/RFC-0016-development-signing-trust-v0.md)

