# STEP-0210: M22 S6 — fixed-width literal call-argument IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0209
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Accept fixed-width literal arguments (`I64.literal(N)`,
`I64.literal(-N)`, `U64.literal(N)`) in user-call argument positions,
completing the multi-token constant half of the argument surface before
nested call arguments introduce recursion.

## 2. Contract and mechanism

- A literal-argument attempt is detected by the receiver token (`I64`
  or `U64`), `.`, `literal`, `(`. The magnitude must be canonical, the
  closing parenthesis must follow, `I64` magnitudes are bounded by
  9223372036854775807 (positive) / 9223372036854775808 (negative) and
  `U64` by 18446744073709551615; negative `U64` literals are malformed.
  Malformed attempts refuse `ERR:E-SH-IR-CALL-SHAPE`; out-of-range
  magnitudes refuse the existing `ERR:E-SH-IR-I64-RANGE` /
  `ERR:E-SH-IR-U64-RANGE`.
- The argument kind is the literal's fixed-width kind and is checked
  against the callee's parameter kind at that position (bare `Int`
  constants against `I64` parameters keep refusing
  `ERR:E-SH-IR-CALL-TYPE`). Argument SSA ids and instruction numbering
  follow the STEP-0208 rules: source order, results from the caller's
  parameter count, call result = parameter count + constant count.
- Constant instructions emit before the call in source order with
  unquoted numeric data (negative literals include the sign in the
  data). The instruction range covers the magnitude digits only for
  positive literals and the sign plus digits for negative literals,
  matching the Rust token ranges; the argument search cursor advances
  past the whole literal shape so duplicate digits resolve positionally.

## 3. Executable evidence

Three new positive differentials: `add(v, I64.literal(2))` (binding +
literal), `add(I64.literal(1), I64.literal(-5))` (positive and negative
literals, ranges 121..122 and 137..139) and `bits(v, U64.literal(7))`.
All deserialize, pass the independent verifier, round-trip canonically
and are byte-identical to Rust `lower_core`; the cumulative positive
differential set is now twenty-five programs. Two new typed refusals
align with the Rust gate: out-of-range `I64.literal(9223372036854775808)`
argument (`ERR:E-SH-IR-I64-RANGE`, Rust `E2001`) and a bare `Int`
constant against an `I64` parameter (`ERR:E-SH-IR-CALL-TYPE`, Rust
`E2002`). The selfhost parser suite stays 2/2 green on the real Windows
x64 runner.

## 4. Residuals

Nested call arguments (the recursive half of multi-token arguments),
literal arguments to fixed-width operations, checked arithmetic
(Result-shaped, requires match blocks), general statements and blocks
inside function bodies, and full-corpus lowering remain open. This step
proves fixed-width literal argument construction, not general expression
arguments or bootstrap closure.
