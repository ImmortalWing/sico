# STEP-0223: M22 complete checked-arithmetic all-return match slice

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0222
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0223.ps1`

## 1. Objective

Extend STEP-0222's exact Result-shaped match lowering from
`checked_add` to `checked_sub`, `checked_mul` and `checked_div`
for both `I64` and `U64`.

## 2. Contract and mechanism

The accepted syntax and fail-closed boundary are unchanged from
STEP-0222. The operation allowlist now contains exactly the four checked
arithmetic operations, and the canonical entry instruction serializes
the selected operation without changing SSA allocation, spill/project
behavior, patterns, ranges or block ordering. Unknown operations retain
the typed `ERR:E-SH-IR-CALL-TARGET` refusal.

## 3. Executable evidence

Six new real-runner differentials cover signed and unsigned
`checked_sub/mul/div`. Each emitted module deserializes, passes the
independent verifier, canonically round-trips and matches Rust
`lower_core` byte-for-byte. Together with STEP-0222, the cumulative
positive set is 40 programs and all four checked operations use the same
verified three-block shape.

## 4. Residuals

Literal checked operands were subsequently closed for this bounded shape
by STEP-0224. Nested checked operands, alternate arm order/payload shapes,
non-return arm bodies, general statements and full-corpus lowering remain
open. S5 codegen and S6 bootstrap are unchanged.
