# STEP-0240: M22 structured `while` lowering with break/continue in the Sico frontend

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0239
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0240.ps1`

## 1. Objective

Lower structured `while` loops — header branch, body back-edge, loop exit,
and `break`/`continue` via a single `if` inside the body — byte-identically
to the Rust `lower_core` general-CFG path.

## 2. Contract and mechanism

`control_while_function_ir` handles leading lets, one `while` whose
condition is the frozen `equal_fixed`/`less_fixed` predicate, a body of
cell statements (atom, fixed-literal, fixed-width operation and typed call
right-hand sides), an optional single `if` whose then-arm is exactly
`break` or `continue` (no else), and post-loop continuation statements
ending in return. Without the body `if`, four blocks emit: entry (jump to
header), header (branch to body/exit), body (back-edge jump), exit
(continuation return). With the body `if`, seven blocks: the break block
jumps to the exit, the continue block jumps to the header, the if's empty
else block jumps to the remainder of the body, and block ranges follow the
probe-established rule (header/exit/body-carrying the `while` line, if
blocks the `if` line). Value ids stay globally sequential in block order.

To fit the frozen `MAX_LOCALS_PER_FUNCTION = 256` under the general-CFG
cell lowering (source lets plus match-payload cells), the call right-hand
side moved into `while_call_rhs_packed` (packed `kind\nargs\nemit` return
unpacked via `split_lines`, instruction counts derived by `comma_count`),
operand resolution into `operand_nonparam_kind` /
`operand_cell_or_literal_json`, and no-op branches share one `swl_dummy`
cell via `set`. Nested `while`, multiple body `if`s, direct body
`break`/`continue`, and `while` RHS nested calls remain typed refusals or
outside the executed subset.

## 3. Executable evidence

Three positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte: a plain accumulation loop with
operation right-hand sides, a loop with `break` under a cell/literal
condition, and a loop with `continue` mixing operation and cell state. The
cumulative positive set is 110 programs.

## 4. Residuals

Nested loops, multiple body conditionals, direct body break/continue and
call nested arguments inside while right-hand sides remain outside the
executed subset. Full-corpus lowering, S5 codegen and S6 bootstrap remain
open.
