# STEP-0106: cancellation, timeout, race and select

> - status: complete
> - phase: M11
> - started: 2026-09-01
> - completed: 2026-09-01
> - owners: autonomous-agent

## 1. Objective

Build the downward cancellation tree, canonical race/select resolution, timer readiness and provable deadlock detection on the STEP-0105 scheduler core, integrated with M10's one-terminal-winner path, without changing the sequential-v1 guest profile or any M10-frozen claim.

Verifiable results:

- downward cancellation: cancelling a task or scope commits every descendant `cancelled` in deterministic reverse ownership order, abandons their pending Host operations (late worker records become stale), and never widens authority upward;
- race/select at the scheduler level: ≤256 operands, winner chosen by the accepted canonical order (readiness class, registration order, task id — not wall-clock), losers cancelled deterministically, a loser can neither emit a second terminal state nor retain Host work;
- timer readiness participates in the same canonical order (Cancellation < HostCompletion < Timer);
- deadlock detection: no runnable tasks and no pending Host operations while tasks remain suspended is a typed outcome, not a hang;
- exit matrices — completion-vs-cancel, timeout-vs-cancel, parent-vs-child failure, simultaneous-ready — each produce exactly one result;
- nested cancellation of a 1,024-task chain remains within queue/time/memory bounds; collection ordering matches the contract under adversarial publish order.

## 2. Context and evidence

- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md) §6.3: race/select chooses the canonical first item and cancels losers; parent cancellation propagates down; child cancellation cannot widen authority upward; select/race operands ≤ 256.
- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md) §9: race/select source spellings stay refused until an independent RFC; §4: collect-all returns creation order.
- [`STEP-0105`](./STEP-0105-scheduler-core.md): scheduler core, ingress, canonical drain order, `parent_of` ownership graph, abandon discipline.
- M10: `TerminalArbiter` one-winner commit and `CancelToken` sources; STEP-0105 wired run cancellation to `TerminalKind::Cancelled`.

## 3. Scope

In scope:

- scheduler-core cancellation tree (`cancel_task`/`cancel_scope`), select/race resolution, timer-class readiness, deadlock detection, typed faults;
- runner integration: run-level cancellation drives the tree (root task cancelled through the same path any descendant will take);
- adversarial matrix/scale evidence, validator, registry updates.

Out of scope:

- race/select source syntax and semantics (separate RFC per RFC-0036 §9);
- guest-visible suspension (still sequential-v1);
- channels/streams task-awareness (STEP-0107); DAP thread mapping (STEP-0108);
- wall-clock timer wheel: timers are Host-driven; the scheduler owns their readiness class and resolution order only.

## 4. Options and decision

### 4.1 Where race/select lives

- **Candidate A (chosen): scheduler-level resolution over registered operands.** A select is a bounded set of task operands owned by one scope; the winner is the canonical first ready operand; losers are cancelled through the tree and their operations abandoned. Proven by synthetic workloads driving the core directly, the same discipline STEP-0105 established. Source-level `race`/`select` spellings remain refused (RFC-0036 §9) and no IR changes are needed.
- Candidate B: add source syntax + IR ops now. Rejected: RFC-0036 explicitly defers the spelling to an independent RFC; smuggling it in would break the accepted contract surface.
- Candidate C: wall-clock race in the runner. Rejected: ADR-0010 freezes canonical (not wall-clock) winner selection; wall-clock races are un-auditable.

### 4.2 Cancellation propagation

Chosen: cancellation is a scheduler state operation, not a completion record. `cancel_task` commits `Cancelled` to the task and every descendant in reverse ownership order (deepest, then highest id) and abandons each cancelled task's pending operations, so a late Host worker record is stale and teardown discipline holds. Cancellation never travels upward: the API accepts only the target task/scope, and descendants are resolved through the ownership graph. The run-level path (M10 arbiter commits `Cancelled`) routes the root task through the same `cancel_task` entry, so the v1 root and any future guest task share one code path.

### 4.3 Deadlock as a typed outcome

Chosen: `detect_deadlock` proves the absorbing state — no queued or runnable tasks, no non-terminal Host operations, at least one suspended task — and returns a typed `SchedulerFault::Deadlock` naming the blocked tasks. It is checked synthetically and defensively in the run path; sequential-v1 guests cannot reach it today, but the proof is part of the scheduler contract a future suspension profile must honor.

## 5. Plan

1. This step doc (no new RFC: ADR-0010 §6.3 + RFC-0036 already freeze the contract; source spelling stays refused).
2. `scheduler.rs`: cancellation tree, select/race resolution with `MAX_SELECT_OPERANDS = 256`, deadlock detection, new typed faults.
3. Runner integration: cancellation outcome routes through `cancel_task`; defensive deadlock check on teardown failure.
4. Tests: the four one-result matrices, loser discipline, 1,024-chain cancellation scale, adversarial collection ordering, deadlock proofs.
5. `tools/validate-step-0106.ps1`, registry updates, full regression, commit.

## 6. Changes

