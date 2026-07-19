# M9 security, performance and exit audit

> - status: accepted
> - date: 2026-07-19
> - phase: M9
> - decision: GO

## Decision

M9 is **GO**. STEP-0085–0093 independently cover the frozen stream contract, affine resources, structured sequential tasks, cancellable streaming I/O, scoped HTTP, persistent-runner isolation, bounded watch/REPL state, the explicit-main syntax decision and shell-free tooling plans. `tools/validate-step-0084.ps1` also reran the complete workspace format, Clippy and test gates plus the M8 final-pipeline benchmark, so the required M0–M8 regression is green.

This decision is limited to the actual Windows x64 runner evidence below. It does not turn contract-only or cross-compiled artifacts into Linux, macOS, Android or public-production support claims.

## Exit-gate matrix

| Gate | Evidence | Result |
|---|---|---|
| Stream contract | RFC-0030 + parsed `sico:script/streams@0.1.0` WIT | pass |
| Resource lifetime and throughput | exact close/drop, stale-handle trap, 1/16/256 MiB passthrough | pass |
| Structured task/cancel | sequential task scope, E5101/E5102, cancelled exit 123 | pass |
| Blocked I/O cancellation | bounded depth-1 workers; blocked read/pump/write return 123 | pass |
| Scoped HTTP | default deny, exact host/port, bounds, timeout/cancel, no redirect following | pass |
| Persistent isolation/watch | fresh Store per execution; `0,122,0` recovery in one PID; invalid source not run | pass |
| REPL bounds | 4 KiB line, 256 cells, 16 KiB history, rollback/reset/exclusive export | pass |
| Top-level syntax | explicit typed `main` accepted; shorthand candidates fail closed with E1013 | pass |
| Editor/AI execution | shared direct-argv plan, `shell:false`, bounded capture, honest debug refusal | pass |
| M0–M8 regression | workspace fmt, Clippy `-D warnings`, all-target/all-feature tests, M8 benchmark | pass |

## Security findings

- Stream handles live in a per-Store resource table, are capped at 64 and cannot survive Store teardown. A closed/stale handle traps instead of aliasing a later resource.
- Streaming uses blocking OS backpressure plus depth-1 worker channels; it never adds an unbounded output capture queue.
- Network authority is absent by default. HTTP validates scheme, canonical host, exact port, method and sizes before DNS; wildcard grants, wrong ports, HTTPS and unsupported methods fail closed.
- A persistent runner reuses Engine/Component/Linker compilation state only. Store, resource table, I/O workers, cancellation state, budgets and provider authority are recreated for each run.
- LSP and AI return data-only execution plans. Metacharacters remain individual argv values, shell execution is false, and neither surface gains filesystem, process or network authority.

No unresolved M9 P0 correctness or authority-expansion issue was found. The audit did find and repair a validation-quality defect: STEP-0084 had reduced the benchmark to ten samples, making nearest-rank P95 equal the maximum. Restoring the script's 20-sample contract makes the percentile meaningful. The validator now records comparison with STEP-0076's explicitly non-SLA goals instead of contradicting M8 §11 by treating host contention as a correctness failure.

## Performance evidence

Latest STEP-0094 quality run on Windows x64:

| Measurement | Result | Gate |
|---|---:|---|
| Guest passthrough | 1 MiB and 16 MiB exact | exact |
| Host passthrough | 256 MiB exact; 10.8 MiB peak runner RSS | memory bounded independently of input |
| Blocked read / pump / write cancellation | 127 / 124 / 136 ms total, each with a 100 ms timer | post-token delivery well below 100 ms |
| HTTP cancellation | 130 ms total with 100 ms timer | pass |
| HTTP timeout | 5,165 ms for a 5 s deadline | pass |
| Persistent cached rerun | 5.994 ms median over 32 runs | < 20 ms |
| REPL 256-cell growth | 74 ms; 7.8 MiB peak RSS | bounded |
| M8 `sico run` cold P95 | 83.6441 ms final aggregate; 109.0825 ms CPU-saturated run | 20 samples/run; compared with non-SLA 200 ms goal |
| M8 `sico run` warm P95 | 57.0656 ms final aggregate; 126.6373 ms CPU-saturated run | 20 samples/run; saturated run records non-SLA goal miss |
| M8 runner-only P95 | 36.8591 ms final aggregate; 88.4598 ms CPU-saturated run | 40 samples/run; confirms host-wide slowdown |

The cancellation totals include process creation and the 100 ms test timer; token-to-observation is therefore much less than the displayed wall time. These are local measurements, not cross-platform SLAs.

## Platform matrix

| Platform | Build/contract evidence | M9 runner execution | Claim |
|---|---|---|---|
| Windows x64 | GNU workspace plus MSVC runner builds/tests | stream, cancel, HTTP, watch and Script execution | runtime-verified for M9 |
| Linux/WSL | older M8 evidence only | no STEP-0085–0094 native rerun | not M9-verified |
| macOS | declarations/contracts only | none | not verified |
| Android/Harmony | separate blocked/deferred tracks | none | not verified |

## Accepted limits and next milestone

M9 deliberately remains a sequential executor; HTTP v0 has no TLS, streaming upload or chunked response; watch covers one file; REPL is expression-only; OS signals are not converted into typed cancellation; Runtime source locations and DAP are unavailable. These are explicit limits rather than hidden support claims.

The next active milestone is [`M10 runtime observability and debugging`](../plans/M10-runtime-observability-debugging.md). It starts with STEP-0095 and focuses on one dependency chain: versioned debug/source identities → runtime source frames → signal-to-cancellation delivery → event/log protocol → minimal DAP and client integration. HTTP TLS and parallel task scheduling remain separately deferred so M10 does not mix unrelated Runtime redesigns.

## Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0094.ps1
```
