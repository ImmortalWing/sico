# STEP-0243: M22 checker E1xxx syntax identities

> - status: complete / partial-S2
> - phase: M22 S2 checker
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU

## Objective

Implement the frozen E1001–E1016 syntax diagnostic identities in the
Sico-written integrated parser and consume them from the checker. Detection
must follow source structure and delimiters rather than fixture names,
expectation comments, or source hashes.

## Changes

- `compiler_parser.syntax_diagnostic` tracks top-level declaration blocks,
  nested `using` / `task` / `match` scopes, function/type/call delimiters, and
  the canonical module/use/interface-version shapes.
- The checker returns typed `invalid-input` outcomes carrying the stable
  E1001–E1016 identity before its legacy header report.
- The real-runner suite covers all 12 frozen B-syntax mutations plus E1013
  top-level execution, E1014 module shape, E1015 use shape, and E1016 interface
  version cases.
- The existing 215-source partition test remains green, proving the new
  structural checks do not misclassify the frozen lexical, accepted, or
  semantic-error sources.

## Validation

```powershell
.\tools\validate-step-0243.ps1
```

The validator runs the Rust parser oracle, source-bundle integrity tests,
checker and compiler frontend real-runner suites, all-target runner clippy,
STEP uniqueness, and patch whitespace checks.

## Residuals

S2 remains partial. Its lexical subset and E1001–E1016 syntax identities are
now executable, but 34 non-lexical frozen-corpus refusals still require
semantic/module identities (`E2001`–`E8010`). The old free-form missing-colon
header report is also outside the canonical catalog. S3/S4/S5 remain partial;
S6 A=B=C bootstrap has not started.
