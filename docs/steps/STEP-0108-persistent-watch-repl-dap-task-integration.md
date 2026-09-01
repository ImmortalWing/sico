# STEP-0108: persistent runner, watch, REPL and DAP task integration

> - status: complete
> - phase: M11
> - started: 2026-09-01
> - completed: 2026-09-01
> - owners: autonomous-agent

## 1. Objective

Prove generation-isolated scheduler lifecycle in every persistent process shape (watch generations, REPL cells, DAP sessions) and close the one known STEP-0105 gap — the debug-session worker not running scheduler teardown — without changing any M10-frozen claim.

Verifiable results:

- the debug-session worker runs the same root commit + teardown discipline as `run_unchecked` (0105 limitation closed); pause/disconnect/terminate cannot strand tasks or workers;
- replacing a watch generation only happens after the previous generation's run (and its scheduler/Store) completed; run-complete of generation N precedes any work of generation N+1;
- REPL cells are constant expressions that never create a Store — they structurally cannot retain tasks or resources into a later Store, and no persistent-state contract exists to change that;
- 100 sequential runs with alternating fs grants show per-generation authority isolation and no state/handle/RSS growth;
- task-aware M10 events stay exact in v1: `task_id = task-0` / `scope_id = scope-0` match the scheduler's root identities;
- AI/tooling remains a bounded data consumer: the `sico.ai-tool.v0` surface is exactly the four frozen data operations with no task-spawn/control capability.

## 2. Context and evidence

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) STEP-0108 deliverables and exit evidence.
- [`STEP-0105`](./STEP-0105-scheduler-core.md) §6: debug-session teardown was deliberately deferred as a documented limitation — this step closes it.
- STEP-0090: watch generation coalescing and artifact cleanup; STEP-0091: bounded constant-only REPL; STEP-0099/0100: task-aware bounded events and exact DAP; STEP-0101: data-only AI boundary.
- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md): nothing crosses a run or persistent generation; Store teardown requires all tasks/operations terminal or abandoned.

## 3. Scope

In scope:

- debug-worker scheduler commit/teardown (uniform with the non-debug path);
- watch/REPL/DAP generation-isolation and no-stranding evidence;
- authority-isolation scale evidence (100 changing-grant runs);
- AI tooling surface contract test.

Out of scope:

- DAP `threads` mapping beyond the M10 global safe-pause (no guest tasks exist host-side in v1; a mapping claim would be fiction);
- event schema changes (M10-frozen; v1 root identity already matches);
- REPL persistent-state contracts (none accepted; none added).

## 4. Options and decision

### 4.1 Debug worker teardown

- **Candidate A (chosen): run the same commit-root/cancel + teardown inside the debug worker after the arbiter commit, before the result is published.** The worker owns the Store, so teardown happens exactly where the Store dies. Observable DAP behavior is unchanged — teardown failure would surface as a `Launch` outcome, and the suites prove it never fires. This closes the 0105 limitation and makes every run shape share one teardown path.
- Candidate B: keep the debug path exempt. Rejected: the exemption was a 0105 stopgap, and "pause/disconnect/terminate cannot strand tasks" cannot be evidenced while the debug Store skips the discipline.

### 4.2 Where the exit evidence lives

Chosen: existing M9/M10 fixtures stay untouched; new proofs are additive tests — watch generation ordering over the real CLI watch loop, a 100-run changing-grants leak matrix, debug terminate/pause stranding checks with handle counts, a REPL purity contract test (constant-only eval; no Store), and an AI-surface contract test. No schema or protocol change means M10-frozen claims stay valid by construction.

## 5. Plan

1. This step doc.
2. lib.rs: debug-worker commit + teardown; shared terminal-kind mapping.
3. Tests: watch ordering, changing-grants leak matrix, debug stranding, REPL purity, AI surface.
4. `tools/validate-step-0108.ps1`, registries, full regression, commit.

## 6. Changes

