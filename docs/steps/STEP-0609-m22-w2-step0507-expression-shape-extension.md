# STEP-0609 — M22 W2 `step0507` expression-shape extension (no-argument `sico.list.empty` in return position)

- Date: 2026-11-08
- Track: M22 compiler self-host, goal **W2**（`AGENTS.md` §10.5 / `docs/plans/M22-compiler-self-host.md`）
- Slice type: **IMPLEMENTATION (guest frontend, insert-only)** — one new lowering route + one ladder block in `selfhost/parser.sico`; frozen ledger renumbered (pure line-shift); no gate flipped, nothing committed or pushed.
- Worktree HEAD at start: `90e37db`（`M22 match dispatcher slot cache: byte-identical pure speedup (3-7% fuel)`）.
- `selfhost/parser.sico` after this slice: sha256 `dec08b6a…`, 23317 lines = baseline `b67ec566…` (23168) **+149 / −0** (insert-only).

## 1. Goal and honest scope

`step0507` is one of the five exit-blocking red tests registered by STEP-0602/0604. The target test

```
sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly
  (runner/sico-runner/tests/selfhost_compiler.rs:3469)
```

loops 21 `codegen.sico` function names; for each it builds `codegen_function_source(name)` (stubs for every callee + the function) and asserts `run_guest(source)` canonical IR == `rust_ir(source)` byte-for-byte.

At baseline only **`cgs_result_return_hex`** of the 21 failed, with `Domain { code: "invalid-input", message: "ERR:E-SH-IR-EXPRESSION" }`. This slice closes that gap and turns the test green, **without** loosening any typed-refusal pin and **without** adding a `match`-site to the frozen ledger.

## 2. Root cause (measured, differs from the earlier recon lead)

The earlier hypothesis (EXPRESSION-layer / `if`-condition gap) was **not** the blocker. Bisection over the 21 names pinned it to the **stub generator**, not the target function body:

