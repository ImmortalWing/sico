# STEP-0623 — M22 W3-a: variant-construct leading-`let` materialization closes the B4 frontend gap

- Date: 2026-11-17
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **implementation complete + validated locally** — the B4 frontend
  construct gap (`let` alias of a parameter read back as a bare construct-field
  value) now lowers to canonical IR byte-identical to the Rust oracle and the
  guest emitter **accepts** it (`no-refusal`), where before it was refused
  `ERR:E-SH-CG-IR-EMIT`. 132 byte-guard baselines and the whole-file
  `parser.sico` IR pin stay green. **No gate flip**; M22 overall remains
  **NO-GO**. Evidence level: internal-fixture.
- Companion slice: the `checked_add` match-site ledger grew 403 → 405 (two new
  `checked_add` heads in the new leading-`let` scan); the re-freeze is recorded
  in the same change set.

## 1. Why this slice

STEP-0621's read-only reconnaissance pinned the W3-a co-gap to a **frontend IR
shape divergence**, not a guest-only emitter hole: the guest parser's variant
route (`record_variant_function_ir`) accepts a body with leading `let`
statements but **silently drops them** — it materializes neither the binding
nor any instruction for it — so when a construct field reads the binding back
by bare name, the read site emits a `read_local` with no backing cell, and the
guest emitter refuses the whole component (`ERR:E-SH-CG-IR-EMIT`).

The oracle bisect fixture **B4** (`let raw = input.stdin` then
`stdout: raw`) is the minimal reproducer. Its Rust-oracle canonical IR is:

```
1 Bytes Project{base:V0, field:"stdin"}   <- the leading let, materialized
2 Bytes Project{base:V0, field:"stdin"}   <- stderr: input.stdin (inline)
3 I64  ConstI64(0)                        <- exit_code
4 Construct ScriptOutput{stdout:V1, stderr:V2, exit_code:V3}
5 Variant ok[V4] -> return V5
```

The decisive shape: the bare `stdout: raw` field **references the let's value
id (`V1`) directly and emits zero instructions** — it does not re-lower the
projection, and it does not emit a `read_local`.

## 2. What was actually wrong (measured)

A guest canonical-IR dump of B4 (via the injection driver used by the cogap
probe) showed, before this slice:

```
instructions[0] = {"op":"read_local","data":{"local":0}}   <- no backing cell
```

i.e. the leading `let raw` produced **no instruction at all**, and the
construct field's bare `raw` was lowered to a `read_local` of a non-existent
local — exactly the "no `locals` backing" divergence STEP-0621 recorded. The
Rust oracle, by contrast, materializes the let as instruction 1 (a `project`)
and references it by value id.

A controlled bisect (a leading `let` that is **not** read back) confirmed the
drop is unconditional: the let produced no instruction even when materializable,
so the gap is in the variant route's leading-`let` handling, not in the read
site alone.

## 3. The fix (`selfhost/parser.sico`, variant route only)

The change is confined to the **variant-construct route**; the plain construct
route (`record_construct_function_ir`) is passed neutral "no lets" arguments so
its behavior is byte-identical to before (the 132 byte-guard corpus walks that
route and stays green by construction).

1. **`construct_let_materializable`** (new): a leading `let <name> =
   <param>.<field>` is materializable iff the right-hand side is exactly a
   parameter field projection of a named record (rhs-first token is the base
   parameter, the dot follows, the field resolves to a non-empty kind). Any
   other leading statement ends the leading-`let` region.
2. **Single-scan leading-`let` collector** in `record_variant_function_ir`: a
   token walk from the body start to the `return`, tracking `if`/`end` nesting
   depth and collecting only **statement-level** (preceded by `<nl>`) lets at
   depth 0. For each materializable let it (a) records `name -> value id`
   (ids allocated from `parameter_count` upward), and (b) emits the `project`
   instruction JSON, range-anchored at the body start (a let precedes the
   `return` whose expression anchors the construct, so the anchor must sit at
   or before the first let).
3. **`construct_lowering`** takes the collected let names/ids and the
   pre-materialized let instructions; it prepends the let instructions ahead of
   the construct and starts the construct's own value ids after the lets. For a
   bare-name field value it resolves the name through the let map to a
   **zero-instruction value reference** (`cl_value_ids = 0`), and only emits a
   separating comma when an instruction was actually produced (so the no-let
   path is byte-identical).
