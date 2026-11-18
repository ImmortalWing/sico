# STEP-0625 — M22 W3-a: the intrinsic-membership mask lowers the emitter's fuel enough to reveal the next real blocker (the map helper arms)

- Date: 2026-11-18
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **implementation + measurement complete locally (fuel-wall optimization
  cut, first of the owner-authorized track)** — the intrinsic-membership mask
  optimization removes the emitter's super-linear three-level rescan
  (`cgs_module_has_intrinsic` → `cgs_function_has_intrinsic` →
  `cgs_block_has_intrinsic` → `cgs_instruction_is_intrinsic` → hot leaf
  `cgj_value_end`) and replaces it with a per-function precomputed mask +
  O(1) membership test (`cgs_intrinsic_mask` / `cgs_intrinsic_present`). The
  byte output is **unchanged** (self-host + a `*_byte_exactly` pin both green).
  Re-running the STEP-0608 whole-formatter probe shows `A:main` is **no longer
  `FuelExhausted`**: it now reaches a **fast typed refusal** in ~76 s instead of
  burning the full 800e9 with nothing to compare. The lifted fuel wall **reveals
  the next real blocker**: the whole-formatter run-entry IR is refused with
  `ERR:E-SH-CG-IR-FLAT` because the `map` helper values
  (`sico.map.get/put/empty[Text,U64]`, used heavily by `formatter.sico`) have
  **no emission arm** in `selfhost/codegen.sico`. **No gate flip**; M22 overall
  remains **NO-GO**. Evidence level: internal-fixture.
- Companion: this is the first landed slice of the fuel-wall optimization track
  that STEP-0624 handed off as the W3-a endgame (AGENTS §10.1.4). It lowers fuel
  and, as a side effect, un-masks the map-arm gap the wall had been hiding.

## 1. Why this slice

STEP-0624 measured that the whole-formatter component face had evolved from a
typed refusal into `FuelExhausted` (~76 s / 800e9) — the guest now *attempts* to
emit but runs out of fuel, so there were no component bytes to compare and no
named shape to fix. STEP-0611 had already located the assembly hot spot: the
**intrinsic-membership** question "does this module/function/block contain any
helper-requiring intrinsic?" was answered by a **three-level rescan** on every
emitter decision, with the hot leaf `cgj_value_end` doing a per-byte
`sico.bytes.at` substring walk. Total fuel is O(H² × F × B × I). The
owner-authorized lever (D-A/D-B) is **lowering assembly complexity**, not
raising the fuel limit. This slice lands the cheapest, highest-leverage cut of
that track.

## 2. Method (byte-neutral optimization on `selfhost/codegen.sico`)

The guest already had the machines: `cgs_intrinsic_mask` (`codegen.sico:3434`)
builds a bitmask of the helper-vocabulary intrinsics a module uses in one O(N)
pass, and `cgs_intrinsic_present` (`codegen.sico:3449`) tests a bit in O(1).
The three helper-table builders each recomputed ~15 `cgs_module_has_intrinsic`
calls (each a fresh three-level rescan) to decide which helper sections to
emit. This slice hoists **one** `let cXXsh_mask = cgs_intrinsic_mask(bytes)`
per builder and swaps every `cgs_module_has_intrinsic(bytes, X)` for
`cgs_intrinsic_present(cXXsh_mask, X)`:

- `cgs_core_types_hex` (~`:5475`, `cctsh` mask)
- `cgs_core_funcs_hex` (~`:5605`, `ccfsh` mask)
- `cgs_core_code_hex` (~`:6956`, `cccsh` mask)

The emitted byte stream is **identical**: `cgs_intrinsic_present(mask, X)` is
true exactly when `cgs_module_has_intrinsic(bytes, X)` was, because the mask bit
for `X` is set exactly when some module function carries `X`. Signatures,
call sites, and the helper-section layout are unchanged. The now-orphaned
`cgs_module_has_intrinsic` (`codegen.sico:3297`) has **zero call sites** and is
deleted in the same change so the emitter's dependency closure reflects the
real (smaller) footprint.

## 3. Measured result

Re-ran the existing `#[ignore]` probe
`step0608_measure_whole_formatter_component_emission`
(`runner/sico-runner/tests/selfhost_formatter_component_measurement.rs`),
standard budget (fuel 800e9 / timeout 120 s / memory 2560 MiB, **not raised**),
wasmtime `target/tooling/wasmtime-v46.0.1-x86_64-mingw/`. Raw log:
`target/tmp/step0625/formatter-after-mask.log`.

