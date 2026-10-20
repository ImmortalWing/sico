# STEP-0305: M22 W2 slice 13 — user-call arguments to text.concat

> - status: implementation complete; independent CI success (STEP-0310 adjudication)
> - phase: M22 compiler self-host, execution card 22-C slice 13
> - date: 2026-09-27
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0304 left `normalize_source` at `ERR:E-SH-IR-CALL-ARGUMENT`.
A temporary, removed diagnostic probe identified the first blocked
argument as `repeat_indent(indent_level)` inside
`sico.text.concat(repeat_indent(indent_level), format_code(tokens, code_end))`.

This slice permits a declared, Text-returning user call as either
argument of the existing bounded `sico.text.concat` RHS lowering. It
requires a complete call segment, uses the declared function ordinal and
return kind, emits argument instructions before the call, and advances
SSA IDs before the concat intrinsic. Wrong return type stays a typed
`ERR:E-SH-IR-CALL-TYPE` refusal. No language, WIT, Host or authority
contract changes.

## Evidence and remaining frontier

- A real-runner fixture with *two* user calls inside one concat emits
  Rust canonical IR bytes exactly; a wrong-typed callee refuses and the
  valid source compiles in a fresh Store afterward.
- The full formatter's first refusal is still
  `ERR:E-SH-IR-CALL-ARGUMENT`, but a removed diagnostic probe showed the
  site moved beyond `repeat_indent`/`format_code` to a nested `sico`
  intrinsic argument (the `text.trim(item(...))` region). Thus the
  28/30 canary is unchanged while the refusal site advances; R3
  consecutive-stall count remains **0**.

Fresh local `tools/validate-step-0305.ps1` passed: 37/37 real-runner
compiler tests, 2/2 parser tests, 1/1 local-bounds test and all
inherited STEP-0262/0302/0303/0304 checks. Root and runner
`cargo fmt --all -- --check`, `tools/validate-step-0124.ps1`, and
`git diff --check` passed. A separate fresh `tools/validate-step-0261.ps1`
passed (`parity=17-functions`). A separate fresh
`tools/report-m22-canary.ps1` measured 28/30 (93.3%) formatter functions,
`normalize_source` / `ERR:E-SH-IR-CALL-ARGUMENT`, full-source exit 122,
on the Windows x64 GNU runner with a 5,000,000,000 fuel cap. These are
internal fixtures, not self-host bootstrap or independent CI evidence.

Independent CI is pending. M22 remains **NO-GO**: the rest of
`normalize_source`, `main`, remaining selfhost sources, S5 byte-equal
codegen, S6 bootstrap and S7 exit audit remain open. M23 implementation
has neither Route A nor Route B.

## Later CI adjudication

STEP-0310 verified GitHub Windows GNU workflow run 36366432845 on cumulative SHA 8bee75b as completed/success. This adjudicates the committed snapshot only; M22 remains NO-GO.
