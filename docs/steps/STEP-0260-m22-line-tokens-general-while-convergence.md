# STEP-0260: M22 line_tokens general-while convergence byte-exact

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence (execution-queue item: `line_tokens`)
> - completed: 2026-09-23
> - evidence: internal-fixture, Windows x64 real runner (pinned rustc 1.98.0 MSVC host per the STEP-0254 environment note)

## Objective

Close the STEP-0256 frontier: lower `line_tokens` (while body interleaving a
32-branch if/else chain, two `sico.bytes.slice` matches with ok-arm bindings,
list-returning calls and nested `sico.bytes.utf8_decode` arguments)
byte-exactly, advancing the formatter prefix from fifteen to sixteen
functions, and converge the general-while machine on the committed
`selfhost_parser` differential shapes (`I64.bit_and` set RHS, `break`,
`continue`).

## Changes

- `gw_condition_packed` general call conditions: user-function conditions
  with arguments that are parameters, cells or constants are lowered through
  `while_call_rhs_packed` (mirroring the RHS arg packing), emitting the
  callee `call` with canonical operand-read/const instructions. The
  single-argument `sico.bytes.at` condition slice is unchanged; arity and
  parameter-kind checks now guard only that slice. Non-bool callees and
  unsupported argument shapes keep typed refusals.
- `gw_slice_subject_packed` comma scan no longer stops after the first
  comma, so three-argument `sico.bytes.slice(src, start, sub(finish, start))`
  subjects lower as before the scan regression.
- Match ok-arm bindings are registered as `#match<binding>` cells (was
  `#<binding>`, which the Rust canonical locals name); `local_binding_index`
  resolves a plain reference to the most recent `#match`-prefixed cell, so
  arms can reference their bindings; locals typing distinguishes subject
  cells by `cell_kinds == "result"` rather than by name prefix.
- `List[<scalar>]` type surface: `declared_return_kind`,
  `binding_parameter_kind` and `parameter_kind_by_id` return the canonical
  `{"kind":"list","data":{"kind":<element>}}` type JSON and stride over list
  parameters; `binding_parameter_index` strides list declarations too.
  Instruction JSON emitters (`read_local`, `call`, `intrinsic`) and the
  general-while function header (`return_type`, `locals`) embed a kind that
  already starts with `{` verbatim (`type_json`), so list cells and calls
  round-trip byte-exactly.
- `gw_rhs_packed` fixed-operation RHS: `I64`/`U64` `bit_and`/`bit_or`/
  `bit_xor`/`shl`/`shr` calls with cell/parameter/literal operands lower
  through `fixed_operation_instruction_json` in the general-while machine
  (previously only the legacy `control_while` machine accepted them).
- `general_while_function_ir` `break`/`continue`: both statements are
  accepted inside the single while body (any body/then/else/join region) and
  rejected in entry/after regions or after a terminator (`E-SH-IR-STATEMENT`
  or the existing region guards). `continue` seals its block to the header
  block immediately; `break` blocks are sealed to the after block at
  `end while`; both compose with the deferred end-if machinery. Statement
  must be the sole token on its line. Unreachable code after either keeps
  the existing typed refusal.
- Locals discipline: the general-while machine approached
  `MAX_LOCALS_PER_FUNCTION` (256); 25 single-use `let <x>_ok = I64.literal(1)`
  markers were replaced by one shared `gok` cell via `set`, and the break/
  continue handlers share one locals set. No behavioural change.
- All diagnostic line-number suffixes (`-L<n>`, plus the round-2
  `-E1`/`-D0`/`-RS` experiment tags) were stripped; refusal identity is
  byte-stable again.

## Executed validation

- `line_tokens` differential: the sixteen-function formatter prefix
  (through `line_tokens`) is byte-identical to the Rust canonical IR dump
  (`lt_expected.json`), 52 blocks including both match regions; full JSON
  equality, not only per-function.
- `selfhost_compiler` differential suite 16/16 green, serial; the
  `scan_space`/`scan_ident`/`scan_integer`/`scan_comment`/`scan_string`/
  `punctuation_kind`/`item`/`append_pair`/`formatter-intrinsic-prefix`
  regions are unchanged and the append-pair/guard-chain byte-exactness holds.
- `selfhost_parser` 2/2 green: the 99-case scalar/return/match differential
  is byte-identical to Rust lowering (includes `I64.bit_and` in a while set
  RHS, `break` and `continue` in if-inside-while shapes) and the
  noncanonical-input refusal identity assertion passes.
- `selfhost_checker` 8/8, `selfhost_formatter` 1/1, `selfhost_local_bounds`
  1/1, `selfhost_declaration_parser` 2/2, `selfhost_lexer` 2/2,
  `selfhost_tokens` 3/3, `control_flow`, `bit_ops`, `checked_mul_div`,
  `error_propagation`, `for_loop`, `language_batch2`, `list_append_cow`,
  `list_i64_family`, `map_set`, `recursion`, `stdlib_batch2`,
  `stream_sum_pilot`, `stream_transform`, `block_solver`, `prng_xorshift`,
  `bootstrap_bundle`, `capability_state_machine`, `runner`,
  `package_builder`, `api_agent_pilot`, `vision_package`, `http2` — all
  green on the pinned MSVC toolchain, serial.
- Hand-run regression matrix (`scan_space` and the woe/dep/m47/m48/m66/
  chop3 structural canaries) all exit 0 on the rebuilt self-host compiler.
- `validate-step-0260.ps1` (marker pins + serial differential + local
  bounds + full-formatter canary + `git diff --check`) passes.

## Honest residuals

- Full-source `formatter.sico` now typed-refuses at `SKIP:GENERAL-WHILE-NESTED`
  (`format_code` uses nested `while`); nested-loop lowering is the next
  frontier, followed by the remaining eleven functions through `main`.
- The match surface is still the declared slice (`sico.bytes.slice` and
  `sico.list.get` subjects, ok/error arms); `normalize_source`/`main` shapes
  are not claimed.
- `break`/`continue` are single-while only (the machine already rejects
  nested whiles); `break`/`continue` with values or labels are not Sico
  surface.
- S5 Component codegen widening, S6 `A == B == C` bootstrap and the M22
  exit audit remain open; M22 stays NO-GO.
