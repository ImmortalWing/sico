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
gap is a **guest frontend IR-shape divergence**, not a guest-only emitter hole:
the guest parser emits cell-indexed `read_local`/`write_local` for `let`
bindings (tracked in an internal `cell_locals` list, `parser.sico:8098-8102`)
but never serializes a `locals` member into the canonical IR, so the emitter
cannot resolve the cell. The oracle frontend, for a straight-line body, lowers
`let` to a **plain value binding** with no cell at all
(`sico-ir/src/lower.rs:2739-2746`, `lower_straight_line`).

## 4. Next knife (design grounding, not yet landed)

Fix belongs in `selfhost/parser.sico` let-lowering. Two candidate shapes were
weighed and the oracle evidence picks one:

- **(a) serialize the tracked cells as the function's `locals` member** —
  rejected. The oracle's straight-line path never puts a `let` in a cell at
  all: `lower_straight_line` (`sico-ir/src/lower.rs:2739-2746`) does
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
  (`:998-1020`).

Either way the IR shape must stay byte-equal to the oracle for every frozen
shape — the 132 byte-exact guards (including cell-based B2/B3, currently green)
pin this, so the change is a dedicated frontend slice with oracle-differential
validation, not a drive-by. B4's specific shape (`let raw = input.stdin`, a
parameter projection bound once and read once on a straight line) must lower to
a plain binding referencing the projected value id, exactly as the oracle does.

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
