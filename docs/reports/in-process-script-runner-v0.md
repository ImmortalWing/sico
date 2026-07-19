# Structured in-process sico-runner evidence

> - date: 2026-07-17
> - step: STEP-0081
> - status: complete
> - runtime: Wasmtime 46.0.1 (in-process crate), runner exit mapping per RFC-0029

## Implemented boundary

`runner/sico-runner` is a product crate excluded from the workspace (the workspace GNU toolchain cannot link the Wasmtime crate on this host; the runner builds with the pinned MSVC toolchain and does not weaken the GNU gate). It invokes the `sico:script/program@0.1.0.run` export directly — the STEP-0076-tested fallback path — in-process, one fresh Store per call with no WASI imports at all: the guest receives no environment, no working directory, no filesystem and no network by construction.

Every Runtime outcome is a typed `RunOutcome`; nothing is recovered from engine error text:

- `Output(stdout, stderr, exit 0..=119)` — exact guest exit values pass through; out-of-range values are rejected as `Trap`, never truncated;
- `Domain{code, message}` — guest `ScriptError` → exit 122;
- `Cancelled` — a `CancelToken` consulted at every epoch tick → exit 123;
- `Timeout` — the runner's own epoch-deadline marker error → exit 124;
- `FuelExhausted` — read back as zero remaining fuel after a failed call → exit 125;
- `MemoryLimit` — the runner's own `ResourceLimiter` records every denial → exit 125;
- `Trap` — any other guest trap, malformed result or exit-contract violation → exit 125;
- `Launch` / `Incompatible` → exits 126/127; CLI/IO failures of the runner itself → 121.

M8 input bounds (1,024 arguments, 64 KiB per argument, 1 MiB total, 8 MiB stdin) are enforced before any guest execution. Canonical-ABI hostcall fuel is budgeted per call from the measured input size.

## Evidence

The runner's own test suite (6 tests, release mode, MSVC) plus `tools/validate-step-0081.ps1`:

- echo roundtrips with Unicode arguments and NUL/non-UTF-8/1 MiB binary stdin are byte exact; exit code 42 passes through exactly; exit 120 is rejected, not truncated;
- domain error maps to 122 with exact code/message;
- malicious guests — `unreachable` trap, fuel exhaustion on a non-terminating loop, wall-clock timeout, a memory ceiling below the program minimum, and a malformed-result core guest — all fail closed, and the host keeps executing normal guests afterwards;
- cancellation reaches a blocked guest within ~50 ms (exit 123);
- a composed WASI command is not a runnable Program for the direct runner and is refused without execution;
- CLI smoke: `sico-runner echo.component.wasm -- alpha` mirrors both channels with exit 0; reject maps to 122 with a JSON diagnostic (`sico.runner.outcome.v0`) on stderr; missing file → 121; garbage artifact → 127.

## Honest limits

- The runner executes Program Components directly; the composed Adapter path (with its 0/1 WASI exit semantics) remains the `sico-app compose` artifact. Unified `sico run` selects and caches between them in STEP-0082.
- Timeout is epoch-based (5 ms ticks) and not a deterministic bound; fuel is the deterministic non-termination gate.
- The runner crate requires the MSVC toolchain on this host; the workspace gate is unchanged and stays GNU.
- STEP-0077–0080 evidence and the full workspace regression remain green.

## Reproduction

```powershell
.\tools\validate-step-0081.ps1
# or manually:
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'  # inside vcvars64
cargo test --release --offline --manifest-path runner\sico-runner\Cargo.toml
```
