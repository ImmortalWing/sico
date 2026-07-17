# STEP-0074: Production deployment origin and one-command workflow

> status: complete-local
> phase: M7 production deployment track
> started: 2026-07-17
> completed: 2026-07-17
> owners: autonomous-agent

## 1. Objective

Turn the repository-local release chain into an operator-deployable origin and remove the three-command source → Component → `.sapp` → Runtime loop from the normal developer path.

The verifiable result is:

1. a bounded, read-only HTTP origin can serve an RFC-0024 registry tree with health/readiness endpoints;
2. the origin fails closed on traversal, mutation methods, oversized requests and non-loopback exposure without explicit operator consent;
3. an installed SDK can run one source file through the existing compiler/package/Runtime boundaries with one `sico-app dev` command;
4. packaging and smoke scripts produce a self-contained operator bundle without requiring Docker;
5. documentation distinguishes a production-capable origin from a publicly deployed production service.

## 2. Context and evidence

- The owner requested an actual production deployment environment and a simpler use flow on 2026-07-17.
- [`M7 exit audit`](../reports/m7-exit-audit.md) records that signed registry semantics are locally complete but a public service and real production identity are absent.
- [`ADR-0006`](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md) requires the transport to remain untrusted and every artifact to be verified locally.
- [`ADR-0007`](../adr/ADR-0007-openjdk-style-modular-monorepo.md) keeps compiler, application and Host dependency ownership separate.
- The SDK already ships both `sico` and `sico-app`, but the convenient source workflow is PowerShell-only.

## 3. Scope

Included:

- a std-based `sico-registry` origin binary;
- health, readiness, GET and HEAD endpoints;
- immutable registry-tree serving with bounded concurrency and request/file sizes;
- a portable production-origin bundle and local smoke test;
- `sico-app dev` as shell-free process orchestration over the existing compiler and Runtime;
- user/operator documentation and audit validators.

Excluded:

- creating a legal publisher identity or production signing keys;
- accepting cloud/vendor terms, buying a domain or publishing publicly;
- TLS termination inside the origin binary;
- mutable upload APIs, registry-account auth, search or billing;
- claiming Linux, mobile or Internet production evidence from a Windows loopback test.

## 4. Options and decision

### Container-first service

Rejected as the baseline because this runner has neither Docker nor Podman. A container remains a packaging adapter, not a runtime prerequisite.

### General mutable registry API

Deferred. It adds authentication, rate limiting, database migrations and key-custody risk before the read path is proven. RFC-0024 records and blobs are immutable, so an offline publisher plus a read-only origin is the smaller fail-closed production boundary.

### Read-only native origin behind TLS reverse proxy

Accepted. The origin binds loopback by default, serves only an already-verified registry tree, does not receive signing keys, and leaves certificates, domains and edge controls to an operator-selected reverse proxy.

For developer UX, a new crate dependency from the compiler to Runtime was rejected. `sico-app dev` invokes the sibling `sico` executable directly and then uses its existing package/Runtime implementation, preserving the module boundary.

## 5. Plan

1. Record the production deployment architecture in ADR-0008.
2. Add the bounded read-only origin and adversarial HTTP tests.
3. Add `sico-app dev` and end-to-end CLI tests.
4. Add operator packaging/smoke scripts and deployment documentation.
5. Run targeted and workspace quality gates; synchronize roadmap/status/handoff.

## 6. Changes

- Added the `sico-registry-server` workspace crate and `sico-registry` operator binary.
- Added bounded HTTP/1.1 health/readiness, immutable GET/HEAD serving, canonical-root enforcement, method refusal and resource limits.
- Added `sico-app dev`, sibling compiler discovery and installed-SDK Runtime discovery without introducing a compiler crate dependency.
- Added `package-registry-origin.ps1`, a self-contained operator ZIP/manifest, STEP validator and Windows release integration.
- Updated the module-boundary contract from 22 to 23 packages and recorded the explicit process-orchestration exception in ADR-0007.
- Updated user, operator, release and architecture documentation.

## 7. Validation

Executed successfully:

```powershell
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
& .\tools\validate-step-0074.ps1
& .\tools\validate-module-boundaries.ps1
& .\tools\release-windows.ps1 -AllowDirty -SkipQualityGates -OutputRoot target/step-0074-release
```

The release-composition run intentionally produced `publishable=false` and `DO-NOT-PUBLISH.txt`; it validated packaging only. No external upload occurred.

## 8. Metrics

- `sico-registry.exe`: 1,973,760 bytes (optimized Windows GNU build).
- deterministic operator ZIP: 730,797 bytes, SHA-256 `4f0a38b6654184cd47d3a702ec956e1ad3dce4b1f9a3486addbb8b9ae74f4756` on two consecutive builds.
- HTTP origin integration tests: 3 passed.
- `sico-app dev` real Runtime result: `42`.
- module-boundary validator: 23 workspace packages.
- Full workspace format, Clippy and tests: passed.

## 9. Risks and follow-ups

- A loopback smoke is not public-service evidence; STEP-0074 is `complete-local`, not public-production GO.
- TLS, DDoS controls, backups and observability depend on the selected production host.
- Public publication remains blocked on owner-controlled identity, domain, custody and legal decisions.
- A later step may add a provider-specific deployment adapter after the owner selects infrastructure.

## 10. Audit links

- [`ADR-0008`](../adr/ADR-0008-production-registry-origin.md)
- [`RFC-0024`](../rfc/RFC-0024-signed-registry-metadata-v0.md)
- [`M7 exit audit`](../reports/m7-exit-audit.md)
- [`production deployment review`](../reports/production-deployment-origin-v0.md)
