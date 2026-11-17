# STEP-0621 — M22 W3-a: emitter refusal mask hygiene — named typed identities for the core-module empty-string collapse

- Date: 2026-11-17
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **implementation complete + verified locally** — permanent hygiene
  improvement, zero regression. No gate moved.
- Evidence level: internal-fixture (deterministic selfhost probe `--ignored`
  full `selfhost_compiler` suite, baseline-differentiated)

## 1. Why this slice

`STEP-0620` isolated the next emitter wall to "a `read_local` of a let-bound
value consumed as an intrinsic argument" but could not localize the failing
check: every candidate emitter branch returned the same generic
`ERR:E-SH-CG-IR-EMIT`, and `component_refusal_for_ir`
(`codegen.sico:490-499`) folded any **empty** core-module result into that same
generic code. `STEP-0613f` §7 already recommended the companion fix — let the
refusal identity surface the inner condition instead of collapsing it. This
slice lands that hygiene so the next diagnostic (and every future one) reads a
named cause rather than a folded one.

## 2. Change (pure rename, refusal branches only)

`cg_core_module_hex` returned `""` at six structural gates and
`cg_component_hex` at one; `component_refusal_for_ir` turned each empty string
into the generic `E-SH-CG-IR-EMIT`. Each gate now returns its own typed
identity, which `component_refusal_for_ir` passes through verbatim (its
`starts_with(.., "ERR:")` arm already forwards any `ERR:` string):

| site (`codegen.sico`) | gate | new identity |
|---|---|---|
| `cg_core_module_hex:401` | no `functions` member | `ERR:E-SH-CG-IR-CORE-FUNCTIONS` |
| `:405` | `cgs_function_count` = 0 | `ERR:E-SH-CG-IR-CORE-COUNT` |
| `:412` | core types segment empty | `ERR:E-SH-CG-IR-CORE-TYPES` |
| `:416` | core funcs segment empty | `ERR:E-SH-CG-IR-CORE-FUNCS` |
| `:420` | core exports segment empty | `ERR:E-SH-CG-IR-CORE-EXPORTS` |
| `:427` | core code segment empty | `ERR:E-SH-CG-IR-CORE-CODE` |
| `cg_component_hex:451` | core module empty (non-ERR) | `ERR:E-SH-CG-IR-CORE-MODULE` |

Net `+7/−7`. No emission (success) path touched — every changed line is a
`return` of a refusal identity, so byte-exact emission output is unchanged by
construction.

## 3. Diagnostic use + the wall it exposed (bounded, honest)

With the mask lifted, a fresh `m22_w3a_cogap_refusal_identity` run plus a
per-layer `DIAG-*` instrumentation ladder (temporary, fully removed) localized
the F1/F2/F3/B4 refusal to **`cgs_read_local_hex` → `cgs_cell_slot` returning
0** (`ERR:DIAG-READLOCAL-NOCELL`). The B4 canonical IR dump is decisive:

- B4 source `let raw = input.stdin` lowers to
  `{"op":"read_local","data":{"local":0}}` (result 1, ty `bytes`), then
  `project base:0 field:stdin`.
- The `run` function IR carries **`parameters` only — no `locals` member**.
- `cgs_cell_slot` (`codegen.sico:2330-2361`) and `cgs_cell_slots`
  (`:2237-2259`) both iterate the IR `locals` member; with it absent they
  return 0 ⇒ `missing local cell`.

Cross-check against the Rust oracle: the oracle's `LocalPlan` builds
`cell_slots` from `function.locals` only (`sico-codegen-wasm/src/lib.rs:1914-1926`)
and its `ReadLocal` lowering errors `missing local cell` when
`cell_slots.get(local)` is absent (`:2722-2723`). So a `read_local` whose cell
is not backed by a declared `locals` entry is refused by **both** sides — the
gap is a **guest frontend IR-shape divergence**, not a guest-only emitter hole.

**CORRECTION (read-only scout, this session — supersedes the narrower
"emits a cell but omits the `locals` member" reading):** a deeper scout of
`parser.sico` shows the divergence is **two compounding defects**, and the
`let`-statement story is worse than "unbacked cell":

1. The guest is **unconditionally cell-based** — `grep -c '"locals"'
   parser.sico` = **0**: no route (straight-line or control-flow) ever
   serializes a `locals` member. Two functions written to the oracle's
   plain-binding semantics (`straight_let_function_ir` `:22886`,
   `chained_let_function_ir` `:23140`) exist but have **zero call sites** (dead
   code).
2. **The leading `let` in B2/B3/B4 is not cell-backed — it is dropped
   entirely.** All three take the same `record_variant_function_ir`
   (`parser.sico:12101`), whose only gate is "exactly one `return` whose
   expression starts with `ok`/`error`"; it does **not** require a
   single-statement body, so a leading `let` is accepted and then **silently
   ignored**. The B4 IR dump confirms it: the `let raw = input.stdin` produces
   **no instruction at all** (no `write_local`, no `project` for the `let`
   itself) — only the *use site* emits an unbacked `read_local local:0`.