- `runner/sico-runner/src/scheduler.rs` (+490 lines): `MAX_SELECT_OPERANDS = 256` (ADR-0010). Downward cancellation tree: `cancel_task` (BFS over parent links, commits `cancelled` to every non-terminal descendant in reverse ownership order — deepest scope, then highest task id — and abandons each member's pending Host operations so late worker records are stale), `cancel_scope` (whole scope subtree), `cancel_root` (the run-level entry shares the descendant path). Cancellation is idempotent on terminal members and never travels upward. `select(&[TaskId]) -> SelectOutcome { winner, record, losers }`: operands deduped, ≤256, all known; winner = canonical-first ready operand (`Cancelled` first, then completion-ready by registration-time class/order, then other terminal; ties on task id — never wall-clock); the winner's queued record is consumed; every other operand is cancelled through the tree. `detect_deadlock` proves the absorbing state (no runnable task, no non-terminal operation, ≥1 suspended task) and returns typed `SchedulerFault::Deadlock(tasks)`. Four new stable fault variants: `SelectOperandsExceeded`, `NoSelectOperands`, `NoReadyOperand`, `Deadlock`.
- `runner/sico-runner/src/lib.rs` (+20/-4): the arbitrated run outcome now routes `Cancelled`/`Timeout` through `cancel_root` (the same tree entry any descendant will take) before teardown; a teardown failure with a provable absorbing state is reported as the typed deadlock message instead of the raw teardown fault. All other outcomes keep the STEP-0105 `commit_root` path. No M10-frozen behavior changes: outcome classes, exit codes, DAP and event paths untouched.
- `runner/sico-runner/tests/runner.rs` (+123 lines): `scheduler_cancellation_tree_scales_and_select_stays_canonical` (1,024-deep chain cancellation within bounds + 16 rounds of adversarially shuffled publish order with canonical winner and reverse-order loser commits), `cancelled_guest_run_drives_the_scheduler_tree_to_teardown` (M10 token cancellation of a blocked guest exits 123 through the tree path, and a second run on the same prepared program proves no scheduler state leaks).
- `tools/validate-step-0106.ps1` (new): fmt, clippy, scheduler unit tests, release runner suite with evidence capture, typed fault-surface stability (old and new variants), diagnostics/semantic-case oracles (no new codes — race/select source spelling stays refused), STEP-0087/0090/0104/0105 regression gates.

Design finding recorded by the matrix suite: a select loser whose Host completion was already delivered keeps exactly one queued record (it drains once in canonical order) while the task itself commits `cancelled` and its operation is abandoned — one terminal state per task is preserved, and record delivery is not a terminal state.

Test-infrastructure fix pulled into this step (blocking both validators): the STEP-0098 Windows console-control fixtures raced a blind 150–200 ms sleep against console-handler installation in the child, and died with the default-handler kill (`STATUS_CONTROL_C_EXIT`, empty stderr) whenever startup was slow — A/B-verified pre-existing at HEAD. Both Python fixtures now retry (≤6 attempts, 300 ms) only on that exact pre-handler-install signature and fail immediately on any other mismatch, so the gate stays honest: each attempt is a real run, and the success criterion (exit 123 with the typed cancelled payload) is unchanged.

## 7. Validation

Executed 2026-09-01 on Windows x64 GNU, Rust 1.98.0, Wasmtime 47.0.2:

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` green (one `collapsible_if` fix).
- Scheduler unit tests: 16/16 green, including the new cancellation-tree propagation/idempotence, scope cancel, completion-vs-cancel and timeout-vs-cancel matrices, simultaneous-ready registration-order resolution, parent-vs-child failure one-terminal-each, select operand bounds (0/257/unknown/not-ready typed), deadlock proof/resolution, and the 1,024-deep chain cancellation.
- Runner integration suite (release, single-threaded): 33/33 green; evidence log at `target/evidence/step-0106/runner-tests.txt`. The new `scheduler_cancellation_tree_scales_and_select_stays_canonical` (chain scale + 16 adversarial publish-order rounds) and `cancelled_guest_run_drives_the_scheduler_tree_to_teardown` (exit 123 through the tree path; immediate rerun on the same prepared program is clean) are present and green.
- Exit-evidence mapping: the four one-result matrices, loser discipline (cancelled + abandoned + second-terminal refusal), 1,024-task nested cancellation within bounds, and adversarial collection ordering are covered by the suites above; timer readiness participates in canonical order through the timeout-vs-cancel matrix.
- Diagnostics/semantic-case oracles green with no new codes: race/select source spellings remain refused (RFC-0036 §9).
- Regression gates: STEP-0087, STEP-0090, STEP-0104, STEP-0105 validators all green.
- Pre-existing fixture flake fixed (see §6): both Windows console-control fixtures now retry only on the exact pre-handler-install kill signature; verified 3/3 green in isolation after the fix, then green inside the full validator run.

## 8. Metrics

Non-SLA measurements (Windows x64, 2026-09-01):

| Workload | Wall time |
|---|---:|
| 1,024-deep task chain cancellation (unit) | ~4.3 ms |
| 1,024-deep task chain cancellation (release evidence) | 6.7 ms |
| select over 8 operands × 16 adversarial rounds | included above (suite total 0.15 s debug) |

Cancellation cost is linear in subtree size with microsecond-scale per-task commits; select resolution is bounded by 256 operands.

## 9. Risks and follow-ups

- Race/select source syntax still needs its own RFC; this step must not be cited as evidence the source surface exists.
- The 1,024-task cancellation chain lives in one scope (task parent chains are not scope nesting); scope depth stays at 64.
- STEP-0107 will make channels/streams cancellation-aware resources; the abandon discipline here is the pattern they reuse.

## 10. Audit links

- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md)
- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md)
- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md)
- [`STEP-0105`](./STEP-0105-scheduler-core.md)
