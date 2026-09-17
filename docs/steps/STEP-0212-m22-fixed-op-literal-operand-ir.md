# STEP-0212: M22 S6 — fixed-width literal operands for fixed-width operations

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0211
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Accept fixed-width literal operands (`I64.literal(N)`, `I64.literal(-N)`,
`U64.literal(N)`) in fixed-width operation positions
(`U64.shl(a, U64.literal(3))`), closing the literal-operand residual
recorded by STEP-0209/0211.

## 2. Contract and mechanism

- Operation shape validation becomes width-aware: each operand is a
  caller parameter (one token) or a fixed-width literal attempt (six or
  seven tokens, width from the sign position regardless of validation
  outcome), so out-of-range and malformed literals surface their own
  stable errors — `ERR:E-SH-IR-U64-RANGE` / `ERR:E-SH-IR-I64-RANGE` /
  `ERR:E-SH-IR-CALL-SHAPE` — instead of a premature arity refusal.
  Operand kinds are still checked against the receiver kind
  (`ERR:E-SH-IR-CALL-TYPE`).
- Literal operands emit their constant instructions before the
  operation in source order, with unquoted numeric data (sign included
  for negatives) and ranges covering the digits only, or the sign plus
  digits for negatives. Operand values are SSA ids: the caller
  parameter's id, or the parameter count plus the number of preceding
  literal operands. The operation result is the caller's parameter
  count plus the literal-operand count.
- Shared width/atom-index helpers derive each operand's token span from
  the operation receiver anchor, keeping the check, value, count and
  emission passes consistent without cursor state.

## 3. Executable evidence

Three new positive differentials — `U64.shl(a, U64.literal(3))`
(const r1 digits range, `shl {value:0, amount:1}` r2),
`I64.bit_and(a, I64.literal(-1))` (negative data -1, sign range
71..73) and `I64.bit_or(I64.literal(1), a)` (literal operand value 1
ahead of binding 0) — deserialize, pass the independent verifier,
round-trip canonically and are byte-identical to Rust `lower_core`; the
cumulative positive differential set is now thirty-two programs. One
new typed refusal aligns with Rust `E2001`:
`U64.literal(18446744073709551616)` as an operand refuses
`ERR:E-SH-IR-U64-RANGE`. The selfhost parser suite stays 2/2 green on
the real Windows x64 runner.

## 4. Residuals

Checked arithmetic (Result-shaped, requires match blocks), general
statements and blocks inside function bodies, nested calls as operation
operands, and full-corpus lowering remain open. This step proves
fixed-width literal operands, not full expression grammar or bootstrap
closure.
