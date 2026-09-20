# STEP-0250: M22 while-path Bytes parameters

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence
> - completed: 2026-09-21
> - evidence: internal-fixture, Windows x64 GNU real runner, PowerShell 5.1 host

## Objective

Close the typed frontier recorded by STEP-0249: the while-function lowering
rejected every function carrying a `Bytes` parameter before any statement
lowering, because `scalar_type_kind` refused the type name and the refusing
`ERR:` string was then misclassified as a call-type mismatch by argument
checks.  `scan_space(src: Bytes, start: U64, size: U64)` — the next
formatter canary function — cannot lower without this.

## Changes

- Added `Bytes` → `"bytes"` to `scalar_type_kind`.  The IR, verifier and
  deterministic codegen already carry `Type::Bytes` and the
  `sico.bytes.at(Bytes, U64, I64) -> I64` intrinsic signature; the
  `while_bytes_at_rhs_packed` helper from STEP-0249 already expected the
  `"bytes"` parameter kind.  This change only unblocks the parameter
  machinery that feeds them.
- Re-measured the canary frontier: `scan_space` now advances past its
  signature, its `bytes.at` let and the while-condition lowering, and the
  typed refusal moved from `E-SH-IR-CALL-TYPE` to `E-SH-IR-STATEMENT` at
  the while-body if/else region machinery — the statement walk routes only
  let/set/while, emits a lone if condition, and never opens then/else
  statement regions (the b3–b6 assembly scaffold exists but no region ever
  targets it).
- Validator frontier policy: `validate-step-0247/0248/0249.ps1` now pin the
  typed fail-closed class (any `ERR:E-SH-IR-*`, never a guest trap) so the
  committed validators stop churning as the frontier advances;
  `validate-step-0250.ps1` pins the exact current code
  `E-SH-IR-STATEMENT`.

## Executed validation

- minimal-probe evidence through the real runner: a `Bytes`-parameter
  while-function with a fixed-width-literal set body previously failed
  `E-SH-IR-PARAMETER-TYPE` and now lowers; `scan_space`-shaped probes
  advance to the if/else region refusal;
- `validate-step-0250.ps1` green (markers, self-host component builds,
  local-bounds regression, canary pinned at `E-SH-IR-STATEMENT`, no trap);
- `git diff --check` passes.

## Honest residuals

- The while-body if/else statement regions are the open S4 frontier: the
  general shape needs the canonical Rust block order (entry, while header,
  body entry, while-after, then, else, join) with source-order value
  allocation, replacing the hardcoded guard-shape assembly.
- Straight-line `let`-RHS intrinsic calls (e.g. `sico.bytes.at` outside a
  while body) still refuse with `E-SH-IR-EXPRESSION`; S5/S6/S7 remain open.
  M22 therefore remains NO-GO.
