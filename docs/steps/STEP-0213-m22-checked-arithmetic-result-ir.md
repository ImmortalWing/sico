# STEP-0213: M22 S6 — checked arithmetic Result IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0212
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Lower checked arithmetic returned directly
(`return I64.checked_add(...)`) together with `Result[I64/U64,
NumericError]` return types. The `match` decomposition form was probed
and is explicitly out of scope: a straightforward match lowering
produced IR that the independent Rust verifier itself rejects
(`InvalidIr TypeMismatch` on `function[0].block[1]`), so match-shaped
checked arithmetic was left for investigation. STEP-0214 supersedes that
probe finding: the retained minimal source passes Rust lowering and the
independent verifier when it uses the established spill/read/project
shape; the earlier transient probe was not valid Rust-oracle evidence.

## 2. Contract and mechanism

- `Result[OK, NumericError]` signatures parse into the canonical type
  text `{"kind":"result","data":{"ok":{"kind":"OK"},"error":{"kind":
  "named","data":"NumericError"}}}`; return-type emission branches on
  the `{` prefix so scalar types keep the wrapped `{"kind":...}` form.
- `checked_add`/`checked_sub`/`checked_mul`/`checked_div` dispatch like
  the fixed-width operations (receiver `I64`/`U64`, `.method(`) and
  share the width-aware operand machinery: parameters, fixed-width
  literals (consts emit first in source order), kind checks against the
  receiver, shape/arity/argument/type refusals unchanged.
- The checked instruction is
  `{"op":"checked_add|checked_sub|checked_mul|checked_div",
  "data":{"left":L,"right":R}}` with the full result JSON as its
  instruction type and a range spanning the whole source expression;
  the expression's type text equals the return-type text, so the
  module-wide type check holds by construction.
- Calls may target `Result`-returning functions: the call instruction's
  `ty` branches on the `{` prefix the same way, and a `Result`-typed
  nested call against a scalar parameter still refuses
  `ERR:E-SH-IR-CALL-TYPE`.

## 3. Executable evidence

Five new positive differentials — `I64.checked_add` with a literal
operand and with two bindings, `I64.checked_mul`, `U64.checked_div`
(`Result[U64, NumericError]`), and a `Result`-returning callee passed
through a second function — deserialize, pass the independent verifier,
round-trip canonically and are byte-identical to Rust `lower_core`; the
cumulative positive differential set is now thirty-seven programs. Two
new typed refusals align with Rust: a `U64.literal` operand on an
`I64` checked op (`ERR:E-SH-IR-CALL-TYPE`, Rust `E2001`) and a checked
result returned against a scalar `I64` return type
(`ERR:E-SH-IR-TYPE-MISMATCH`). The selfhost parser suite stays 2/2
green on the real Windows x64 runner.

## 4. Residuals

Match-shaped checked arithmetic (the `case ok/error` decomposition),
general statements and blocks inside function bodies, nested calls as
operation operands, and full-corpus lowering remain open. STEP-0214 has
removed the alleged Rust-oracle blocker but has not implemented the shape
in the Sico-written frontend. This step proves directly returned checked
arithmetic, not match lowering or bootstrap closure.
