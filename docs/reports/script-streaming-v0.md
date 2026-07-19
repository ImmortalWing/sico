# Streaming resource Canonical ABI and stream codegen evidence

> - date: 2026-07-18
> - step: STEP-0086
> - status: complete

## Implemented boundary

RFC-0030 streaming is real code end to end: the `sico:script/streams@0.1.0` interface (imported resources `input-stream`/`output-stream` with `SubResource` bounds, `stream-error` enum, methods plus the freestanding `pump`) is emitted by the compiler, lowered through the STEP-0083 transport-memory pattern, and implemented by `sico-runner` with `wasmtime::component::ResourceTable` handles that die with the Store.

Guest side: `sico.stream.stdin/stdout/stderr` construct handles (Canonical ABI resource indices); `sico.stream.read(handle, max)` (clamped 64 KiB/call), `sico.stream.write(handle, bytes)` (all-or-error ≤ 1 MiB/call), `sico.stream.flush(handle)`, `sico.stream.pump(input, output)` (bounded host loop returning total bytes), `sico.stream.close_input/close_output` (canonical `resource.drop`). Errors map to the frozen four-text table (`io`/`cancelled`/`closed`/`resource-limit`) in the data segment; discriminants outside the WIT order trap. Programs not using streams keep byte-identical artifacts.

Runner side: handles are per-Store resource-table entries (≤ 64 live); stale handles after drop are rejected by the canonical layer (measured trap, exit 125); `read` blocks on the OS stream (backpressure), `write` is all-or-error; budgets top up per host call (1M fuel / 64 MiB hostcall transport) so streamed volume is unbounded while compute between calls stays bounded — the epoch deadline remains the ultimate non-termination bound.

Mixing rule (RFC-0030): when the compiled component imports the streams interface, the CLI never buffers stdin (runner inherits it, backpressure reaches the producer) and the runner leaves the buffered `script-input.stdin` empty — each channel is consumed exactly once. Detection is exact import-name matching on both sides (`wasmparser` in the CLI, `component_type().imports` in the runner).

## Evidence

`tools/validate-step-0086.ps1`:

- read-once: `hello stream` round-trips through a streaming handle into the buffered result, exit 0;
- guest pump (recursive per-chunk loop): 1 MiB → exactly 1,048,576 bytes, 16 MiB → exactly 16,777,216 bytes, exit 0;
- host pump (`sico.stream.pump`): **256 MiB → exactly 268,435,456 bytes with runner peak WorkingSet 11,284,480 bytes (10.8 MiB)** — streaming memory is bounded independently of total input size (M9 §9 key goal, measured);
- use-after-close: `close_input` then `read` on the stale handle traps (exit 125, fail closed by the canonical resource table);
- mixing rule: a streams component reads the OS stdin through the stream, not the buffered channel;
- full workspace regression: `cargo fmt --check`, `cargo clippy -D warnings` (0 findings), `cargo test --workspace --all-targets --all-features` (67 suites) green; `sico-runner` release tests 6/6 green (MSVC); STEP-0083 pilots (args/word-count/JSON/fs) unaffected.

## Honest limits

- The guest per-chunk pump is recursive (the surface has no loop construct): runner wasm stack is set to 4 MiB (bounded), covering ~8k iterations at 64 KiB chunks (~512 MiB) — 256 MiB guest-side recursion fits, but the arena also accumulates per-chunk allocations, so unbounded guest-side transforms land with the STEP-0087 flow backend; bulk forwarding beyond arena capacity uses `sico.stream.pump`.
- v0 cancellation is observed between host calls (and between pump iterations); interrupting a blocked synchronous host read is the STEP-0088 async-runner item.
- Guest-initiated use-after-drop traps (125) per the canonical ABI rather than returning typed `closed`; `closed` covers host-side staleness checks.
- Backpressure is the OS pipe itself (blocking read/write); no intermediate queue exists to grow.
