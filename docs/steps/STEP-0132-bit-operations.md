# STEP-0132: fixed-width bit operations

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); profile item 5 (fixed-width
>   arithmetic ... bit and/or/xor/shift suitable for bitboards)

## 1. What was implemented

The RFC-0038 bit-operation slice, end to end:

- **Source surface**: `I64.bit_and/bit_or/bit_xor/shl/shr` and the same
  five on `U64`. Both operands share the operand type; wasm masks shift
  counts to [0, 63]. `shr` is arithmetic (`i64.shr_s`) on `I64` and
  logical (`i64.shr_u`) on `U64`.
- **Semantics**: `infer_fixed_width_call` accepts the five operations and
  types them as the operand type (infallible — no `Result` wrapper).
- **IR**: five pure operations (`BitAnd/BitOr/BitXor/Shl/Shr`, each two
  `ValueId` operands) with verifier rules: both operands and the result
  share the fixed-width type; the result is not a `Result`.
- **Codegen**: single-slot emission — `i64.and/or/xor/shl` and
  `shr_s/shr_u` selected by the result type.
- **Snapshot name maps** in the frozen lowering tests gained additive
  `bit-and/bit-or/bit-xor/shl/shr` entries; the frozen cases themselves
  are byte-identical (they contain no bit operations).

## 2. End-to-end evidence

`tests/end-to-end/bit-ops-bitboard.sico` — a bitboard-style popcount loop
(`shr` + `bit_and` with an explicit scan flag), plus asserted identities:
`and(44,14)=12`, `or(44,14)=46`, `xor(44,14)=34`, `shl(1,3)=8`,
`shr_u(8,3)=1`, and `shr_i64(-8,1)=-4` (arithmetic). Compiles with
`sico build --profile script-v0`, runs through the real runner printing
`bit-ops-ok`. Covered by `runner/sico-runner/tests/bit_ops.rs` (2 tests:
deterministic result, repeated-run isolation).

## 3. Residuals

- Rotate operations and `U64.shr` by runtime-computed amounts ≥ 64 rely on
  wasm's implicit masking; documented behavior, no extra surface.
- Bit operations on non-fixed-width types (unbounded `Int`) remain
  check-only per the RFC (`Int` runtime form unfrozen).

## 4. Validation

`cargo test` root workspace: all green (frozen core-lowering snapshots
byte-identical). Runner workspace: all suites green including the new
`bit_ops.rs` (serial run per the documented process-global-test
constraint). `cargo fmt` clean; `cargo clippy -D warnings` clean;
`git diff --check` clean; `tools/validate-step-0131.ps1` green (the
matrix fixture gained the bit-operations row).
