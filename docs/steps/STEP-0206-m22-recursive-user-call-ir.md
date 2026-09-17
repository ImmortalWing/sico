# STEP-0206: M22 S6 — typed recursive user-call IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0205
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Add the first verifier-accepted user-function `call` operation to the
Sico-written lowering path. The bounded slice targets calls to the current
function so call construction, positional type checking and argument SSA
identity are proven before the frontend is widened to multiple function
definitions.

## 2. Contract and mechanism

- A direct recursive call resolves to canonical `FunctionId(1)` for the
  one-function module accepted by this slice.
- Each argument must be one of the current function's parameters. Its SSA
  ID is preserved directly; arguments may be reordered.
- Arity is exact and every argument kind is checked against the parameter
  kind at that call position. Stable failures distinguish target, shape,
  argument, arity and type errors; no partial IR is emitted.
- The call result begins at `parameters.len()` and has the function return
  type. The block terminator returns that result.
- The operation range covers the complete call expression, including
  punctuation and inter-argument whitespace, matching Rust `token_range`.

## 3. Executable evidence

The new positive differential is:

```sico
function swap(left: Int, right: Int) returns Int:
  return swap(right, left)
end function
```

The guest output deserializes, passes the independent verifier, round-trips
canonically and is byte-identical to Rust `lower_core`. It proves
`Operation::Call { function: 1, arguments: [1, 0] }`, result `ValueId(2)`
and the complete source range. The cumulative positive differential set is
now eleven programs.

A two-argument call to a one-parameter recursive function is rejected by
both Rust lowering and the guest; the guest returns typed `invalid-input`
with `ERR:E-SH-IR-CALL-ARITY`. The complete selfhost parser suite remains
2/2 green on the real Windows x64 runner.

## 4. Residuals

Cross-function calls and multiple definitions remain open. Constant or
nested call arguments, fixed-width operations, general blocks and
full-corpus lowering also remain open. This step proves the shared call
operation foundation, not general call support or bootstrap closure.
