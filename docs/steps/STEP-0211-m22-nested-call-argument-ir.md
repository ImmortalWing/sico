# STEP-0211: M22 S6 — nested call-argument IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0210
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Lower nested call arguments (`f(g(v))`, `f(g(1))`, `f(v, g(v))`,
`f(g(v), g(v))`) with a recursive-descent argument walker, completing
the recursive half of the multi-token argument surface. The flat
argument functions of STEP-0208/0210 are replaced by a unified recursive
pipeline.

## 2. Contract and mechanism

- Arguments are segmented at depth-zero commas (parenthesis-aware via
  the existing matcher); each argument is a caller parameter, a
  single-token constant, a fixed-width literal, or a nested call
  resolved by name across the module (`ERR:E-SH-IR-CALL-TARGET` for
  unknown callees). Kinds are checked against the callee's parameter
  kind per position; nested calls carry their callee's return type.
- Inner instructions emit before outer instructions in source order.
  Each expression's result id equals its base id plus its subtree's
  instruction count minus one; sibling bases advance by their subtree
  counts, starting from the caller's parameter count. The outer call's
  result is the caller's parameter count plus the total argument
  instruction count (a region-level sum, deliberately without the
  per-expression self increment).
- Nested call instructions carry the callee's declaration-order id, the
  nested `arguments` array from the same validation pass, and a range
  covering the whole nested source expression. Token offsets derive
  from a word-index walk anchored at the outer call's range start, so
  no cursor state flows through recursion.

## 3. Executable evidence

Four new positive differentials — `f(g(v))`, `f(g(1))`, `f(v, g(v))`,
`f(g(v), g(v))` — deserialize, pass the independent verifier, round-trip
canonically and are byte-identical to Rust `lower_core`; the cumulative
positive differential set is now twenty-nine programs. Two new typed
refusals align with the Rust gate: a nested `Int` result against `I64`
parameters (`ERR:E-SH-IR-CALL-TYPE`, Rust `E2001`/`E2002`) and an
unknown nested callee (`ERR:E-SH-IR-CALL-TARGET`, Rust `E2031`). The
selfhost parser suite stays 2/2 green on the real Windows x64 runner.

## 4. Residuals

Fixed-width literal arguments to fixed-width operations, checked
arithmetic (Result-shaped, requires match blocks), general statements
and blocks inside function bodies, and full-corpus lowering remain open.
This step proves recursive nested call arguments, not full expression
grammar or bootstrap closure.
