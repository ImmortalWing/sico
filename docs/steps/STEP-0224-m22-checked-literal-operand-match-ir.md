# STEP-0224: M22 checked-arithmetic literal operands in match IR

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0223
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0224.ps1`

## 1. Objective

Allow width-matched fixed literals in either operand position of the
STEP-0223 checked-arithmetic all-return match shape.

## 2. Contract and mechanism

The lowering reuses the width-aware operand parser and literal emitter
already proven for fixed-width operations. Literal constants are emitted
in source order before the checked instruction; operand ids point to the
parameter or emitted constant; the Result, spill, project and fallback
ids advance canonically after those constants. The checked operation
range covers the outer call, not an inner literal close parenthesis.
Malformed, out-of-range and wrong-width literals retain typed refusals.

## 3. Executable evidence

Two real-runner differentials cover a signed negative right operand and
an unsigned left operand on different checked operations. Both emitted
modules deserialize, pass the independent verifier, canonically
round-trip and match Rust `lower_core` byte-for-byte. The cumulative
positive set is 42 programs.

## 4. Residuals

Nested user calls as checked operands were subsequently closed for one
level by STEP-0225. Deeper nesting, alternate arm shapes, non-return
arm bodies, general statements and full-corpus lowering remain open.
S5 codegen and S6 bootstrap are unchanged.
