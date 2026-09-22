# STEP-0253: M22 general while no-else frames byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence
> - completed: 2026-09-22
> - evidence: internal-fixture, Windows x64 GNU real runner, PowerShell 5.1 host

## Objective

Close the STEP-0252 `scan_integer` frontier by reproducing Rust `lower_core`'s
canonical if-without-else block allocation inside the general while builder,
including nested and sequential frame behavior rather than only the first
observed function.

## Changes

- Replaced the overloaded active-length frame index with append-only frame
  ordinals linked through `gfr_parent`; `gdepth` alone tracks active nesting.
  This preserves the already-supported nested else/join tail behavior while
  allowing later sequential `if` frames to receive fresh ordinals.
- Added parallel deferred-else tables. An if-without-else records its empty
  else block during closure, assigns its final id after all primary blocks,
  and emits the canonical jump back to the join with the original if-header
  range. Branch targets are resolved against the final primary block count.
- Advanced the source cursor across every `end if`, so a following sequential
  condition cannot accidentally anchor its range to the `if` token inside the
  preceding `end if`.
- Corrected two SSA accounting defects exposed by the larger canary: nonempty
  single-instruction JSON now counts as one in `gw_instr_count`, and a return
  RHS advances from its last result by exactly one instead of adding the whole
  instruction count a second time.
- Added byte-exact prefix differentials for `scan_integer`, `scan_comment`,
  `scan_string`, and `punctuation_kind`, advancing the formatter prefix from
  nine to thirteen functions.

## Executed validation

- The four new formatter prefix differentials compare byte-for-byte with Rust
  canonical IR through the real runner. They exercise one and multiple
  if-without-else regions, nested and sequential if frames, deferred empty
  else blocks, a one-instruction user-call argument, and a multi-instruction
  return call.
- `sico_compiler` is 13/13 green; the prior `scan_space` and `scan_ident`
  regressions remain byte-exact. `selfhost_local_bounds` remains green and the
  compiler Component still builds within the frozen verifier limit.
- The full formatter canary reaches `item` and fails closed with exit 122 /
  `ERR:E-SH-IR-CALL-TARGET` at `match sico.list.get(...)`; it does not trap or
  emit partial IR.
- `validate-step-0253.ps1` and `git diff --check` pass.

## Honest residuals

- The next unsupported canonical region is the `item` function's
  `sico.list.get` result match. Existing checked-arithmetic match lowering
  does not yet dispatch collection intrinsics, so the intrinsic is currently
  interpreted as an unresolved user-call target and typed-refused.
- Deferred empty else blocks are implemented for the measured general-while
  path; this step does not claim arbitrary nested control lowering outside the
  formatter canary or a new language surface.
- S5 general deterministic Component codegen, S6 `A == B == C` bootstrap
  packaging/budget evidence, and S7 exit audit remain open. M22 remains NO-GO.
