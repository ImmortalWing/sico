# STEP-0624 — M22 W3-a: the whole-formatter component blocker has evolved from a typed emitter refusal to the fuel wall

- Date: 2026-11-17
- Branch: `codex/m22-w2-step0613` (worktree `sico-m22-w3a`)
- Owner: agent-directed M22 W3-a continuation (goal mode)
- Status: **measurement complete locally (pure measurement slice, zero source /
  zero fixture / zero limit changes)** — re-running the STEP-0608 whole-formatter
  component probe after the STEP-0619 construct-field-value closure and the
  STEP-0623 variant leading-`let` materialization shows the blocker has **changed
  kind**: the whole `formatter.sico` component is no longer refused before
  emission (`ERR:E-SH-CG-IR-EMIT`, ~0.03 % fuel in STEP-0608) — the guest now
  **attempts to emit** the whole component and **exhausts the 800e9 fuel budget**
  (`FuelExhausted`, ~76 s). The minimal multi-block `run` shapes the W3-a line
  was building toward now **emit successfully**. **No gate flip**; M22 overall
  remains **NO-GO**. Evidence level: internal-fixture.
- Companion: this is the measurement that re-points the W3-a endgame from
  "add emitter arms" to "the owner-authorized fuel-wall optimization cut"
  (`text_concat` assembly + match-dispatcher rescan, AGENTS §10.1.4).

## 1. Why this slice

STEP-0608 measured the whole `formatter.sico` COMPONENT face for the first time
and got a **typed refusal** `ERR:E-SH-CG-IR-EMIT` after burning only ~2.19e8 /
800e9 ≈ **0.03 %** fuel — the guest refused **before emitting**, so there were no
component bytes to compare and the named gap was `sico.bytes.is_utf8` used as an
`if` condition. Every slice since then (STEP-0610 `is_utf8` arm, STEP-0613
O1+G1 multi-block `run`, STEP-0619 construct-field-value closure, STEP-0621
refusal-mask hygiene, STEP-0623 variant leading-`let` materialization) removed
frontend/emitter gaps **upstream of the emitter**. This slice re-runs the same
STEP-0608 probe to answer one question: **what does the whole-formatter
component face refuse now?**

## 2. Method (pure measurement, nothing changed)

Re-ran the existing `#[ignore]` probe
`sico_compiler... step0608_measure_whole_formatter_component_emission`
(`runner/sico-runner/tests/selfhost_formatter_component_measurement.rs`) with
`--ignored --exact --nocapture`, standard budget (fuel 800e9 / timeout 120 s /
memory 2560 MiB, **not raised**), wasmtime
`target/tooling/wasmtime-v46.0.1-x86_64-mingw/`. Zero changes to
`selfhost/**`, fixtures, corpus, ledger, or limits. Raw log:
`target/tmp/step0624/formatter-component-probe.log`.

## 3. Measured result

- **Whole formatter, entry `main`**: oracle `28459` B; guest `FuelExhausted`
  (wall `76.251 s`) — `byte_equal`/`validator` n/a (guest exhausted fuel).
- **Whole formatter, entry renamed `run`**: oracle `28459` B; guest
  `FuelExhausted` (wall `76.223 s`).
- **Direct-to-emitter refusal identity** (guest canonical IR handed straight to
  `component_refusal_for_ir`): whole-formatter run-entry IR → `other:
  FuelExhausted` (81.3 s); main-entry IR → `other: FuelExhausted` (81.5 s). The
  emitter **no longer names a shape refusal** — it runs out of fuel emitting.
- **Bisect ladder (all `no-refusal`, i.e. the emitter accepts and emits)**:
  `run ok-only`, `run + bytes.is_utf8 if`, `run + utf8_decode + text.encode`,
  `run + error nested construct only` — every one emits a component
  (`component_hex_chars=4` = empty-component marker for the tiny stub, wall
  75–265 ms). The **minimal multi-block `run` + helper + Result-wrap shapes the
  W3-a line was building toward now emit.**

## 4. What this means (the re-qualification)

- The STEP-0608-era blocker (`is_utf8` as an `if` condition → emitter refused
  before emitting) is **gone**: the frontend/emitter now accepts and emits the
  constructs the formatter `main`/`run` actually use, including the multi-block
  branched `run` with leading `let`s, helper calls, and `ok`/`error` Result
  wrapping.
- The **remaining** whole-formatter blocker is the **emitter-side fuel wall**:
  emitting the whole component exhausts 800e9. This is the independently
  characterized super-linear assembly cost (`text_concat` left-fold + match
  dispatcher rescan; STEP-0548/0606/0611). Owner D-B explicitly declined to
  raise the fuel limit to buy back sites, so the lever is **lowering the
  assembly complexity**, not adding emitter arms.
- Therefore the W3-a endgame is **no longer gated on any named emitter arm**.
  It is gated on the same fuel-wall optimization cut that gates the component
  axis of the `match`-site ledger (MS-081 etc.) — an owner-authorized
  (D-A) op-level profiling → optimization track that should be run in a fresh,
  fully-budgeted session on the hot file `selfhost/codegen.sico`.

## 5. Honest scope

- Single file (`formatter.sico`), standard budget, single machine snapshot,
  **not** independent CI.
- The `FuelExhausted` is the **emitter's** fuel (guest executing the emitter
  component), the same wall as the 335-site `match`-site ledger component axis —
  not a new refusal introduced here.
- The prefix ladder's first non-emitting boundary (`helper_bytes=563`,
  `ERR:E-SH-IR-UNRESOLVED`) and the minimal-run control
  (`ERR:E-SH-IR-CALL-TARGET`) are **probe-fixture stub-shape signals** (the
  ladder stubs a helper the fixture does not declare), not statements about the
  whole file — the whole file's blocker is the fuel wall.
- Pure measurement: no source, fixture, corpus, ledger, or limit changed; no
  emitter arm added; no gate flipped.

## 6. Next cut (not started)

Lower the emitter-side assembly complexity so the whole formatter (and the
`FuelExhausted` ledger sites) emit within the frozen 800e9 budget — via the
owner-authorized fuel-wall optimization track (`text_concat` assembly and/or
match-dispatcher rescan), on `selfhost/codegen.sico`, in a fresh fully-budgeted
session. Until then W3-a stays open and M22 overall stays NO-GO.
