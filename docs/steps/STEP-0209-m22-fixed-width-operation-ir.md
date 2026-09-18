# STEP-0209: M22 S6 — fixed-width operation IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0208
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Lower the source-level fixed-width bit and shift operations
(`I64`/`U64` `bit_and`/`bit_or`/`bit_xor`/`shl`/`shr`) to the direct IR
operations the Rust backend emits, proving the receiver-kind pipeline and
operand checks before fixed-width literal arguments (multi-token shapes)
widen the argument surface.

## 2. Contract and mechanism

- A return expression shaped `I64.<op>(x, y)` / `U64.<op>(x, y)` with
  `<op>` in the five-operation set dispatches to a fixed-operation path;
  `I64.literal(...)` and unknown methods still fall through to the
  existing fixed-width literal chain.
- Operands must be exactly two caller parameters whose kinds equal the
  receiver kind (`i64`/`u64`). Stable failures keep the call family
  codes: shape (`CALL-SHAPE`), two-operand arity (`CALL-ARITY`),
  unresolvable atoms (`CALL-ARGUMENT`) and kind mismatch (`CALL-TYPE`).
- The emitted instruction is a direct operation, not a call: bit
  operations use `data {"left":L,"right":R}`, shifts use
  `data {"value":V,"amount":A}`, the instruction type and the
  expression type are the receiver kind, the result id is the caller's
  parameter count (no constants can pass the kind gate), and the range
  covers the complete source expression.
- Rust refuses the same violations (`E2002` operand type, `E2001`
  operand count, `E2031` unresolved names); source-level `I64.add`
  remains nonexistent (`E2031`), consistent with the M22 plan §8
  friction record.

## 3. Executable evidence

Five new positive differentials — `I64.bit_and`, `I64.bit_or`,
`I64.bit_xor`, `U64.shl`, `U64.shr`, each over two parameters —
deserialize, pass the independent verifier, round-trip canonically and
are byte-identical to Rust `lower_core`. Three new typed refusals
(Int operand against `I64`, three operands, unresolved operand name)
match Rust refusals. The cumulative positive differential set is now
twenty-two programs and the selfhost parser suite stays 2/2 green on the
real Windows x64 runner.

## 4. Residuals

Fixed-width literal and nested call arguments, checked arithmetic
(Result-shaped, requires match blocks), general statements and blocks
inside function bodies, and full-corpus lowering remain open. This step
proves direct fixed-width operations over parameter operands, not
general expressions or bootstrap closure.
