# STEP-0208: M22 S6 — constant call-argument IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0207
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Allow call arguments to be single-token constants in addition to caller
parameters, so the lowering proves source-ordered const instruction
emission, SSA result numbering across parameter/constant mixes, and
per-atom source ranges before nested call arguments widen the surface
further.

## 2. Contract and mechanism

- Each argument is one atom. Caller parameters keep their SSA identity;
  bare canonical integers, `true`/`false` and string tokens classify as
  `const_int`/`const_bool`/`const_string`. Anything else (including
  multi-token shapes such as fixed-width literals or nested calls) is
  still a typed refusal — `ERR:E-SH-IR-CALL-ARGUMENT` for unresolvable
  atoms, unchanged shape errors for malformed argument lists.
- Constant instructions are emitted before the call in source argument
  order. Result ids start at the caller's parameter count and increment
  per constant; the call result id is the caller's parameter count plus
  the constant count, and the terminator returns it.
- Argument kinds (parameter kind or constant kind) are checked against
  the callee's parameter kind at each position; mismatches stay
  `ERR:E-SH-IR-CALL-TYPE`. `Int` constants against `I64` parameters are
  refused, matching the Rust type gate.
- Constant instruction bytes follow the established formats: `const_int`
  data is a quoted decimal string, `const_bool` data is bare
  `true`/`false`, `const_string` data is the raw escaped lexeme including
  quotes. Each atom's range is its raw source byte span; atom search
  starts after the call's open parenthesis and advances past every
  argument so duplicate atom texts resolve positionally.

## 3. Executable evidence

Four new positive differentials: `add(1, 22)` (two const ints, results
0/1, call result 2), `add(v, 7)` (mixed binding + const, const result 1),
`flag(true)` (bare bool data) and `label("hi\n")` (escaped string data,
raw 6-byte range). All four deserialize, pass the independent verifier,
round-trip canonically and are byte-identical to Rust `lower_core`. The
cumulative positive differential set is now seventeen programs.

Three new typed refusals aligned with the Rust semantic gate: `Int`
constant against an `I64` parameter (`ERR:E-SH-IR-CALL-TYPE`), unknown
identifier atom (`ERR:E-SH-IR-CALL-ARGUMENT`) and non-canonical literal
atom `01` (`ERR:E-SH-IR-CALL-ARGUMENT`). The complete selfhost parser
suite stays 2/2 green on the real Windows x64 runner.

## 4. Residuals

Fixed-width literal arguments, nested call arguments, fixed-width
operations, general statements and blocks inside function bodies, and
full-corpus lowering remain open. This step proves constant argument
construction, not general expression arguments or bootstrap closure.