So B2/B3 are **not** green because the intrinsic result id is referenced
directly (an assumption this STEP previously recorded); they are green because
**the bound name is never read back** — the value is silently lost, and their
IR is **also byte-divergent from the oracle**, just silently. Any fix must
cover the construct/variant routes (`record_construct_function_ir` `:12009`,
`record_variant_function_ir` `:12101`, `construct_local_index` `:11196`,
`construct_local_kind` `:11234`, `construct_expression_parts` `:11316`,
`construct_field_value_ir` `:11620`), not only `straight_cell_function_ir`
(`:10164`).

## 4. Next knife (design grounding, not yet landed)

Fix belongs in `selfhost/parser.sico` let-lowering. Two candidate shapes were
weighed and the oracle evidence picks one:

- **(a) serialize the tracked cells as the function's `locals` member** —
  rejected. The oracle's straight-line path never puts a `let` in a cell at
  all: `lower_straight_line` (`sico-ir/src/lower.rs:2146-2153`; note the
  `lower.rs:2739` cited earlier in this STEP was the wrong line) does
  `builder.bindings.insert(name, (value, ty))` and **never touches
  `builder.locals`** (confirmed: zero `builder.locals` references in that
  function), so an oracle straight-line `let` produces **no cell and no
  `locals` member** in the canonical IR. Serializing the guest's cells would
  make every straight-line `let` carry a cell + `locals`, byte-diverging from
  the oracle.
- **(b) route straight-line `let` to a plain value binding** (oracle
  `lower_straight_line` semantics: the bound value id is referenced directly,
  no `write_local`/`read_local`, no `locals` declaration) — **this is the
  aligned direction**. Only general-CFG `let`s (loop body / arm / join) keep
  the cell + `locals` representation, matching `lower_function`'s general path
  (`:998-1020`) — and note the guest's control-flow routes **also** never
  serialize `locals`, a second, wider divergence left to a separate slice.

The IR shape must stay byte-equal to the oracle for every frozen shape — the
byte-exact guards pin this, so the change is a dedicated frontend slice with
oracle-differential validation, not a drive-by. B4's specific shape (`let raw =
input.stdin`, a parameter projection bound once and read once on a straight
line) must lower to a plain binding referencing the projected value id, exactly
as the oracle does; and because the binding is currently **dropped**, the fix
must also make the bound value actually flow to the use site rather than only
removing the unbacked `read_local`. Scope spans `straight_cell_function_ir`
(`:10164`), `record_construct_function_ir` (`:12009`),
`record_variant_function_ir` (`:12101`), the `construct_local_*` /
`construct_expression_parts` / `construct_field_value_ir` family, and wiring the
already-oracle-shaped dead code (`straight_let_function_ir` `:22886`,
`chained_let_function_ir` `:23140`) into `scalar_ir`'s ladder (`:21744`).

**Byte anchor (oracle canonical IR for B4, dumped via
`crates/sico-ir/tests/dump_oracle_ir.rs` with `ORACLE_SOURCE`, this session):**

```
fn[0] locals=0                                  // no locals member
block 0:
  1 Bytes Project { base: ValueId(0), field: "stdin" }   // let raw = input.stdin
  2 Bytes Project { base: ValueId(0), field: "stdin" }   // stderr: input.stdin
  3 I64 ConstI64(0)
  4 Construct ScriptOutput { stdout: ValueId(1), stderr: ValueId(2), exit_code: ValueId(3) }
  5 Variant { name: "ok", payload: [ValueId(4)] }
  -> return ValueId(5)
```

So the oracle lowers `let raw = input.stdin` to **one `project` instruction**
(result 1) and binds `raw` as a plain value binding = `ValueId(1)`; the
construct's `stdout` field references `ValueId(1)` **directly**. There is **no
`read_local`, no `write_local`, no `locals`**. The guest already has a working
`project` emitter (`cgs_project_hex`; the B0 control's `input.stdin` field
lowers through it), so `project` is not the gap — the sole frontend gap is that
the guest lowers the `let` to an unbacked `read_local local:0` instead of
materializing the `project` and referencing its value id at the use site. The
targeted B4 fix is therefore: lower the leading `let` in
`record_variant_function_ir` to the `project` instruction (result id 1) and make
the construct's `stdout` field reference that value id — matching the anchor
byte for byte and dropping the `read_local` entirely.

## 5. Validation

- Full `selfhost_compiler` suite, single-threaded, `--locked --offline`:
  **182 passed / 7 failed / 6 ignored** — byte-identical to the unmodified
  `HEAD 30a39fb` baseline (the 7 failures were re-run at HEAD via `git stash`
  in the same session and fail identically: the frozen match-site ledger
  MS-081 fuel wall, T1/T2 axes, escape/construct refusals — all pre-existing,
  none introduced here).
- `git diff --check` clean; all temporary `DIAG-*` markers and the probe's
  temporary B4 IR dump removed (working tree = only `selfhost/codegen.sico`
  `+7/−7`).

## 6. Honest scope

- This slice **closes no M22 gate**; M22 overall remains **NO-GO** (S3/S4/S5
  partial, S6 not entered, S7 NO-GO).
- Claim is limited to: mask hygiene is permanent and zero-regression; the
  `read_local` NOCELL root cause is localized to the guest-frontend
  `locals`-member omission, evidenced by the B4 IR dump + oracle cell_slots
  cross-check (internal-fixture).
- The match-site-ledger refreeze (STEP-0619 residue, 396→403) and the
  emitter-vocabulary census remain separate, independent measurement axes,
  untouched here.
