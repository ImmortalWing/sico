# STEP-0244: M22 checker core semantic identities

> - status: complete / partial-S2
> - phase: M22 S2 checker
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU

## Objective

Implement the frozen E2001, E2002, E2010, E2011, and E2020 checker subset
from source declarations and expressions. The implementation must build real
symbol tables and must not identify fixtures from paths, comments, hashes, or
case names.

## Changes

- New `compiler_semantics` module builds bounded struct-of-arrays tables for
  newtypes, records, fields, invariants, and simple function signatures.
- Its second pass checks return literals, nominal/record call arguments,
  record constructor completeness/unknown fields, and literal `<=`
  invariants only when enough type evidence is available.
- Nine frozen E2xxx sources now return their exact Rust diagnostic identity;
  65 accepted sources remain accepted and 25 unsupported semantic/module
  refusals remain explicitly open.
- The legacy free-form header scanner is deleted. Function headers missing a
  colon now fail with the disjoint typed
  `E-SH-SYNTAX-FUNCTION-COLON` identity instead of returning exit code 0, and
  valid resource/interface method declarations are no longer falsely
  reported.

## Validation

```powershell
.\tools\validate-step-0244.ps1
```

The validator runs the Rust semantic oracle, frozen bundle integrity, the
real-runner checker/compiler/parser suites, all-target clippy, STEP uniqueness,
and whitespace checks.

## Residuals

S2 remains partial with 25 unsupported frozen refusals: match/result mapping,
effects/capabilities, affine/future/task/stream, component/revision, and the
standalone module E8010 identity. S3/S4/S5 remain partial and S6 A=B=C has not
started.
