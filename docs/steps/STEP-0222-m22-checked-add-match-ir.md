# STEP-0222: M22 checked-add all-return match IR

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0221
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0222.ps1`

## 1. Objective

Lower the first Result-shaped checked-arithmetic control-flow slice: a
two-parameter `I64.checked_add` or `U64.checked_add` match whose
`ok(binding)` arm returns the projected payload and whose `error(_)`
arm returns the same-width zero literal.

## 2. Contract and mechanism

- The accepted shape is deliberately exact: two same-width parameters,
  one computed checked-add subject, `ok(binding)` followed by
  `error(_)`, and both arms return.
- The computed Result is spilled before arm value allocation. Canonical
  IR is three blocks: entry emits `checked_add` and `write_local`;
  the ok arm emits `read_local` and `project(ok)`; the error arm emits
  the width-correct zero constant.
- Source ranges, SSA ids, local identity, variant patterns and block ids
  are emitted in Rust `lower_core` order. The independent verifier and
  canonical serializer remain the oracle.
- A non-zero error fallback is outside this slice and fails closed with
  `ERR:E-SH-IR-MATCH-SHAPE`; no partial IR bytes are returned.

## 3. Executable evidence

Two new real-runner programs (`I64` and reversed-operand `U64`) build
the Sico parser Component, deserialize its output, pass the independent
IR verifier, round-trip through canonical JSON, and match Rust
`lower_core` byte-for-byte. The cumulative positive differential set is
34 programs. The non-zero fallback refusal proves the bounded shape does
not silently widen.

## 4. Residuals

`checked_sub/mul/div` were subsequently closed for this same bounded
shape by STEP-0223. Literal/nested checked operands, alternate arm
order or payload shapes, non-return arm bodies, general statements and
full-corpus lowering remain open. This STEP advances S4 only; it does not
close general match support, codegen, bootstrap or M22.
