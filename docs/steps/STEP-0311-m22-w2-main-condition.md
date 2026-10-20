# STEP-0311: M22 W2 slice 18 — main entry condition `sico.bytes.is_utf8(input.stdin)`

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 18
> - date: 2026-09-29
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0309 left `formatter.sico` at canary 29/30 with the full source
refusing `ERR:E-SH-IR-SCICOND` at `main`'s condition; the 0306–0310
snapshot `9c94e7d` was CI-adjudicated (run 36406396074) on the same
day, and the owner designated `codex/m22-w1-ci` the authoritative M22
line with commit/push authorized. This slice declares one composite
shape: the script-entry parameter signature (`input: ScriptInput`,
emitted as `{"kind":"named","data":"ScriptInput"}`) plus the bare
intrinsic-call condition `sico.bytes.is_utf8(<param>.stdin)`, which
`gw_condition_packed` previously refused unconditionally. The lowering
emits `project{base:<param id>,field:"stdin"}` (range covering
`.stdin`) followed by the `sico.bytes.is_utf8` intrinsic with `bool`
result, byte-matching the Rust oracle. The parameter must resolve by
name to a `ScriptInput`-typed parameter (`parameter_type_word`), the
field must be `stdin`, and exactly one argument is accepted; any other
`sico.*` condition keeps refusing `SCICOND`. No language, WIT or Host
contract changes.

## Evidence and verdict

- The probe fixture (ScriptInput record, `probe(input: ScriptInput)`
  returning `U64`, the declared condition) emits Rust canonical IR
  byte-for-byte on the real runner, including both instruction ranges
  and SSA ids.
- Typed refusals: non-`stdin` field → `CALL-TYPE`; bare parameter
  without a field → `CALL-ARGUMENT`; two arguments → `CALL-ARITY`;
  field access on a non-ScriptInput parameter → `CALL-TYPE`; any other
  `sico.*` condition → `SCICOND`. The valid fixture compiles after
  each refusal in a fresh Store.
- The full source now refuses one shape deeper: `main` /
  `ERR:E-SH-IR-CALL-SHAPE` (the `let source =
  sico.bytes.utf8_decode(input.stdin)` RHS), full-source exit 122.
  The canary stays **29/30 (96.7%)** because `main` is not fully
  compiled yet; the frontier moved, so the R3 consecutive-stall count
  remains **0**.
- S5 codegen corpus, S6 `A == B == C` bootstrap, S7 exit audit and
  independent exact-SHA CI were not run for this slice at authoring
  time (CI pending on push). M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted `main`-condition differential (`selfhost_compiler.rs`) | suite 44/44 passed including the new `sico_compiler_lowers_main_condition_byte_exactly` and the repinned full-source refusal test, Windows x64 GNU real runner |
| `tools/validate-step-0311.ps1` | exit 0; `STEP_0311_OK main-condition=byte-exact scriptinput-parameter=named formatter-canary=29/30 current-frontier=MAIN-LET-RHS` with the inherited STEP-0262/0302–0309 chain green |
| `tools/report-m22-canary.ps1` | exit 0; 29/30, `main` / `ERR:E-SH-IR-CALL-SHAPE`, full-source exit 122; 5,000,000,000 fuel cap |
| runner `cargo fmt -- --check` | exit 0 |
| `tools/validate-step-0124.ps1 -SelfTest`; `git diff --check` | both exit 0; negative roadmap cases 5/5 |
| independent CI | pending; GitHub is unreachable from this host (2026-09-29), the snapshot is pushed to gitcode `origin` and awaits a GitHub Windows GNU run for exact-SHA adjudication |

## Frontier pointer updates

The moving-frontier pin test was renamed
`sico_compiler_refuses_full_formatter_at_main_condition` →
`..._at_main_let_rhs` with its assertion updated to the observed
frontier, and `current-frontier=MAIN-SCICOND` pointers in
`validate-step-0261/0262/0302–0309.ps1` were repinned to
`MAIN-LET-RHS` (the same living-pointer practice used when 0306–0309
repinned them to `MAIN-SCICOND`). The historical differential tests
were not renamed.
