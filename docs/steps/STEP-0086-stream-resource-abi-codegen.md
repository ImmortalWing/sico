# STEP-0086: resource Canonical ABI and stream codegen

> - status: complete
> - phase: M9
> - started: 2026-07-18
> - completed: 2026-07-18
> - owners: autonomous-agent

## 1. Objective

Implement RFC-0030: owned/borrowed stream handles over the Canonical ABI, exact drop, no use-after-close, bounded host calls — proven by chunked passthrough at bounded memory.

## 2. Delivered

- `sico:script/streams@0.1.0` emitted by the compiler (resources with `SubResource` bounds, methods, freestanding `pump`) and implemented by `sico-runner` (per-Store `ResourceTable`, ≤ 64 live handles).
- Intrinsics: `sico.stream.stdin/stdout/stderr/read/write/flush/pump/close_input/close_output`; read clamp 64 KiB, write all-or-error ≤ 1 MiB, error enums map to the frozen four-text table.
- Exact drop via canonical `resource.drop`; stale-handle reuse traps (exit 125, measured).
- Mixing rule implemented on both sides: streams components skip buffered stdin (CLI inherits stdin to the runner; runner leaves `script-input.stdin` empty).
- Per-call budget top-up (1M fuel / 64 MiB hostcall) — streamed volume unbounded, compute between calls bounded.
- Runner wasm stack 4 MiB (bounded) for the recursive guest pump until the STEP-0087 flow backend.

## 3. Exit evidence

`tools/validate-step-0086.ps1` green; 1/16 MiB guest pump exact; 256 MiB host pump exact with 10.8 MiB peak runner RSS; use-after-close traps; full workspace regression (fmt/clippy 0 findings/67 suites) and runner release tests 6/6 green; STEP-0083 pilots unaffected. Report: [`script-streaming-v0`](../reports/script-streaming-v0.md).

## 4. Honest limits

Guest per-chunk transform beyond arena capacity (~64 MiB) awaits the STEP-0087 flow backend; blocked-host-read interruption lands with the STEP-0088 async runner; guest use-after-drop traps rather than returning typed `closed`.

## 5. Links

- [`RFC-0030`](../rfc/RFC-0030-script-streaming-v0.md)
- [`M9 plan`](../plans/M9-streaming-async-interactive.md)
