# STEP-0237: M22 operation and call right-hand sides in cell mode

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0236
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0237.ps1`

## 1. Objective

Extend cell-mode (`set`-containing) right-hand sides from atoms and fixed
literals to fixed-width operations and typed user calls, byte-identical to
the Rust `lower_core` general-CFG path where every cell read emits
`read_local` in source order.

## 2. Contract and mechanism

In `straight_cell_function_ir` a multi-token right-hand side now
classifies three shapes. `I64/U64.literal(N)` keeps the STEP-0236 const
path. A recognised fixed-width operation resolves both operands as
parameters (no instruction), prior cells (`read_local` at the running
result id), or same-width fixed literals (`const_i64`/`const_u64` via the
new `fixed_literal_operand_json` helper), left before right, then emits the
operation with the expression span; operand kinds are checked against the
receiver kind. A call shape resolves comma-separated arguments the same
way (parameters, cells, scalar constants, fixed literals) with position
type checking and arity/shape refusals; emitting arguments advance the
result base and the call takes the final base. Every `let`/`set` still
closes with `write_local`, so the instruction count includes the operation
or call instruction itself.

Nested calls inside cell-mode call arguments, and operation/call returns
in cell mode, remain outside the executed subset.

## 3. Executable evidence

Eight positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte: set with cell-and-parameter
`bit_and`, chained `shl`/`bit_or` sets with literals, an operation
right-hand side on `let`, a set with a call taking a cell and a fixed
literal, a set with a call taking a cell and a scalar integer, a set with
a call taking bool/text constants, and a zero-argument call. Two negative
fixtures prove a wrong-kind operation operand and an unknown call target
in set position fail closed with typed refusals. The cumulative positive
set is 96 programs.

## 4. Residuals

Nested calls inside cell-mode call arguments, and operation/call return
expressions in cell mode, remain outside the executed subset. Nested
control flow, alternate match-arm shapes, full-corpus lowering, S5
codegen and S6 bootstrap remain open.
