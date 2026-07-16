# STEP-0065: Secure update, rollback and recovery

> - status: complete
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Implement a deterministic local update client that consumes STEP-0064 signed records, persists monotonic trust, stages verified bytes before a logical atomic commit, retains last-known-good revisions and permits rollback only through explicit recovery authorization or local policy.

## 2. Context and evidence

Ten threat-matrix cases are assigned to STEP-0065: four update, two disclosure and four rollback cases. Registry transport remains untrusted and all tests remain local.

## 3. Scope

Included: signed update snapshots; monotonic/fresh trusted state; mix-and-match rejection; staged immutable revisions; crash/corruption recovery; signed append-only advisories; exact affected digests; recovery-role authorization; explicit local recovery policy; identity and capability re-evaluation.

Excluded: public update endpoint, OS installer integration, production keys, real disclosure publication, network availability, Android/Harmony activation and public transparency.

## 4. Plan

1. Freeze update snapshot, advisory, recovery and trusted-state schemas. — complete
2. Verify one consistent transaction against registry and publisher records. — complete
3. Persist contiguous checksummed trust generations and immutable revisions. — complete
4. Retain last-known-good state across corrupt/partial candidate failures. — complete
5. Require signed or explicit local recovery and capability re-evaluation. — complete
6. Add corpus, RFC, report, validator and full regression. — complete

## 5. Changes

- Added registry-threshold signed update snapshots binding exact policy, checkpoint, namespace, channel, release, package, capability and advisory state.
- Added fixed-time freshness, persisted last-trusted time and monotonic policy/checkpoint/snapshot/channel/release/advisory enforcement.
- Added consistent transaction checks across every digest, length, sequence and identity. Package activation repeats strict `.sapp`, manifest app ID/version and capability-closure verification.
- Added staging readback, immutable revision/metadata writes and contiguous create-new state generations. Corrupt staging cannot replace the active revision and a valid retry recovers.
- Added canonical checksummed local state with startup verification of every generation and retained package.
- Added revocation-threshold signed append-only advisories with exact release/package digests.
- Added recovery-threshold signed authorization and exact explicit local recovery policy. Recovery preserves the latest trust head, cannot cross production identity/app ID and re-evaluates capability fingerprints.
- Accepted [RFC-0025](../rfc/RFC-0025-secure-update-recovery-v0.md), added the [36-case corpus](../../tests/ecosystem/secure-update-cases.json), [review](../reports/secure-update-recovery-v0.md) and validator.

## 6. Validation

The focused secure-update suite passed 8/8 tests. All 1,592 one-byte mutations of a signed snapshot were rejected. Two actual deterministic `.sapp` revisions exercised replay, mix-and-match, expiry, corrupt staging/retry, restart, state/revision tampering, advisory exactness, signed recovery, explicit local recovery and capability permission reconfirmation.

Full formatting, Clippy with warnings denied, workspace tests, local-link validation, `git diff --check` and `validate-step-0065.ps1` are required immediately before commit.

Result: `STEP_0065_OK tests=8 mutations=1592 cases=36 threats=10 updates=verified advisories=append-only recovery=signed-or-explicit retained=2 external_side_effects=0 next=STEP-0066`.

## 7. Metrics

| Metric | Result |
|---|---:|
| secure-update tests | 8 passed |
| snapshot mutations | 1,592/1,592 rejected |
| machine cases | 36 |
| assigned threats | 10 |
| revision fixtures | 590 / 679 bytes |
| state generations | 1,036 / 1,098 bytes |
| retained revisions | 2 |
| recovery modes | signed + explicit local |
| external operations | 0 |

## 8. Risks and follow-ups

The initial publisher policy remains caller-pinned and update activation requires its exact version/digest; policy advance is never inferred from registry metadata. The unkeyed state checksum detects corruption but not whole-directory restore by a local attacker; platform adapters need OS-protected storage or a secure counter for that stronger threat. Namespace latest-history service selection, public advisory transparency, OS installer/launcher integration and mobile activation remain outside this local step.

## 9. Audit links

- [ADR-0006](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [RFC-0022](../rfc/RFC-0022-ecosystem-compatibility-contract-v0.md)
- [RFC-0024](../rfc/RFC-0024-signed-registry-metadata-v0.md)
- [RFC-0025](../rfc/RFC-0025-secure-update-recovery-v0.md)
- [secure update review](../reports/secure-update-recovery-v0.md)
