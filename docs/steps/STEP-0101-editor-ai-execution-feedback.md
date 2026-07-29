# STEP-0101: editor and AI execution feedback integration

> - phase: M10
> - status: complete / GO
> - date: 2026-07-23

## Result

The tooling contract now defines strict `sico.debug-launch-plan.v0`. LSP `sico.debug` returns this plan from an explicit Component/debug-map/debug-identity/source/document descriptor. The plan launches `sico-dap` with five direct argv values, `shell: false`, no working-directory authority, 1 MiB DAP framing, SHA-256-bound artifact expectations and editor-owned terminate/disconnect lifecycle.

The AI tool can produce the same data-only debug plan but cannot spawn the adapter, open source/artifact paths, alter grants or send DAP requests. Its new `summarize_execution` operation accepts at most 256 strict `sico.execution-event.v0` values plus an optional strict `sico.runtime-fault.v0`. It rejects cross-run/generation mixes, non-monotonic sequence IDs, missing/duplicate terminal records and fault identity mismatch. The returned `sico.execution-summary.v0` contains stable event counts, terminal class, typed fault identity and bounded source frames only; guest output bytes and fault messages are deliberately omitted.

Existing run/watch/repl/check plans remain direct `sico` argv with `shell: false`. Debug support does not retroactively claim runtime source locations for those non-debug plans.

## Evidence

- `tooling/schema/debug-launch-plan-v0.schema.json`
- `crates/sico-tooling-protocol/src/lib.rs`
- `crates/sico-language-server/src/lib.rs`
- `crates/sico-ai-tools/src/lib.rs`
- `tools/validate-step-0101.ps1`

Tests cover metacharacter-bearing paths as individual argv values, exact LSP descriptors, data-only AI plans, removal of untrusted stdout payloads and rejection of mixed identities.

## Reproduction

```powershell
./tools/validate-step-0101.ps1
```

Expected final line begins with `STEP_0101_OK`.

## Evidence boundary

This step is local protocol/editor evidence. It does not claim a particular editor extension has been published, that an external AI model was invoked, or that production/mobile/platform gates closed. STEP-0102 owns the aggregate security, performance and platform decision.

