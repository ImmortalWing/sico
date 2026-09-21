# STEP-0251: M22 general while-body if/else regions byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence
> - completed: 2026-09-21
> - evidence: internal-fixture, Windows x64 GNU real runner, PowerShell 5.1 host

## Objective

Close the frontier recorded by STEP-0250: `scan_space` — the eighth formatter
function — needs general `if`/`else` statement regions inside a `while` body,
lowered byte-identically to the Rust `lower_core` general-CFG path.

## Changes

- Added `general_while_function_ir` to `selfhost/parser.sico`, dispatched
  from `scalar_ir` before `control_while_function_ir` whenever a function
  carries exactly one `while`, at least one `if`, and no `break`/`continue`
  (any other shape falls through to the frozen paths unchanged).  The
  builder implements the canonical lazy block discipline: `while` allocates
  header and body eagerly and leaves the exit block pending; `if` allocates
  then/else at the branch; `end if` allocates the join and seals the open
  arms; `end while` seals the backedge and allocates the exit; value ids
  stay globally sequential in source order.  Frames are append-only stacks
  (no `list.set` exists); terminators resolve at assembly through
  branch/return/seal registries.
- Supporting helpers: `gw_decimal_value` (digits to U64, the only way to
  bring numbers back across the Text-only packing idiom), `gw_instr_count`
  (brace-depth-aware top-level comma count — raw `comma_count` over-counts
  instruction JSONs that contain commas), `gw_compare_packed` (the shared
  `equal_fixed`/`less_fixed` condition machinery for both `while` and `if`
  headers, including operand kind checks against the receiver width), and
  `gw_rhs_packed` (the unified let/set/return right-hand side resolver over
  `sico.bytes.at`, dotted fixed-width literals, typed user calls and atoms,
  reusing `while_bytes_at_rhs_packed`, `fixed_literal_const_instruction`,
  `while_call_rhs_packed` and the atom resolution).
- Extended the differential: `sico_compiler_lowers_the_scan_space_region_
  byte_exactly` pins the formatter prefix through `scan_space` byte-exactly
  against the Rust oracle (eight functions).
- Completed the STEP-0250 contract on the strength of the new differential:
  `declared_return_kind` gains `Bytes` → `"bytes"` (Bytes return types
  now lower; the selfhost_parser suite asserts the previously-refused
  Bytes-parameter shape lowers while genuinely undeclared types such as
  `Widget` still refuse with `E-SH-IR-PARAMETER-TYPE`), and
  `validate-step-0250.ps1` moves to the typed fail-closed frontier policy
  introduced with the STEP-0249 batch.
- Re-measured the full-formatter canary frontier: scan_space and its
  nested-if region lower byte-exactly; the next refusal is
  `ERR:E-SH-IR-EXPRESSION` in `scan_ident` (straight-line `let`-RHS
  intrinsic calls remain unsupported on the `straight_let` path).
- Removed the debug scaffolding used during alignment; the committed
  builder carries only typed `E-SH-IR-*` refusals.  The oracle-dump probe `crates/sico-ir/tests/dump_oracle_ir.rs` lands as env-gated no-op alignment tooling (set ORACLE_SOURCE / ORACLE_OUT / ORACLE_FN to dump).

## Executed validation

- probe evidence through the real runner: the scan_space-shaped source and
  the whole formatter prefix through `scan_space` (4,635 bytes, ten blocks
  for `scan_space`, sequential value ids) compare byte-identical with the
  Rust `lower_core` canonical serialization;
- `sico_compiler` differential now 8/8 including the new scan_space region
  test; `selfhost_checker` 8/8, `selfhost_parser` 2/2, `bootstrap_bundle`,
  `list_append_cow` and `selfhost_local_bounds` stay green;
- `validate-step-0251.ps1` green (markers, both self-host component builds,
  byte-parity suites, canary pinned at the new typed boundary, no trap);
- `git diff --check` passes.

## Honest residuals

- Straight-line `let`-RHS intrinsic calls (e.g. `sico.bytes.at` outside a
  `while` body) still refuse with `E-SH-IR-EXPRESSION`; the general builder
  intentionally refuses `break`/`continue`-carrying bodies (those stay on
  the frozen STEP-0240 paths), nested `while`s, and if-without-else.
- S5 general deterministic Component codegen, S6 A=B=C bootstrap packaging
  and budget gates, and S7 exit audit remain open.  M22 therefore remains
  NO-GO.
