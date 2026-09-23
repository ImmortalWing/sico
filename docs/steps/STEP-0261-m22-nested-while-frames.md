# STEP-0261: M22 nested-while stack frames — `source_has_lex_error` byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence (execution-queue item: nested while, `format_code` region prefix)
> - completed: 2026-09-23
> - evidence: internal-fixture, Windows x64 real runner (pinned rustc 1.98.0 GNU host with repo-local MSYS2 binutils — the evidence machine changed on 2026-09-23 and no longer has MSVC; see the host note below)

## Objective

Close the STEP-0260 frontier (`SKIP:GENERAL-WHILE-NESTED`): replace the
general-while machine's single-slot while frame (`ghdr_id`/`gbody_id`/
`gafter_id` globals) with a packed frame stack so nested and sequential
whiles lower byte-exactly, and land the call-surface shapes the first
nested-while function (`source_has_lex_error`) needs: `sico.list.length`
on parameters/cells, `sico.text.split_lines` on non-literal subjects, and
user calls nested as call arguments. The formatter prefix advances from
sixteen to seventeen functions.

## Changes

- `general_while_function_ir` while frames: a `gwh_frames` text stack
  (packed `header\nbody\ncond\nstart\nend\nbrk_base` per frame) with a
  `gwh_len` pop counter replaces the single header/body/after slots.
  While-open is legal in `entry`, `body` (nested while) and `after`
  (sequential while) regions; a while opened inside an if-arm
  (`then`/`else`/`join`) is the new declared refusal
  `SKIP:GENERAL-WHILE-IF-REGION` (canary territory for `opener_close`/
  `normalize_source`). `ghdr_id`/`gwhile_start`/`gwhile_end` remain as
  top-of-stack mirrors restored at each close, so `continue` keeps sealing
  to the innermost header.
- `end while` pops the top frame: seals the current block to the frame
  header, allocates the after block at the emission frontier (matching the
  Rust `sort_by_key((never_current, became_current, creation))` layout —
  stamped after-blocks order by flow, empty ones trail), seals only the
  breaks recorded since this frame opened (`brk_base` scoping), appends the
  branch terminator entry `header\ncond\nbody\nafter` to `gwt_entries`, and
  restores `body` (outer frame alive) or `after` (stack empty) as the
  region.
- Terminator assembly replaces the single `gbi == ghdr_id` branch emission
  with a scan over `gwt_entries` (same priority position: before match/if/
  return/jump lookups), so every while level emits its canonical branch.
- `gw_rhs_packed` `sico.list.length`: routed through
  `gw_intrinsic_call_packed` (`u64` result) at the empty fixed-op-name
  gate, covering parameter and cell subjects.
- `gw_rhs_packed` `sico.text.split_lines`: non-literal single arguments
  now route through `gw_intrinsic_call_packed` with the canonical
  list-of-string result JSON (parameter subjects emit the intrinsic with
  the parameter operand directly; cell subjects emit `read_local` +
  `intrinsic`), replacing the string-literal-only gate that returned
  `ERR:E-SH-IR-EXPRESSION`.
- `while_call_rhs_packed` nested user-call arguments: an argument segment
  that is itself a declared-function call (callee found, parens matched to
  the segment end) lowers through a recursive `while_call_rhs_packed` pack
  with the segment's own expression span, then emits the inner `call` with
  `function_declaration_ordinal`, the packed argument values, and the
  inner call's exact source range; the segment width/type checks treat the
  nested call as one operand. The call tail check now accepts a close
  paren followed by line end, `)` or `,` (a trailing comma/paren tail is
  not parseable at statement level, so the relaxation cannot bypass a
  typed refusal).
- Local budget: the machine lands at exactly 240 locals (limit 256,
  bounds test requires busiest + 16 ≤ 256); the frame stack cost was paid
  by dropping the `gbody_id` slot, reusing `gwf` for frame/terminator
  part lists and `gbsi` as the terminator scan index.
- Housekeeping: `selfhost_local_bounds` now always prints the busiest
  function plus the top-5 local counts (`-- --nocapture`) so frontier work
  sees its real headroom; `cargo fmt --all` repaired a pre-existing import
  ordering in `crates/sico-ir/tests/dump_oracle_ir.rs` (STEP-0251-era
  temporary diagnostic; the workspace fmt gate is green again).

## Executed validation

- `tools/validate-step-0261.ps1` green end-to-end on this host: marker
  checks, absence of the retired `SKIP:GENERAL-WHILE-NESTED` guard,
  `sico-cli` build, `selfhost_compiler` 19/19 (new
  `sico_compiler_lowers_the_source_has_lex_error_region_byte_exactly`),
  `selfhost_parser` 2/2 (99-fixture differential), `selfhost_local_bounds`
  1/1, guest bootstrap build, seventeen-function prefix byte-compare
  against the frozen `tools/fixtures/step-0261/shle_expected.json`
  (117,109 bytes, verified byte-equal to the Rust `lower_core` +
  `canonical_json` oracle), `call_left` frontier probe
  (`ERR:E-SH-IR-EXPRESSION`), full-formatter canary exit 122 without a
  trap, `git diff --check`.
- `cargo fmt --all -- --check` and `cargo clippy --locked --offline`
  (runner workspace, all targets) pass.

## Honest residuals

- The formatter canary now stops at `call_left` (`ERR:E-SH-IR-EXPRESSION`):
  the guard-chain machine is arity-1-only and has no set-arms, bare-cell
  conditions, or nested guard chains; `format_code` additionally needs
  user-call conditions on body `if`s in the general-while machine
  (`ERR:E-SH-IR-CALL-ARGUMENT` on the nested-call condition probe). Both
  are the next execution-queue increments; no partial lowering was emitted
  for them.
- A while opened inside an if-arm is refused
  (`ERR:E-SH-IR-GWSKIP:SKIP:GENERAL-WHILE-IF-REGION`); `opener_close` and
  `normalize_source` sit behind it.
- S5 general Component codegen, S6 A=B=C/bootstrap packaging and budget
  gates, and the S7 exit audit remain open. M22 therefore remains NO-GO.

## Host note (environment repair, not a product change)

The STEP-0260 manuscript pinned MSVC because that machine's artifact cache
was MSVC-built. The 2026-09-23 evidence machine has no Visual Studio and
builds under `1.98.0-x86_64-pc-windows-gnu` with
`target/tooling/msys2-binutils/mingw64/bin` (gcc for `ring`, dlltool) on
PATH — the same provisioning `tools/run-ci.ps1` applies.
`tools/validate-step-0261.ps1` pins the GNU flow accordingly.
