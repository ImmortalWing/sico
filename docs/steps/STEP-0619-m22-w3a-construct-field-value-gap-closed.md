# STEP-0619 — M22 W3-a: frontend construct-field-value gap closed (F1/F2/F3/B4 → `frontend: ok`)

- Date: 2026-11-17
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **implementation complete + verified locally** — the construct gap
  STEP-0613f diagnosed and STEP-0613h attempted is now **landed and verified**.
  Downstream emitter wall exposed as predicted. Known residual: the
  `match`-site ledger and emitter-vocabulary censuses are **not** re-frozen
  (independent measurement axes, §6).
- Evidence level: internal-fixture (deterministic selfhost probe `--ignored`,
  byte-exact guard 132/0/1, IR token dump)

## 1. Why this slice

`STEP-0613f` proved the co-gap blocker is a **frontend** construct-field-value
lowering gap; `STEP-0613h` wrote a 434-line repair design but reverted it after
a runtime error cascade. This slice re-applied that design, closed the two known
runtime error classes, and — via an IR token dump — found and fixed the **real**
remaining root cause, which neither prior STEP had identified.

## 2. What landed (`selfhost/parser.sico`)

The STEP-0613h 7-function design (re-applied from the preserved attempt), plus
one decisive one-line offset fix:

- `construct_intrinsic_kind`, `construct_project_instruction_json`,
  `construct_local_index`, `construct_local_kind`, `construct_expression_kind`,
  `construct_expression_parts`, `construct_intrinsic_parts` — the general
  construct field-value / expression resolver (oracle-contrast direction from
  STEP-0613f §5).
- `construct_field_call_count` widened 4→8 args (intrinsic callee recognized as
  call-shaped); its call site updated.

### Two STEP-0613h runtime error classes closed

1. **sibling-branch duplicate `let`** → `set` for mutable accumulators.
2. **`sico.text.concat` arity** — strictly 2-arg; all misused sites re-nested
   (arity audit 0 mismatches).

### The real root cause (this slice's decisive finding)

An IR token dump (read from the emitted IR `ty`, the only channel that surfaces
through the probe's refusal wrapper) showed `construct_local_kind` was handing
`construct_expression_kind` an RHS range that **started one token too late** —
`.<bytes<.<utf8_decode<(…)` instead of `sico<.<bytes<…`. The cause: **Sico's `=`
is not a standalone token** (confirmed by the canonical let-lowering at
`parser.sico:8045-8046`, which steps `let` → binding → rhs in **two**
`next_index` hops). `construct_local_kind` computed `clk_assign` = rhs-first
token (let+2) but then set `clk_rhs_first = next_index(clk_assign)` (let+3),
skipping the leading `sico` / `input` identifier. `construct_intrinsic_kind`
therefore never saw the `sico` prefix and returned an empty kind.

Fix: `clk_rhs_first = clk_assign` (rhs-first token), so the full
`sico.*` / `name.field` RHS reaches the kind resolver.

> An earlier run in this slice appeared to close F1/F2/F3/B4, but that was an
> artifact of a temporary diagnostic returning a non-empty `ERR:` string that
> satisfied the caller's `length(kind)>0` check. Removing it returned the cases
> to REFUSED. The offset fix above is the genuine closure — verified with **no**
> diagnostic present.

## 3. Verification

Probe `m22_w3a_cogap_refusal_identity` (`--ignored --exact --nocapture`):

| Case | Before (STEP-0613f/h) | After (this slice) |
|------|----------------------|--------------------|
| F1 utf8_decode+text.encode | `frontend: REFUSED … E-SH-IR-CONSTRUCT-VALUE` | **`frontend: ok`** → reaches emitter |
| F2 text.encode only | REFUSED | **`frontend: ok`** |
| F3 utf8_decode only | REFUSED | **`frontend: ok`** |
| B4 let alias + bare field value | REFUSED | **`frontend: ok`** |
| B1/B2/B3/B5/B6 | ok | ok (no regression) |

- Byte guard `selfhost_compiler byte_exactly -- --test-threads=1`:
  **132 passed / 0 failed / 1 ignored** (the ignored = `text.trim` co-gap,
  STEP-0613e) — **no regression**.
- `git diff --check`: clean.

## 4. Downstream wall exposed (as STEP-0613f §6 predicted)

Once constructible, F1/F2/F3/B4 reach the emitter and meet the **next** wall:
`ERR:E-SH-CG-IR-EMIT` — the emitter-arm coverage gap (the supported subset at
`codegen.sico:1790-1793` covers `project` / `const_i64` / `const_u64` /
`construct` / `ok`/`error` variant, not the `intrinsic` op these fixtures now
carry). This is a **separate** knife, exactly the "may, once fixtures can be
constructed, expose the emitter arms for separate verification" consequence
STEP-0613f recorded. Closing it is **not** in this slice's scope.

B0 (control echo, bare `stdout: input.stdin` with no `let`) still refuses with
`ERR:E-SH-IR-VARIANT-PAYLOAD` — a pre-existing shape (`payload_type_json` does
not model `input.<field>` projections), independent of this construct work and
outside the STEP-0613f declared 6-case scope.

## 5. Process note (honest)

Two intermediate states were recovered from during this slice: a mid-edit panic
(a 3-arg `sico.text.concat`, fixed to nested 2-arg) and one transient probe
process crash that did **not** reproduce on re-run. All temporary diagnostics
(`diag_token_range`, the CEK/CEPK/CVK dumps, the `m22_w3a_dump_construct_ir`
probe) were removed; the landed state carries none of them. The incomplete
pre-fix attempt is preserved off-repo at
`/tmp/parser.sico.step0619-incomplete-attempt`.

## 6. Known residual (honest, NOT closed here)

- **`match`-site ledger** (`selfhost/match-component-coverage-v1.json`) — the new
  functions add **7 `U64.checked_add` match sites** (396 → 403), so
  `tools/census-m22-match-sites.py --check` reports
  `CENSUS_MISMATCH expected=396 actual=403` and the firmware test
  `sico_compiler_reproduces_the_frozen_match_site_ledger` (hard-asserts
  `sites.len()==396`, forbids empty-digest and `pending`) will fail. An honest
  re-freeze needs the full 403-site measurement run (historically ~6000 s) plus
  `EXPECTED_SITES`/firmware-assertion bumps. This is an **independent
  `checked_add`-coverage axis**, orthogonal to the construct closure; the
  construct functions' correctness is covered by the byte guard (132/0/1) and
  the whole-file `parser.sico` differential pin. **Deferred to its own
  measurement slice.**
- **Emitter-vocabulary census** — likewise requires a root-workspace
  `sico-ir::dump_oracle_ir` build; deferred with the ledger re-freeze.

## 7. Gate discipline

**No gate flipped.** The construct closure advances the W3-a component face but
M22 overall remains **NO-GO** (S3/S4/S5 partial, S6 not entered, S7 NO-GO per
STATUS/ROADMAP). This slice claims no runtime/platform support beyond the
internal-fixture probe and byte guard.

## 8. Files

- `selfhost/parser.sico` — construct-field-value repair landed (7 functions +
  the `clk_rhs_first` offset fix).
- `docs/steps/STEP-0619-m22-w3a-construct-field-value-gap-closed.md` — this record.
- `docs/steps/README.md` — STEP index row.
