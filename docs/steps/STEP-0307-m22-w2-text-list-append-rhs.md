# STEP-0307: M22 W2 slice 15 — Text list append RHS

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 15
> - date: 2026-09-28
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0306 left `normalize_source` at `ERR:E-SH-IR-GWPACK-OTHER` on the
`output` binding's `sico.list.append` RHS. W1 independent CI and the
STEP-0282 W2 baseline remain the entry evidence; ADR-0017 Option A and
RFC-0047 are accepted. This slice lowers `sico.list.append` for a
`List[Text]` atom and a `Text` atom (local, parameter or string literal)
as a complete RHS. It checks delimiter/arity, both argument types and
SSA counts before emitting canonical IR. Other list element kinds and
nested argument expressions stay refused. No language, WIT or Host
contract changes.

The unique exit test is a real-runner byte differential against Rust IR
covering an empty string and a Text local in the same function. Extra
arguments and a U64 item refuse with stable typed codes; the valid
fixture succeeds after refusals in a fresh Store.

## Evidence and verdict

- The targeted real-runner differential passed with the two append
  forms byte-identical to Rust canonical IR, typed `CALL-SHAPE` /
  `CALL-TYPE` refusals and recovery.
- A temporary diagnostic probe, removed from the final code, located
  the next `normalize_source` refusal at byte offset 21778: the `active`
  binding's `sico.map.put[Text,U64](active, block_key, U64.literal(0))`.
  The stable code is `ERR:E-SH-IR-CALL-SHAPE`. The formatter canary
  remains 28/30, full-source exit 122; the refusal advanced, so the R3
  consecutive-stall count remains **0**.
- No independent exact-SHA CI, S5 codegen corpus, S6 bootstrap or S7
  audit ran. M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted `sico_compiler_lowers_text_list_append_rhs_byte_exactly` | 1/1 passed, Windows x64 GNU real runner |
| `tools/validate-step-0307.ps1` | exit 0; 39/39 compiler tests, 2/2 parser tests, 1/1 local-bounds test, inherited STEP-0262/0302–0306 checks green |
| `tools/report-m22-canary.ps1` | exit 0; 28/30, `normalize_source` / `ERR:E-SH-IR-CALL-SHAPE`, full-source exit 122; 5,000,000,000 fuel cap |
| root/runner `cargo fmt --all -- --check` | both exit 0 |
| `tools/validate-step-0124.ps1 -SelfTest`; `git diff --check` | both exit 0; negative roadmap cases 5/5 |
| independent CI | not run; no commit/push authorized |
