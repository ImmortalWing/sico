# STEP-0297: M22 W2 S5 — nested bytes.slice match subject

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C S5 (a lowering slice, not milestone S5 codegen)
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0296 left the formatter canary at 23/30 with `set_nearest_match` refusing
as `ERR:E-SH-IR-CALL-SHAPE`. The first uncovered shape was a `bytes.slice`
match subject whose length is `sub(sico.bytes.length(bytes), U64.literal(n))`.
The existing slice lowering covered `sub` of two single atoms; its dispatch
must remain available for the historical formatter functions.

This slice adds a separate, bounded subject lowering for a Bytes parameter or
local, a U64 literal start, and `sub(bytes.length(Bytes parameter or local),
U64 literal)`. It checks the closed argument counts, the `sub` signature and
the operand kinds, then emits the canonical read/constant/intrinsic/call
sequence with source ranges matching the Rust oracle. The old slice path is
selected for all other third arguments. No language, WIT, Host or authority
contract changes.

## Evidence and remaining frontier

- The real runner emits byte-exact Rust IR for a minimal if/match function
  with the nested slice subject. A fourth slice argument, a second
  `bytes.length` argument and a third `sub` argument refuse as
  `ERR:E-SH-IR-CALL-SHAPE`; a Text operand refuses as
  `ERR:E-SH-IR-CALL-TYPE`. The valid source compiles after the refusal
  sequence in a fresh Store.
- The formatter prefix through `set_nearest_match` still refuses, now at the
  `sico.map.put[Text,U64]` return expression with
  `ERR:E-SH-IR-EXPRESSION`. This function is not yet covered.
- Formatter canary: **23/30**, first uncovered `set_nearest_match`,
  `ERR:E-SH-IR-EXPRESSION`, full-source exit 122. The typed frontier moved
  from `CALL-SHAPE`, so R3 consecutive-stall count remains **0**.
- The busiest link-unit function remains `general_while_function_ir` at
  240/256 locals; the new helper does not consume its local headroom.
- `tools/validate-step-0261.ps1` and `0262.ps1` re-pin the next typed
  frontier; historical 17- and 22-function snapshots remain unchanged.

- `tools/validate-step-0262.ps1`: compiler 27/27, parser 2/2, local bounds
  1/1, historical 22-function snapshot, 23-function prefix and f24 typed
  frontier passed.
- `tools/validate-step-0261.ps1`: compiler 27/27, parser 2/2, local bounds
  1/1, historical 17-function snapshot and full-source typed frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 27, runner fmt and clippy passed.
- Root and runner `cargo fmt --all -- --check`,
  `tools/validate-step-0124.ps1`, and `git diff --check` passed.

Independent CI on this STEP's exact SHA is pending.
M22 remains **NO-GO**: seven formatter functions, remaining selfhost source
coverage, milestone S5 byte-equal codegen, S6 bootstrap and S7 exit audit
remain open.
