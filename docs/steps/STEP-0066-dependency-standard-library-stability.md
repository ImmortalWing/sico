# STEP-0066: Standard-library and dependency stability

> - status: complete
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Freeze independent compatibility identifiers and implement deterministic source-aware dependency resolution, canonical immutable lock graphs, yank/revocation policy and verified transitive capability closure.

## 2. Scope

Included: canonical SemVer precedence/ranges; exact registry/namespace/package identities; explicit compatibility profiles; deterministic highest compatible resolution; ambiguity/cycle/conflict rejection; canonical lock digest; standard-library module/API contract; yank versus revocation; Host/user capability intersection; local fixtures and corpus.

Excluded: public package index, network fetching, SAT features, build scripts, native dependencies, production registry mutation and platform-specific standard-library adapters.

## 3. Plan

1. Freeze compatibility and standard-library records. — complete
2. Implement canonical SemVer and bounded author ranges. — complete
3. Resolve only exact source identities into immutable nodes. — complete
4. Verify graph closure, compatibility and capabilities. — complete
5. Separate new-resolution yank from explicit existing-lock revocation. — complete
6. Add corpus, RFC, report, validator and regression. — complete

## 4. Changes and validation

Added canonical SemVer precedence and exact/caret/tilde/interval requirements; exact registry/namespace/package identities; highest-compatible deterministic resolution; ambiguity, cycle and conflict refusal; immutable lock nodes; reachable graph verification; eleven independent non-app compatibility fields; standard-library module API digests; exact root/transitive capability closure; Host grant intersection; and separate yank/revocation behavior.

Eight tests passed. Catalog reversal produces byte-identical locks. A higher attacker version at another registry is ignored. All 1,140 one-byte lock mutations fail the externally bound digest. The 32-case corpus and seven assigned threats validate mechanically.

Result: `STEP_0066_OK tests=8 mutations=1140 cases=32 threats=7 nodes=2 semver=canonical confusion=rejected capabilities=closed external_side_effects=0 next=STEP-0067`.

## 5. Risks

The catalog is a verified local input, not a public index client. v0 intentionally excludes feature solving, optional/build/native dependencies and network behavior. SemVer numeric identifiers are `u64` bounded. Standard-library contracts freeze module API digests but do not claim implementations for deferred platforms.

## 6. Links

- [RFC-0026](../rfc/RFC-0026-dependency-lock-compatibility-v0.md)
- [review](../reports/dependency-standard-library-v0.md)