4. **`construct_field_value_ir`** gains the let map: a single-token field value
   that names a materialized let returns the empty string (the "reference, emit
   nothing" marker), which `construct_lowering` turns into a direct value-id
   reference.

The empty-ledger path (`let_names` empty, `let_instructions` empty) reduces
every changed function to its previous behavior, which is what keeps the
construct-route corpus and the whole-file pin green.

## 4. Validation (all run locally, single-threaded)

- **B4 flips to accepted**: the cogap refusal-identity probe now reports
  `BISECT B4 ... refusal identity -> no-refusal: component_hex_chars=4`, where
  before it was `refusal: ERR:E-SH-CG-IR-EMIT`. B0 (control), B2, B5, B6 stay
  `no-refusal` / unchanged.
- **Byte-identical to the oracle**: a guest canonical-IR dump of B4 now shows
  `instructions = [Project(V1), Project(V2), ConstI64(V3), Construct(V4,
  {stdout:1,stderr:2,exit_code:3}), Variant(V5)]` — matching the Rust oracle
  instruction-for-instruction, including the zero-instruction `stdout: raw`
  reference to `V1`.
- **132 byte-guard baselines green**: `selfhost_compiler byte_exactly` →
  `132 passed; 0 failed; 1 ignored`. No IR baseline moved, so no guard needed
  re-baselining.
- **Whole-file `parser.sico` IR pin green**:
  `sico_guest_arena_accommodates_full_parser_selfhost_input` → `1 passed`.
  The full self-host parser still lowers cleanly (no trap, no refusal) after
  the new scan.

## 5. Companion ledger re-freeze (403 → 405)

The new leading-`let` scan carries two new `U64.checked_add` `match` heads (the
field-offset range guard and the construct-first-id offset), so the frozen-S
candidate set grew **403 → 405**. `tools/census-m22-match-sites.py`
`EXPECTED_SITES` and its `rule` text were bumped to 405, the two ledger-count
assertions in `selfhost_compiler.rs`
(`sico_compiler_reproduces_the_frozen_match_site_ledger` /
`every_match_site_is_measured_on_the_component_axis`) were bumped 403 → 405,
and the ledger was re-derived from the current sources.

**Measured evidence (this slice).** Because the full 97-group recompute is
dominated by the pre-existing fuel-wall groups (~60+ min, orthogonal to this
slice), the ledger re-freeze is confirmed two ways:

- **Surgical 8-group measurement.** The measurement loop was given a read-only
  group filter (`SICO_M22_MATCH_GROUPS`, default unset = measure every group) so
  a probe run could re-measure only the groups this slice can touch — the seven
  construct groups plus `record_variant_function_ir` — in ~9 min instead of
  ~60+. The census script merges measured outcomes **per group** and keeps the
  recorded verdict of every group the probe did not measure, so the partial
  report refreshes exactly the touched groups. The 8 measured verdicts matched
  the pre-existing ledger verdicts group-for-group (`construct_field_comma`
  `ir-equal+component-equal`; the other seven `ir-equal+component-other:
  FuelExhausted`), confirming the construct-route changes are byte-identical and
  the variant route's component axis stays on the pre-existing fuel wall.
- **`census --update` then `--check`.** `LEDGER_WRITTEN sites=405 pending=0`,
  then `LEDGER_CURRENT sites=405`.

**Honest scope on the ledger claim.** The full 97-group slow recompute was not
re-run to completion in this slice (pre-existing fuel-wall cost, not a regression
introduced here); the ledger's agreement with the current sources rests on the
surgical 8-group measurement of every group this slice can affect plus the
unchanged sources of the other 89 groups. A future full recompute would be the
exhaustive confirmation axis.

## 6. Residuals and honest scope

- **Plain construct route still drops leading lets.** Only the variant route
  was fixed (the B4 oracle shape). A construct function with a leading `let`
  read back by bare name would still diverge; that is a separate shape not
  covered by the B4 oracle and left to a later slice.
- **General-CFG lets** (loop/arm/join) still use the cell/`locals` model and
  are untouched; STEP-0621 recorded the broader `locals`-serialization
  divergence as a separate slice.
- **No gate flip.** M22 overall remains **NO-GO**; this slice clears one
  frontend construct shape on the internal-fixture axis only.