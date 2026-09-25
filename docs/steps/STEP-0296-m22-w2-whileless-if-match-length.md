# STEP-0296: M22 W2 S4 — while-less if/match and nested text length

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C S4
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0295 advanced the formatter canary through `nearest_match` (23/30),
and independent Windows GNU CI succeeded on `f7288b7` (run 36166445685).
`set_nearest_match` was the next typed frontier at `ERR:E-SH-IR-STATEMENT`.
The first refusal was the scalar-IR dispatcher: it sent functions with an
`if` and a `match`, but no `while`, away from the already established
general control-flow lowering. Routing that shape through the general
machine makes a minimal while-less if/match function byte-equal to the Rust
IR oracle. `parser.scalar_ir` grows from 161 to 162 locals; the busiest
link-unit function remains `general_while_function_ir` at 240 of 256.

The next refusal was a nested `sico.text.length(Text)` on the right of
`U64.less_than` in an `if` condition. The comparison lowering now emits the
local read (when needed), `sico.text.length`, then the fixed comparison in
canonical order and with Rust-identical source ranges. The declared argument
shape is a single Text parameter or local. Extra and missing arguments refuse
as `ERR:E-SH-IR-CALL-SHAPE`; a U64 parameter or local refuses as
`ERR:E-SH-IR-CALL-TYPE`. A valid input after these refusals verifies a fresh
Store. No language, WIT, Host or authority contract changes.

## Evidence and remaining frontier

- The real runner emits byte-exact Rust IR for a while-less if/match function
  with a plain U64 condition, a Text parameter passed to `text.length`, and a
  Text local passed to `text.length`.
- The formatter prefix through `set_nearest_match` still refuses with
  `ERR:E-SH-IR-CALL-SHAPE`: the next unsupported shape is the `bytes.slice`
  match subject with a nested `sub(bytes.length(...), U64.literal(1))` length
  argument. This is a typed boundary, not coverage of function 24.
- Formatter canary: **23/30**, first uncovered `set_nearest_match`,
  `ERR:E-SH-IR-CALL-SHAPE`, full-source exit 122. The refusal moved from
  `STATEMENT` without canary growth, so R3 consecutive-stall count stays **0**.
- `tools/validate-step-0262.ps1`: compiler 26/26, parser 2/2, local bounds
  1/1, historical 22-function snapshot, 23-function prefix and f24 typed
  frontier passed.
- `tools/validate-step-0261.ps1`: compiler 26/26, parser 2/2, local bounds
  1/1, historical 17-function snapshot and full-source typed frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 26, runner fmt and clippy passed. The
  first local run found only a Rust formatting difference in the new test;
  runner fmt repaired it and the full validator passed on rerun.
- Root `cargo fmt --all -- --check`, `tools/validate-step-0124.ps1`, and
  `git diff --check` passed.

S4 independent CI on its own exact SHA is recorded after completion. M22
remains **NO-GO**: seven formatter
functions, remaining selfhost source coverage, S5 codegen, S6 bootstrap and
S7 exit audit are still open.