- `codegen_stub` (`selfhost_compiler.rs:3404`) maps a stub's return type to a trivial body: `"List[U64]" => "sico.list.empty[U64]()"`, `"Bytes" => "sico.text.encode(\"\")"`, else a literal.
- `cgs_result_return_hex` calls `cgs_value_index`, whose return type is `List[U64]`, so its stub body is `return sico.list.empty[U64]()`.
- **That stub alone fails.** Minimal repro (independent `run_guest`, this slice's probe):

```
function cgs_value_index(bytes: Bytes, run: U64) returns List[U64]:
  return sico.list.empty[U64]()
→ Domain { code: "invalid-input", message: "ERR:E-SH-IR-EXPRESSION" }
```

So the guest frontend could **not** lower a no-argument `sico.*` intrinsic in **return position** (general case). The existing return-intrinsic route `direct_host_call_return_ir` (`selfhost/parser.sico:8615`) only claims `sico.bytes.length` with `parameter_count==1`, a `Bytes` argument and a `u64` return; `cgs_value_index` has 2 parameters and a `List[U64]` return, so the route SKIPs and the body falls through to the `scalar_ir` literal/fixed-op fallback → `ERR:E-SH-IR-EXPRESSION`.

The **Rust oracle accepts** this shape (`lower_core`): one instruction `{"result":<pc>,"ty":{"kind":"list","data":{"kind":"u64"}},"operation":{"op":"intrinsic","data":{"name":"sico.list.empty[U64]","arguments":[]}},"range":{…}}` with terminator `return <pc>`, where `<pc>` = `parameter_count`.

## 3. Fix — one narrow additive route

New function `direct_list_empty_return_ir` in `selfhost/parser.sico`, modeled on `direct_host_call_return_ir` and wired into the `scalar_ir` route ladder next to the `direct_host_json` block. It claims **only** a straight-line body of the exact form

```
function <name>(…) returns List[<Elem>]:
  return sico.list.empty[<Elem>]()
end function
```

(any `parameter_count`), for `<Elem>` ∈ {`U64` → `{"kind":"list","data":{"kind":"u64"}}` / `sico.list.empty[U64]`, `Text` → `{"kind":"list","data":{"kind":"string"}}` / `sico.list.empty[Text]`}, and only when the declared `return_kind` matches the list type. It emits `parameters_ir(...)` + one `intrinsic_instruction_json(...)` (result id = `parameter_count`, `arguments:[]`, range = return-expression start..end) + `terminator return <id>`, wrapped in the standard function envelope. Everything else returns `"SKIP"`, so **every existing refusal identity is preserved** (the pinned three-argument `return sico.bytes.at(...)` → `ERR:E-SH-IR-EXPRESSION` is untouched because the route requires the `list.empty` path and zero call arguments).

### 3.1 Frontend constraint discovered en route (load-bearing)

A first attempt factored the `end function` offset arithmetic into a separate helper `function_end_offset(src, off)` (`min(off+13, len)` via a `next_index` chain, chosen to avoid a new `match U64.checked_add` census site). That **regressed** the whole-file parser IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` with `ERR:E-SH-IR-EXPRESSION`. Isolated repro established the rule:

- a **straight-line** function (only `let`/`set`/`return`, no `match`/`while`/`if`) routes to a narrow path that refuses even `let z = U64.literal(5); return z`;
- `let x = <user_call>; return x` **is** accepted, and a function containing a `match`/`if` gets the general path.

So the `end function` offset is computed **inside** `direct_list_empty_return_ir` (which has `if`s → general path) via a `next_index` chain over `gw_find_code_text(src,"end function",fn_start)` with a `sico.bytes.length(src)` saturation bound, and the helper was deleted. This keeps the route **free of a `match U64.checked_add`**, so the frozen 396-site ledger count is unchanged.

### 3.2 Ladder `source_cursor` advancement (no new census site)

The ladder block advances `source_cursor` past the claimed function via
`find_text_from(src, "\n", gw_find_code_text(src,"end function",fn_start))` then `next_index(…, source_len)` — past the `end function` line, no `match U64.checked_add`, so no second census site. `source_cursor` is internal (not emitted), so this coarser advance is safe; the **emitted** `range.end` inside the route stays the byte-exact `offset+13`.

## 4. Frozen-ledger impact — pure line-shift, zero content change

The insert-only edit shifts `parser.sico` line numbers below the anchor. `tools/census-m22-match-sites.py` (plain rewrite, no `--update`, no firmware run) renumbers by site key `path:function:ordinal` and **inherits** verdicts/digests:

- `match U64.checked_add` statement count: **408 before and after** (raw grep and census-regex) ⇒ **no new census site**; `EXPECTED_SITES` stays 396.
- Renumber diff vs the pre-edit ledger: **229 sites moved**, and **0 changes** to `verdict`, `ir_sha256`, `component_sha256`, `case_shape`, `head`, `parameter_count`.
- Post-renumber `--check` → `LEDGER_CURRENT sites=396`.

This is the STEP-0605-established mechanical renumber (identity-keyed digest inheritance), **not** a digest re-freeze.

## 5. Verification (all run this slice, Windows x64 GNU, `1.98.0`, `--locked --offline`)

| Test | Result |
|---|---|
| `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly` (target) | **ok** (was RED) |
| `sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly` | **ok** (was RED — see §6) |
| `sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly` | ok (still green) |
| `sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly` | ok (still green) |
| `sico_compiler_keeps_the_neighbouring_host_call_shapes_typed_refused` | ok (refusal frontier intact) |
| `sico_compiler_refuses_invalid_set_arm_match_shapes_with_typed_identity` | ok |
| `sico_compiler_refuses_unresolved_record_field_projections_with_a_typed_identity` | ok |
| `sico_guest_arena_accommodates_full_parser_selfhost_input` (whole-file parser IR pin) | **ok** (35 s) |

`cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` → clean; `git diff --check` → clean. The target test's byte-equality is real: guest stdout == `rust_ir(source)` (the test asserts `assert_bytes_equal`).

## 6. Correction to the prior diagnosis: `step0526` shares this root cause

STEP-0602/0604/0605 recorded `step0526` as RED with an **independent** root cause ("non-Output nested `while`" in `cgs_function_instr_hex`). This slice measured that at baseline (`parser.sico` = `b67ec566…`) `sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly` fails at `cgs_function_instr_hex` with the same `must compile through the guest frontend` / `ERR:E-SH-IR-EXPRESSION`, and that this slice's fix turns it **green**. So `step0526`'s blocker was (also) a `List[U64]`-returning stub in `cgs_function_instr_hex`'s closure hitting the same return-intrinsic gap — **not** an independent `while` gap. The earlier "independent root cause" record is therefore corrected by this slice.

**Net: 2 of the 5 exit-blocking red tests (`step0507`, `step0526`) are now green.** `step0519`/`step0520` were already green (STEP-0605). The remaining exit-blocker set and the separate fuel-wall face are untouched.

## 7. Honest caveats / not done

- **No gate flipped; M22 overall still NO-GO.** This closes named red tests; it does not close any of the seven exit gates, the fuel wall (331 FuelExhausted ledger sites), or W3-a.
- **Not committed / not pushed** (no owner authorization). Working-tree changes: `selfhost/parser.sico`, `selfhost/match-component-coverage-v1.json` (renumber only). `runner/sico-runner/tests/selfhost_compiler.rs` is byte-identical to HEAD (a temporary probe used during bisection was removed and the file restored).
- **`sico.text.encode("")` Bytes-return stub** was left alone (the knife stays narrow). No measured test needs it: all 21 `step0507` functions and the 8 `step0526` functions pass without it, and `codegen.sico` has no `Bytes`-returning function.
- **Full `selfhost_compiler` suite not run** — only the 8 named tests above via `--exact`. The ledger's slow 95-group per-site recompute was **not** run (the renumber inherits digests by key; no recompute needed for a pure line-shift).
- **Ledger firmware** `sico_compiler_reproduces_the_frozen_match_site_ledger` still red at MS-081 (`FuelExhausted`), pre-existing and orthogonal to this slice (unchanged by it — MS-081 is a `checked_add` site untouched by this route).
- Concurrent work in the tree (`STEP-0608` W3-a measurement files, `docs/steps/README.md`) is **not** mine and was preserved untouched.