# STEP-0312: M22 W2 slice 19 — `let` with `sico.bytes.utf8_decode(<param>.stdin)` RHS

> - status: implementation complete locally; independent CI pending (gitcode pushed, GitHub unreachable from this host)
> - phase: M22 compiler self-host, execution card 22-C slice 19
> - date: 2026-09-29
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0311 lowered `main`'s `sico.bytes.is_utf8(input.stdin)` condition and
the `ScriptInput` named-parameter signature; the full source then refused
`ERR:E-SH-IR-CALL-SHAPE` at `main`'s first statement. This slice declares
one shape: the `let` binding whose RHS is
`sico.bytes.utf8_decode(<ScriptInput-param>.stdin)`. `gw_rhs_packed`'s
`utf8_decode` branch now recognises the dotted single argument before the
bare-atom paths and emits `project{base:<param id>,field:"stdin"}`
(range covering `.stdin`) followed by the `string`-result
`sico.bytes.utf8_decode` intrinsic (range spanning the full call), which
the existing let handler wraps with its `write_local`. Other argument
shapes keep their existing typed refusals. No language, WIT or Host
contract changes.

## Evidence and verdict

- The probe fixture emits Rust canonical IR byte-for-byte on the real
  runner, including the project/intrinsic/write_local SSA chain and
  ranges. Non-`stdin` field → `CALL-TYPE`; field access on a
  non-ScriptInput parameter → `CALL-TYPE`; bare parameter → `CALL-TYPE`;
  two arguments → `CALL-SHAPE` (via the existing single-token argument
  check). The valid fixture compiles after each refusal in a fresh
  Store.
- The full source now refuses one shape deeper: `main` /
  `ERR:E-SH-IR-CALL-TARGET` at `return error(ScriptError(...))` —
  `error`/`ok` are not yet declinable callees, so the Result-wrapping
  return is the next frontier. The canary stays **29/30 (96.7%)**; the
  frontier moved, so the R3 consecutive-stall count remains **0**.
- S5 codegen corpus, S6 `A == B == C` bootstrap, S7 exit audit and
  independent exact-SHA CI were not run for this slice at authoring
  time. M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted `let`-RHS differential (`selfhost_compiler.rs`) | suite green including the new `sico_compiler_lowers_let_utf8_decode_field_rhs_byte_exactly`, Windows x64 GNU real runner |
| `tools/validate-step-0312.ps1` | exit 0 with the inherited STEP-0311→0309→…→0262 chain green |
| `tools/report-m22-canary.ps1` | exit 0; 29/30, `main` / `ERR:E-SH-IR-CALL-TARGET`, full-source exit 122; 5,000,000,000 fuel cap |
| runner `cargo fmt -- --check` | exit 0 |
| independent CI | pending; GitHub unreachable from this host (2026-09-29), snapshot pushed to gitcode `origin` |

## Frontier pointer updates

The moving-frontier pin test was renamed
`sico_compiler_refuses_full_formatter_at_main_let_rhs` →
`..._at_main_error_return` with its assertion updated to
`ERR:E-SH-IR-CALL-TARGET`, and the `MAIN-LET-RHS` pointers in
`validate-step-0261/0262/0302–0309/0311.ps1` were repinned to
`MAIN-ERROR-RETURN`. The historical differential tests were not renamed.
