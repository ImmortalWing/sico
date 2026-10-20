# STEP-0308: M22 W2 slice 16 — map.put fixed-literal value

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 16
> - date: 2026-09-28
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0307 left `normalize_source` at `ERR:E-SH-IR-CALL-SHAPE` on
`sico.map.put[Text,U64](active, block_key, U64.literal(0))`. W1
independent CI and STEP-0282 W2 baseline are the entry evidence;
ADR-0017 Option A and RFC-0047 are accepted. This slice admits one
complete `U64.literal(...)` as the final `map.put[Text,U64]` RHS
argument. Existing argument-kind checking and canonical emission remain
the oracle; extra arguments and wrong literal type stay typed-refused.
No language, WIT or Host contract changes.

## Evidence and verdict

- A real-runner differential with a local Map, Text key and U64 literal
  passed byte-for-byte against Rust canonical IR. Extra argument and
  wrong literal kind returned `CALL-SHAPE`; the valid fixture succeeded
  after refusals in a fresh Store.
- Replacing only `normalize_source`'s final
  `sico.text.concat(sico.text.join(output, "\\n"), "\\n")` with a
  literal makes the complete preceding formatter prefix byte-exact
  against Rust IR in the real runner. The actual source still refuses
  at `ERR:E-SH-IR-CALL-ARGUMENT` on that final nested join. The
  formatter canary remains 28/30, full-source exit 122, while the
  refusal site advanced; R3 consecutive-stall count remains **0**.
- S5 codegen corpus, S6 `A == B == C` bootstrap, S7 exit audit and
  independent exact-SHA CI were not run. M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted map.put and normalize-source-prefix differentials | 2/2 passed, Windows x64 GNU real runner |
| `tools/validate-step-0308.ps1` | exit 0; 41/41 compiler tests, 2/2 parser tests, 1/1 local-bounds test, inherited STEP-0262/0302–0307 checks green |
| `tools/report-m22-canary.ps1` | exit 0; 28/30, `normalize_source` / `ERR:E-SH-IR-CALL-ARGUMENT`, full-source exit 122; 5,000,000,000 fuel cap |
| root/runner `cargo fmt --all -- --check` | both exit 0 after one test formatting repair |
| `tools/validate-step-0124.ps1 -SelfTest`; `git diff --check` | both exit 0; negative roadmap cases 5/5 |
| independent CI | not run; no commit/push authorized |
