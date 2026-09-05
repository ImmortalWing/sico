# STEP-0130: general control flow (while / if-else / break / continue / set)

> - status: complete
> - phase: M14 (first gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-05
> - completed: 2026-09-05
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); profile items 1, 3, 4 (partial: List cells), 7 (source-level PRNG prerequisite: cells)

## 1. What was implemented

The RFC-0038 application profile's control-flow slice, end to end:

- **lexer**: `while`, `break`, `continue`, `set` keywords (appended to the
  token enum so existing snapshot discriminants stay stable).
- **parser**: `BlockKind::While` opener + `end while` close; `else`/`set`/
  `break`/`continue` flow through as lines.
- **HIR**: `LineKind::While/Else/Break/Continue/Set`; `While` added to
  `opens_block`.
- **IR**: `Function.locals` (mutable local cells, serde-default so every
  existing fixture is unchanged) and `Operation::ReadLocal/WriteLocal`;
  verifier rules (locals ≤ 256, type-checked read/write).
- **lowering** (`sico-ir/src/lower.rs`): a general structured CFG lowering
  (`lower_general`) replacing the special-case dispatch for any body with
  control flow — arbitrary nesting of `while`, `if`/`else`, fall-through
  match arms (with payload bindings through compiler-generated spill
  cells for non-parameter subjects), `set` reassignment, `break`/`continue`.
  The frozen all-return-match, revision-if and straight-line paths are kept
  byte-identical for existing shapes (dispatch predicates
  `is_strict_match_shape`/`is_revision_if_shape`). Blocks materialize in
  first-emission order with terminator-reference remapping so instruction
  result ids stay canonical per block.
- **codegen**: cell slots per IR local (aggregates supported via slot-wise
  copy), `ReadLocal/WriteLocal` emission, Result ok/error field maps on
  cell reads (tolerating unflattenable error payloads such as
  `NumericError`), variant flags for match subjects.
- **semantics**: while/if conditions must be `Bool`; `set` type-checked
  against the declared cell; region scoping mirrors the lowering (names
  bound inside a region are dropped when it closes).

## 2. End-to-end evidence

`tests/end-to-end/control-flow-counter.sico` — a pure-Sico loop that
counts 1,2,4 (continue skips 3, break stops at 5) using checked
arithmetic unwrapped through fall-through match arms — compiles with
`sico build --profile script-v0` and runs through the real runner:
`tests/control_flow.rs` (3 tests: deterministic result, repeated-run
isolation, typed `set without prior let` refusal). Manual probes during
development also verified a 1..=10 sum (exit 55, stdout `sum-ok`) and
loop+if nesting.

## 3. Residuals (declared, per the RFC matrix)

- `break`/`continue` outside a loop: **check-accepted, build-refused**
  (typed refusal in lowering; a catalog diagnostic requires a semantic-case
  corpus change, deferred to the corpus STEP).
- Aggregate (List) cells are Script-profile-only (component-profile
  aggregate cells refuse as "non-scalar local").
- No implicit fall-off-the-end return: a body without a return on every
  path lowers to `Unreachable` (pre-existing behavior, unchanged).
- **runner bug fixed en route**: the watch cancel bridge consumed a
  request file that existed before the first generation published,
  permanently skipping it (`main.rs` now reads only while a target is
  published); the flaky `runner_watch_accepts_generation_bound...` test
  was reproduced, root-caused and fixed.

## 4. Validation

`cargo test` workspace: all green (including the 12 frozen core-lowering
snapshot cases byte-identical, STEP-0089 oracles, module boundaries).
Runner workspace serial: 37 lib + 1 dap + 3 control-flow + 6 http2 +
37 integration, all green. `cargo clippy -D warnings` clean in both
workspaces; `cargo fmt` clean; `git diff --check` clean.
