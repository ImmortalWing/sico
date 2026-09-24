# M23 Language v1 batch 3 — expression ergonomics

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved; contract drafting may proceed before M22 S7 closes, implementation may not; amended 2026-09-24 (STEP-0264, M22–M26 route replan v1 §4): the implementation gate becomes dual-exit — Route A after an M22 S7 GO, Route B immediately after an M22 R3 stop-loss declaration (S6 convergence declared not demonstrated on the current surface); the mid-flight-churn rationale is preserved because Route B never fires while a closure attempt is in flight

## 1. Objective

Close the language-surface friction that RFC-0044/0045/0046 measured but
did not remove, so that ordinary application logic (and the M22
self-host frontend that consumes the same surface) stops paying the
recorded ceremony tax. Batch 3 is per-item, RFC-0033-gated: no item
lands without its own accepted RFC carrying the four-part evidence
(explicit desugar, source-map identity, formatter idempotence, typed
diagnostics).

Items, exactly as recorded in the M22 plan §8:

1. **Infix comparison and logical operators.** The only infix operator
   is `==` and only in `if` conditions (RFC-0044). `<`, `<=`, `>`,
   `>=`, `!=`, `&&`, `||`, `!` do not exist; comparison is
   `I64.less_than(...)` match ceremony, conjunction is nested `if`s.
   The smallest closed operator set and its desugar shapes must be
   decided by the RFC, not grown ad hoc. Measured consumer: the
   `selfhost/*.sico` sources themselves (countable ceremony per line of
   real compiler code).
2. **Expression-position conditions.** `match`/`if` freeze the
   all-return statement shape (STEP-0083): `let x = match …` and
   `let x = if …` do not exist. This is the single largest structural
   verbosity tax recorded; the RFC decides the smallest let-bound form.
3. **Bare-literal re-measurement (policy item, not a grammar
   commitment).** RFC-0046 D3 stands until the recorded re-measurement
   lands: re-weigh bare-literal ceremony on a real generated-code
   corpus (AI output reviewed per the STEP-0175-era DX audit method),
   not on recollection. The RFC either records "decision stands" with
   the new measurement, or proposes the smallest typed-literal change
   with full RFC-0033 evidence.
4. **`text.chars`** (queued by RFC-0046 §2): decide with a consumer
   measurement whether the dedicated intrinsic beats the composed
   `text.length`/`char_at` loop; land only with the same evidence bar.

## 2. Entry gate

- **Dual-exit M22 gate (STEP-0264, 2026-09-24; supersedes the single
  "waits for M22 closure" condition).** Implementation may start under
  either route:
  - **Route A — M22 S7 exit audit GO.** Batch 3 implements on a proven
    closure; the self-host re-baseline decision (exit gate 5) follows.
  - **Route B — M22 R3 stop-loss declared** (M22 plan R-phase: 4
    consecutive implementation STEPs with zero `formatter.sico` canary
    growth and an unmoved refusal frontier; see
    [`M22–M26 route replan v1`](./M22-M26-route-replan-v1.md) §3–§4).
    Batch 3 implements immediately — its measured consumers already
    exist (§8) and the richer surface is the registered re-baseline
    target for a restarted M22 after this milestone's exit audit. The
    prior M22 stop-loss verdict stays in the dual-implementation
    register; M22 re-entry re-runs S1/S2 differentials on the new
    surface under the unchanged ADR-0015 contract.
  - The original churn rationale survives in both routes: language
    churn must not invalidate a mid-flight bootstrap closure, so Route
    B fires only after convergence is declared absent, never during a
    closure attempt. Only RFC/contract drafting and measured-consumer
    evidence proceed while M22 is mid-flight regardless of route.
- Per-item accepted RFC (RFC-0033 gate): measured consumer, explicit
  desugar, source-map identity, formatter idempotence, typed
  diagnostics for pre-existing identifiers that used the new keywords/
  operators.
- No reserved STEP numbers: implementation STEPs are allocated only
  when an RFC is accepted and the exit test is bounded.

## 3. Exit gates

1. Every accepted batch-3 item executable end-to-end: e2e byte-exact
   corpus through the real runner, including limit+1 refusals.
2. Formatter idempotence and canonical output on every new shape;
   frozen snapshots additive-only (existing corpus bytes unchanged).
3. Machine support matrix rows + validator extension; undeclared
   check/build/run gap count stays zero for the application profile.
4. Typed diagnostics with stable codes for every new refusal class;
   action-hint catalog extended (M21 §3.3 contract).
5. Self-host re-baseline registered: the Sico frontend (parser/checker/
   formatter slices) either consumes the batch-3 surface or a declared,
   disjoint subset — recorded in the M22 dual-implementation register,
   not silently divergent.
6. M0–M22 regression green + explicit exit audit with per-gate evidence.
7. AI-generation-noise claims (if any) only from owner-credentialed
   live-model runs; offline measurement is labeled offline.

## 4. Non-goals

- No new IR operations (desugars to existing CFG/match/intrinsic
  machinery only).
- No generics, macros, operator overloading, or implicit conversions.
- No bare-literal grammar change without the recorded re-measurement.
- No churn to frozen profiles or byte-frozen corpora (append-only).
- No claim that batch 3 makes the language "v1-final" — that verdict
  belongs to M25's freeze.

## 5. Risks

