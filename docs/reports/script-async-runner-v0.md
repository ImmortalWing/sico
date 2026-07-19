# Async runner and streaming stdio evidence

> - date: 2026-07-18
> - step: STEP-0088
> - status: complete

## Implemented boundary

The runner's stream host calls no longer block the runner thread inside OS IO. A dedicated worker thread per direction (stdin reads, stdout/stderr writes+flushes) sits behind rendezvous channels (bounded depth 1); every host call waits in a 2 ms cancel-check loop, so a blocked call observes the run's `CancelToken` within one poll interval and returns `stream-error.cancelled` instead of hanging the run. The epoch deadline still guards guest-side loops; fuel/hostcall budgets still top up per stream call; the mixing rule and the 64 KiB read / 1 MiB write bounds are unchanged.

Async model note (honest): the offline toolchain has no async executor crate, so the Store stays synchronous and cancellation is delivered at host-call granularity through the worker/channel layer — no wasmtime `async_support` claim is made. The `--cancel-after-ms N` runner flag exists as the deterministic cancellation hook used by the evidence (ops/test, documented in usage).

## Evidence

`tools/validate-step-0088.ps1`:

- regression through the new IO layer: read-once echo exact; 256 MiB pump exact (268,435,456 bytes);
- blocked-read cancellation: stdin open with nothing written, `--cancel-after-ms 100` → guest sees `stream-error.cancelled`, control exit 123, total wall 120 ms;
- blocked-pump cancellation: same setup through `sico.stream.pump` → exit 123, total wall 125 ms;
- blocked-write cancellation: stdout is a full unread redirected pipe while a 256 MiB input is pumped → exit 123 in 134 ms; CLI result reporting does not reacquire the worker-held stdout lock;
- cancellation latency after the token fires is single-digit milliseconds, far inside the M9 §9 100 ms goal (measured);
- full workspace fmt/Clippy/tests green on GNU; `sico-runner` release Clippy and 8 tests green on MSVC; standalone STEP-0079 prototype Clippy green on MSVC; STEP-0084/0086/0087/0088 validators all green on 2026-07-19.

## Honest limits

- A worker thread blocked inside an OS read/write can stay blocked after cancellation and dies with the process (bounded: one-shot runner process, at most two IO workers per run). The runner thread and typed cancellation outcome do not wait for it.
- `--cancel-after-ms` is the only external cancellation trigger so far; CLI signal handling (Ctrl+C → CancelToken) is STEP-0093 integration work.
- Output is never whole-captured: pump streams chunk-by-chunk; buffered stdout (M8 profile) keeps its 8 MiB contract unchanged.
