# Sico observability/debug contracts v0

This directory is the machine-readable source of truth for M10 observability and minimal DAP boundaries. Normative semantics and ownership are defined by [`RFC-0035`](../docs/rfc/RFC-0035-runtime-observability-debug-v0.md).

## Files

- [`schema/observability-contract-v0.schema.json`](./schema/observability-contract-v0.schema.json): strict shapes for debug identity/map, Runtime fault and one execution event;
- [`schema/dap-claimed-subset-v0.schema.json`](./schema/dap-claimed-subset-v0.schema.json): exact DAP claim document shape;
- [`schema/cancellation-race-v0.schema.json`](./schema/cancellation-race-v0.schema.json): cancellation/terminal-race matrix shape;
- [`contracts/dap-claimed-subset-v0.json`](./contracts/dap-claimed-subset-v0.json): sole DAP request/event allowlist;
- [`contracts/cancellation-race-v0.json`](./contracts/cancellation-race-v0.json): typed cancellation cases and both-order race expectations;
- [`fixtures/contract-cases-v0.json`](./fixtures/contract-cases-v0.json): positive documents plus unknown-field, duplicate, unsafe-integer, stale-digest and over-limit rejection fixtures;
- [`tools/validate-step-0095.ps1`](../tools/validate-step-0095.ps1): offline strict validator.

## Current evidence boundary

STEP-0096 now proves deterministic compiler debug maps, a compact Component digest link and exact identity sidecars through the authority-free `sico-observability` crate. It does not prove that the runner produces source frames/events, that Ctrl+C becomes typed cancellation, or that a DAP adapter exists. Those claims remain STEP-0097–0101.

## Validate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0095.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0096.ps1
```
