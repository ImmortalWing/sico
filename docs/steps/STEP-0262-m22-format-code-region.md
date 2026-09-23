# STEP-0262: M22 format_code region — `call_left`/`format_code`/`repeat_indent` byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence (execution-queue item: formatter tail, `call_left` → `repeat_indent`)
> - completed: 2026-09-23
> - evidence: internal-fixture, Windows x64 real runner (pinned rustc 1.98.0 GNU host with repo-local MSYS2 binutils)

## Objective

Close the STEP-0261 frontier (`call_left`, `ERR:E-SH-IR-EXPRESSION`): lower the
three functions between `source_has_lex_error` and the `Map[Text,U64]` region —
`call_left` (two parameters, no-else ifs with set arms, a bare-cell `if left:`
condition, a nested guard chain inside the then-arm and a user-call tail),
`format_code` (single while over a four-level if/else tree with user-call
conditions, `sico.text.concat` intrinsics including a nested concat argument)
and `repeat_indent` (single while with a concat of a string literal) —
byte-exactly, advancing the formatter prefix from seventeen to twenty-two
functions.

## Changes

- **`sico.text.concat` intrinsic RHS**: new `gw_text_concat_packed` lowers
  two-argument concats with parameter/cell/string-literal operands and
  recursive nested `concat(concat(...), ...)` arguments; the inner call emits
  with its own exact source span; the outer intrinsic uses the full call span
  (matching the Rust oracle's `read_local`/`const_string`/`intrinsic`
  sequences). The tail check accepts a close paren followed by line end, `)`
  or `,` (parse-level safety identical to STEP-0261).
- **Bare-cell if conditions**: `gw_condition_packed` recognizes a condition
  word that resolves to a `bool` cell and emits `read_local` over the atom
  span (`if left:` in `call_left`).
- **Entry-region control flow**: the general-while machine now accepts `if`
  statements and `return` statements in the entry region (the retired
  `SKIP:GENERAL-WHILE-ENTRYIF` refusal and the entry-return guard are gone);
  the final region check accepts `entry`/`join` in addition to `after`, and
  the `gseen_while` requirement (`SKIP:GENERAL-WHILE-NOWHILE`) is retired —
  the machine is now a general CFG lowering for while-less functions too.
- **While-less routing**: the dispatcher gains a last-resort
  `general_while_function_ir` attempt after `nested_intrinsic_return_ir`,
  gated on `if_count > 0 || set_count > 0` so pure single-expression returns
  keep their existing tail-serializer path (frozen refusal identities for
  `identity`/`a + b`/unknown-atom fixtures are byte-stable). `control_if`
  declines with `SKIP:CONTROL-IF-DECLINED` when it would have returned
  `ERR:E-SH-IR-EXPRESSION`; the straight-line machines (`straight_cell`,
  `straight_environment`) keep their hard refusals but are now gated on
  `if_count == 0` so if-bearing bodies never enter them.
- **Outermost no-else region**: closing the outermost if frame (`gdepth == 1`)
  sets the region to `join` instead of the parent arm, so statements after an
  entry-level `end if` lower with the correct final region; while-body
  behavior is unchanged (set/let/return/end-while accept both region classes).
- **Deferred empty-else ordering**: trailing never-stamped else blocks are
  numbered by their owning `if`'s source position (`gdefer_rs`, unique per
  `if` keyword) instead of close order — matching the Rust oracle, which
  creates the else block at open time and orders never-stamped blocks by
  creation. `call_left`'s nested-frame case (outer `if left:` else preceding
  the inner guards' elses) is the first exerciser; the branch targets in the
  terminator assembly use the same rank.

## Executed validation

- `tools/validate-step-0262.ps1` green end-to-end: marker checks, absence of
  the retired entry-if/nowhile guards, `sico-cli` build, `selfhost_compiler`
  20/20 (new `sico_compiler_lowers_the_format_code_region_byte_exactly`),
  `selfhost_parser` 2/2 (99-fixture differential with frozen refusal
  identities — the `identity`, `a + b` and unknown-bare-let-atom pins were
  regressed by intermediate fallback designs and are green at the final
  gating), `selfhost_local_bounds` 1/1, guest bootstrap build,
  twenty-two-function prefix byte-compare against the frozen
  `tools/fixtures/step-0262/fc_expected.json` (142,568 bytes, verified
  byte-equal to the Rust `lower_core` + `canonical_json` oracle), the
  `nearest_match` frontier probe (`ERR:E-SH-IR-UNRESOLVED`), full-formatter
  canary exit 122 without a trap, `git diff --check`.
- `cargo fmt --all -- --check` and `cargo clippy --locked --offline` (runner
  workspace, all targets) pass with zero warnings.

## Honest residuals

- The formatter canary now stops at `nearest_match`
  (`ERR:E-SH-IR-UNRESOLVED`): `Map[Text,U64]` parameters are outside every
  parameter-kind surface (`scalar_type_kind` rejects Map). The
  `nearest_match`/`set_nearest_match`/`match_arm_levels` region is the next
  bounded extension, followed by `close_code`/`direct_close`/`opener_close`
  (`opener_close` is the first while-inside-if function and will exercise
  `SKIP:GENERAL-WHILE-IF-REGION`), `normalize_source` and `main`.
- S5 general Component codegen, S6 A=B=C/bootstrap packaging and budget
  gates, and the S7 exit audit remain open. M22 therefore remains NO-GO.
