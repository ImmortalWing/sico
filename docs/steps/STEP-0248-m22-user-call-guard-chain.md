# STEP-0248: M22 user-call guard-chain convergence

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU real runner

## Objective

Extend the canonical sequential guard-chain path from fixed-width predicates to
ordinary typed Sico calls without adding source-specific templates.  Cover
literal call arguments and both Bool and Text returns against the Rust oracle.

## Changes

- Added one reusable user-call expression serializer for guard conditions and
  final return expressions.  It resolves declaration ordinals, checks argument
  arity/types through the shared environment machinery, emits literal
  instructions at the current SSA base and preserves exact source ranges.
- Allowed a guard chain to contain one or more guards.  This closes
  `ident_start`, whose final value is a call, as well as `ident_continue`, whose
  first condition is a call.
- Generalized the same CFG builder to Text-returning chains.  Guard-arm and
  final string constants now use canonical `const_string` serialization.
- Added compact first-difference diagnostics to the byte-parity regression so
  failures report a bounded JSON neighborhood instead of entire byte arrays.
- Advanced the formatter prefix oracle through `word_kind`: the first seven
  functions are canonical-IR byte-identical to `sico_ir::lower_core`.

## Executed validation

- frozen M22 source and script-build manifests verify at 215/215 entries;
- `selfhost_compiler`: 7/7 through the real Script runner, including the
  formatter prefix through `word_kind` against the Rust canonical serializer;
- the full formatter canary remains fail-closed with the now-nearest
  `ERR:E-SH-IR-EXPRESSION` boundary and no guest trap;
- `git diff --check` passes.

## Honest residuals

- The first unproved formatter region now begins at `scan_space`, which
  combines mutable locals, `while`, nested `if/else`, calls and byte intrinsics.
  Passing the seven-function prefix is not a complete formatter canary.
- S5 general deterministic Component codegen, S6 A=B=C/bootstrap packaging and
  budget gates, and S7 exit audit remain open.  M22 therefore remains NO-GO.
