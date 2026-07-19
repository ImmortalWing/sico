# Unified sico run, eval and safe caches evidence

> - date: 2026-07-17
> - step: STEP-0082
> - status: complete

## Implemented boundary

`sico run FILE.sico -- ARGS` compiles through the script-v0 profile, caches the Program Component under the frozen `SICO-SCRIPT-SOURCE-CACHE-V0` identity (ten fixed-order fields: compiler executable digest, compiler build ID, semantics `sico.ir.v0`, Script WIT `sico:script@0.1.0`, adapter digest, exact source bytes, app id/version, profile `script-v0`, manifest schema `sico.sapp.manifest.v1`, codegen options; variable fields as u64-LE length + bytes), and executes through `sico-runner` across the executable boundary — no compiler-to-Runtime crate dependency. `sico run -` consumes process stdin as source and gives the guest empty stdin; one stream is never consumed twice.

`sico eval "EXPR"` proves the expression through the scalar frontend and evaluates it at compile time; v0 accepts compile-time-constant `Int` expressions only (`40 + 2` → stdout `42`), everything else exits 120 with a typed diagnostic. No new source grammar was added; the Candidate A script is synthesized as verified IR in memory.

Cache behavior: entries are created via create-new temporary files with atomic commit; byte-identical completed entries are reused; conflicting content under one key fails closed (`Conflict`); corrupt entries fail structural revalidation and are refused (`Corrupt`) — never executed, never overwritten silently. Hits skip compilation entirely. The cache directory is `SICO_CACHE_DIR` or the platform user cache.

Exit mapping end to end: compile diagnostics → 120, CLI/IO/cache failures → 121, runner outcomes pass through unchanged (0–119, 122–127), missing/incompatible runner → 127. `SICO_RUNNER` is authoritative when set (a broken override fails closed at launch); otherwise a sibling executable or PATH. Diagnostics are JSON (`sico.run.diagnostic.v0`) on stderr with `--json`; stdout carries only guest stdout.

## Evidence

`tools/validate-step-0082.ps1` records `STEP_0082_OK`:

- repeat runs of an unchanged source reuse the byte-identical entry (content and write-time unchanged, correct output, no rewrite);
- a changed source compiles to a fresh entry (run exits 7) while the old entry stays;
- `run -` gives the guest empty stdin; changed/unchanged runs keep stdout exact;
- `eval "40 + 2"` prints 42; a non-constant expression exits 120;
- the reject fixture exits 122; a missing runner override exits 127; a corrupted cache entry exits 121;
- `--json` failures emit the `sico.run.diagnostic.v0` schema.

## Honest limits

- Machine-code caching: the runner's engine-cache feature (`wasmtime-internal-cache`) is not available in the offline dependency cache of this host, so per-process compilation is the measured behavior for now; the source cache still removes repeated compiler work. This is a dependency-availability deferral, not a design rejection, and STEP-0084 records the numbers.
- The runner timeout remains epoch-based; cache entries are performance artifacts, never trust roots — the runner revalidates at execution regardless of cache state.
- STEP-0077–0081 evidence and the full workspace regression remain green.

## Reproduction

```powershell
.\tools\validate-step-0082.ps1
```
