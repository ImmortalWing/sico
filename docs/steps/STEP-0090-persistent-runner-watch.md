# STEP-0090: persistent development runner and watch mode

> - status: complete
> - phase: M9
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Add a bounded `sico watch` development loop that reuses runner compilation state without reusing guest execution authority or mutable Store state.

## 2. Delivered

- `Runner::prepare_program_with_net` caches compiled Components in an eight-entry digest cache and produces a reusable Component+Linker generation.
- Every `PreparedProgram::run` constructs a fresh Store, resource table, IO workers, cancellation state, budgets and Host state. A trap or typed domain failure does not poison later executions.
- `sico watch FILE -- ARGS...` compiles the initial Script, starts one persistent `sico-runner --watch` process and atomically publishes only successful later compilations through exclusive temporary artifacts.
- Polling is bounded to 5–1000 ms (25 ms default); changes must remain stable for 100 ms, so rapid writes coalesce. Source and watched Component sizes are capped at 8 MiB and 64 MiB.
- Provider grants and arguments are fixed for the lifetime of the process. Temporary Components are removed when the watch child exits.

## 3. Exit evidence

`tools/validate-step-0090.ps1` is green. Its final run kept one runner PID across generations `1,2,3`, observed exits `0,122,0`, did not run an invalid compilation, coalesced two rapid valid writes, recovered after a typed failure and removed the temporary artifact. Repeated release runs measured 5.8–6.1 ms warm medians across 32 prepared executions (latest 5,934 us). The runner integration test also executes a trap between healthy prepared runs.

Report: [`persistent-runner-watch-v0`](../reports/persistent-runner-watch-v0.md).

## 4. Honest limits

Watch v0 uses portable polling rather than native filesystem notifications, watches one source file, and refuses Components that import streaming stdin because an inherited terminal stream cannot be replayed safely. It does not preserve guest globals, resources or REPL state; that isolation is intentional. Interactive session state belongs to STEP-0091.

## 5. Links

- [`M9 plan`](../plans/M9-streaming-async-interactive.md) (§8)
- [`STEP-0089`](./STEP-0089-scoped-http-component-provider.md)
- [`persistent runner report`](../reports/persistent-runner-watch-v0.md)
