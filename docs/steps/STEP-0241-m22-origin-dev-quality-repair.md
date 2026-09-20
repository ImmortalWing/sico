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

- General-CFG expression lookup now resolves a previously declared local
  cell before a same-named parameter, matching Rust `lower_general`.
  Previously the Sico frontend accepted the source but returned the
  parameter SSA value while Rust emitted `read_local`, creating valid yet
  semantically different IR.
- The reachable refusal identity `ERR:E-SH-IR-STATEMENTATEMENT` is corrected
  to the stable `ERR:E-SH-IR-STATEMENT` spelling.
- The real-runner differential suite covers parameter/local shadowing, and
  its refusal corpus covers a literal return after a straight-line binding.
- STATUS, ROADMAP and the M22 plan index now distinguish partial S1–S4 work
  from the unentered S5 codegen and S6 A=B=C bootstrap gates.

## Validation

```powershell
.\tools\validate-step-0241.ps1
```

The validator runs runner formatting, the `selfhost_parser` real-runner
suite, runner clippy, patch whitespace checks, and assertions for the two
repaired source/test contracts.

## Residuals

The 69 positive differential fixtures are bounded internal evidence. They
do not close the plan's frozen-corpus formatter/checker/parser/lowering
gates. RFC-0011 deterministic Component codegen (S5), ADR-0015 A=B=C
bootstrap/package/budget evidence (S6), and the M22 exit audit remain open.
