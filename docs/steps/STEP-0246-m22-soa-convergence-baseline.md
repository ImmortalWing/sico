# STEP-0246: M22 SOA convergence and evidence repair baseline

> - status: complete / S2 evidence restored + S3→S4 convergence baseline; M22 NO-GO
> - phase: M22 S3/S4
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU real runner

## Objective

Repair the frozen-corpus integrity failure found by the 2026-09-20 quality
review and replace the compiler's exact-source IR templates with one reusable
SOA lowering path.  Freeze the entry/module boundary without extending the
language surface or weakening the bootstrap contract.

## Changes

- Accepted ADR-0016: bounded SOA is the M22 v0 frontend representation;
  compilation units carry explicit entry/imported-module roles.
- `update-m22-corpus.ps1` hashes canonical UTF-8 LF bytes, so Windows CRLF
  checkout materialization cannot alter the frozen manifest.
- Re-froze `block-solver.sico` to the repository LF bytes and synchronized the
  Script-build manifest identity.  The updater's `-Verify` pass is idempotent.
- `selfhost_checker` now checks every frozen source length and SHA-256 before
  executing it; a stale source can no longer pass only because its diagnostic
  identity happens to remain unchanged.
- Converted `parser.sico` into the reusable `parser` module and added a thin
  Script ABI driver for its existing runner suite.
- `compiler.sico` now imports `parser.lex_words` and `parser.scalar_ir` for its
  normal IR output.  The identity/constant/add exact-source fallback is removed.
- Added general `Int + Int` lowering to that shared path so all six pre-existing
  compiler IR baselines remain covered without a source template.

## Validation

```powershell
.\tools\validate-step-0246.ps1
```

The validator checks the canonical manifests independently, runs the official
updater in verify mode, builds the parser driver and compiler with the local
Sico tool, executes both through the real runner, checks canonical IR structure
for identity and `add_int`, checks the bounded Core-Wasm seam, and runs
`git diff --check`.

The GNU runner regression was also executed after the manifest repair:

- `bootstrap_bundle`: 4/4;
- `selfhost_checker`: 8/8, including all 215 canonical sources and SHA checks;
- `selfhost_compiler`: 5/5 (four-test aggregate plus the updated typed-refusal
  case rerun);
- `selfhost_parser`: 2/2, with the 110-shape byte-exact assertions contained in
  the aggregate lowering test.

## Honest residuals

- `formatter.sico` reaches the shared lowering path but still exposes typed
  coverage gaps; the complete-source canary is not green.
- Large mixed-shape sources can still trap in the current SOA lowering machinery;
  traps must become typed refusals before S4 can close.
- S5 only retains the bounded nonnegative-Int Core-Wasm encoder.  General
  deterministic Script Component codegen is not implemented.
- S6 `A == B == C`, `.sapp` packaging, corpus replay and budget evidence have
  not started.  M22 remains NO-GO.
