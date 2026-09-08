# STEP-0137: general recursion with a typed stack-budget exhaustion

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); profile item 2 (general
>   recursion with a runtime stack/fuel bound and a typed exhaustion
>   outcome)

## 1. What was implemented

Profile item 2 closes with no new language surface: recursion already
passed `sico check`, and after STEP-0130 general control flow + STEP-0137's
lowering fix below, recursive functions compile and run to exact results.
The remaining work was the runtime bound and its typed outcome.

### 1.1 Lowering fix: payload bindings on computed match subjects

The frozen all-return match path refused `case ok(x)` when the match
subject was a computed value (`match I64.checked_sub(n, …)` inside a
function), blocking the recursive-call idiom. Both the frozen and general
match paths now spill a computed subject into a compiler-generated
`#match<name>` cell (the mechanism STEP-0130 introduced for `set`
targets). Programs accepted before either took the parameter path or
already spilled, so frozen shapes stay byte-identical (verified by the
frozen lowering snapshots).

### 1.2 Runner: typed `RunOutcome::StackLimit`

- `classify_error` maps the wasmtime `Trap::StackOverflow` to the new
  `RunOutcome::StackLimit` (after timeout/cancel/provider/memory/fuel
  checks keep their precedence). Wire form: class
  `resource-limit.stack`, exit 125, fault id `runtime.stack-limit`.
- The guest instantiate+invoke now runs on a dedicated 64 MiB-stack
  worker thread (plain and debug paths). The deterministic
  `max_wasm_stack` accounting (4 MiB, pre-existing) trips first, so deep
  recursion yields the typed outcome instead of a native process crash —
  the probe crashed the old runner process with "has overflowed its
  stack" before this change.
- Exhaustion leaves the Host reusable: the same `Runner` prepares and
  completes a normal program after a `StackLimit` run (exit gate 4).

## 2. End-to-end evidence

- `tests/end-to-end/recursion-depth.sico`: 10,000-deep `countdown` plus
  `fib(12)=144` (self-referential, mutually nested recursive calls),
  guest-asserted, prints `recursion-ok`.
- `tests/end-to-end/recursion-unbounded.sico`: infinite `dive` —
  `RunOutcome::StackLimit` with `--json`
  `{"class":"resource-limit.stack","exit":125}` and the process exits
  cleanly.
- Runner coverage: `runner/sico-runner/tests/recursion.rs` (exact bounded
  results; typed exhaustion + reusability after exhaustion).

## 3. Residuals

- The stack budget is a fixed runner configuration (4 MiB wasm stack),
  not per-program tunable; a per-profile budget knob would need an RFC.
- Fuel exhaustion (`RunOutcome::FuelExhausted`) remains the
  non-termination bound for runaway loops; both typed outcomes coexist by
  budget class.

## 4. Validation

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test recursion -- --test-threads=1`: 2/2 green.
- Full root + runner workspace suites green; `cargo fmt` / `clippy -D warnings` clean; `tools/validate-step-0131.ps1` green (matrix `recursion` row moved to executable); `git diff --check` clean.