- The self-host frontend re-baseline is real work; if batch 3 lands a
  wide operator set, the Sico parser slices must re-freeze. Mitigation:
  the RFC fixes the smallest closed set before implementation.
- `&&`/`||` short-circuit semantics vs the all-return match desugar:
  the RFC must prove the desugar preserves evaluation order and
  side-effect visibility, or restrict v0 to non-effect operands.
- Diagnostics churn: new operator errors must not renumber existing
  stable codes.

## 6. Evidence classes

`internal-fixture` for all corpora; AI-noise reductions are
`blocked-external-evidence` until owner credentials enable live-model
re-measurement. No platform claims arise from this milestone.

## 7. Kickoff requirement

The first implementation STEP (after RFC acceptance) is an inventory
STEP in the STEP-0129/0142 pattern: a measured probe of the current
surface (operator/keyword absence, ceremony counts on the selfhost
corpus), the RFC set, and the machine matrix extension — before any
grammar change lands.

## 8. Per-item work protocol (refined 2026-09-22; no STEP numbers reserved)

Every item follows the same four-phase protocol. Phase names are working
labels for planning, not allocated STEP identifiers.

### 8.1 Item 1 — infix comparison and logical operators

- **Measured consumer (inventory phase).** On the eleven `selfhost/*.sico`
  files (14,018 lines) plus the frozen application corpora, count and
  freeze a ceremony table: (a) `Result[Bool, NumericError]` match blocks
  whose only purpose is one comparison (`less_than`/`greater_than`/…),
  (b) nested-`if` conjunction/disjunction sites two or more levels deep,
  (c) `checked_add`/`checked_sub` match blocks per arithmetic site. Report
  density per 1,000 lines. No operator decision is argued from
  recollection.
- **RFC proof obligations.** Decide the smallest closed operator set
  (evaluation candidates: `< <= > >= !=` and `&& || !`; the RFC decides,
  the punctuation table is not grown ad hoc); give the explicit desugar to
  existing match/intrinsic machinery with **no new IR operations**;
  prove evaluation-order and short-circuit behavior for `&&`/`||` — v0 may
  restrict to side-effect-free operands, and any restriction must be
  enforced by a typed refusal, not a silent fallback; source-map identity
  (operator spans map onto the desugared region, diagnostics point at the
  operator); formatter idempotence with a canonical spacing table;
  new typed diagnostic codes for every new refusal class, append-only (no
  renumbering of existing stable codes); compatibility audit for
  pre-existing identifiers against the new tokens.
- **Bounded exit test.** New-shape corpus byte-exact end-to-end through
  the real runner including limit+1 refusals; machine support matrix row +
  validator; frozen snapshots unchanged (append-only); self-host
  re-baseline entry recorded (§9).

### 8.2 Item 2 — expression-position conditions

- **Measured consumer.** Count the mutable-cell + all-return match +
  join-read sites that exist only to bind one conditional value, in the
  selfhost sources and the M14 solver corpus; freeze the count.
- **RFC proof obligations.** Choose the smallest let-bound statement form
  only (`let x = if …` / `let x = match …`); general nested expression
  conditions stay out of v0; the desugar is the already-supported
  cell + branches + join read with no new IR operation; formatter
  canonical shape and idempotence; typed refusals for unsupported operand
  shapes.
- **Bounded exit test.** Same bar as 8.1 (corpus, matrix, snapshots,
  re-baseline entry).

### 8.3 Item 3 — bare-literal re-measurement (policy item)

- **Corpus.** Owner-reviewed AI-generated Sico from the STEP-0175-era DX
  audit method, plus the selfhost sources; the corpus is frozen before
  counting.
- **Metric.** Literal-ceremony tokens (`I64.literal(...)` and friends) per
  expression and per KB, alongside the same counts for a hypothetical
  typed-literal form.
- **Output.** A decision record appended to the RFC-0046 outcome: either
  "D3 stands" with the new measurement, or the smallest typed-literal RFC
  carrying the full RFC-0033 evidence. No grammar change may precede this
  record.

### 8.4 Item 4 — `text.chars` (queued by RFC-0046 §2)

- **Measured consumer.** Compare the composed
  `text.length`/`char_at` loop against the candidate intrinsic on the
  frozen word-scanning corpora (formatter `word_kind` region, checker
  scans) — call count, allocation count and byte output.
- **Decision rule.** The intrinsic lands only if measurably better on the
  frozen corpora **and** it carries the same RFC-0033 evidence bar;
  otherwise the composed form is recorded as final and the item closes.

## 9. Sequencing and dependency map (refined 2026-09-22)

- One kickoff inventory covers all four probes (§7); after that the items
  are independent — each may proceed or close on its own RFC without
  waiting for the others.
- Ordering constraint that does not move: **implementation** of any item
  waits for the M22 S7 exit audit (§2). RFC drafting and measurement may
  run in parallel with M22 S5–S7.
- M22 interaction per accepted item: an entry in the M22
  dual-implementation register plus an explicit self-host re-baseline
  decision — the Sico frontend either consumes the new surface or a
  declared, disjoint subset is recorded; a wide operator set forces the
  parser slices to re-freeze, which is exactly why the RFC must fix the
  smallest closed set first (§5 risk 1).
- Items 1 and 2 share one diagnostic-namespace decision (append-only
  codes); landing them under one RFC-set review keeps the code registry
  coherent, but they remain separately gated items.
