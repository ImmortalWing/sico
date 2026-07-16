# RFC-0026: Dependency lock and compatibility v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: dependency resolver v0
> - supersedes: -
> - superseded-by: -

## Summary

Define source-aware dependency requirements, canonical SemVer selection, immutable lock graphs, independent compatibility surfaces, standard-library API identity and exact capability closure.

## Semantics

A dependency identity is the byte-exact lowercase tuple `registry_id/namespace/package_name`; source order and display names never supply fallback authority. Requirements support exact, caret, tilde and one lower-inclusive/upper-exclusive interval over bounded canonical SemVer 2.0.0. Prereleases require explicit prerelease bounds. Build metadata has no precedence; two different candidates at equal precedence are rejected as ambiguous rather than tie-broken silently.

Resolution chooses the highest compatible non-yanked, non-revoked candidate for each exact identity. Cycles, conflicting transitive requirements, missing nodes and unknown compatibility identifiers fail closed. Catalog order cannot change output.

`sico.dependency.lock-graph.v0` sorts roots and nodes and binds publisher-policy ID, exact SemVer, release/package digests, eleven non-app compatibility identifiers, dependency edges and capabilities. Its SHA-256 is carried by the release record. Verification requires canonical bytes, the expected digest, exact catalog content and a reachable acyclic graph.

Root capabilities plus every locked node capability must exactly equal the declared closure and remain a subset of Host/user grants. Dependencies gain no ambient authority.

Yank excludes a candidate only from new resolution; it cannot rewrite an existing exact lock. Revocation is evaluated separately and defaults to deny. Recovery may allow only an explicitly enumerated package digest.

`sico.standard-library.contract.v0` has its own SemVer and sorted module/API digests plus language, Component, WASI and capability contracts. The lock binds its digest and the Host must support those identifiers. Compiler, IR, package, manifest, publisher, registry, application, capability and Host versions remain independent.

## Security and limitations

Candidate records are verified catalog inputs derived from STEP-0064 releases; this local resolver does not fetch or authenticate a public index. v0 has no features, optional dependencies, SAT backtracking, build scripts or native packages. Numeric SemVer core/prerelease components are bounded to `u64` and oversized values fail closed.

## Validation

Acceptance requires seven assigned threats, 32 machine cases, deterministic catalog-order tests, confusion/cycle/conflict/capability/yank/revocation/compatibility negatives, 1,140 lock mutations, full regression and no external service.

## Links

- [STEP-0066](../steps/STEP-0066-dependency-standard-library-stability.md)
- [RFC-0022](./RFC-0022-ecosystem-compatibility-contract-v0.md)
- [RFC-0025](./RFC-0025-secure-update-recovery-v0.md)
- [Semantic Versioning 2.0.0](https://semver.org/)
