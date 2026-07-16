# STEP-0062: Ecosystem/release threat model and compatibility contract

> - status: complete
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Freeze the platform-independent trust and compatibility boundary required before production publisher, registry, update or dependency implementation, while preserving the truthful M6 Android blocker and deferring the unplanned Harmony track.

## 2. Context and evidence

RFC-0015–0019 provide canonical `.sapp`, development signing, capability closure and local Host verification. The new platform recheck proves that Android SDK/NDK/ADB/Gradle/Java/device and Android Rust targets are absent; no Harmony toolchain, plan or code exists. Rust stable 1.97.0 remains usable for platform-independent work through the explicit installed toolchain.

External primary references reviewed on 2026-07-16: TUF specification v1.0.35, Sigstore identity/transparency documentation, OCI Distribution Specification and SemVer 2.0.0.

## 3. Scope

Included: publisher/namespace/registry/update/dependency/disclosure/rollback/compatibility threats; role-separated trust; immutable release identity; version surface separation; mobile deferral boundary; reproducible contract validation.

Excluded: production credentials, legal publisher identity, public namespace, live registry, network protocol implementation, production signing, updates, dependency resolver, Android completion and Harmony implementation.

## 4. Options and decision

Registry-rooted trust and one long-lived production key were rejected. Accepted ADR-0006 uses role-separated signed metadata over content-addressed `.sapp` bytes, retained monotonic client state and mandatory local re-verification. RFC-0022 keeps application SemVer separate from package, schema, language, WIT, capability, publisher-policy and Host compatibility.

The earlier serial milestone gate was narrowed by repository-owner direction: platform-neutral STEP-0062–0068 work may proceed, but M6 remains NO-GO and mobile evidence remains mandatory for any mobile support or final project-completion claim.

## 5. Plan

1. Recheck the actual local mobile and Rust environment.
2. Audit current package/trust/Host contracts.
3. Create threat and compatibility matrices.
4. Accept ADR-0006 and RFC-0022 within contract-only scope.
5. Add a validator and synchronize project status/roadmap/handoff.

## 6. Changes

- Added 32-case ecosystem threat matrix across eight areas.
- Added 12-surface compatibility matrix.
- Added an immutable environment recheck instead of rewriting the earlier Android snapshot.
- Accepted ADR-0006 and RFC-0022.
- Changed M7 to a platform-independent in-progress track with explicit mobile exit gates.
- Corrected stale ADR/RFC/STEP indexes through STEP-0062.
- Updated the STEP-0061 validator's sequencing assertion while retaining its Android runner NO-GO checks.

## 7. Validation

```powershell
& .\tools\validate-step-0062.ps1
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:__COMPAT_LAYER = 'RunAsInvoker'
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
git diff --check
```

Contract validator result: `STEP_0062_OK threats=32 areas=8 compatibility_surfaces=12 trust=role-separated registry=untrusted mobile=deferred m6=blocked next=STEP-0063`.

The Rust commands use the installed stable toolchain, whose compiler is exactly 1.97.0. The version-named pinned toolchain is not installed. `RunAsInvoker` prevents Windows installer-name heuristics from demanding elevation for the existing `install_open-*.exe` test binary; the process otherwise fails before test code executes with OS error 740. Android target checks were not run and no mobile execution is claimed.

## 8. Metrics

- 32 threats, four in each of eight areas.
- 12 independent compatibility surfaces.
- No runtime performance measurement; this is a contract step.

## 9. Risks and follow-ups

- STEP-0063 still requires owner decisions for production identity, key custody, recovery and privacy of transparency identities.
- STEP-0064 requires explicit authorization before any public registry or namespace operation.
- Android remains blocked on both implementation and runners. Harmony is only deferred scope, not an established supported target.
- Final M7/project completion cannot be claimed from platform-neutral work alone.

## 10. Audit links

- [`ADR-0006`](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [`RFC-0022`](../rfc/RFC-0022-ecosystem-compatibility-contract-v0.md)
- [`review report`](../reports/ecosystem-release-contract-v0.md)
- [`threat matrix`](../../tests/ecosystem/threat-matrix.json)
- [`compatibility matrix`](../../tests/ecosystem/compatibility-matrix.json)
- [`environment recheck`](../../tests/platform/environment-2026-07-16-recheck.json)
