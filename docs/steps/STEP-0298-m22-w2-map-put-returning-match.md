# STEP-0298: M22 W2 slice 6 — map.put return and returning match arms

> - status: complete / slice 6 GO on isolated branch
> - phase: M22 compiler self-host, execution card 22-C slice 6 (lowering, not milestone S6 bootstrap)
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0297 left the formatter canary at 23/30 with `set_nearest_match`
refusing as `ERR:E-SH-IR-EXPRESSION` at its `map.put` return. This slice
lowers the closed `sico.map.put[Text,U64](Map[Text,U64] atom,
Text expression, U64 atom)` return shape. The Text expression may be the
single-Bytes-atom `sico.bytes.utf8_decode` intrinsic. Generic type arguments,
argument count and operand kinds are checked before emission; malformed inputs
retain typed refusals. This adds no language, WIT, Host or authority contract.

The same function has a match whose `ok` and `error` arms both return. The
general control-flow machine now records whether the ok arm terminated and
does not attach a jump from either returning arm to the match join. It still
emits the canonical join block required by the Rust IR, which connects to the
enclosing `if` join. Existing non-returning match arms keep their jumps.

## Evidence and remaining frontier

- A real-runner `map.put` return fixture is byte-exact against the Rust IR.
  A fourth argument refuses as `ERR:E-SH-IR-CALL-SHAPE`; a wrong Map,
  Bytes or U64 operand refuses as `ERR:E-SH-IR-CALL-TYPE`. The valid fixture
  compiles again after the refusal sequence in a fresh Store.
- The complete formatter prefix through `set_nearest_match` is byte-exact
  against the Rust IR, including the two returning match arms and outer if.
- Formatter canary: **24/30**, first uncovered `match_arm_levels`,
  `ERR:E-SH-IR-CALL-SHAPE`, full-source exit 122. Canary coverage grew,
  so R3 consecutive-stall count remains **0**.
- The busiest link-unit function is `general_while_function_ir` at 240/256
  locals. The first implementation briefly reached 242 and failed the local
  headroom gate; removing two redundant locals restored the gate before
  this STEP's validation.
- `tools/validate-step-0261.ps1` and `0262.ps1` re-pin the new typed
  frontier while retaining the historical 17- and 22-function snapshots.

- `tools/validate-step-0262.ps1`: compiler 28/28, parser 2/2, local bounds
  1/1, historical 22-function snapshot, 24-function prefix and f25 typed
  frontier passed.
- `tools/validate-step-0261.ps1`: compiler 28/28, parser 2/2, local bounds
  1/1, historical 17-function snapshot and full-source typed frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 28, runner fmt and clippy passed.
- Root and runner `cargo fmt --all -- --check`,
  `tools/validate-step-0124.ps1`, and `git diff --check` passed.

Independent GitHub Actions Windows GNU run
[36210200271](https://github.com/ImmortalWing/sico/actions/runs/36210200271)
completed successfully on the exact STEP-0298 commit
`5ba175fb09f9c61aa5a0314af815dce6e39dd540`. Slice 6 is GO on the
isolated branch.
M22 remains **NO-GO**: six formatter functions, remaining selfhost source
coverage, milestone S5 byte-equal codegen, S6 bootstrap and S7 exit audit
remain open. `origin/dev` parallel history is still unmerged.
