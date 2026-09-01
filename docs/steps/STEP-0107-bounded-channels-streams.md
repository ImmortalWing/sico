# STEP-0107: bounded channels, streams and backpressure

> - status: complete
> - phase: M11
> - started: 2026-09-01
> - completed: 2026-09-01
> - owners: autonomous-agent

## 1. Objective

Add task-aware bounded channels to the scheduler core with explicit close/move/cancellation discipline and shared run-level accounting, per ADR-0010 and the M11 plan, without changing the sequential-v1 guest profile or any M10-frozen claim.

Verifiable results:

- bounded channels: per-channel item capacity 0..=1,024 (0 = rendezvous) plus an explicit byte budget (≤16 MiB); buffered items and record bytes charge the run's metadata budget;
- backpressure by suspension, not spinning: a full channel blocks the producer (task suspended, out of the ready queue); delivery wakes consumers FIFO; a freed slot promotes the oldest blocked producer;
- explicit close drains buffered items then answers `closed`; failure close (producer cancelled/failed) propagates `failed` to waiters; use-after-close, double-close, non-owner close and oversized single items are stable typed faults;
- affine ownership: one owner per channel, `move_channel` transfers it, the old owner's close is a typed failure;
- cancellation integration: cancelling a task removes it from every wait queue and failure-closes the channels it owns; teardown idempotently closes any remaining channels in reverse order (ADR-0010 §31: success/fault/cancel/teardown share one cleanup path);
- exit fixtures: capacity 0/1/max/max+1, slow consumer, early close, producer failure, cancellation; channel-deadlock is provable; a 256 MiB relay through a 4-item channel keeps RSS independent of total stream size.

## 2. Context and evidence

- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md): channel capacity 1,024 items + explicit byte budget; affine resource ownership; cancellation closes child resources in deterministic reverse order; idempotent cleanup path.
- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md) §9: channel/stream source spellings are not part of the accepted surface; this step is runtime-only, same discipline as STEP-0105/0106.
- [`STEP-0105`](./STEP-0105-scheduler-core.md): scheduler core, budgets, ingress.
- [`STEP-0106`](./STEP-0106-cancellation-race-select.md): cancellation tree and abandon discipline that channels hook into.

## 3. Scope

In scope:

- channel table inside `SchedulerCore` (open/send/recv/close/move), waiter queues, backpressure, budgets, typed faults;
- cancellation/teardown/deadlock integration;
- synthetic fixtures for every exit-evidence line; runner-side RSS relay evidence.

Out of scope:

- channel/stream source syntax, IR ops or Component ABI (no guest-visible surface yet — sequential-v1 guests cannot suspend host-side; a guest ABI would be dead weight until a suspension profile exists);
- payload storage: in v1 the channel core is the accounting/synchronization engine — items carry `(sender, bytes)`, payload bytes stay in task/guest memory. Payload buffers land with the profile that can actually hand them across a suspension;
- select-over-channels composition (the STEP-0106 select already resolves over tasks; channel operands join when a source profile needs them).

## 4. Options and decision

### 4.1 Blocking discipline

- **Candidate A (chosen): block by suspension.** A send to a full channel (or any send on a rendezvous channel without a waiting receiver) suspends the producer in a FIFO waiter queue; a recv on an empty open channel suspends the consumer. Wakeups are exact (delivery/freed slot/close), so there is no polling and no busy spinning to measure away. The suspended task is not in the ready queue, which the fixtures assert directly.
- Candidate B: typed `WouldBlock` returns and caller-side retry. Rejected: it externalizes backpressure into busy loops — exactly the failure the exit evidence forbids — and it cannot express rendezvous.

### 4.2 Close semantics

Chosen: `close` is owner-only and idempotent-refused (double close is a typed fault, matching the affine discipline); after close, buffered items drain in FIFO order and only then does `recv` answer `closed`; waiters at close time wake immediately with the close kind (`closed`/`failed`). Producer failure (cancelled or failed task owning the channel) failure-closes its channels so consumers can never hang on a dead producer.

### 4.3 Deadlock interplay

Chosen: `detect_deadlock` keeps its STEP-0106 rule — runnable work or pending Host operations disprove deadlock — which is exactly right for channels: a channel waiter can only be woken by a runnable counterpart or an external completion, so all-suspended with no pending Host operations is a true absorbing state even when the waits are channel waits.

## 5. Plan

