# STEP-0105: single-Store cooperative scheduler core

> - status: complete
> - phase: M11
> - started: 2026-08-31
> - completed: 2026-08-31
> - owners: autonomous-agent

## 1. Objective

Build the bounded scheduler core that owns task identity, lifecycle and completion ingress inside one fresh Store per run, per ADR-0010 and RFC-0036, without changing the sequential-v1 guest-execution profile or any M10-frozen claim.

Verifiable results:

- a `SchedulerCore` inside `sico-runner` owning: bounded task table (1,024 live tasks), scope tree (depth 64), FIFO ready queue (1,024), bounded Host completion ingress (1,024 records / 16 MiB), scheduler metadata budget (16 MiB), cumulative run-level budget accounting;
- deterministic run-local identities (monotonic task/scope/operation ids) and the lifecycle `created → runnable ↔ suspended → completing → succeeded | failed | cancelled` with illegal-transition refusals;
- canonical same-turn readiness order: readiness class, registration order, then task id;
- stale, duplicate, late and cross-run/generation completion records rejected closed;
- every run starts a root task, every Host operation registers and completes through the ingress, and Store teardown requires all tasks terminal with deterministic reverse-order scope cleanup;
- scale evidence: 1, 2, 16, 256 and 1,024-task workloads execute within run bounds; limit+1 cases fail with stable typed outcomes; repeated runs show no task/handle/RSS growth across Store teardown.

## 2. Context and evidence

- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md): ownership graph, lifecycle, terminal rules §6.3 canonical ordering, hard limits table.
- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md): eager-start spawn, sequential-v1 projection (`TaskScopeOpen/Close` free, spawn = eager call, await/collect = identity), IR verifier task discipline.
- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) STEP-0105 deliverables and exit evidence.
- M10: `TerminalArbiter` one-winner path, `CancelToken`, run/generation identity, bounded events (256 records / 4 MiB).
- M9: fuel/epoch watchdog, bounded stream host calls on depth-1 worker channels, cancellation exit 123.
- STEP-0104: guest tasks are intentionally invisible to the host in sequential-v1 (no runtime cost for scope markers).

## 3. Scope

In scope:

- scheduler core data structures, state machine, limits and typed faults in `sico-runner`;
- run/generation-scoped identity validation for Host completion records;
- integration: one scheduler per run in `run_unchecked`, root task lifecycle, Host-operation registration/completion through the ingress, teardown discipline;
- synthetic and real scale/adversarial evidence, validator, registry updates.

Out of scope (later M11 steps):

- guest-visible task suspension, guest-side yield points or any reinterpretation of sequential-v1 Spawn (forbidden by RFC-0036);
- race/select runtime semantics and downward cancellation tree (STEP-0106);
- channels/streams as task-aware resources (STEP-0107);
- watch/REPL/DAP task integration (STEP-0108); DAP `threads` mapping stays the M10 global safe-pause;
- Linux native runner parity (STEP-0109).

## 4. Options and decision

### 4.1 What the scheduler schedules in v1

- **Candidate A (chosen): Host-operation turns.** The scheduler core owns the run's task/scope/queue/identity state and drives every Host operation (fs/stream/http/stdin/stdout) as a registered operation whose completion enters through the bounded identity-checked ingress. Guest code keeps the sequential-v1 profile: source-level tasks complete eagerly inside the guest, so at most the root task plus in-flight Host operations are live host-side. The state machine and all bounds are proven by synthetic workloads driving the core directly; real guest spawn workloads (1..1,024 tasks) prove run-level budget accounting end-to-end.
- Candidate B: emit host-visible task markers from codegen (task-scope ops become Host calls). Rejected: violates RFC-0036 §5.4 ("no runtime cost" projection) and couples the scheduler to ABI round-trips the frozen profile deliberately avoids.
- Candidate C: engine async/stack-switching guest suspension now. Rejected: unproven on the pinned Wasmtime 47.0.2 debug path, breaks M10's verified pause semantics, and is explicitly a non-goal of RFC-0036.

Reversal condition: a future scheduler RFC may add a deferred-start/suspension profile; the core's identity, bounds and ingress discipline are designed to carry over unchanged.

### 4.2 Completion identity

