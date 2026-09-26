# STEP-0300: M22 W2 slice 8 — while inside an if arm

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 8
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0299 advanced formatter compilation coverage to 27/30 with a
byte-exact prefix through `direct_close`. `opener_close` then refused as
`ERR:E-SH-IR-GWSKIP:SKIP:GENERAL-WHILE-IF-REGION`: its `while` sits inside
the `then` arm of an `if`.

The general control-flow machine now permits a `while` in that `then`
region. Its while frame records the entering if-region and if-depth before
the region becomes the loop header/body; `end while` checks the depth and
restores the parent region. The existing top-level and nested-while paths
retain their block identities. A redundant condition-index local was
inlined so `general_while_function_ir` remains within the reserved
240/256 local-variable headroom gate. `else` and join-region loop entry
remain typed refusals. No language, WIT, Host or authority contract changes.

## Evidence and remaining frontier

- The real runner emits byte-exact Rust IR for the complete formatter prefix
  through `opener_close`, including the nested loop and all 16 canonical
  blocks in that function. The earlier 27-function prefix stays covered.
- The prefix through `normalize_source` refuses as
  `ERR:E-SH-IR-GWPACK-OTHER`; the valid `opener_close` prefix compiles in a
  fresh Store after that refusal.
- Formatter compilation canary: **28/30**, first uncovered
  `normalize_source`, `ERR:E-SH-IR-GWPACK-OTHER`, full-source exit 122.
  Coverage grew, so R3 consecutive-stall count remains **0**.
- `tools/validate-step-0261.ps1` and `0262.ps1` re-pin the next typed
  frontier while retaining historical 17- and 22-function snapshots.

- `tools/validate-step-0262.ps1`: compiler 32/32, parser 2/2, local bounds
  1/1, historical 22-function snapshot, 28-function prefix and f29 typed
  frontier passed; the runner test separately checks byte equality.
- `tools/validate-step-0261.ps1`: compiler 32/32, parser 2/2, local bounds
  1/1, historical 17-function snapshot and full-source typed frontier passed.
- `tools/validate-step-0245.ps1`: semantic 6+11, bundle 4, checker 9
  (frozen 215 plus W1 five), compiler 32, runner fmt and clippy passed.
- Root and runner `cargo fmt --all -- --check`,
  `tools/validate-step-0124.ps1`, and `git diff --check` passed.

Independent CI on this STEP's exact SHA is pending.
M22 remains **NO-GO**: two formatter functions, remaining selfhost source
coverage, milestone S5 byte-equal codegen, S6 bootstrap and S7 exit audit
remain open. `origin/dev` parallel history is still unmerged.
