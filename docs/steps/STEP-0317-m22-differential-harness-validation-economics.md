# STEP-0317: M22 differential-harness validation economics

> - status: complete / validation infrastructure; M22 NO-GO unchanged
> - phase: M22 support (self-host differential iteration cost; owner directive 2026-09-29 「规划好这些优化，完成M22」)
> - completed: 2026-09-29
> - evidence: internal-fixture, Windows x64 real runner (pinned rustc `1.98.0-x86_64-pc-windows-gnu`, repo-local msys2-binutils), warm/ cold-cache serial and parallel in-process measurements

## Objective

The 2026-09-29 owner directive asked for planned validation-cost optimization
before continuing M22. Measured baseline on this host: one filtered
`selfhost_compiler` invocation paid ~22 s of fixed cost (the in-process
`sico build` of the 16,390-line selfhost tree through the debug-built Rust
frontend, once per test process), the 51-test differential suite ran 57.05 s
serial, and the full runner suite 227.8 s serial.

## Changes

- `runner/sico-runner/tests/support/selfhost_common.rs` (new, shared by the
  selfhost differential binaries):
  - On-disk component cache keyed by SHA-256 over the selfhost source texts
    plus every compiler-crate source file, so any edit that could change
    build output rotates the key. Atomic temp-file+rename store, bounded
    pruning (64 entries), `SICO_SELFHOST_COMPONENT_CACHE=0` kill switch.
  - `PreparedPool`: lazily-grown pool of engine-isolated `PreparedProgram`
    slots (each slot its own `Runner`/Engine, at most 4), so one slot's
    timeout watchdog cannot advance another slot's epoch. Every `run` still
    allocates a fresh Store, resource table and fuel budget. Slots are built
    on demand, so a single-fixture invocation compiles exactly one.
- `tests/selfhost_compiler.rs`: `compile_guest` reads the cache;
  `run_guest_with_args` uses the pool instead of the STEP-0254 global
  mutex. Per-fixture limits unchanged (fuel 5,000,000,000 / 30 s / 512 MiB).
- `tests/selfhost_parser.rs`: same cache + pool; previously rebuilt the
  parser component once per test with no reuse at all.
- `runner/sico-runner/Cargo.toml`: `[profile.dev.package."*"] opt-level = 1`
  — differential wall time sits inside wasmtime/cranelift and the in-process
  frontend, all of which are non-member dependencies of this workspace.
  Workspace members keep debug codegen so test-file iteration rebuilds stay
  fast.
- `tools/run-ci.ps1`: `Invoke-Step` now reports elapsed time on every
  PASS/FAIL line, making the full loop's cost distribution visible per step.

## Measured results (this host)

| Loop | Before | After |
|---|---|---|
| Filtered single-test invocation, cold component cache | 23.59 s | 4.55 s |
| Filtered single-test invocation, warm cache | 23.59 s | 1.82 s |
| `selfhost_compiler` suite, serial | 57.05 s | 12.98 s |
| `selfhost_compiler` suite, parallel through the pool | 57.05 s | 9.35 s (51/51 green) |
| Full runner suite, serial (`--test-threads=1`, CI shape) | 227.8 s | 102.7 s |

One-time cost: the opt-level 1 dependency rebuild measured 2 min 56 s.
CI's fresh runner pays that rebuild inside its existing budget and gains the
faster execution; the component cache is local-loop only (CI cache is cold
by construction).

## Oracle discipline (unchanged)

- Every fixture still runs in a fresh Store with the original limits;
  byte-exact comparison against the freshly computed Rust oracle happens on
  every run.
- A stale cache entry can only fail a differential (old component versus
  fresh oracle), never pass one falsely; the key rotates on any selfhost or
  compiler-crate edit, and the kill switch forces a fresh build when
  investigating.
- No language support, lowering frontier or bootstrap claim changes; the
  formatter canary stays 30/30 and the parser frontier stays
  `lex_words` / CALL-TARGET.

## Honest residuals

- The root-workspace test step of `run-ci.ps1` is unchanged this STEP; its
  cost is now visible per step and is the measured follow-up candidate
  (separate STEP, because root dev-profile policy affects every build).
- `selfhost_formatter.rs` and `selfhost_checker.rs` still drive
  `sico format`/`check` per fixture in-process (a different shape); recorded
  as cache candidates, not migrated.
- Parallel invocation benefits the local loop only; CI keeps
  `--test-threads=1` for the process-wide RSS/handle budget asserts.
- This is test-harness overhead reduction, not guest self-compilation
  performance evidence; it enters no M22 exit gate (gate 4 still requires
  the S6 budget record).
