# STEP-0087: Task/Future/Stream source backend (sequential executor)

> - status: complete
> - phase: M9
> - started: 2026-07-18
> - completed: 2026-07-18
> - owners: autonomous-agent

## 1. Objective

Structured task scopes, single terminal winner, cancellation edges and bounded stream flow from the M9 plan — delivered as the parallelism-1 sequential executor of the RFC-0004 task model.

## 2. Delivered

- `async function` lowers as an ordinary function; `spawn f(args)` is an eager direct call; `await name` is the identity; `task group:` is a straight-line scope (`crates/sico-ir/src/lower.rs`).
- Structured discipline statically enforced and measured: `E5101` (await-once), `E5102` (no task escaping its scope). `E5102` is checked from the inferred `Task` type, including indirect escape through a local binding; it is not limited to syntactic `return spawn`.
- Cancellation edges: a task failing with `ScriptErrorCode.Cancelled` terminates the scope via the typed Result channel and maps to the runner's control outcome `Cancelled` (exit 123); host-side CancelToken → stream-error `cancelled` (STEP-0088) maps into the same code.
- Bounded stream flow re-verified from STEP-0086 (1/16 MiB guest pump, 256 MiB host pump at 10.8 MiB RSS).
- `collect_tasks` and first-class `Future`/`Stream` types stay typed refusals (measured), not silent gaps.

## 3. Exit evidence

`tools/validate-step-0087.ps1` green: sequential pair, cancellation exit 123, double-await `E5101`, typed indirect escape `E5102`, and `collect_tasks` refusal. Report: [`script-task-sequential-v0`](../reports/script-task-sequential-v0.md).

## 4. Honest limits

Parallelism 1 (no scheduler, no races); in-flight parallel cancellation, timeouts and `collect_tasks` remain for the async-runner work; guest per-chunk flow stays recursive/arena-capped.

## 5. Links

- [`M9 plan`](../plans/M9-streaming-async-interactive.md) (§6)
- [`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md)
- [`STEP-0086 evidence`](../reports/script-streaming-v0.md)