Chosen: every Host completion record carries `{run_id, generation_id, task_id, operation_id}` and payload bytes; the ingress rejects records whose run/generation does not match the live run, whose operation is unknown/terminal, or that duplicate a delivered record. Operation and registration ids increase monotonically; nothing from generation N can resolve in generation N+1.

### 4.3 Bounds as typed outcomes

Chosen: every limit+1 case returns a stable `SchedulerFault` variant (task table full, ready/completion queue full, metadata budget exceeded, scope depth exceeded, illegal transition, unknown/stale/duplicate completion). No silent clamping, no panic paths.

## 5. Plan

1. This step doc; RFC not required (ADR-0010 + RFC-0036 already freeze the contract).
2. `runner/sico-runner/src/scheduler.rs`: core + limits + typed faults + canonical ordering.
3. Integration in `run_unchecked` and Host call paths (RunState carries the scheduler).
4. Tests: state-machine properties, adversarial completion corpus, scale workloads, teardown stability.
5. `tools/validate-step-0105.ps1`, registry updates, full regression, commit.

## 6. Changes

- `runner/sico-runner/src/scheduler.rs` (new, ~940 lines): `SchedulerCore` owned by one fresh Store per run. Bounded task table (`MAX_LIVE_TASKS = 1,024`), scope tree (`MAX_SCOPE_DEPTH = 64`, `MAX_SCOPE_CHILDREN = 1,024`), FIFO ready queue (`MAX_READY_QUEUE = 1,024` with per-task dedup), Host completion ingress (`MAX_COMPLETION_RECORDS = 1,024` / `MAX_COMPLETION_BYTES = 16 MiB`), exact metadata accounting (`MAX_METADATA_BYTES = 16 MiB`), cumulative `host_operations_total`. Monotonic `TaskId`/`ScopeId`/`OperationId`; lifecycle `created → runnable ↔ suspended → completing → succeeded | failed | cancelled` with `IllegalTransition` refusals and exactly-once terminal commit. `CompletionRecord` carries `{run_id, generation_id, task, operation, payload_bytes}`; the readiness class is read from the operation registration at drain time, never from the publisher (spoof-proof). `drain_turn` sorts by readiness class → registration order → task id (ADR-0010 §6.3). `teardown` requires all tasks and operations terminal and closes scopes deepest/highest-id first. 17 stable `SchedulerFault` variants with `Display` strings as the typed contract; no silent clamping, no panic paths.
- `runner/sico-runner/src/lib.rs` (+147 lines): `pub mod scheduler`; `RunState` carries the `SchedulerCore`; `Runner` gains a `run_counter` used with the per-run id to form `RunIdentity { run_id: "run-<16 hex>", generation_id: 1 }`. `run_unchecked` makes the root task runnable before instantiate, maps the arbitrated `RunOutcome` to the root task's single `TerminalKind` (`Output→Succeeded`, `Cancelled|Timeout→Cancelled`, everything else→`Failed`), then `commit_root` + `teardown`; any scheduler fault surfaces as `RunOutcome::Launch("scheduler: …")`. All four real async Host boundaries register operations and complete through the identity-checked ingress: `input-stream.read`, `output-stream.write`, `output-stream.flush` (payload = outcome bytes / 0), and the HTTP request path (payload = response body bytes) with `abandon_operation` fallback for the timed-out unjoinable-worker case (ADR-0010 adversarial trace 3). The debug-session worker intentionally does not run `teardown` (M10-frozen DAP behavior unchanged; recorded as a limitation).
- `runner/sico-runner/tests/runner.rs` (+414 lines): five integration tests — `scheduler_scale_workloads_execute_within_run_bounds` (synthetic 1/2/16/256/1,024-total-live-task workloads, canonical order check), `scheduler_limit_plus_one_cases_fail_with_typed_outcomes` (task table, scope children, scope depth, ready queue, completion records, completion bytes), `scheduler_adversarial_completion_records_fail_closed` (cross-run, cross-generation, unknown operation, wrong-task stale, duplicate, late-after-abandon, post-teardown), `repeated_runs_show_no_task_handle_or_rss_growth_across_store_teardown` (101 echo runs through the full Host-op ingress path, Windows handle/RSS bounds), `guest_task_workloads_at_scale_run_within_bounds` (real .sico guest spawn workloads at 1/2/16/256/1,024 tasks built and executed under default run bounds).
- `tools/validate-step-0105.ps1` (new): fmt, clippy `-D warnings`, scheduler unit tests, release runner integration suite (single-threaded, metrics captured to `target/evidence/step-0105/`), typed fault-surface stability check, task-collect e2e, STEP-0087/0090/0104 regression gates.

