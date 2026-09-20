# STEP-0229: M22 loop-driven straight binding environment

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0228
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0229.ps1`

## 1. Objective

Replace the fixed one/two-binding ceiling with a loop-driven typed SSA
environment for straight-line blocks.

## 2. Contract and mechanism

For blocks with three or more `let` statements, the Sico frontend now
walks each statement in source order and records parallel name, type and
SSA-value tables. Every fixed-width bitwise or shift expression may
consume same-width parameters or any earlier binding. Results receive
consecutive SSA ids and source ranges are re-anchored per physical
statement. The final return may select a parameter or any recorded local
of the declared result type.

Each operation remains exactly binary. Duplicate bindings, forward or
unknown references, width mismatches, trailing operands and intervening
unsupported statements fail closed.

## 3. Executable evidence

One fixture lowers a four-binding I64 dependency graph and returns its
last result. Another lowers five U64 bindings while returning a parameter,
proving that emitted instructions and the terminator are independently
resolved. Both run through the Sico-built parser Component, deserialize,
pass the independent verifier, canonically round-trip and match Rust
`lower_core` byte-for-byte. A negative fixture proves that a forward
reference is rejected. The cumulative positive set is 52 programs.

## 4. Residuals

The loop-driven path currently accepts simple parameter/prior-binding
operands. Fixed-width literals remain covered by the one-binding path and
the first expression of the two-binding path, but are not yet generalized
across longer blocks. User calls, scalar constants, mutation, nested
control flow, alternate match-arm shapes, full-corpus lowering, S5
codegen and S6 bootstrap remain open.
