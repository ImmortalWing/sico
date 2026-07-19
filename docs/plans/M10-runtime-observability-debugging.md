# M10 plan: Runtime observability and debugging

> - status: planned; STEP-0095 next
> - created: 2026-07-19
> - phase: M10
> - entry requirement: M9 GO with frozen execution-plan v0 and explicit source-map/debug limits

## 1. Outcome

Turn M9's honest tooling boundary into a diagnosable execution path. A user or bounded client must be able to launch a Script without a shell, interrupt it through a typed cancellation path, receive structured lifecycle/log/fault events and map Runtime failures back to stable Sico source coordinates. A minimal DAP surface may be claimed only after actual breakpoint, stack and variable evidence exists.

## 2. Scope and non-goals

M10 owns source/debug identities, Runtime frames, signal cancellation, bounded execution events, DAP and LSP/AI consumption of those contracts.

M10 does not add HTTP TLS/proxies, parallel task scheduling, unrestricted process authority, multi-file watch graphs, declaration-capable REPL cells, public deployment or mobile Runtime support. Those require separate roadmap decisions and must not widen M10's authority model.

## 3. Proposed execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0095 | observability/debug/source-identity RFC | versioned schemas; stable artifact/source identity; redaction, bounds and cancellation matrix |
| STEP-0096 | compiler debug-map artifact | deterministic function/instruction-to-UTF-8-span map; malformed/tampered maps fail closed |
| STEP-0097 | structured Runtime faults and source frames | traps, limits and host failures carry typed frames without parsing stderr |
| STEP-0098 | OS signal and client cancellation bridge | Ctrl+C/process request reaches CancelToken, one terminal winner, typed exit 123 where guaranteed |
| STEP-0099 | bounded execution event/log protocol | lifecycle/stdout/stderr/fault events, sequence IDs, truncation/backpressure and redaction evidence |
| STEP-0100 | minimal DAP implementation | launch, breakpoint, pause/continue, stack and bounded variables proven against real Components |
| STEP-0101 | editor and AI execution feedback integration | LSP/DAP/client adapter plus data-only AI summaries consume the same versioned events |
| STEP-0102 | M10 security/performance/platform exit audit | cancellation latency, map overhead, event bounds, debugger isolation and actual platform matrix |

## 4. Contract gates

- Debug maps bind exact source bytes, compiler identity and Component digest; stale or mismatched maps are rejected.
- Runtime outcomes remain typed. Human stderr is never the protocol of record.
- Log/event queues have configured byte and item bounds, stable ordering and explicit truncation markers.
- Signal, timeout, client cancellation, guest completion and Host failure race to exactly one terminal outcome.
- Debugger attachment grants observation/control only within the launched Script process and never adds filesystem/network/process capability.
- AI tooling consumes bounded structured data and does not gain execution authority.

## 5. Performance and exit gates

M10 will measure debug-map artifact size/build overhead, source-frame lookup latency, event throughput/peak RSS, blocked-operation cancellation latency, breakpoint pause/continue latency and repeated debug-session isolation. Numeric budgets are fixed in STEP-0095 from the M9 baselines before implementation claims begin.

M10 is GO only when real Runtime faults map to stable Sico frames, Ctrl+C/client cancellation is typed and race-safe, event/log transport remains bounded, the claimed DAP subset is exercised end to end, M0–M9 regression stays green and platform support is limited to actual runner evidence.
