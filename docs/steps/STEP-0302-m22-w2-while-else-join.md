# STEP-0302: M22 W2 slice 10 — while in else and join regions

> - status: implementation complete locally; independent CI pending
> - phase: M22 compiler self-host, execution card 22-C slice 10
> - date: 2026-09-27
> - evidence class: internal-fixture, Windows x64 GNU real runner

## Entry and bounded shape

STEP-0301 left the `normalize_source` prefix at
`ERR:E-SH-IR-GWSKIP:SKIP:GENERAL-WHILE-IF-REGION` with 28/30 formatter
functions compilable. The general control-flow machine already stores the
entering region and if-depth in each while frame and restores them on
`end while`, but admitted only an if `then` arm. This slice admits `else`
and post-if `join` regions through the same frame path. It does not widen
the language, WIT, Host, or authority contracts.

## Evidence and remaining frontier

- Two bounded fixtures place a while in an `else` arm and after an `end if`;
  both emit the Rust canonical IR bytes exactly through the real runner.
- The `normalize_source` prefix now refuses with
  `ERR:E-SH-IR-CALL-ARGUMENT`. A following known-valid prefix still
  compiles in a fresh Store. Compilation coverage remains **28/30**; the
  typed frontier moved, so the R3 consecutive-stall count remains **0**.
- STEP-0261/0262 canaries are re-pinned to the new frontier while preserving
  their historical prefix snapshots.

Local validation: `tools/validate-step-0302.ps1` passed (compiler 34/34,
parser 2/2, local-bounds 1/1, inherited STEP-0262 prefix/frontier probes);
`tools/validate-step-0261.ps1` passed with the historical 17-function
snapshot and the re-pinned current frontier. Root `cargo fmt --all --
--check`, `tools/validate-step-0124.ps1`, and `git diff --check` passed.
`tools/report-m22-canary.ps1` reported 28/30 (93.3%),
`normalize_source` / `ERR:E-SH-IR-CALL-ARGUMENT`, full-source exit 122.
The full workspace suite was not run for this parser-only lowering slice.
Independent CI is pending. M22 remains **NO-GO**: the rest of `normalize_source`, `main`,
remaining selfhost sources, S5 byte-equal codegen, S6 bootstrap, and S7
exit audit remain open. M23 implementation has neither Route A nor Route B.
