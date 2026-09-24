# ADR-0017: Self-host data-model architecture — records RFC vs SOA permanence v0

> - status: proposed (R0 decision draft; **recommendation: Option A, amended 2026-09-24 per owner criteria directive** 「我只看最终效果和稳定性、长期可维护性」; pending owner acceptance)
> - date: 2026-09-24
> - owners: autonomous-agent
> - supersedes: -
> - superseded-by: -
> - depends: ADR-0015, ADR-0016, M22 plan, [`M22–M26 route replan v1`](../plans/M22-M26-route-replan-v1.md), [`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md), STEP-0264, STEP-0265, STEP-0266, STEP-0268

## Context

ADR-0016 froze struct-of-arrays (SOA) as the M22 v0 representation while
`List[record]` stays outside the executable language set, and declared adding
records "a separate RFC and not an M22 prerequisite".  The 2026-09-20 quality
review nevertheless listed the SOA transition shape as architecture debt
(P0-2): no written column specification, no generality-exit criterion, and no
convergence gate — the self-host track could neither demonstrate nor disprove
that the current path reaches S6.

STEP-0264 (route replan) therefore mandates an R0 architecture decision
before any further lowering STEP.  STEP-0265 produced the option cost table;
STEP-0266 produced the measurements: canary 22/30 formatter functions
(frontier `nearest_match`, `ERR:E-SH-IR-GWPACK-OTHER`); fuel near-linear in
input lines (no cliff through 16k lines); wall time dominated by per-function
complexity (≈2 s per formatter function on the debug runner).

The R1 numbers close the *performance* question (SOA is not fuel-blocked) and
leave the decision to be made on product grounds.  The owner set the decision
criteria on 2026-09-24: 「我只看最终效果和稳定性、长期可维护性」 — final
effect, stability and long-term maintainability.  This document was first
drafted recommending Option B under a cost-minimisation-default; STEP-0268
records the criteria evaluation that amended the recommendation to Option A.

## Decision drivers (owner-weighted)

- **最终效果 (final effect):** what the shipped language and toolchain look
  like at v1.0 and beyond — not what reaches S6 soonest.
- **稳定性 (stability):** horizon-weighted.  Short-term artifact stability
  matters less than structural stability of the canonical data model, because
  the M22 track is currently NO-GO with no working state to protect.
- **长期可维护性 (long-term maintainability):** total maintenance cost over
  the language's life, including the cost of changes everyone can already see
  coming (application-profile ergonomics, AI-generated code).
- 可验证性: every claim stays byte-exact against the Rust oracle under a
  declared canary; RFC-0033 evidence discipline is unchanged.
- 实现成本: real but explicitly demoted by the owner's criteria — it is a
  schedule input, not a quality dimension.

## Considered options

### Option A — records RFC now (recommended)

Open the full records RFC under the RFC-0033 gate: record type declarations,
field access, record literals, typed record lists (`List[record]`) — the
minimal closed set, no inheritance/generics/methods.  Re-baseline the selfhost
sources on records after RFC acceptance (M23 Route-B-style re-baseline rules:
re-run S1/S2 differentials and the canary on the new surface; ADR-0015
contract unchanged).  SOA interim discipline (ADR-0016 invariants) stays in
force until the re-baseline lands — no drift in the meantime.

Evaluation against the owner's criteria:

- *Final effect:* strictly better end state.  A language with a real
  structural data model; the self-host source layer shrinks dramatically (the
  SOA simulation is the dominant share of `parser.sico`'s 12.7k lines); the
  M14 application profile and AI-generation ergonomics unblock — the same
  RFC serves M14/M23 rather than M22 carrying the cost alone.
- *Stability:* worse short-term (a new language semantics surface inside the
  trusted Rust kernel; S6 delayed by RFC + re-baseline), better long-term.
  The delay costs no working state — M22 is NO-GO today.  The change is
  gated by the full RFC-0033 evidence bar (explicit desugar, source-map
  identity, formatter idempotence, typed diagnostics, measured consumer) and
  frozen corpora, the same discipline that carried RFC-0044/0045/0046.
- *Long-term maintainability:* decisively better, and B does not actually
  avoid the rewrite.  Under B, records pressure from M14/M23 remains (no
  record type is scheduled anywhere in M23's four items; the need does not
  disappear, it is unscheduled), so the likely B trajectory is: maintain the
  12.7k-line SOA artifact for months, then re-baseline anyway when records
  land.  B therefore pays SOA maintenance *plus* the later rewrite; A pays
  the rewrite once and ends with a single canonical data model instead of
  two parallel universes (SOA simulation + real types) in the interim.

Cost, honestly: months-scale language work across parser, semantics, IR,
verifier, codegen, formatter and diagnostics; new oracle-gap risk that the
corpus discipline must carry; S6 closure delayed by the RFC plus
re-convergence.

### Option B — SOA permanence with convergence gates (fallback)

Keep SOA as the M22 data model through S6, upgrade ADR-0016's framing to
permanent-with-revisit-conditions, and close P0-2 by contract: written column
specification; canary = `report-m22-canary.ps1` coverage; coverage-growth
metric per STEP; R3 stop-loss (4 consecutive STEPs with zero canary growth and
unmoved refusal frontier).  Records de-coupled as an M14/M23-driven RFC with
an explicit re-baseline follow-up decision if it lands.

Evaluation: cheapest path to S6 and stable short-term, but under the owner's
criteria it is dominated — same eventual rewrite, strictly more total
maintenance, and a worse product in the interim.  It remains the correct
choice only if the owner declines the Option A cost.

### Option C — continue undecided

Rejected unchanged: this is the P0-2 debt itself.

## Decision

**Recommendation: Option A** (amended 2026-09-24 per the owner's criteria
directive; the initial draft recommended B under cost-minimisation — see
STEP-0268 for the evaluation trail).  Status `proposed`, pending owner
acceptance.  Scope of the records RFC: the minimal closed set only (record
declarations, field access, literals, typed record lists); RFC-0033 gate
intact; SOA invariants remain enforced until the re-baseline lands; ADR-0015
closure contract and ADR-0016 compilation-unit contract unchanged.

If the owner selects Option B instead, this ADR records that choice as its
Decision (Option B text above), status moves to accepted, and the W1
consolidation debts of STEP-0265 §5 become mandatory queue items.

## Consequences

Positive: one canonical data model for the language; the self-host source
layer becomes typed and shrinks; the M14/M23 ergonomics backlog and
AI-generation noise get their structural fix; the review's duplication and
fingerprint-rule debts are retired by construction rather than patched; S6
re-converges on a surface that is strictly easier to lower than SOA
simulations.

Negative: S6 is delayed by the RFC plus re-baseline (M22 stays NO-GO
longer); a new semantics surface in the Rust kernel must be carried through
the whole evidence chain; the R3 stop-loss and canary machinery (STEP-0266)
remain armed and must be re-baselined onto the records surface, which is real
re-work, not a reset.

## Validation

STEP-0266 validated the R1 instrumentation; STEP-0268 records the
criteria-based evaluation amending this recommendation.  This ADR's own
validation gate if Option A is accepted: the records RFC carries the RFC-0033
four-part evidence plus a measured-consumer section on both the selfhost
corpus and an application-profile corpus; the first post-re-baseline M22 STEP
regenerates the canary report on the records surface and re-pins the baseline;
the R3 counter restarts only after that re-pin.

## Revisit conditions

- Owner declines Option A's cost → Option B text becomes the Decision and its
  canary/stop-loss machinery activates as written.
- Records RFC fails its evidence gate → fall back to Option B without
  prejudice; the SOA invariants never lapsed in the interim.
- Post-re-baseline canary shows records-surface convergence slower than the
  STEP-0266 SOA baseline for three consecutive R-reviews → explicit
  continue/rollback decision with the dual-implementation register as
  evidence.

## Links

- [`ADR-0015`](./ADR-0015-compiler-bootstrap-closure-v0.md) (closure contract)
- [`ADR-0016`](./ADR-0016-selfhost-soa-compilation-unit-v0.md) (SOA invariants, compilation-unit contract)
- [`M22 plan`](../plans/M22-compiler-self-host.md) §3.2 (R-phase queue)
- [`M22–M26 route replan v1`](../plans/M22-M26-route-replan-v1.md) §3 (R0–R3)
- [`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md) §4–§6
- STEP-0264 (replan), STEP-0265 (assessment), STEP-0266 (R1 numbers), STEP-0268 (criteria evaluation / recommendation amendment)
- [`m22-quality-review-2026-09-20`](../reports/m22-quality-review-2026-09-20.md) P0-2, §7 items 2/3/5
