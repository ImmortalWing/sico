# Secure update and recovery v0 review

> - status: complete
> - date: 2026-07-16
> - related-step: STEP-0065
> - environment: Windows x86_64 host; Rust 1.97.0 stable GNU toolchain; local filesystem transport

## 1. Question

Can Sico apply and recover local production releases without accepting replay, freeze, mixed metadata, partial activation, advisory suppression or implicit rollback?

## 2. Method

Added four canonical update schemas and a local `UpdateClient`. Two deterministic `.sapp` revisions use the STEP-0063 publisher policy and STEP-0064 namespace/release/channel/checkpoint records. The second revision adds a real `wasi:clocks` Component import and `clock.read` capability so permission re-evaluation is exercised from verified package content.

Activation writes staged bytes, re-reads and verifies them, persists immutable metadata/revision files, then creates the next contiguous checksummed state generation. Tests cover replay, mix-and-match, corrupt staging, retry, restart, missing revision, state tampering, advisory exactness, signed recovery and explicit local recovery.

## 3. Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:__COMPAT_LAYER = 'RunAsInvoker'
& "$HOME/.cargo/bin/cargo.exe" test --offline -p sico-ecosystem --test secure_update -- --nocapture
& "$HOME/.cargo/bin/cargo.exe" clippy --offline -p sico-ecosystem --all-targets -- -D warnings
& .\tools\validate-step-0065.ps1
```

`RunAsInvoker` prevents Windows installer-name heuristics from intercepting the `secure_update-*.exe` test binary with OS error 740 before Rust test code starts. It does not elevate the process.

## 4. Raw evidence

- [`update.rs`](../../crates/sico-ecosystem/src/update.rs)
- [`secure_update.rs`](../../crates/sico-ecosystem/tests/secure_update.rs)
- [`secure-update-cases.json`](../../tests/ecosystem/secure-update-cases.json)
- [`threat-matrix.json`](../../tests/ecosystem/threat-matrix.json)

Every file mutation is confined to unique process-scoped temporary roots. No network endpoint, public advisory, production credential, registry account or mobile installer is used.

## 5. Results

| Result | Value | Evidence label |
|---|---:|---|
| secure-update tests | 8 passed | verified |
| signed snapshot mutations | 1,592/1,592 rejected | verified |
| machine-readable cases | 36 | verified |
| assigned ecosystem threats | 10 | verified |
| first `.sapp` revision | 590 bytes | measured |
| second capability-bearing revision | 679 bytes | measured |
| first trusted state | 1,036 bytes | measured |
| second trusted state | 1,098 bytes | measured |
| retained revisions after update | 2 | verified |
| recovery modes | signed threshold + explicit local | verified |
| external operations | 0 | verified by scope and test-root review |

## 6. Interpretation

The implementation closes ECO-UPDATE-001–004, ECO-DISCLOSURE-002–003 and ECO-ROLLBACK-001–004 at the platform-independent local level. A transport attacker can withhold data and cause denial of service, but cannot activate mixed, stale or corrupt bytes. Corrupt staging leaves the active state unchanged and a valid retry succeeds.

Recovery is a new append-only activation event, not trust-state rollback. Latest policy/checkpoint/snapshot/channel/release/advisory sequences remain retained while the active package points to a previously verified revision. Both recovery modes bind exact current/target digests; app ID and production identity cannot change. Capability drift forces permission review.

## 7. Limitations

- The initial `PublisherPolicy` is a caller-pinned trust input. v0 update activation requires its exact version/digest; policy advance remains an explicit STEP-0063 ceremony and is not inferred from registry metadata.
- The state checksum detects corruption but is not a hardware anti-rollback counter. The update root requires OS access control; whole-directory restore by a local attacker needs a later platform secure-storage adapter.
- STEP-0064 local namespace consumption currently uses the genesis grant; hosted latest namespace-history selection remains service work.
- Advisory completeness is relative to retained signed snapshots. There is no public witness, gossip or guarantee that an unreachable server disclosed all issues.
- There is no OS package manager, launcher health check, delta transfer, network retry policy or Android/Harmony activation.

## 8. Decision impact

RFC-0025 is accepted for local update/recovery v0. STEP-0066 can now resolve dependencies to immutable locked release/package digests and feed the verified transitive capability closure into this activation boundary.

## 9. Links

- [STEP-0065](../steps/STEP-0065-secure-update-rollback.md)
- [RFC-0025](../rfc/RFC-0025-secure-update-recovery-v0.md)
- [RFC-0024](../rfc/RFC-0024-signed-registry-metadata-v0.md)
- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