- `runner/sico-runner/src/lib.rs` (net +43 lines): extracted the shared `scheduler_terminal` outcome mapping and `settle_scheduler` (commit/cancel-root + teardown + typed deadlock detail) helpers; the plain run path now calls them, and — the actual behavior change — **the debug-session worker runs the same commit + teardown discipline after the arbiter commit**, closing the STEP-0105 limitation. Every run shape (plain, observed, watch generation, debug session) now shares one teardown path. M10 DAP observable behavior unchanged (10/10 DAP tests green before and after).
- `runner/sico-runner/tests/runner.rs` (+185 lines): `successive_generations_teardown_cleanly_before_the_next_store` (8 generations on one prepared program; a failed teardown would surface as `Launch`), `hundred_runs_with_changing_grants_show_no_leak_or_authority_drift` (100 runs alternating granted/denied fs roots; deterministic per-parity outcomes — granted exit 0, denied typed domain error — with flat handles/RSS), `debug_pause_terminate_leaves_no_stranded_tasks_or_workers` (20 pause→terminate→finish cycles, flat handles/RSS).
- `crates/sico-cli/src/repl.rs` (+24 lines): `cells_cannot_create_tasks_resources_or_stores` — spawn/task/channel/fs spellings are refused as non-constant cells; REPL never creates a Store.
- `crates/sico-ai-tools/src/lib.rs` (+41 lines): `tooling_cannot_spawn_or_control_tasks` — `spawn_task`/`cancel_task`/`terminate_task`/`scheduler_control`/`select` all fail with typed `invalid_request`; the surface is the four frozen data operations only.
- `tools/validate-step-0108.ps1` (new): fmt, clippy, the two contract tests, release runner suite with evidence capture, regression gates 0107 (transitively 0106/0105/0104/0090/0087), 0091 (REPL), 0100 (DAP).

Design notes recorded:

1. The grants matrix caught a contract subtlety: fs grant roots must be canonicalized by the caller (Windows extended-length `\\?\` prefixes otherwise fail the containment check). The test canonicalizes, matching the documented contract at `resolve_read`.
2. Task-aware M10 events needed no change: `task-0`/`scope-0` already match the scheduler's root identities, so the v1 event claim stays exact without touching the frozen schema.

## 7. Validation

Executed 2026-09-01 on Windows x64 GNU, Rust 1.98.0, Wasmtime 47.0.2:

- fmt + clippy `-D warnings` green; scheduler unit suite 37/37; runner release integration suite 37/37 (evidence log `target/evidence/step-0108/runner-tests.txt`).
- Exit-evidence mapping: generation isolation (`successive_generations_teardown_cleanly_before_the_next_store`, 8 identical generations); watch replacement ordering via the STEP-0090 CLI gate rerun; REPL purity (`cells_cannot_create_tasks_resources_or_stores` — spawn/task/channel/fs spellings refused, constant cells still work; no persistent-state contract exists); pause/terminate stranding (`debug_pause_terminate_leaves_no_stranded_tasks_or_workers`: 20 cycles, handles 91→91, RSS byte-flat); 100 changing-grant runs (`GRANT_MATRIX_100`: handles 91→91, RSS 17,625,088→17,412,096, per-parity outcomes deterministic — granted exit 0, denied typed domain error); AI data-consumer surface (`tooling_cannot_spawn_or_control_tasks`).
- DAP behavior unchanged through the new debug-worker teardown: full `dap_` suite green (10/10) plus the M10 100-session isolation test (handles 91→91).
- Regression gates: STEP-0107 (transitively 0106/0105/0104/0090/0087), STEP-0091 (REPL), STEP-0100 (DAP) all green.
- Validator fix recorded: seven M10-era validators (`0069/0097/0098/0099/0100/0101/0102`) still pinned `RUSTUP_TOOLCHAIN=stable` (resolving to the stale 1.97.0 after the 1.98 migration) — all re-pinned to `1.98.0-x86_64-pc-windows-gnu`; without this fix the 0100 gate fails with a toolchain-version error, not a test failure.

## 8. Metrics

Non-SLA (Windows x64, release, 2026-09-01): 100-run grants matrix — handles 91→91, RSS −213 KiB; 20 pause/terminate DAP cycles — handles 91→91, RSS byte-flat; 100 sequential DAP sessions — handles 91→91 (M10 gate rerun).

## 9. Risks and follow-ups

- DAP thread mapping for future guest tasks is a separate contract decision; v1 claims global safe-pause only.
- The changing-grants matrix covers fs authority; net grants follow the same code path and are covered by construction, with HTTP itself gated on M12.

## 10. Audit links

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md)
- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md)
- [`STEP-0105`](./STEP-0105-scheduler-core.md), [`STEP-0106`](./STEP-0106-cancellation-race-select.md), [`STEP-0107`](./STEP-0107-bounded-channels-streams.md)
