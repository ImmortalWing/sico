# STEP-0622 — M22 W3-a: frozen `checked_add` match-site ledger re-frozen 396 → 403

- Date: 2026-11-17
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **ledger re-freeze complete + firmware-confirmed green** — the `match`-site
  ledger drift that STEP-0619 registered as a known residual is now cleared.
  This is a **pure measurement / re-freeze slice**: zero guest-source change,
  zero limit change, zero gate flip.
- Evidence level: internal-fixture (deterministic ledger firmware
  `sico_compiler_reproduces_the_frozen_match_site_ledger`, measured per-group
  against the Rust oracle on both the canonical-IR and Component axes with a
  real `wasmparser::Validator` run)

## 1. Why this slice

STEP-0619 closed the W3-a frontend construct-field-value gap by adding the
`construct_expression_parts` / `construct_intrinsic_parts` /
`construct_field_value_ir` routes to the frozen S (`selfhost/parser.sico`).
Seven of the `match` statements those routes carry are `U64.checked_add`
heads, so the frozen-S candidate set grew from **396** to **403**. STEP-0619
deliberately deferred the ledger re-freeze to an independent measurement slice
(its §6 residual): the construct correctness is covered by the byte-exact
guards plus the whole-file `parser.sico` IR differential pin, so the
`checked_add` coverage axis is orthogonal.

This slice is that independent slice. It is **方案 2 片 0** of the owner-selected
multi-slice plan: clear the pre-existing 396→403 ledger drift first (orthogonal
to the frontend, does not touch the hot files), so the later frontend slices
start from a clean, honest baseline.

## 2. What the drift actually was (measured, not assumed)

`tools/census-m22-match-sites.py --check` reported `CENSUS_MISMATCH
expected=396 actual=403`. A read-only diff of the frozen ledger against the
current enumeration (keyed `path:function:ordinal`, line-independent) located
the delta exactly:

- **0 sites disappeared.**
- **7 sites are new**, all in the three STEP-0619 construct routes:
  - `construct_expression_parts` — 2 sites (MS-220, MS-221)
  - `construct_intrinsic_parts` — 4 sites (MS-222..MS-225)
  - `construct_field_value_ir` ordinal 3 (MS-231)

Of these, **MS-231 is not a new ledger row**: `construct_field_value_ir`
already existed in the ledger at ordinals 0-2, so the census retained its
recorded verdict (`ir-equal+component-other:FuelExhausted`) under the same
`path:function:ordinal` key; only its line moved. The genuinely new rows are
the **6** in `construct_expression_parts` and `construct_intrinsic_parts`,
which start as `pending`.

So the drift is a **real candidate-set growth** (new `checked_add` heads), not a
counting-artifact or a line shift.

## 3. What landed

- `tools/census-m22-match-sites.py` — `EXPECTED_SITES` 396 → 403 and the
  ledger `rule` description updated to name the STEP-0619 origin of the 7 new
  sites.
- `runner/sico-runner/tests/selfhost_compiler.rs` — the two ledger-count
  assertions (`sico_compiler_reproduces_the_frozen_match_site_ledger` and
  `every_match_site_is_measured_on_the_component_axis`) 396 → 403.
- `selfhost/match-component-coverage-v1.json` — rewritten to 403 sites by the
  census tool (no `--update` yet), keeping every existing verdict/digest and
  adding the 6 new sites as `pending`.

The 6 new `pending` sites are then **measured**, not assumed: the ledger
firmware iterates every distinct enclosing function in the ledger, compares it
against the Rust oracle on both axes (canonical IR + Component bytes, with a
real validator run), and writes each group's measured outcome to
`%TEMP%/sico-m22-match-coverage/actual.json` incrementally. `census
--update` then merges those measured verdicts into the frozen table.

## 4. Verification

- `python tools/census-m22-match-sites.py --check` → `LEDGER_CURRENT sites=403`
  after the merge.
- `cargo test --locked --offline --manifest-path ./runner/sico-runner/Cargo.toml
  --test selfhost_compiler -- --test-threads=1
  sico_compiler_reproduces_the_frozen_match_site_ledger` → green.

(Final measured verdicts of the 6 new sites and the firmware result are
recorded in §5 once the single-threaded recompute completes; raw log
`target/tmp/step0622/ledger-recompute.log`.)

## 5. Result

The single-threaded recompute measured all **97** distinct enclosing-function
groups (403 sites) against the Rust oracle and wrote every outcome to
`actual.json`; `census --update` merged them into the frozen table:

- `LEDGER_WRITTEN sites=403 pending=0` — every site now carries a **measured**
  verdict, none pending.
- The two genuinely-new groups measured:
  - `construct_expression_parts` (MS-220, MS-221) →
    `ir-equal+component-other:FuelExhausted`
  - `construct_intrinsic_parts` (MS-222..MS-225) →
    `ir-equal+component-other:FuelExhausted`

  Both are **IR-axis equal, Component-axis not compared because the guest run
  exhausted fuel** — the same shape as the other large `parser.sico` functions
  on the known fuel-wall axis (owner D-B: limits are not raised to buy back
  sites). This is the honest measured outcome, not a fixture skip.
- `construct_field_value_ir` ordinal 3 (MS-231) kept its pre-existing verdict
  `ir-equal+component-other:FuelExhausted` under the same
  `path:function:ordinal` key (only its line moved with the STEP-0619 edits).

A fast re-verification that replicates the firmware's ledger-vs-measured
comparison over the fresh `actual.json` reports **403 sites / 97 groups /
0 mismatches**, i.e. the re-frozen ledger is exactly the measured table.

### The firmware first run stopped at MS-024 (pre-existing, not this slice)

The confirmation firmware run `panic`ked at the **first** mismatch —
`MS-024: parser.sico:while_call_rhs_packed` compared-source digest (measured
`a7539b1c…` vs the stale frozen `1b23b03f…`). This is the **known independent
item** STEP-0613c §6 already registered (source digest moved by the
STEP-0613c span fix / STEP-0619 construct edits in `parser.sico`; the
whole-file `parser.sico` IR pin still passes). Because the firmware stops at
the first mismatch, that run never reached the new sites. The `--update`
merge above re-froze MS-024 (and any other moved digest) to the measured
value, which is the same coordinated re-freeze discipline STEP-0605 used for
a parser edit. The confirmation re-run (task `bash-atvibuxk`) walks all 403
sites after the re-freeze; raw log
`target/tmp/step0622/ledger-confirm.log`. **It came back green:
`test result: ok. 1 passed; 0 failed; 0 ignored` over all 403 sites
(`finished in 5571.47s`, `FIRMWARE_EXIT=0`)** — the re-frozen ledger is
exactly what the firmware now reproduces, with no `pending`, no empty digest
and no mismatch stopping it early.

## 6. Honest scope / residuals

- This slice only re-freezes the `checked_add` **candidate ledger**. It does
  **not** close W3-a, does not flip any M22 gate, and does not change the
  overall M22 NO-GO (S3/S4/S5 partial, S6 not entered, S7 NO-GO).
- The `emitter-vocabulary` census refresh (STEP-0619 residual) is a separate
  axis and is **not** done here.
- Several ledger sites remain `FuelExhausted` / refused (e.g. MS-081 group) —
  these are the known fuel-wall axis, unchanged by this slice.
- Hot-file discipline: `selfhost/parser.sico` and `selfhost/codegen.sico` are
  **not** touched by this slice.
