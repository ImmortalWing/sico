# STEP-0241: M22 origin/dev lowering quality repair

> - status: complete / partial-S4
> - phase: M22 S4 quality repair
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU

## Objective

Repair two defects found by reviewing `origin/dev` before integration and
restore the M22 status labels to the accepted plan definitions. This step
does not claim frozen-corpus completion, Component codegen, or bootstrap.

## Changes

- The origin General-CFG path resolves a declared local cell before a
  same-named parameter, matching Rust `lower_general`. After integration,
  the newer cell frontend closes the same hazard conservatively with the
  typed `ERR:E-SH-IR-CELL-SHADOW` refusal. Neither path can emit the former
  verifier-valid but semantically divergent IR.
- The reachable refusal identity `ERR:E-SH-IR-STATEMENTATEMENT` is corrected
  to the stable `ERR:E-SH-IR-STATEMENT` spelling.
- The real-runner suite covers parameter/local shadowing and pins the
  advanced frontend's typed fail-closed behavior.
- STATUS, ROADMAP and the M22 plan index now distinguish partial S1–S4 work
  from the unentered S5 codegen and S6 A=B=C bootstrap gates.
- Integration with the local modular frontend also repairs three
  behavior-preserving `selfhost_compiler` test lints exposed by the
  all-target runner clippy gate.
- Eleven unindexed origin STEP records that reused the authoritative
  STEP-0213–0223 identifiers are removed after their implementation and
  regression evidence is integrated. The validator now requires exactly one
  M22 STEP record for every identifier from STEP-0213 through STEP-0241.

## Validation

```powershell
.\tools\validate-step-0241.ps1
```

The validator runs runner formatting, the `selfhost_parser` real-runner
suite, runner clippy, patch whitespace checks, and assertions for the two
repaired source/test contracts.

## Residuals

The 110 positive differential fixtures are bounded internal evidence. They
do not close the plan's frozen-corpus formatter/checker/parser/lowering
gates. RFC-0011 deterministic Component codegen (S5), ADR-0015 A=B=C
bootstrap/package/budget evidence (S6), and the M22 exit audit remain open.