Design findings recorded by the adversarial suite:

0. The M11 plan's "fuel/epoch/cancellation checkpoints" deliverable maps to the existing M9 watchdog/cancel path: the scheduler does not add guest-visible checkpoints in v1 (sequential-v1 forbids it); it observes the arbitrated outcome at the run boundary and every Host-operation suspension point registers/completes through the ingress, which are the only host-visible checkpoints this step can honestly claim.
1. The ready-queue bound is only reachable through a stale queue entry left by a task that went terminal while queued (the queue dedups per task and the queue cap equals the task cap); the limit+1 test exercises exactly that path and proves the stale entry is skipped exactly once on pop.
2. The 16 MiB metadata budget is structurally unreachable with every count capped at 1,024 (hundreds of KiB worst case); it is enforced anyway against future larger records and has no reachable limit+1 case.
3. The Windows console-control fixture (`runner_cli_observes_real_windows_console_control`, STEP-0098) races a fixed 150 ms CTRL_BREAK against cold debug-binary startup; A/B-verified flaky at HEAD without these changes. The validator follows the STEP-0098 precedent and runs the runner suite in release.

## 7. Validation

Executed 2026-08-31 on Windows x64 GNU, Rust 1.98.0, Wasmtime 47.0.2:

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` green.
- Scheduler unit tests: 8/8 green (lifecycle, spawn fill + limit+1, scope children/depth, ingress stale/duplicate/cross-run, canonical ordering, teardown reverse order, exact metadata accounting).
- Runner integration suite (release, single-threaded): 31/31 green, including the five STEP-0105 tests; evidence log at `target/evidence/step-0105/runner-tests.txt`. All 26 pre-existing runner tests unchanged and green — no M9/M10 regression (bounded events, redaction, exact DAP behavior untouched).
- Exit-evidence mapping: 1/2/16/256/1,024-task workloads within bounds (`scheduler_scale_workloads_execute_within_run_bounds` + `guest_task_workloads_at_scale_run_within_bounds`); every reachable limit+1 typed (`scheduler_limit_plus_one_cases_fail_with_typed_outcomes`; metadata budget structurally unreachable, documented); canonical same-turn order (unit + scale tests); stale/duplicate/cross-run/cross-generation/wrong-task/late/post-teardown records fail closed (`scheduler_adversarial_completion_records_fail_closed`); repeated runs show no handle/RSS growth across Store teardown (`repeated_runs_show_no_task_handle_or_rss_growth_across_store_teardown`: 101 runs, handles 91→91, RSS flat).
- End-to-end: `sico run tests/end-to-end/script-task-collect.sico` prints exactly `alpha! beta!` through the integrated scheduler path.
- Regression gates: STEP-0087, STEP-0090 and STEP-0104 validators green.
- Known environmental flake (pre-existing, A/B-verified at HEAD without these changes): `runner_cli_observes_real_windows_console_control` races a fixed 150 ms CTRL_BREAK against cold debug-binary startup; green in release and when warm in debug. The validator runs the runner suite in release per the STEP-0098 precedent.

## 8. Metrics

Non-SLA measurements from the release runner suite (Windows x64, 2026-08-31):

| Workload | Wall time |
|---|---:|
| synthetic 1-task lifecycle | 4 µs |
| synthetic 1,024-task lifecycle (saturation) | 6.2 ms |
| real guest 1 spawn | 18 ms |
| real guest 1,024 spawns | 96 ms |
| 101 repeated echo runs | handles 91→91; RSS 18,112,512→18,112,512 bytes |

Scheduler synthetic costs are microsecond-scale per task record; real guest scale is dominated by eager sequential execution, as the sequential-v1 profile predicts.

## 9. Risks and follow-ups

- v1 has no guest-visible suspension; a future profile must not reinterpret the identity/ingress contracts frozen here.
- Host operations are the only host-visible suspension points; stream/channel task-awareness arrives in STEP-0107.
- The 16 MiB metadata budget is charged by exact accounting of table/queue records, not estimates.

## 10. Audit links

- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md)
- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md)
- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md)
- [`STEP-0104`](./STEP-0104-semantic-ir-structured-concurrency.md)
