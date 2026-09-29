# STEP-0314: M22 W2 slice 21 — `return ok(ScriptOutput(...))` construct closes the formatter canary

> - status: implementation complete locally; independent CI pending (gitcode pushed, GitHub unreachable from this host)
> - phase: M22 compiler self-host, execution card 22-C slice 21
> - date: 2026-09-29
> - evidence class: internal-fixture, Windows x64 GNU real runner
>
> Verification replay (Windows x64 GNU, pinned toolchain 1.98.0-x86_64-pc-windows-gnu):
> `tools/report-m22-canary.ps1` → functions 30/30 (100%), `frontier_function=""`,
> `frontier_code=""`, `full_source_exit=0`, fuel cap 5,000,000,000;
> `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler
> -- --test-threads=1` → 47 passed including the two new tests;
> `cargo test --test selfhost_local_bounds` → busiest `general_while_function_ir`
> 240/256; `tools/validate-step-0314.ps1` → exit 0 with the inherited chain green.

## Entry and bounded shape

STEP-0313's frontier was `main` refusing `ERR:E-SH-IR-CALL-TARGET` at the
`return ok(...)` wrap. A probe replicating `main`'s exact statement mix
(nested `if` with a user-call condition, the `normalize_source` user call
as a `let` RHS, and the trailing `error` return) compiled once the ok
return was replaced, proving the refusal sat exactly at the ok wrap. This
slice declares one shape: the Result-wrapping ok return
`return ok(ScriptOutput(stdout: sico.text.encode(<string cell or param>),
stderr: sico.text.encode(<string literal>), exit_code: I64.literal(<canonical
magnitude>)))` in a function whose declared return type is
`Result[ScriptOutput, ScriptError]`. The new `gw_ok_wrap_rhs_packed` helper
(dispatched from `gw_rhs_packed` for `ok` receivers ahead of the intrinsic
paths; an empty return means "not this shape" and falls through to the
residual `while_call` refusal, matching the error branch's silent
fall-through levels) packs the three field values with the established
helpers — `gw_intrinsic_call_packed` for the stdout cell/param encode,
`scalar_const_instruction_json` + `intrinsic_instruction_json` for the
stderr literal encode, `fixed_literal_const_instruction` for the exit-code
literal — then emits the `construct ScriptOutput{stdout,stderr,exit_code}`
and the `variant ok` wrap with `script_result_return_json`'s frozen
script-ABI result type. Instruction order, SSA ids and all four span
kinds (atom-span read_local, call-span intrinsic, token-span const_string,
digit-span const_i64) reproduce the Rust oracle byte-for-byte. No
language, WIT or Host contract changes.

## Evidence and verdict

- The probe fixture (a `main`-shaped function plus two stub callees) emits
  Rust canonical IR byte-for-byte; the differential suite covers it
  including the nested-if region and the user-call `let`.
- Typed refusals: swapped field order → `CALL-SHAPE`; wrong construct name
  (`ok(ScriptError(...))`) → `CALL-TARGET` (residual fall-through); string
  literal in the stdout encode → `CALL-ARGUMENT`; non-Result signature →
  `CALL-TYPE`; missing closing paren → `CALL-SHAPE`; `U64.literal` for
  `exit_code` → `CALL-TYPE`. The valid fixture compiles after each refusal
  in a fresh Store. Deviations outside the declared shape (negative or
  non-literal exit codes, non-encode stdout/stderr expressions, extra
  fields) remain typed refusals, not silent fallbacks.
- **The formatter canary reaches 30/30 (100%)**: the full
  `selfhost/formatter.sico` source compiles through the guest compiler and
  its IR matches `rust_ir(FORMATTER_SOURCE)` byte-for-byte
  (`sico_compiler_lowers_full_formatter_byte_exactly`). The frontier is
  closed; the R3 consecutive-stall count stays 0 by construction (canary
  grew).
- S5 codegen corpus, S6 `A == B == C` bootstrap, S7 exit audit and
  independent exact-SHA CI were not run for this slice at authoring time.
  M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted ok-return differential (`selfhost_compiler.rs`) | suite green including the new `sico_compiler_lowers_ok_return_construct_byte_exactly`, Windows x64 GNU real runner |
| full-source differential (`sico_compiler_lowers_full_formatter_byte_exactly`) | byte-exact against the Rust oracle, 30/30 functions covered |
| `tools/validate-step-0314.ps1` | exit 0 with the inherited STEP-0313→0312→0311→0309→…→0262 chain green |
| `tools/report-m22-canary.ps1` | exit 0; 30/30 (100%), `frontier_function=""`, `frontier_code=""`, full-source exit 0; 5,000,000,000 fuel cap |
| runner `cargo fmt -- --check` | exit 0 |
| independent CI | pending; GitHub unreachable from this host (2026-09-29), snapshot pushed to gitcode `origin` |

## Local-bounds note

The ok-wrap machinery lives in the new `gw_ok_wrap_rhs_packed` helper
(following the STEP-0249 extraction precedent), leaving `gw_rhs_packed` at
175 locals and the link unit's busiest function unchanged
(`general_while_function_ir` at 240/256, headroom exactly 16 against the
frontier-reserve assertion). The next frontier-reserve consumer must
refactor `general_while_function_ir`.

## Frontier pointer updates

The moving-frontier pin test was replaced by
`sico_compiler_lowers_full_formatter_byte_exactly` (the full source is now
the byte-exact acceptance, not a refusal), and the `MAIN-OK-RETURN`
pointers in `validate-step-0261/0262/0302–0309/0311/0312/0313.ps1` were
repinned to `FORMATTER-CLOSED`. validate-step-0261's stale full-source
SCICOND assertion was updated to expect the closed frontier (exit 0).
The historical differential tests were not renamed.
