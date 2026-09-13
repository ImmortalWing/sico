# STEP-0164: M18 portfolio pilots — API agent, streaming tool, Web/UI application

> - status: complete (internal-fixture class; three of four runnable pilots)
> - phase: M18 plan §3 portfolio (owner-approved 2026-09-04; refined 2026-09-13)
> - completed: 2026-09-13
> - owners: autonomous-agent
> - artifacts:
>   - pilot 1 [`pilots/api-agent/`](../../pilots/api-agent/) + `runner/sico-runner/tests/api_agent_pilot.rs` (2/2)
>   - pilot 2 [`pilots/stream-sum/`](../../pilots/stream-sum/) + `runner/sico-runner/tests/stream_sum_pilot.rs` (2/2)
>   - pilot 3 [`pilots/web-log-viewer/`](../../pilots/web-log-viewer/) + `crates/sico-cli/tests/generate_web_ui_pilot.rs` + `web/log-viewer.html` + evidence `docs/evidence/m18/web-ui-pilot.json`
>   - pilot 5: the STEP-0149 clean-room consumer `pilots/log-analyzer` (9/9 byte-exact) doubles as the independent package/Component consumer

## 1. Pilot 1 — secure API agent (M12)

`api-agent.sico`: three-step agent flow over `sico.http2.request` (GET
config → verify `mode=strict` marker → GET task → verify protocol text →
POST result → verify acceptance), with fail-closed typed errors at every
step (`agent-fail:<reason>`). Evidence (`api_agent_pilot.rs`, 2/2):
TLS fixture server with pinned CA, per-step sequential response plans,
happy path byte-exact (`agent:config-ok|task-ok|result-accepted`), and
an unexpected-config run refusing closed (`agent-fail:unexpected config`,
exit 1). Authority: single granted endpoint, no other capabilities.

## 2. Pilot 2 — streaming data tool (M9/M11)

`stream-sum.sico`: chunked log sampler over the streaming stdin/stdout
channels — bounded 4 KiB reads, per-chunk classification line
(`chunk-<n>:error|info|clean`), non-UTF-8 chunk is a typed fail-closed
error. Constant memory (one chunk; nothing accumulates). Evidence
(`stream_sum_pilot.rs`, 2/2): classification invariants across pipe-
scheduled chunk boundaries (strictly increasing indices, marker-dominate
assertions) and the typed partial-failure path (exit 122, prior chunks'
output already streamed).

## 3. Pilot 3 — Web/UI application (M15 + RFC-0042)

`log-viewer.sico`: a stateless Web/UI component — no args renders the
all-logs view; one argument carrying the JSON event
`{"kind":"click","node_id":"filter-errors"}` renders the errors-only
view. Output = the RFC-0042 v0 UI tree as JSON. The generated
`web/log-viewer.html` embeds the compiled core module + canonical-ABI
shim + RFC-0042 renderer; headless Edge 152 executes the event script
(initial → filter-errors → filter-all) and the DOM records every tree
(`docs/evidence/m18/web-ui-pilot.json`, 3 renders). Recorded v0
artifact: `join_from` emits a trailing separator (deterministic;
documented in the evidence).

## 4. Pilot 5 — package consumer

Covered by STEP-0149 (`pilots/log-analyzer`, 9/9 byte-exact through the
lock + signed packages + user WIT, no core patches).

## 5. Validation

workspace 372/0; runner suite green (solo; the two RSS/warm-median
budget asserts remain the documented environment-sensitive class).
All pilots build with the unmodified compiler/Runtime and run through
the real Component Runtime or real browser engine.
