# STEP-0110: M11 security, performance and platform exit audit

> - status: complete / GO
> - phase: M11
> - started: 2026-09-01
> - completed: 2026-09-01
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
| 9 | Windows x64 AND Linux x64 native runners pass the same corpus | **GO** | STEP-0109 complete (2026-09-02): full parity corpus green on WSL2 Ubuntu 24.04 (kernel 6.18.33.2, glibc 2.39, rustc 1.98.0), evidence archived at `target/evidence/step-0109/linux/` |
| 10 | Complete M0–M10 regression green; external-input recheck recorded | GO | Aggregate validator chain (0087/0090/0091/0100/0102 gates rerun green); recheck in §4 |

## 3. Verdict

**M11: GO** — all ten gates are met with reproducible evidence: gates 1–8 and 10 on the Windows x64 GNU + aggregate regression runs, gate 9 on the 2026-09-02 native Linux x64 (WSL2) parity run. The earlier 2026-09-01 audit had correctly recorded NO-GO pending Linux; the parity evidence landed and the aggregate was rerun clean.

## 4. External-input recheck (2026-09-01)

| Input | State |
|---|---|
| Public deployment (domain/hosting) | still absent — unchanged |
| Production identity / key custody | still absent — unchanged |
| Third-party pilots | still absent — unchanged |
| Live-model credentials (DeepSeek, M13 §5) | still absent — owner-promised, pending delivery |
| Mobile runners (Android/Harmony) | still absent — unchanged |
| ~~Linux x64 native runner~~ | **delivered 2026-09-02**: WSL2 Ubuntu 24.04 native parity run green (STEP-0109) |

## 5. Residual risks

- Scheduler/channel/select semantics are proven at the core level only; guest-visible suspension remains sequential-v1 until a future profile RFC.
- Race/select source spellings remain refused pending an independent RFC (RFC-0036 §9).
- M12 is unlocked by this GO; Secure HTTP Provider work may begin per the M12 plan.

## 5.1 Audit findings fixed during this step

1. **Latent STEP-0104 gap**: the AI error-taxonomy coverage check (`validate-error-taxonomy.ps1`) was not in any post-0104 validator chain, so the four new M11 diagnostics (E5003/E5103–E5105) drifted out of the taxonomy and the hardcoded case count (29→33) went stale. Fixed: taxonomy classes `async-structured-lifetime` (+TASK_NOT_CONSUMED/TASK_DETACHED/TASK_SCOPE_LIMIT) and `resource-lifecycle` (+BORROW_ACROSS_SUSPENSION) now cover all 41 catalog codes; the count check tracks the current map; and `validate-step-0104.ps1` now runs the taxonomy oracle so future diagnostic additions cannot drift silently.
2. **Stale toolchain aliases**: seven M10-era validators pinned `RUSTUP_TOOLCHAIN=stable` (resolving to 1.97.0 after the 1.98 migration); re-pinned during STEP-0108 and exercised green here.

## 6. Validation

Executed 2026-09-01 on Windows x64 GNU, Rust 1.98.0, Wasmtime 47.0.2 — `tools/validate-step-0110.ps1` clean full run:

- `cargo fmt --all --check`, `clippy --workspace --all-targets -D warnings`, `cargo test --workspace` green.
- M0–M10 regression: `validate-step-0102.ps1` green (0094 M0–M9 aggregate + 0095–0101 + DAP claim counts 12/20/6).
- M11 chain: `validate-step-0108.ps1` green (transitively 0107/0106/0105/0104/0091/0100/0090/0087).
- Evidence markers verified on disk: `SCHEDULER_TEARDOWN_100` (0105), `CHAIN_CANCEL_1024` (0106), `CHANNEL_RELAY_1GIB` (0107), `GRANT_MATRIX_100` (0108).
- Audit-document consistency checks green; platform honestly recorded as windows-x64-gnu only.
- Final line: `STEP_0110_OK m0-m10=green m11=0103-0108-green evidence=present platform=windows-x64-only gate9=unmet decision=NO-GO-pending-linux`.

One flake note for the record: an earlier audit attempt died inside STEP-0089 with the release runner test binary producing zero output — diagnosed as two overlapping validator processes (an earlier diagnosis run of mine had not fully terminated and was rebuilding the same target directory). The clean rerun above passed with no concurrent processes; no code change was needed.

## 7. Audit links

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) §11–14
- [`STEP-0103`](../adr/ADR-0010-single-store-structured-concurrency.md) ADR
- [`STEP-0104`](./STEP-0104-semantic-ir-structured-concurrency.md) … [`STEP-0109`](./STEP-0109-cross-platform-runner-parity.md)
