# Production deployment origin v0 review

> - status: complete-local
> - step: STEP-0074
> - date: 2026-07-17
> - platform: Windows x86_64 loopback
> - public production evidence: false

## Decision

Sico now has a production-capable read origin and release artifact, but not a publicly deployed production service. `sico-registry` serves an existing signed RFC-0024 transport tree without holding publisher keys or accepting mutations. `sico-app dev` reduces the normal local source workflow to one command while preserving the compiler/application process boundary.

ADR-0008 is accepted for this v0 origin boundary. Public GO still requires owner-selected infrastructure, domain/TLS, production publisher identity/key custody, external availability and restore evidence.

## Implemented evidence

| Surface | Result | Evidence level |
|---|---|---|
| liveness/readiness | `/healthz` and `/readyz` return bounded JSON | verified on loopback |
| immutable object read | exact `GET` bytes and metadata-only `HEAD` | verified on loopback |
| mutation refusal | non-GET/HEAD returns `405` | verified |
| path boundary | traversal, encoding, query, directory and missing paths fail closed | verified |
| resource bounds | request header, file bytes, I/O time and connections bounded | verified |
| network exposure | loopback default; non-loopback requires explicit acknowledgement | verified by CLI contract |
| developer workflow | `sico-app dev ... answer.sico` returns `42` | Runtime-verified |
| operator bundle | release binary, empty data root, README, launcher, license and manifest | packaged and self-checked |
| release integration | Windows release contains the separate operator ZIP and manifest | verified with non-publishable dirty smoke |
| public service | no domain, TLS, production identity or remote probe | blocked/external |

## Measurements

Measured on 2026-07-17 from the STEP-0074 release smoke:

- optimized `sico-registry.exe`: 1,973,760 bytes;
- deterministic operator ZIP: 730,797 bytes;
- operator ZIP SHA-256 on two consecutive builds: `4f0a38b6654184cd47d3a702ec956e1ad3dce4b1f9a3486addbb8b9ae74f4756`;
- one-command application result: `42`;
- new HTTP integration tests: 3 passed;
- workspace module packages: 23.

The size values are regression observations, not SLAs. The archive was produced with `-AllowDirty -SkipQualityGates` only to validate release composition and was correctly marked `DO-NOT-PUBLISH`; it is not a release candidate.

## Reproduction

```powershell
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
& .\tools\validate-module-boundaries.ps1
& .\tools\validate-step-0074.ps1
& .\tools\package-registry-origin.ps1
```

## Residual production gates

1. Select and authorize the hosting account/region and production hostname.
2. Configure real TLS, edge limits, monitoring and alerting.
3. Define legal publisher identity, namespace, offline/online custody roles and recovery owners.
4. Publish real signed metadata without placing keys in the origin.
5. Run external availability, cache, failure, backup and restore drills.

## Links

- [STEP-0074](../steps/STEP-0074-production-deployment-origin-and-ux.md)
- [ADR-0008](../adr/ADR-0008-production-registry-origin.md)
- [Operator guide](../../deploy/registry-origin/README.md)
