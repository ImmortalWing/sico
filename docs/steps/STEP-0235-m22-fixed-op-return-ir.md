# STEP-0235: M22 fixed-width operation returns in the straight environment

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0234
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0235.ps1`

## 1. Objective

Unify fixed-width operation returns (`return I64.bit_and(a, b)`) with the
loop-driven straight binding environment, the last return shape Rust
`lower_core` accepts in straight-line functions.

## 2. Contract and mechanism

A multi-token return whose second word is `.` now distinguishes two shapes:
`literal` routes to the STEP-0234 fixed-literal helper; any other name
resolves through `fixed_op_name` and, when recognised (`bit_and`/`bit_or`/
`bit_xor`/`shl`/`shr`), validates exactly like a binding-position operation:
the environment operand check (parameters, prior locals and same-width
fixed literals in either position), source-ordered const emission before
the operation, operand value resolution against the emission base, and the
operation result taking the next SSA id after its constants. The operation
range is the whole return expression span; the receiver kind (`i64`/`u64`)
is checked against the declared return kind. `shl`/`shr` keep their frozen
`value`/`amount` operand keys.

Wrong-width operands, unknown atoms, extra operands, missing parentheses
and return-kind mismatches remain typed refusals.

## 3. Executable evidence

Five positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte: an alias-fed `bit_and` return, U64
`shl` bindings feeding a `shr` literal return, a left-literal `bit_xor`
return consuming a local, and a two-binding U64 chain ending in a literal
`bit_and` return. Two new negative fixtures prove wrong-width operands and
extra operands in return position fail closed with typed refusals. The
cumulative positive set is 81 programs.

## 4. Residuals

All Rust `lower_core` straight-line return shapes are now represented in
the environment. Mutation (`set`), nested control flow, alternate
match-arm shapes, full-corpus lowering, S5 codegen and S6 bootstrap remain
open.
