# STEP-0094: M9 security, performance and exit audit

> - status: complete
> - phase: M9
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent
> - decision: GO

## Objective

Independently rerun every M9 contract/security/performance boundary, retain the complete M0–M8 regression, publish an execution-only platform matrix and issue an honest M9 GO/NO-GO decision.

## Delivered

- `tools/validate-step-0094.ps1` isolates and reruns STEP-0084 plus STEP-0086–0093, while explicitly checking the STEP-0085 WIT/RFC contract.
- [`M9 exit audit`](../reports/m9-exit-audit-v0.md) records the gate matrix, latest measurements, security findings, honest platform claims and residual limits.
- STEP-0084 performance validation now uses the benchmark's required 20 samples, fixing a false-P95 failure mode caused by ten-sample nearest-rank P95 equalling the maximum. Non-SLA goals are reported as comparisons, matching M8 §11, while functional/security regressions remain hard failures.
- [`M10`](../plans/M10-runtime-observability-debugging.md) is planned as the next coherent dependency chain; no M10 implementation is claimed by this step.

## Exit evidence

- M8 aggregate: full workspace fmt/Clippy/tests green; 20-sample cold/warm/runner-only measurements recorded with explicit non-SLA comparison; decision GO.
- M9: 1/16/256 MiB exact streaming, 10.8 MiB peak runner RSS, blocked I/O/HTTP cancellation exit 123, scoped-network denial, persistent Store isolation, 5.994 ms warm median, 7.8 MiB REPL peak and typed syntax/tooling evidence all green.
- Platform claim: M9 Runtime verified on Windows x64 only. No Linux/macOS/mobile M9 execution is inferred.

## Honest limits

Parallel task scheduling, TLS HTTP, declaration REPL cells, typed Ctrl+C delivery, Runtime source frames and DAP remain unavailable. The first four do not invalidate the frozen M9 v0 contracts; source frames/signals/debugging form the planned M10 scope.
