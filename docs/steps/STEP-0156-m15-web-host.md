# STEP-0156: M15 web host v0 — shim substrate, cross-host matrix, real-browser evidence

> - status: complete (M15 exit gates 1, 5, 6 substance; full exit audit in STEP-0158)
> - phase: M15 plan §3.1/§3.4 per accepted ADR-0014 + RFC-0042
> - completed: 2026-09-11
> - owners: autonomous-agent
> - artifacts: [`web/harness.html`](../../web/harness.html) (self-contained: JS canonical-ABI shim + base64-embedded core module), `web/guest.wordcount.core.wasm`, [`crates/sico-cli/tests/generate_web_harness.rs`](../../crates/sico-cli/tests/generate_web_harness.rs) (#[ignore] generator), [`tools/validate-step-0156.ps1`](../../tools/validate-step-0156.ps1), evidence [`docs/evidence/m15/`](../evidence/m15/)

## 1. What was done

The ADR-0014 substrate implemented for the frozen Script v0 world:

- **Core-module extraction**: the embedded core module is pulled from the
  compiled Component (`wasmparser` ModuleSection range) — the browser
  executes exactly the bytes the native runner executes (module identity
  preserved across hosts).
- **JS canonical-ABI shim** (`sicoScriptRun`): lowers arguments/stdin
  through `cabi_realloc` into the guest linear memory, calls core `run`,
  lifts the 32-byte result area per the frozen layout (tag @0, script-
  output payload @8, exit_code s64 @16), strict-UTF-8 fail-closed on the
  error message, `cabi_post_run` teardown. Views are re-created after
  every grow (realloc detaches the buffer).
- **Self-contained harness page**: 5 stdin-driven cases over the
  business component `script-word-count` (word counting), results
  serialized into the DOM.
- **Real-browser evidence**: headless Microsoft Edge (Chromium)
  **152.0.4191.66** (`--headless=new --dump-dom`) executed the page; the
  DOM-carried results were captured and compared against the native
  runner byte-for-byte.

## 2. Cross-host matrix (exit gates 1 + 5 partial)

| case | native (Windows runner) | web (Edge 152) | match |
|---|---|---|---|
| two-words | `32` | `32` | ✓ |
| trim-and-punct | `33` | `33` | ✓ |
| empty | `30` | `30` | ✓ |
| one-word | `31` | `31` | ✓ |
| utf8 | `32` | `32` | ✓ |

5/5 byte-exact; machine-readable fixture
`docs/evidence/m15/cross-host-matrix.json` validated by
`validate-step-0156.ps1` (green).

## 3. Declared degradation (per ADR-0014, never hidden)

- No M12 HTTP authority on the web path (v0); no connection pooling
  (declared unrepresentable).
- M10 debug triplet: module identity survives (same core bytes), but
  in-browser source-frame mapping is v0-refused; run identity = the
  component digest + page origin.
- Limits: shim-side logical bounds only; wasmtime fuel/epoch machinery
  is native-only (typed RunOutcomes stay a native-runner capability).
- One load = one Store equivalent; page reload is a new run.

## 4. Not yet claimed (honesty)

- UI controls (RFC-0042): contract accepted, renderer not implemented —
  M15 exit gates 2–4 (control/event/a11y/hostile-content corpora) remain
  open and are tracked in the exit audit (STEP-0158).
- Engines other than Edge/Chromium 152 remain explicitly unsupported
  (gate 6 honored).
