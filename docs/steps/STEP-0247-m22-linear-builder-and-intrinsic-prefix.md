# STEP-0247: M22 linear builder and intrinsic-prefix convergence

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S3/S4 runtime and lowering convergence
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU real runner

## Objective

Remove the complete-source lexer trap without raising the guest memory limit,
preserve persistent-list semantics, and advance the shared Sico lowering path
from the six scalar probes into real formatter source.

## Changes

- Replaced quadratic copy-on-write list growth in the Wasm standard-library
  helper with a private, geometrically sized table header.  A linear append to
  the latest logical version reuses spare capacity; appending an older alias
  forks to a new table.  The public `(table, count)` value shape and immutable
  List semantics do not change.
- Applied the same versioned-capacity rule to `List[Text]` and numeric
  `List[I64|U64]` helpers.  Foreign/exact-sized tables take the safe copy path.
- Added a real-runner alias regression covering both layouts.  It proves that
  `one,two` remains unchanged after branching from `one`, and repeats the same
  check for `List[U64]`.
- Generalized checked-arithmetic match lowering so an error arm may return an
  existing same-typed parameter, matching the Rust oracle instead of requiring
  only a zero literal.
- Added reusable intrinsic path and instruction serialization and lowered the
  nested `sico.bytes.equal(sico.text.encode(...), ...)` formatter shape.
- Added canonical sequential fixed-width guard-chain lowering with explicit
  fallthrough jump blocks.  The formatter prefix through `ascii_letter` is
  canonical-IR byte-identical to Rust.

## Executed validation

- `sico-codegen-wasm`: 23 passed, 0 failed, 1 ignored maintainer snapshot task;
- `list_append_cow`: 1/1 through the real Script runner;
- `selfhost_compiler`: 7/7, including the formatter intrinsic/guard prefix against
  `sico_ir::lower_core` and canonical serialization;
- the full `formatter.sico` canary no longer traps at the 64 MiB arena.  It now
  fails closed at the next unsupported CFG shape with
  `ERR:E-SH-IR-STATEMENT`.

## Honest residuals

- Fixed-width guard chains are lowered; user-call conditions/final expressions
  are not yet part of that path.  The first open formatter function is
  `ident_start`; this blocks the rest of the formatter and complete self-host
  source canaries.
- S5 general deterministic Component codegen and all S6 ADR-0015 bootstrap,
  package and budget gates remain open.  This STEP does not claim M22 GO.
