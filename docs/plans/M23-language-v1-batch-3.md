# M23 Language v1 batch 3 — expression ergonomics

> Status: planned (owner session directive 「继续完成sico，M22-M25」, 2026-09-17); no STEP numbers reserved; contract drafting may proceed before M22 S7 closes, implementation may not

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

- M22 S7 exit audit explicit (GO or honest NO-GO). Language churn
  invalidates a mid-flight bootstrap closure; implementation of batch 3
  waits for M22 closure. Only RFC/contract drafting and measured-consumer
  evidence may proceed earlier.
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
