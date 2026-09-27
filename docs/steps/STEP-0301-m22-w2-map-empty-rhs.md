# STEP-0301: M22 W2 slice 9 — map.empty local RHS

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 9
> - date: 2026-09-26
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0300 advanced formatter compilation coverage to 28/30. The first
unsupported RHS in `normalize_source` was
`let blocks = sico.map.empty[Text,U64]()`, refused as
`ERR:E-SH-IR-GWPACK-OTHER`. The same function declares another empty
`Map[Text,U64]` local.

This slice lowers only the zero-argument `sico.map.empty[Text,U64]()`
expression in the general RHS path. It verifies the closed generic types,
parentheses and absence of trailing tokens, then emits the canonical map
type and intrinsic instruction. No language, WIT, Host or authority contract
changes.

## Evidence and remaining frontier

- A real-runner fixture assigning an empty map local and returning it is
  byte-exact against Rust IR. Wrong generic type refuses as
  `ERR:E-SH-IR-CALL-TYPE`; an extra argument or trailing token refuses as
  `ERR:E-SH-IR-CALL-SHAPE`. The valid fixture compiles again after these
  refusals in a fresh Store.
- Formatter compilation canary remains **28/30**; the first uncovered
  function remains `normalize_source`, but the refusal moves from
  `ERR:E-SH-IR-GWPACK-OTHER` to
  `ERR:E-SH-IR-GWSKIP:SKIP:GENERAL-WHILE-IF-REGION`. Full-source exit is
  122. The typed frontier moved, so R3 consecutive-stall count remains **0**.
- `selfhost_local_bounds` passed. `tools/validate-step-0261.ps1` and
  `0262.ps1` re-pin the new typed frontier while retaining historical
  17- and 22-function snapshots.

Local validator and independent CI results are recorded after execution.
M22 remains **NO-GO**: two formatter functions, remaining selfhost source
coverage, milestone S5 byte-equal codegen, S6 bootstrap and S7 exit audit
remain open. `origin/dev` parallel history is still unmerged.
