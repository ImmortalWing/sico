# STEP-0299: M22 W2 slice 7 — map.get nested key

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 7
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0298 advanced the formatter canary to 24/30 and left
`match_arm_levels` refusing as `ERR:E-SH-IR-CALL-SHAPE`. Its map.get match
subject uses `sico.u64.to_text(cursor)` as the key. The existing
`sico.map.get[Text,U64]` subject lowering accepted a single key atom only.

This slice accepts that one nested `u64.to_text` call with a U64 parameter
or local. It verifies that the nested call ends exactly at the outer map.get
close, preserving the existing single-atom path and typed refusal for extra
arguments. The key expression range now spans the nested close delimiter.
No language, WIT, Host or authority contract changes.

## Evidence and remaining frontier

- The real runner emits byte-exact Rust IR for the formatter prefix through
  `match_arm_levels`. A second `u64.to_text` argument and a third `map.get`
  argument refuse as `ERR:E-SH-IR-CALL-SHAPE`; the valid prefix compiles
  after these refusals in a fresh Store.
- The formatter prefix through `direct_close` (including `close_code`) is
  independently byte-exact against Rust IR. The canary's compilation count
  is **27/30**, and this separate differential verifies the expanded prefix.
- First uncovered function: `opener_close`, typed refusal
  `ERR:E-SH-IR-GWSKIP:SKIP:GENERAL-WHILE-IF-REGION`, full-source exit 122.
  Coverage grew by three functions, so R3 consecutive-stall count is **0**.
- `selfhost_local_bounds` passed; the busiest function remains below the
  reserved local headroom gate. `tools/validate-step-0261.ps1` and
  `0262.ps1` re-pin the current frontier while retaining the historical
  17- and 22-function snapshots.

- `tools/validate-step-0262.ps1`: compiler 31/31, parser 2/2, local bounds
  1/1, historical 22-function snapshot, 27-function prefix and f28 typed
  frontier passed; the runner test separately checks byte equality.
- `tools/validate-step-0261.ps1`: compiler 31/31, parser 2/2, local bounds
  1/1, historical 17-function snapshot and full-source typed frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 31, runner fmt and clippy passed.
- Root and runner `cargo fmt --all -- --check`,
  `tools/validate-step-0124.ps1`, and `git diff --check` passed.

Independent CI on this STEP's exact SHA is pending.
M22 remains **NO-GO**: three formatter functions, remaining selfhost source
coverage, milestone S5 byte-equal codegen, S6 bootstrap and S7 exit audit
remain open. `origin/dev` parallel history is still unmerged.
