# STEP-0303: M22 W2 slice 11 — local map.get match subject

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 11
> - date: 2026-09-27
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0302 left the `normalize_source` prefix at
`ERR:E-SH-IR-CALL-ARGUMENT`. An instrumented local probe identified the
refusal in `gw_map_get_subject_packed`: the map argument was accepted only
as a function parameter, while `normalize_source` supplies the local
`blocks` map. The probe instrumentation was removed after identifying the
site.

This slice admits a `Map[Text,U64]` local as the first `map.get` argument.
It verifies the local's exact map type, emits a canonical `read_local`
before evaluating the key, and advances SSA IDs so nested key expressions
retain evaluation order. Unknown bindings and wrong types remain typed
refusals. The language, WIT, Host, and authority contracts do not change.

## Evidence and remaining frontier

- A real-runner fixture with a local map and Text key emits exactly the
  Rust canonical IR bytes. A nested `sico.u64.to_text(cursor)` key also
  passes byte equality, checking the map-read/key instruction order.
- A wrong-typed map local refuses as `ERR:E-SH-IR-CALL-TYPE`, after which
  the valid fixture compiles in a fresh Store.
- The formatter canary remains **28/30**. The first uncovered function
  remains `normalize_source`, with its refusal moved from
  `ERR:E-SH-IR-CALL-ARGUMENT` to `ERR:E-SH-IR-EXPRESSION`; R3 consecutive
  stall count remains **0**. Historical prefix snapshots remain pinned.

Local validation: `tools/validate-step-0303.ps1` passed (compiler 35/35,
parser 2/2, local-bounds 1/1, inherited STEP-0262 prefix/frontier probes);
`tools/validate-step-0261.ps1` passed its historical 17-function prefix
and current frontier probes. `tools/report-m22-canary.ps1` independently
reported 28/30 (93.3%), `normalize_source` /
`ERR:E-SH-IR-EXPRESSION`, full-source exit 122. Root and runner
`cargo fmt --all -- --check`, `tools/validate-step-0124.ps1`, and
`git diff --check` passed. The full workspace suite was not run for this
parser-only lowering slice. Independent CI is pending. M22 remains **NO-GO**: the rest of
`normalize_source`, `main`, remaining selfhost sources, S5 byte-equal
codegen, S6 bootstrap, and S7 exit audit remain open. M23 implementation
has neither Route A nor Route B.
