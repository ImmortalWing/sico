# STEP-0093: editor/debug/AI execution integration

> - status: complete
> - phase: M9
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## Objective

Unify editor and AI run/watch/REPL launch data, cancellation ownership, source-map availability and bounded log capture without shell interpolation or a false debugger claim.

## Delivered

- New `sico-tooling-protocol` workspace crate and strict `sico.execution-plan.v0` schema.
- Exact direct argv for check/run/watch/REPL; paths and metacharacters remain individual arguments with `shell: false`.
- Capture limit `1..=1 MiB`, explicit truncate marker, client process-tree cancellation and compile-only source coordinates.
- LSP adds watch/REPL plans while preserving typed debug refusal `-32004`.
- AI tools add data-only `plan_execution`; no process, filesystem, network or model authority is added.
- Module-boundary contract now covers 24 packages and four narrow, reasoned, live dependency exceptions.

## Exit evidence

`tools/validate-step-0093.ps1` is green. It runs unit tests, validates module boundaries/schema, drives the real `sico-ai-tool` binary with literal shell metacharacters, and drives the real `sico-lsp` Content-Length protocol through run/watch/REPL plus debug refusal.

Report: [`tooling-execution-plan-v0`](../reports/tooling-execution-plan-v0.md).

## Honest limits

The shared value is a launch plan, not an executor. Clients must implement the capture/truncation and process-tree termination obligations. Runtime source locations, typed signal-to-exit-123 delivery and DAP hooks remain unavailable and are not claimed.
