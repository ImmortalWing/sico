# RFC-0022: Ecosystem release and compatibility contract v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: ecosystem metadata v0
> - supersedes: -
> - superseded-by: -

## Summary

Define fail-closed identities, canonical release metadata and independent compatibility axes for Sico publication, discovery, dependencies and updates without changing `.sapp` v0 or treating a registry as a trust root.

## Problem

The current manifest accepts a broad app version string and carries a development signature. It does not answer who owns a public namespace, which production policy authorized a release, whether metadata is fresh and consistent, how dependencies are locked, or whether a Host supports the package's language/WIT/capability contracts. Guessing from strings would make update, downgrade and dependency-confusion failures likely.

## Goals and non-goals

Goals:

- define exact identity tuples and canonical record behavior;
- separate application release precedence from package, language, WIT, runtime and capability compatibility;
- preserve local `.sapp` verification and capability intersection;
- define the input contract for STEP-0063–0066 implementation.

Non-goals:

- choosing the legal publisher identity, production root keys, registry domain or service account;
- implementing network APIs, production signing, dependency resolution or updates;
- completing Android or Harmony support;
- changing RFC-0015 `.sapp` bytes or RFC-0016 development signatures.

## Identity and release records

The production identity tuple is:

```text
registry-id / namespace / package-name / publisher-policy-id
```

The immutable release tuple adds canonical SemVer and the exact package digest:

```text
production-identity / app-version / sha256(package-bytes)
```

`manifest.app.id` must equal the app id authorized by the release record, but does not itself prove public ownership. A release record also binds package byte length, `.sapp` format, manifest schema, language semantics, Component/WIT world, WASI contract, capability contract, dependency-lock digest, minimum Host contract and publisher policy version.

Security-relevant records use schema-tagged canonical UTF-8 JSON, fixed field order, sorted unique arrays, bounded strings/counts, lowercase hex digests and unknown-field rejection. The signature input is domain-separated by record type and schema. Concrete byte layouts and limits belong to STEP-0063/0064 and require fixtures before acceptance.

Tags and channels are mutable discovery names only. Resolution returns a signed immutable release record. Clients persist the highest trusted root/snapshot/release sequence and relevant expiry/checkpoint state; a lower or inconsistent state is rejected before package activation.

## Compatibility

The normative surfaces are listed in [`compatibility-matrix.json`](../../tests/ecosystem/compatibility-matrix.json). Their identifiers are independent:

- language semantics and compiler version;
- Sico IR schema;
- Component/WIT world and WASI contract;
- `.sapp` container format and manifest schema;
- production publisher policy and release-record schema;
- application SemVer;
- capability and Host contracts.

A higher application version never implies compatibility with an unknown package, WIT, capability or Host contract. Unknown security-relevant schema, field, enum, algorithm or capability fails closed. Platform parity is evidence, not inference: an unsupported Android or Harmony adapter cannot be relabeled compatible from Desktop results.

Published `app-version` is canonical [SemVer 2.0.0](https://semver.org/). `0.y.z` remains unstable. Build metadata does not affect precedence and must not select different package bytes under the same immutable release identity. Existing M4 development versions remain accepted locally; production publish must diagnose non-canonical SemVer.

## Dependencies

Author-facing ranges may exist in STEP-0066, but every published release binds one canonical immutable lock graph. Each node includes registry id, namespace, package name, exact SemVer, package digest and required compatibility identifiers. Resolver source order is never implicit. The verified transitive capability closure must still equal declared requirements and intersect Host/user grants; dependencies cannot introduce ambient authority.

Yank prevents new resolution but does not rewrite an existing lock. Revocation is a signed security decision evaluated by explicit client policy. Neither silently substitutes different bytes for a locked digest.

## Update and rollback semantics

Update is `discover → verify signed snapshot/release → fetch by digest → verify length/digest → strict .sapp verify → production identity verify → capability/dependency compatibility → stage → atomic activate`.

Failure leaves the active revision unchanged. Recovery may activate a retained older revision only under a signed recovery authorization or an explicit local recovery policy; it cannot cross production identity and cannot restore stale permission records when the capability fingerprint changed.

## Security and privacy

The 32-case [`ecosystem threat matrix`](../../tests/ecosystem/threat-matrix.json) is normative input to later security tests. Registry text is bounded inert data. Logs and records must not contain private keys, access tokens, host paths or user data. Identity-based signing may disclose an OIDC identity in a public transparency log, so the selected production policy requires an explicit privacy decision in STEP-0063.

## Alternatives

- One global version: rejected because it couples unrelated compatibility axes and encourages unsafe guessing.
- Registry account equals publisher: rejected because account compromise or provider migration would redefine application identity.
- Floating dependencies at install time: rejected because identical release metadata would resolve to different code and authority.
- Embed all ecosystem metadata inside `.sapp` v0: rejected because it would conflate portable package bytes with registry freshness, namespace governance and update state.

## Validation and acceptance criteria

STEP-0062 acceptance requires:

- 32 unique threats covering eight areas;
- 12 unique compatibility surfaces with explicit unknown behavior;
- accepted ADR-0006 and this RFC;
- a reproducible validator that also proves M6 remains blocked and the platform recheck does not claim mobile evidence;
- no production credential, public namespace or external publish side effect.

STEP-0063–0066 subsequently accepted the repository-local implementation. STEP-0069 records the final disposition: the local M7 track is complete, while product acceptance remains blocked on independent, production and target-platform evidence; Desktop-only evidence cannot claim mobile support.

## Links

- [`STEP-0062`](../steps/STEP-0062-ecosystem-release-contract.md)
- [`ADR-0006`](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [TUF specification v1.0.35](https://theupdateframework.github.io/specification/v1.0.35/)
- [Sigstore overview](https://docs.sigstore.dev/)
- [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md)
