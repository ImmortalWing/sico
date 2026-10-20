# STEP-0304: M22 W2 slice 12 — user call as comparison left operand

> - status: implementation complete; independent CI success (STEP-0310 adjudication)
> - phase: M22 compiler self-host, execution card 22-C slice 12
> - date: 2026-09-27
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0303 left `normalize_source` at `ERR:E-SH-IR-EXPRESSION`. A temporary
diagnostic probe (removed after use) located the refusal in
`gw_compare_packed` on `U64.equal(close_code(close), expected_close)`:
the fixed comparison's left operand accepted cells, parameters and
literal constructors, but not a user call.

This slice lowers a declared user call in the comparison's left operand
using the existing bounded call-argument packer. It verifies the call's
parentheses and comma, emits argument instructions before the call,
uses the declared return kind and function ordinal, and locates the
right operand after the full nested call. The comparison still enforces
both operand types. An additional SSA repair advances the next result ID
by the emitted count for a `return` expression; returning a parameter
emits zero instructions and must not reset the ID counter to `param+1`.
No language, WIT, Host or authority contract changes.

## Evidence and remaining frontier

- A two-function `helper(value)` comparison and a formatter-prefix
  `close_code(close)` comparison both emit the Rust canonical IR bytes
  exactly in the real runner. The fixtures include a return from the if
  arm followed by a local write and final local read; these exposed the
  stale SSA ID after returning a parameter.
- A wrong-typed call argument refuses as `ERR:E-SH-IR-CALL-TYPE`; the
  valid source compiles afterward in a fresh Store.
- `normalize_source` now reaches `ERR:E-SH-IR-CALL-ARGUMENT` in a nested
  `repeat_indent(indent_level)` argument to `sico.text.concat`. The
  formatter canary remains **28/30**; the refusal site moved, leaving the
  R3 consecutive-stall count at **0**.

Local validation: `tools/validate-step-0304.ps1` passed (compiler 36/36,
parser 2/2, local-bounds 1/1, inherited STEP-0262 prefix/frontier probes).
`tools/validate-step-0261.ps1` separately passed the historical
17-function snapshot and the re-pinned current frontier.
An initial run exposed 241 locals in `general_while_function_ir` against
the 240-local headroom gate; inlining the newly introduced return-count
temporary restored 240 and the validator passed on a fresh run.
`tools/report-m22-canary.ps1` reported 28/30 (93.3%),
`normalize_source` / `ERR:E-SH-IR-CALL-ARGUMENT`, full-source exit 122.
Root and runner formatting checks, `tools/validate-step-0124.ps1`, and
`git diff --check` passed after applying rustfmt. The full workspace suite
was not run for this parser-only lowering slice. Independent CI is pending.
M22 remains **NO-GO**: the rest of
`normalize_source`, `main`, remaining selfhost sources, S5 byte-equal
codegen, S6 bootstrap and S7 exit audit remain open. M23 implementation
has neither Route A nor Route B.

## Later CI adjudication

STEP-0310 verified GitHub Windows GNU workflow run 36366432845 on cumulative SHA 8bee75b as completed/success. This adjudicates the committed snapshot only; M22 remains NO-GO.
