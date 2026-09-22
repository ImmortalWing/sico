# STEP-0256: M22 list.get result-match and nested intrinsic returns byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence (execution-queue items 1–2)
> - completed: 2026-09-22
> - evidence: internal-fixture, Windows x64 real runner (pinned rustc 1.98.0 MSVC host per the STEP-0254 environment note)

## Objective

Close the STEP-0253 `item` frontier (the M22 plan §3.2 execution queue items 1
and 2): lower the `sico.list.get` result-match region and the
`append_pair` nested-intrinsic return byte-exactly, advancing the formatter
prefix from thirteen to fifteen functions.

## Changes

- `parameters_ir` now accepts `List [ <scalar> ]` parameter declarations and
  emits `{"kind":"list","data":{"kind":<element>}}`; scalar shapes and their
  bytes are unchanged. A malformed element bracket is the existing typed
  `ERR:E-SH-IR-PARAMETERS` refusal.
- New `list_aware_parameter_index` walks declarations whose types may be
  list-shaped (the legacy `binding_parameter_index` stride assumes scalar
  types only).
- New `list_get_match_ir` lowers the declared slice
  `match sico.list.get(<param0: List[Text]>, <param1: U64>):` with
  `case ok(binding): return binding` / `case error(_): return <string literal
  or string parameter>` — the `intrinsic` subject instruction with
  `arguments:[0,1]`, the `#match0` cell, ok/error arms, the `project ok`
  block and the `const_string` (or parameter-read) error block, all with
  canonical ranges. Out-of-slice shapes keep typed refusals
  (`ERR:E-SH-IR-MATCH-SHAPE` / `-CALL-TARGET` / `-CALL-ARGUMENT` /
  `-PARAMETERS` / `-TYPE-MISMATCH`).
- New `nested_intrinsic_return_ir` lowers a single-statement body
  `return sico.list.append(<param or sico.list.append(param, param)>, <param>)`
  for the declared `(List[Text], Text, Text) returns List[Text]` signature:
  inner-then-outer `intrinsic` instructions with per-call spans and
  post-order value ids; no `locals` key (empty). Anything else starting with
  a non-whitelisted shape returns `SKIP` to the pre-existing return lowering,
  so no previously supported function changes bytes.
- `scalar_ir` routes a lone `match` statement whose subject starts with
  `sico` to the new list-get path, and tries the nested-intrinsic return
  before the plain-return tail.
- Validators: `validate-step-0252.ps1` / `validate-step-0253.ps1` canary
  pins relaxed to the typed fail-closed class (`ERR:E-SH-IR-` + exit 122, no
  trap) per the STEP-0250 precedent; the exact frontier is pinned by this
  step's `validate-step-0256.ps1`.

## Executed validation

- `sico_compiler` differential suite 15/15 green, serial, through the real
  runner; the `item` and `append_pair` prefix differentials compare
  byte-for-byte with Rust canonical IR, and the thirteen prior prefixes are
  unchanged.
- The refusal-recovery assertion stays green; the full formatter canary now
  stops at `line_tokens` with the typed `ERR:E-SH-IR-CONTROL` (multiple
  matches interleaved with while bodies), exit 122, no guest trap.
- `validate-step-0256.ps1` (serial differential + local-bounds + canary
  pin + `git diff --check`) passes; full runner-workspace regression run
  recorded with this step.

## Honest residuals

- `line_tokens` (while bodies with multiple matches) is the next unsupported
  region and a larger lift than this step; `source_has_lex_error` … `main`
  (fifteen functions) remain typed-refused.
- The list-get slice is deliberately narrow (two parameters, ok/error arms,
  literal or parameter error fallback); match subjects beyond
  `sico.list.get` and append-free nested calls are not claimed.
- S5 Component codegen widening, S6 `A == B == C` bootstrap and the M22
  exit audit remain open; M22 stays NO-GO.
