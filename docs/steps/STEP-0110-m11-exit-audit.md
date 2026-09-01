# STEP-0110: M11 security, performance and platform exit audit

> - status: in-progress
> - phase: M11
> - started: 2026-09-01
> - completed: -
> - owners: autonomous-agent

## 1. Objective

Aggregate audit of M11 (STEP-0103–0109) against the exit gate in the M11 plan §13, including the M0–M10 regression chain, the security/performance evidence register, the platform matrix, and an explicit GO/NO-GO with external-input recheck.

## 2. Gate-by-gate verdict (M11 plan §13)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | ADR-0010 accepted before scheduler implementation | GO | STEP-0103 (2026-07-23) precedes STEP-0105 implementation |
| 2 | Fresh Store per run; persistent-generation isolation | GO | STEP-0105 (Store-scoped scheduler), STEP-0108 (8-generation isolation + watch gates) |
| 3 | No suspension with illegal scratch-arena borrow; post-return correct | GO (vacuous in sequential-v1) | RFC-0036 §5.4: guest tasks complete eagerly; no host-side guest suspension exists to borrow across. Host-call arenas are per-call bounded (M9) and never cross an operation boundary; the ingress discipline (STEP-0105) rejects cross-run records |
| 4 | Affine resources move/close/cancel without alias, leak or cross-Store use | GO | STEP-0105 E5003 + affine cases; STEP-0107 move/close/double-close typed fixtures; teardown stability (handles 91→91) |
| 5 | spawn/await/group/select/race one bounded deterministic outcome | GO | RFC-0036 + STEP-0104 (spawn/await/collect executed); STEP-0106 select matrices single-result; race/select source spelling refused pending its own RFC |
| 6 | Cumulative limits resist limit+1 and stress | GO | STEP-0105 typed limit+1 corpus (tasks/scopes/queues/bytes); STEP-0106 select 257-operand refusal; STEP-0107 channel caps |
| 7 | M10 task-aware events/DAP bounded, identity-matched, authority-neutral | GO | Events carry task-0/scope-0 (STEP-0108 exactness check); DAP suite green through the STEP-0108 debug teardown change; AI surface data-only contract test |
| 8 | Cancellation joins or abandons Host work under bounded policy, no stale publication | GO | STEP-0106 abandon discipline + adversarial ingress; STEP-0107 channel failure-close; http abandon fallback (STEP-0105) |
| 9 | Windows x64 AND Linux x64 native runners pass the same corpus | **UNMET** | STEP-0109 blocked-external-evidence: no Linux host/WSL in this environment (2026-09-01); parity corpus prepared (`tools/validate-step-0109.sh`) |
| 10 | Complete M0–M10 regression green; external-input recheck recorded | GO | Aggregate validator chain (0087/0090/0091/0100/0102 gates rerun green); recheck in §4 |

## 3. Verdict

**M11: NO-GO (platform evidence missing)** — gate 9 is unmet and the plan states "Absence of a Linux x64 native runner makes M11 NO-GO; Windows-only success is insufficient." All other nine gates are GO with reproducible Windows x64 evidence. This mirrors the M6 precedent: an honest NO-GO on an external environment input, with the parity corpus ready to execute (`tools/validate-step-0109.sh`) the moment a native Linux x64 host exists.

## 4. External-input recheck (2026-09-01)

| Input | State |
|---|---|
| Public deployment (domain/hosting) | still absent — unchanged |
| Production identity / key custody | still absent — unchanged |
| Third-party pilots | still absent — unchanged |
| Live-model credentials (DeepSeek, M13 §5) | still absent — owner-promised, pending delivery |
| Mobile runners (Android/Harmony) | still absent — unchanged |
| **Linux x64 native runner** | **absent — now the binding M11 gate** (WSL2 install needs admin + reboot) |

## 5. Residual risks

- Scheduler/channel/select semantics are proven at the core level only; guest-visible suspension remains sequential-v1 until a future profile RFC.
- Race/select source spellings remain refused pending an independent RFC (RFC-0036 §9).
- M12 stays locked: the plan requires M11 GO before M12 begins, so Secure HTTP Provider work does not start until the Linux parity evidence lands.

## 6. Validation

- `tools/validate-step-0110.ps1` (aggregate): fmt, clippy, workspace tests, the STEP-0108 gate chain (transitively 0107/0106/0105/0104/0091/0100/0090/0087), evidence-marker checks across `target/evidence/step-0105..0108/`.
- Results: recorded below at completion.

## 7. Audit links

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) §11–14
- [`STEP-0103`](../adr/ADR-0010-single-store-structured-concurrency.md) ADR
- [`STEP-0104`](./STEP-0104-semantic-ir-structured-concurrency.md) … [`STEP-0109`](./STEP-0109-cross-platform-runner-parity.md)
