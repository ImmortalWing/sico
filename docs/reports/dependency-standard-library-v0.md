# Dependency and standard-library stability v0 review

> - status: complete
> - date: 2026-07-16
> - related-step: STEP-0066
> - environment: Windows x86_64; Rust 1.97.0; in-memory local catalog

## Result

`sico-ecosystem` now resolves exact source identities into deterministic immutable lock graphs. Eight tests cover catalog-order determinism, dependency confusion, SemVer/prerelease/build rules, cycles/conflicts, capability closure, yank/revocation, eleven independent non-app compatibility fields and standard-library API identity.

| Metric | Value |
|---|---:|
| tests | 8 passed |
| lock mutations | 1,140/1,140 rejected |
| machine cases | 32 |
| assigned threats | 7 |
| resolved fixture nodes | 2 |
| external operations | 0 |

The resolver ignores a higher attacker version at another registry because requirements include registry, namespace and package. Lock nodes bind exact publisher/release/package/compatibility data. Root and transitive capabilities must exactly equal the declared closure and intersect grants. Yank changes new selection only; revoked locks require an exact explicit override.

Limitations: catalog entries are already-verified local inputs; no public index, features, build scripts, native packages or SAT solver exists. SemVer numeric values are `u64` bounded. Standard-library module hashes freeze API identity, not an implementation or platform adapter.

## Reproduction

```powershell
& "$HOME/.cargo/bin/cargo.exe" test --offline -p sico-ecosystem --test dependency_resolution -- --nocapture
& .\tools\validate-step-0066.ps1
```

## Links

- [STEP-0066](../steps/STEP-0066-dependency-standard-library-stability.md)
- [RFC-0026](../rfc/RFC-0026-dependency-lock-compatibility-v0.md)
- [resolver source](../../crates/sico-ecosystem/src/dependency.rs)
- [case corpus](../../tests/ecosystem/dependency-stability-cases.json)
