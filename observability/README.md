# Sico observability/debug contracts v0

This directory is the machine-readable source of truth for M10 observability and minimal DAP boundaries. Normative semantics and ownership are defined by [`RFC-0035`](../docs/rfc/RFC-0035-runtime-observability-debug-v0.md).

## Files

- [`schema/observability-contract-v0.schema.json`](./schema/observability-contract-v0.schema.json): strict shapes for debug identity/map, Runtime fault, cancel request and one execution event;
- [`schema/dap-claimed-subset-v0.schema.json`](./schema/dap-claimed-subset-v0.schema.json): exact DAP claim document shape;
- [`schema/cancellation-race-v0.schema.json`](./schema/cancellation-race-v0.schema.json): cancellation/terminal-race matrix shape;
- [`contracts/dap-claimed-subset-v0.json`](./contracts/dap-claimed-subset-v0.json): sole DAP request/event allowlist;
- [`contracts/cancellation-race-v0.json`](./contracts/cancellation-race-v0.json): typed cancellation cases and both-order race expectations;
- [`fixtures/contract-cases-v0.json`](./fixtures/contract-cases-v0.json): positive documents plus unknown-field, duplicate, unsafe-integer, stale-digest and over-limit rejection fixtures;
- [`tools/validate-step-0095.ps1`](../tools/validate-step-0095.ps1): offline strict validator.

## Current evidence boundary

STEP-0096 proves deterministic debug artifacts, STEP-0097 proves typed Runtime source faults, and STEP-0098 proves the bounded cancellation request plus real Windows console bridge. Execution events, cross-platform signal parity and DAP remain STEP-0099–0102 claims.

## Validate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0095.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0096.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0098.ps1
```
