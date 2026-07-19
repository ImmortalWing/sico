# STEP-0088: async runner and streaming stdio

> - status: complete
> - phase: M9
> - started: 2026-07-18
> - completed: 2026-07-18
> - owners: autonomous-agent

## 1. Objective

Cancellation must reach blocked host calls: async-capable Store behavior, timeout/cancel, 64 KiB-class chunks, bounded queues, no whole-output capture.

## 2. Delivered

- Worker-thread IO layer in `sico-runner` (`runner/sico-runner/src/lib.rs`): stdin reads and stdout/stderr writes/flushes run on dedicated threads behind rendezvous channels (bounded depth 1); every host call waits in a 2 ms cancel-check loop and returns `stream-error.cancelled` when the token fires.
- `sico.stream.pump` uses the same cancellable reads/writes; 256 MiB passthrough still exact.
- Worker request enqueue is cancellation-aware even when the depth-1 queue is full; stream errors are terminal, resource drops delete their table entry, and output drop performs only a best-effort non-blocking flush.
- CLI outcome reporting acquires stdout only for successful buffered output, so a cancelled run cannot deadlock behind an OS-blocked streaming writer that owns stdout.
- `--cancel-after-ms N` runner flag as the deterministic cancellation hook for evidence and ops.
- No whole-output capture anywhere on the streaming path (unchanged from STEP-0086, re-verified).

## 3. Exit evidence

`tools/validate-step-0088.ps1`: blocked-read 120 ms, blocked-pump 125 ms and blocked-write 134 ms total wall in the final quality run (spawn + 100 ms timer included), all returning control exit 123; 256 MiB streaming regression exact; workspace and runner tests green. Report: [`script-async-runner-v0`](../reports/script-async-runner-v0.md).

## 4. Honest limits

Sync Store (no async executor offline); cancellation delivered at host-call granularity, not via wasmtime `async_support`; an OS-blocked worker may remain blocked until the one-shot process exits, but cannot block the runner's cancellation outcome; Ctrl+C integration is STEP-0093.

## 5. Links

- [`M9 plan`](../plans/M9-streaming-async-interactive.md) (§6/§9)
- [`RFC-0030`](../rfc/RFC-0030-script-streaming-v0.md) (cancellation matrix)
