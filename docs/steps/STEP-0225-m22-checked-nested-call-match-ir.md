# STEP-0225: M22 nested user calls in checked-arithmetic match IR

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0224
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0225.ps1`

## 1. Objective

Allow a typed user-function call in either operand position of the
bounded checked-arithmetic all-return match shape.

## 2. Contract and mechanism

- The outer checked call is split at its top-level comma while nested
  parentheses are skipped.
- Each operand reuses the recursive argument validator and instruction
  emitter from the user-call path. Nested arguments are arity/type
  checked against the declared callee signature.
- Nested instructions emit in source order before the checked
  instruction. Operand and Result ids are derived from the exact
  recursive instruction counts, preserving canonical SSA ordering.
- A nested call returning the wrong scalar type is rejected with
  `ERR:E-SH-IR-CALL-TYPE` and emits no IR bytes.

## 3. Executable evidence

Two real-runner differentials cover a left nested `I64` call and a
right nested `U64` call on different checked operations. Both emitted
modules deserialize, pass the independent verifier, canonically
round-trip and match Rust `lower_core` byte-for-byte. The cumulative
positive set is 44 programs. A wrong-type nested-call fixture is rejected
by both Rust lowering and the Sico parser.

## 4. Residuals

Multiple nesting levels inside checked operands were subsequently proven
by STEP-0226. Alternate arm shapes,
non-return arm bodies, general statements and full-corpus lowering remain
open. S5 codegen and S6 bootstrap are unchanged.