- **`A:main`** (whole formatter, entry `main`): oracle `28459` B; guest outcome
  = **typed refusal** `Domain code="invalid-input" message="unsupported codegen
  source shape"` at wall **75.694 s** — a **fast refusal**, not `FuelExhausted`.
  In STEP-0624 the same face burned the full budget with no byte comparison; it
  now stops early at a named shape.
- **`B:run`** (entry renamed `run`): oracle `28459` B; same typed refusal at
  **76.815 s**.
- **Direct-to-emitter refusal identity** (guest canonical IR handed straight to
  `component_refusal_for_ir`): whole-formatter run-entry IR →
  **`ERR:E-SH-CG-IR-FLAT`** (82.7 s). This is the precise identity: a value in
  the run body falls outside the flat subset. Per STEP-0499, `ERR:E-SH-CG-IR-FLAT`
  fires for `map`/`set`/non-flattenable-record/mismatched-`list` shapes.
- **Byte-neutrality of the optimization**: the full parser self-host pin
  `sico_guest_arena_accommodates_full_parser_selfhost_input` = **ok (84 s)** and
  the codegen-functions pin
  `sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly` = **ok** —
  both exercise the guest lowering `selfhost/codegen.sico` through itself and
  comparing bytes, so the mask change produces **identical bytes**.

## 4. What this means

- The **intrinsic-membership mask** is the first landed fuel-wall optimization:
  it converts a large class of emitter decisions from O(N) rescans into O(1)
  bit tests. The whole-formatter face no longer exhausts fuel — it reaches a
  typed refusal in the same ~76 s window it used to burn the entire budget.
- Lifting the fuel wall **reveals the blocker it was masking**: the formatter
  uses `map.get` / `map.put` / `map.empty[Text,U64]` heavily
  (`formatter.sico:568/586/598/700/701/735/807/808`), and `selfhost/codegen.sico`
  has **no `cgs_map_*` / `sico.map.` emission arm** (grep-confirmed). The
  run-entry IR therefore hits `ERR:E-SH-CG-IR-FLAT` — the map-shaped values are
  outside the flat subset because nothing lowers them. This is the **next named
  emitter arm**, and it was invisible while the fuel wall dominated.
- The `text.trim` arm (`:4161`) and `list.get` arm (`:6225`) are already
  present, so the map family is the remaining structural gap for this file.

## 5. Honest scope

- Single file (`formatter.sico`), standard budget, single machine snapshot,
  **not** independent CI.
- The optimization is **byte-neutral** (two byte-exactness pins green), but the
  whole-formatter component is still **not emitted** — it now refuses at
  `ERR:E-SH-CG-IR-FLAT` (map arms) instead of `FuelExhausted`. No component
  bytes to compare for the whole file yet.
- "map arms are the blocker" is a **strong inference** (grep-zero map arms +
  formatter's heavy map use + STEP-0613d reconnaissance + the direct-to-emitter
  `ERR:E-SH-CG-IR-FLAT` identity), not yet a landed map arm. The precise first
  refused map intrinsic is read from the direct-to-emitter path, not from a
  bisect ladder that isolates one map call.
- The match-site ledger closure verdicts drift with this change (the emitter
  closure shrank), so `match-component-coverage-v1.json` must be re-measured via
  `sico_compiler_reproduces_the_frozen_match_site_ledger` +
  `census-m22-match-sites.py --update` and the emitter-vocabulary census
  re-checked before this slice is committed.
- One fuel-wall cut landed (intrinsic mask); the `text_concat` left-fold and the
  match-dispatcher rescan (STEP-0611) are **still open** and still bound the
  sites that get past the map arms.

## 6. Next cut (not started)

Add the `sico.map.get` / `sico.map.put` / `sico.map.empty[Text,U64]` emission
arms to `selfhost/codegen.sico`, following the STEP-0610 `is_utf8` /
STEP-0613e `text.trim` delegation pattern and replicating the oracle's
`stdlib.rs` map lowering byte-for-byte (one arm per slice, red-first
differential). Once the map family lowers, the whole-formatter face either
emits (if no further fuel wall) or names the next blocker. This continues the
owner-authorized fuel-wall + named-arm track on `selfhost/codegen.sico`; until
the whole file emits within 800e9 and matches the oracle byte-for-byte, W3-a
stays open and M22 overall stays NO-GO.