1. This step doc (no new RFC: ADR-0010's channel limit row + resource rules freeze the contract; no source surface changes).
2. `scheduler.rs`: channel table, waiter discipline, budgets, faults, cancellation/teardown hooks.
3. Unit fixtures for every exit-evidence line; runner.rs RSS relay evidence.
4. `tools/validate-step-0107.ps1`, registry updates, full regression, commit.

## 6. Changes

- `runner/sico-runner/src/scheduler.rs` (+752 lines): channel table inside `SchedulerCore` — `ChannelId` (monotonic), `open_channel(owner, item_capacity, byte_budget)` (capacity 0 = rendezvous; caps: 1,024 items / 16 MiB bytes / 1,024 channels per run; records charge the metadata budget), `send` (direct handoff to a waiting receiver → bounded buffer → FIFO send-waiter suspension), `recv` (buffered FIFO, promoting the oldest blocked producer into each freed slot → rendezvous handoff → `closed` once drained → recv-waiter suspension), `close_channel` (owner-only, double-close typed, wakes all waiters, buffered items keep draining), `move_channel` (affine ownership transfer). New stable faults: `ChannelTableFull`, `ChannelItemsExceeded`, `ChannelBytesExceeded`, `UnknownChannel`, `ChannelClosed`, `NotChannelOwner`, `ChannelWaitConflict`. Cancellation hook: `cancel_members` removes cancelled tasks from every wait queue and failure-closes the channels they own (waiters wake, `recv` observes `failed`). Teardown hook: remaining open channels close in reverse registration order on the idempotent cleanup path. Backpressure is provably suspension-based: blocked producers are absent from the ready queue (`next_ready` returns `None` while producers are parked).
- `runner/sico-runner/tests/runner.rs` (+69 lines): `channel_relay_keeps_rss_independent_of_stream_size` — 1 GiB of payload accounting relayed through a 4-item/4 MiB channel in fill-then-drain rounds with one reusable 1 MiB buffer; scheduler metadata constant (368 bytes) and process RSS flat (±16 MiB bound), then idempotent teardown close.
- `tools/validate-step-0107.ps1` (new): fmt, clippy, scheduler unit fixtures, release runner suite with relay evidence capture, channel fault-surface stability, diagnostics/semantic-case oracles (no new source surface), STEP-0106 regression gate (transitively 0105/0104/0090/0087).

Design findings recorded by the fixtures:

1. A channel can never hold a send waiter and a recv waiter at once — a send facing a waiting receiver delivers directly, and a recv facing a buffered item or blocked sender consumes directly; the two wait queues are mutually exclusive by construction.
2. The byte budget blocks sends that merely overflow the remaining budget (backpressure) while a single item larger than the whole budget is a typed `ChannelBytesExceeded` — the former is congestion, the latter is a contract violation.
3. Channel waits need no separate deadlock rule: a waiter can only be woken by a runnable counterpart or an external completion, so the STEP-0106 absorbing-state proof applies unchanged.

## 7. Validation

Executed 2026-09-01 on Windows x64 GNU, Rust 1.98.0, Wasmtime 47.0.2:

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` green.
- Scheduler unit tests: 27/27 green, including 11 new channel fixtures — rendezvous both directions, capacity-1 buffer-then-block, typed limits (items/bytes/table) with backpressure blocking at max+1, slow-consumer FIFO fairness with empty ready queue while producers are parked (no spinning), early close draining then `closed`, close waking both waiter kinds, producer-failure propagation via cancellation, waiter removal on cancellation, affine move with old-owner refusal, wait-conflict and terminal-task refusals, and provable channel deadlock resolved by cancellation.
- Runner integration suite (release, single-threaded): 34/34 green; evidence log at `target/evidence/step-0107/runner-tests.txt`.
- Exit-evidence mapping: capacity 0/1/max/max+1 fixtures (`channel_capacity_zero_is_a_rendezvous`, `channel_capacity_one_buffers_then_blocks`, `channel_limits_are_typed_and_backpressure_blocks`); slow consumer (`slow_consumer_gets_fifo_fairness_without_spinning`); early close / producer failure / cancellation (`early_close_*`, `producer_failure_propagates_to_consumers`, `cancellation_removes_waiters_without_waking_them`); backpressure prevents queue growth and busy spinning (ready-queue-empty assertions in the rendezvous/fairness fixtures); move/use-after-close/double-close typed (`move_channel_transfers_ownership_affinely`, `early_close_*`); large-payload RSS independence (`channel_relay_keeps_rss_independent_of_stream_size`: 1 GiB through a 4-item/4 MiB channel, metadata constant 368 B, RSS 14,036,992→14,057,472, handles 97→97).
- Regression gates: `validate-step-0106.ps1` green, transitively 0105/0104/0090/0087; diagnostics/semantic-case oracles unchanged (no new source surface).
- Full workspace `cargo test --workspace` green.

## 8. Metrics

Non-SLA measurements (Windows x64, release, 2026-09-01):

| Workload | Result |
|---|---:|
| 1 GiB channel relay (4-item/4 MiB, 1,024 fill-drain rounds) | <1 ms wall; metadata 368 B constant |
| Channel fixture suite (11 tests) | included in 0.14 s debug scheduler suite |

Channel send/recv/promote are microsecond-scale; the relay proves throughput is accounting-bound, not storage-bound.

## 9. Risks and follow-ups

- v1 channels carry accounting identities, not payloads; a future suspension profile must keep the budget/waiter semantics frozen here when it adds payload buffers.
- No guest-visible channel ABI yet by design; STEP-0108+ integration must not invent one silently.
- The byte budget is per-channel; aggregate pressure is bounded by the run metadata budget and the 1,024-channel table cap.

## 10. Audit links

- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md)
- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md)
- [`STEP-0105`](./STEP-0105-scheduler-core.md)
- [`STEP-0106`](./STEP-0106-cancellation-race-select.md)
