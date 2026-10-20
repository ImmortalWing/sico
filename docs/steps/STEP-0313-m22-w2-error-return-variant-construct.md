# STEP-0313: M22 W2 slice 20 — `return error(ScriptError(...))` Result wrap

> - status: implementation complete locally; independent CI pending (gitcode pushed, GitHub unreachable from this host)
> - phase: M22 compiler self-host, execution card 22-C slice 20
> - date: 2026-09-29
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0312 lowered the `utf8_decode` field-RHS `let`; the full source then
refused `ERR:E-SH-IR-CALL-TARGET` because `error`/`ok` are not declinable
callees. This slice declares one shape: the Result-wrapping error return
`return error(ScriptError(code: ScriptErrorCode.InvalidInput, message:
<literal>))` in a function whose declared return type is
`Result[ScriptOutput, ScriptError]`. Three pieces of machinery land
together because the shape is unobservable without them: (1)
`script_result_return_json` derives the frozen script-ABI result type
JSON from the function signature and the general-while emitter uses it
for `return_type`; (2) `gw_rhs_packed` gains an `error` branch emitting
`variant ScriptErrorCode.InvalidInput` (empty payload), the message
`const_string`, the `construct ScriptError{code,message}`, and the
`variant error` wrap with the exact oracle ranges; (3) the control-if
emitter declines `error`/`ok` returns (`ERR:E-SH-IR-EXPRESSION`, its
established SKIP signal) so if-bearing whileless functions route to the
general-while fallback instead of refusing. Single-statement bodies
claimed by the straight-cell emitter keep refusing (it has no safe
decline channel); that residual is recorded, not hidden. No language,
WIT or Host contract changes.

## Evidence and verdict

- The probe fixture (two error returns through if regions) emits Rust
  canonical IR byte-for-byte, including all four instruction ranges per
  return and SSA ids.
- Typed refusals: non-Result signature → `CALL-TYPE`; undeclared enum
  case → `CALL-TYPE`; swapped field order → `CALL-SHAPE`; single-return
  straight-cell bodies → `CALL-TARGET` (residual, see above). The valid
  fixture compiles after each refusal in a fresh Store.
- The full source now refuses one shape deeper: `main` /
  `ERR:E-SH-IR-CALL-TARGET` at the `return ok(ScriptOutput(...))` wrap.
  The canary stays **29/30 (96.7%)**; the frontier moved, so the R3
  consecutive-stall count remains **0**.
- S5 codegen corpus, S6 `A == B == C` bootstrap, S7 exit audit and
  independent exact-SHA CI were not run for this slice at authoring
  time. M22 remains **NO-GO**.

| Check | Result |
|---|---|
| targeted error-return differential (`selfhost_compiler.rs`) | suite green including the new `sico_compiler_lowers_error_return_variant_construct_byte_exactly`, Windows x64 GNU real runner |
| `tools/validate-step-0313.ps1` | exit 0 with the inherited STEP-0312→0311→0309→…→0262 chain green |
| `tools/report-m22-canary.ps1` | exit 0; 29/30, `main` / `ERR:E-SH-IR-CALL-TARGET` (ok return), full-source exit 122; 5,000,000,000 fuel cap |
| runner `cargo fmt -- --check` | exit 0 |
| independent CI | pending; GitHub unreachable from this host (2026-09-29), snapshot pushed to gitcode `origin` |

## Local-bounds note

`general_while_function_ir` sits at 241/256 locals (headroom 15 against
the +16 frontier-reserve assertion in `selfhost_local_bounds.rs`), so the
return-type branch avoids adding a local there (the helper result is used
inline, calling `script_result_return_json` twice). The next frontier
extension into that emitter will need the headroom refactor the test is
reserving.

## Frontier pointer updates

The moving-frontier pin test was renamed
`sico_compiler_refuses_full_formatter_at_main_error_return` →
`..._at_main_ok_return` (asserted code unchanged at `CALL-TARGET`), and
the `MAIN-ERROR-RETURN` pointers in
`validate-step-0261/0262/0302–0309/0311/0312.ps1` were repinned to
`MAIN-OK-RETURN`. The historical differential tests were not renamed.
