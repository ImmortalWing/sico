# Task/Future/Stream source backend (sequential executor) evidence

> - date: 2026-07-18
> - step: STEP-0087
> - status: complete

## Implemented boundary

M9's structured task model runs as a **sequential executor**: `async function` lowers as an ordinary function, `spawn f(args)` is an eager direct call, `await name` is the identity over the eagerly produced value, and a `task group:` block is a straight-line scope. This is the parallelism-1 execution of the RFC-0004 contract — the structure (lexical scope, join semantics, await-once consumption, no task escaping its scope) is real and statically enforced; there is no scheduler to get wrong, and determinism is absolute.

Enforcement points (all typed, never silent):

- `E5101` await-once: awaiting the same task twice is a semantic refusal (measured);
- `E5102` task-escapes-scope: a spawned task cannot leave its `task group:`, including through `let escaped = spawn ...; return escaped` (measured from the inferred type);
- `collect_tasks` is outside the sequential v0 and fails with a typed `unsupported call target` at lowering (measured);
- cancellation edges: a task that fails with `ScriptErrorCode.Cancelled` terminates the scope through the ordinary Result channel — the runner recognizes the closed error code and reports control outcome `Cancelled` (exit 123, measured); the host-side CancelToken path maps to the same outcome.

Bounded stream flow (the STEP-0086 evidence, re-verified): 1/16 MiB guest-side chunked pump and 256 MiB host-side `sico.stream.pump` at 10.8 MiB peak RSS.

## Evidence

`tools/validate-step-0087.ps1`:

- structured pair: `spawn shout("alpha")` + `spawn shout("beta")` + two awaits → stdout `alpha! beta!`, exit 0 (sequential, deterministic);
- cancellation edge: cancelled task → control outcome `cancelled`, exit 123;
- double await → `E5101`, exit 120; indirect spawn escape through a local binding → `E5102`, exit 120;
- `collect_tasks` → typed lowering refusal;
- full workspace regression green (fmt/clippy 0 findings/67 suites); the cumulative syntax-candidate count moved from 17/8 to 18/7 (`future-task/valid/await-once.sico` now lowers — the only flipped case, verified individually).

## Honest limits

- Parallelism is 1: no concurrent execution, so no data-race surface exists either; "single terminal winner" is deterministic order, not a runtime race. `collect_tasks`, `Future`/`Stream` as first-class types, timeouts and cancellation of in-flight parallel tasks remain for the async-runner work (STEP-0088 and its RFC updates).
- Bounded stream flow for guest-side transforms stays arena-capped (~64 MiB); bulk forwarding beyond that uses the host pump (STEP-0086).
- The guest per-chunk pump remains recursive (no source-level loop construct yet).
