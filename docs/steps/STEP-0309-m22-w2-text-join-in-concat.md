# STEP-0309: M22 W2 slice 17 — final text.join in formatter

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 17
> - date: 2026-09-28
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0308 left `normalize_source` at `ERR:E-SH-IR-CALL-ARGUMENT` on
its final `sico.text.concat(sico.text.join(output, "\\n"), "\\n")`.
W1 independent CI and STEP-0282 W2 baseline are the entry evidence;
ADR-0017 Option A and RFC-0047 are accepted. This slice handles a
complete `sico.text.join(List[Text], Text)` as either concat argument.
The existing bounded Text list binary packer now takes the result kind
and intrinsic name, sharing exact arity, type, source-span and SSA
checks with Text `list.append`. Other list kinds and nested arguments
remain typed-refused. No language, WIT or Host contract changes.

## Evidence and verdict

- The real-runner `text.join`-inside-concat fixture emits Rust canonical
  IR byte-for-byte, including nested argument evaluation and ranges.
  Wrong list type and an extra argument refuse as `CALL-TYPE` and
  `CALL-SHAPE`; the valid fixture succeeds after refusals in a fresh
  Store.
- The actual `formatter.sico` prefix through `normalize_source`
  now emits Rust canonical IR byte-for-byte. `report-m22-canary.ps1`
  measured **29/30 (96.7%)** compiled functions; the next frontier is
  `main` / `ERR:E-SH-IR-SCICOND`, full-source exit 122. The R3
  consecutive-stall count remains **0**.
- S5 codegen corpus, S6 `A == B == C` bootstrap, S7 exit audit and
  independent exact-SHA CI were not run. M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted `text.join` and `normalize_source` differentials | 2/2 passed, Windows x64 GNU real runner |
| `tools/validate-step-0309.ps1` | exit 0; 43/43 compiler tests, 2/2 parser tests, 1/1 local-bounds test and inherited STEP-0262/0302–0308 checks green |
| `tools/validate-step-0261.ps1` (separate fresh run) | exit 0; 43/43 compiler, 2/2 parser, 1/1 local-bounds, historical 17-function prefix and current `MAIN-SCICOND` canary green |
| `tools/report-m22-canary.ps1` | exit 0; 29/30, `main` / `ERR:E-SH-IR-SCICOND`, full-source exit 122; 5,000,000,000 fuel cap |
| root/runner `cargo fmt --all -- --check` | both exit 0 |
| `tools/validate-step-0124.ps1 -SelfTest`; `git diff --check` | both exit 0; negative roadmap cases 5/5 |
| independent CI | not run; no commit/push authorized |
