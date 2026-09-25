# STEP-0293: M22 W2 S3/S4 — u64.to_text in a while-body RHS

> - status: implementation and local real-runner exit complete; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C S1
> - date: 2026-09-25
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and scope

STEP-0292 gives W1 GO for commit `e8495f9` on the isolated
`codex/m22-w1-ci` branch: [fresh Windows GNU CI run](https://github.com/ImmortalWing/sico/actions/runs/36087222284)
completed successfully. The frozen STEP-0282 W2 baseline is 22/30 formatter
functions, `nearest_match` / `ERR:E-SH-IR-GWPACK-OTHER`.

This first R3-counted implementation STEP adds the single-argument
`sico.u64.to_text` intrinsic to `gw_rhs_packed` by reusing the existing typed
`gw_intrinsic_call_packed` path. The helper's one-argument shape and local-cell
read logic remain unchanged. `sico.text.encode` and `sico.bytes.length` were
already in this path; neither is newly claimed here. No language contract,
WIT, or Host authority changes.

## Exit evidence

- New real-runner test compares a while-body `let key =
  sico.u64.to_text(cursor)` against the Rust oracle's canonical IR byte for
  byte. A two-argument mutation is typed-refused as
  `ERR:E-SH-IR-CALL-SHAPE`. Targeted test 1/1 passed.
- `tools/report-m22-canary.ps1` on the current tree: **22/30**, first uncovered
  function `nearest_match`, typed frontier **`ERR:E-SH-IR-CALL-TARGET`**, full
  source exit 122. This moves the refusal frontier while function coverage
  remains 22/30, as anticipated in STEP-0291.
- `tools/validate-step-0262.ps1`: compiler 22/22, parser 2/2, local bounds
  1/1, 22-function IR snapshot and current frontier all passed.
- `tools/validate-step-0261.ps1`: same adjacent runner suites, 17-function
  snapshot and current frontier passed.
- `tools/validate-step-0245.ps1`: first run stopped on a Rust formatting
  difference in the new test. After formatting that statement, the complete
  rerun passed: semantic 6+11, bundle 4, checker 9 (original 215 plus W1 five
  additions), compiler 22 and runner clippy.

## Gate accounting

S1's local shape exit and typed frontier movement are measured. The R3
consecutive-stall count remains **0** after this first implementation STEP.
The next frontier is the `sico.map.get[Text,U64]` match subject, the predeclared
22-C S2. M22 remains NO-GO. This STEP is not finally adjudicated until its
own pushed commit receives independent CI; STEP-0292's run covers the W1
snapshot only.

The current branch remains separate from `origin/dev`, whose parallel
STEP-0279–0283 content conflicts with this branch. No merge into `dev` or
cross-history capability claim follows from this local result.
