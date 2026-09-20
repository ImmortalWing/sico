# STEP-0242: M22 checker lexical partition

> - status: complete / partial-S2
> - phase: M22 S2 checker
> - completed: 2026-09-20
> - evidence: internal-fixture, Windows x64 GNU

## Objective

Replace the checker toy's unchecked lexical path with the integrated lossless
lexer and prove the declared lexical diagnostic subset against every source in
the frozen M22 corpus. This step does not claim semantic checker parity or S2
completion.

## Changes

- `compiler_lexer` now exposes a streaming `has_error` query built on the same
  `token_finish` and `token_kind` core used by lossless token serialization.
  This removes the first implementation's large intermediate token string and
  avoids duplicated classification logic.
- `checker.sico` imports that module and returns the stable typed
  `invalid-input / LEXICAL` refusal before its bounded header checks.
- The real-runner differential executes all 215 frozen sources: all 116
  `LEXICAL` entries refuse with the exact identity and all other 99 entries
  remain outside that lexical subset.
- The compiler frontend token/parsing tests remain green after the lexer
  refactor, so the shared implementation does not silently change its existing
  consumer.

## Validation

```powershell
.\tools\validate-step-0242.ps1
```

The validator verifies the frozen corpus manifest, runs the 215-source checker
partition and compiler frontend regressions through the real runner, applies
all-target clippy, checks the unique STEP range, and rejects whitespace errors.

## Residuals

S2 remains partial: 34 non-lexical Rust refusals carry semantic or module
diagnostics (`E2001`–`E8010`) that this checker does not yet reproduce, and the
header report is not yet a canonical Rust diagnostic. S3/S4/S5 remain partial;
S6 A=B=C bootstrap has not started.
