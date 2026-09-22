# STEP-0252: M22 nested intrinsic user-call condition byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence
> - completed: 2026-09-22
> - evidence: internal-fixture, Windows x64 GNU real runner, PowerShell 5.1 host

## Objective

Advance the formatter canary beyond STEP-0251 by lowering the actual
`scan_ident` condition shape, `ident_continue(sico.bytes.at(...))`, in the
general while-body builder without widening the language contract or hiding an
unsupported expression behind a fallback.

## Changes

- Added `gw_condition_packed` as the condition dispatcher used by both `while`
  and `if` in `general_while_function_ir`. Existing `I64`/`U64` fixed compares
  still delegate unchanged to `gw_compare_packed`.
- Added a narrow typed path for a declared one-parameter `Bool` user function
  whose single `I64` argument is `sico.bytes.at`. It reuses
  `while_bytes_at_rhs_packed`, preserves the Rust oracle's evaluation order
  (`read_local`, default literal, intrinsic, user call), computes independent
  inner/outer source spans, and rejects target, arity, argument and type
  mismatches with existing `E-SH-IR-*` identities.
- Added `sico_compiler_lowers_the_scan_ident_region_byte_exactly`, extending
  the formatter prefix differential from eight to nine functions.
- Added `validate-step-0252.ps1`, which executes the full self-host compiler
  differential, the component local-bound regression, the real-runner full
  formatter frontier, and `git diff --check`.

## Executed validation

- The formatter prefix through `scan_ident` compares byte-for-byte with the
  Rust `lower_core` canonical IR. The focused Rust oracle confirms the exact
  four-instruction nested condition order and source ranges.
- `sico_compiler` is 9/9 green, including the new `scan_ident` differential;
  `selfhost_local_bounds` remains green and the self-host compiler Component
  still builds under the frozen local limit.
- The full formatter canary reaches `scan_integer` and fails closed through
  the real runner with exit 122 / `ERR:E-SH-IR-STATEMENT`; it does not trap or
  emit partial IR.
- `validate-step-0252.ps1` and `git diff --check` pass.

## Honest residuals

- `scan_integer` contains the next unsupported canonical region: an `if`
  without `else` inside the while body. The general builder currently requires
  the paired branch shape and therefore refuses it as a statement boundary.
- General nested expressions remain unsupported; this step admits only the
  measured `Bool` user-call plus `sico.bytes.at` shape and keeps all other
  targets, arities and types fail-closed.
- S5 general deterministic Component codegen, S6 `A == B == C` bootstrap
  packaging/budget evidence, and S7 exit audit remain open. M22 remains NO-GO.
