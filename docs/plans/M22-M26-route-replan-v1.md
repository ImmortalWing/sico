# M22–M26 route replan v1 — convergence-gated self-host, dual-exit batch 3

> Status: registered by STEP-0264 (owner directive 2026-09-24
> 「重新规划路线，审视M22-M26，确保能落实到工程实现」); planning-only
> document — it re-slices scheduling and gates, freezes no contract by
> itself; the R0 architecture decision it mandates is a separate ADR/RFC;
> no STEP numbers reserved.

## 1. Trigger and evidence

The 2026-09-20 quality review (`docs/reports/m22-quality-review-2026-09-20.md`)
measured three structural problems with the M22→M26 route as scheduled:

1. **M22's execution shape had no convergence evidence.** STEP-0213→0262
   (50 implementation STEPs) advanced the lowering frontier one syntax
   shape at a time; the review's verdict was "not a compiler, a shape
   enumeration machine" with generalization radius ≈ 1 sample, and "the
   current path is substantively blocked; re-slicing required".
2. **The serial gate M22(S6/S7) → M23 created a self-binding feedback
   loop.** M23 (batch-3 language ergonomics) is the measured remedy for
   exactly the surface poverty that inflates the self-host code base
   (M22 plan §8), yet M23 implementation was gated behind the closure
   that the impoverished surface makes slower to reach.
3. **M24's entry gates never depended on M22/M23**, yet the roadmap
   dependency shape narrated M23→M24→M25 as a serial main line, leaving
   the vision-closure and UI-library work idle behind an unrelated gate.

This replan does not claim M22 S6 is reachable or unreachable. It makes
that question **decidable with numbers**, and removes the serial
scheduling risk. Every claim below is a scheduling/gate change only;
capability contracts, evidence classes and RFC/ADR discipline are
untouched.

## 2. The restructure in one view

```
                 ┌─ Route A: M22 S7 GO ──────────────→ M23 implements batch 3
M22 R-phase ────┤                                     (self-host re-baseline)
(convergence    │
 gate + stop-   └─ Route B: stop-loss declared ──────→ M23 implements batch 3
  loss)                (S6 not converging on            immediately; M22 restarts
                       current surface)                 re-baselined after M23

M24 ── independent of M22/M23 gates: kickoff inventory may start now
M25 ── entry gate now accepts "M22 verdict registered (GO or stop-loss)" via
       the dual-implementation register; exit gates unchanged
M26 ── unchanged (registered STEP-0263); GUI gate still rides M24 §8.6.1
```

## 3. M22 re-slice: R-phase convergence gate before further lowering STEPs

The per-shape queue (M22 plan §3.2) is suspended as the *organizing
principle* — its items remain the technical content, but they now run
**under** the R-phase gate instead of open-endedly.

- **R0 — architecture decision (one bounded ADR or RFC, before any new
  lowering STEP).** Decide between: (a) an RFC adding executable
  `List[record]` support (RFC-0033-gated; the selfhost corpus is the
  measured consumer), or (b) an ADR freezing struct-of-arrays as the
  permanent self-host architecture with a written specification. Either
  outcome must define the three things the review found missing: the
  canary, the coverage-growth metric, and the failure threshold. This
  closes review item P0-2 by decision instead of drift.
- **R1 — instrumentation (measurement only, no lowering).**
  - Canary: extend the STEP-0261-style validator to report
    `formatter.sico` function coverage (byte-exact functions / 30) and
    the typed refusal frontier, recorded per STEP.
  - Budget curve: a probe on 1k/4k/8k-line corpus inputs recording
    **consumed** fuel, stack high-water and wall time — replacing
    inference from configured caps (closes review P0-3: limits are
    evidence, not configuration).
  - Compilation-unit context: the bounded entry-plus-imports source-set
    contract (review P0-1) is drafted here so the selfhost sources can
    enter the canary corpus as a set, not rejected per-file as `E8010`.
- **R2 — convergence under the gate.** The §3.2 queue resumes. Every
  implementation STEP must either advance canary function coverage or
  move the typed refusal frontier; a STEP that does neither is a
  process violation, not progress.
- **R3 — stop-loss (pre-registered, numbers fixed here).** If **4
  consecutive implementation STEPs** produce zero `formatter.sico`
  canary growth AND an unmoved refusal frontier, M22 S6 convergence is
  declared *not demonstrated on the current surface* — a typed, honest
  verdict recorded in the dual-implementation register. This triggers
  Route B (§4). The stop-loss count is evaluated at each R-review;
  budget-curve evidence (fuel superlinear beyond the 4k→8k step by an
  order of magnitude against runner limits) may trigger the same review
  early.

S5 widening, S6 closure (ADR-0015 `A == B == C`, M7 packaging, budget
record) and S7 exit audit are unchanged in content — they execute after
the R-phase, under the same contracts as before.

## 4. M23: dual-exit implementation gate

M23 plan §2's first entry condition ("implementation of batch 3 waits
for M22 closure") is amended to two explicit routes:

- **Route A — M22 S7 GO.** Batch 3 implements; the self-host
  re-baseline decision (M23 exit gate 5) follows on a proven closure.
- **Route B — M22 stop-loss declared.** Batch 3 implementation may
  start immediately: its measured consumers already exist (M23 plan
  §8.1–8.4, measured against the selfhost corpus), and the richer
  surface is precisely what a re-based self-host needs. M22 then
  re-enters as a new attempt re-baselined on the batch-3 surface after
  the M23 exit audit (S1/S2 differentials re-run; ADR-0015 contract
  unchanged; the prior stop-loss verdict stays in the register).

The original rationale for the gate — language churn invalidates a
mid-flight closure — is preserved in both routes: Route B fires only
after convergence is declared absent, never while a closure attempt is
in flight. RFC drafting and measured-consumer evidence remain
unblocked in both routes (they were already parallelizable).

## 5. M24: parallel start, concretized UI-library sequence

M24's entry gates (M16 GO, M17 gate 1, M7 boundary) are independent of
M22/M23. The kickoff inventory (M24 plan §7) may start immediately, in
parallel with the M22 R-phase; the roadmap narrative "M23→M24" is a
planning convenience, not a dependency.

The §8.6.1 Sico-language native UI library workstream gets a fixed
contract-first sequence so it cannot drift: (1) RFC freezing the
source-level binding surface and typed refusals; (2) ADR freezing the
renderer architecture (AGENTS.md §4 boundaries); (3) deterministic
renderer frame corpus; (4) the format-converter pilot on top. Each
stage's exit test stays as already written in M24 §3/§8.

## 6. M25: entry gate accepts an honest M22 verdict

M25's entry gate keeps "M23 exit audit explicit" and "M24 pilot gate
explicit". It gains one clarification: the M22 row of the
dual-implementation register (M25 exit gate 5) must carry an explicit
verdict — GO, or stop-loss-declared with the Route-B re-baseline
registered — and either satisfies the entry gate. A missing M22 verdict
satisfies nothing (M25's existing rule: missing owner inputs are named
deferrals, never implied success).

## 7. M26: unchanged

M26 (STEP-0263) keeps its gates, including the 2026-09-24 owner
scheduling rule that UI-library gaps close under M24 §8.6.1 before the
GUI gate starts. This replan adds nothing to it.

## 8. First executable action per milestone (落实到工程实现)

| Milestone | First action | Blocked by |
|---|---|---|
| M22 R0 | Draft the SOA-freeze vs `List[record]`-RFC decision document | nothing (document work) |
| M22 R1 | Extend the STEP-0261 validator with canary coverage reporting; land the 1k/4k/8k budget probe | R0 direction (instrumentation shape) |
| M23 | Kickoff inventory STEP (four probes, M23 plan §7) | nothing (measurement only) |
| M24 | Kickoff inventory STEP (M24 plan §7), then UI-library RFC | nothing |
| M25 | Inventory diff of the §13 table (M25 plan §7) | M23/M24 exit audits for the real run; drafting may proceed |
| M26 | Kickoff inventory STEP (M26 plan §7) | nothing for the package slices; GUI gate rides M24 §8.6.1 |

The intent: after this replan, every milestone has an unblocked,
bounded, executable next action that is not a STEP-number reservation.

## 9. Entry gate / Exit gates / Non-goals

**Entry gate (this replan document):** owner directive registered as
STEP-0264; quality-review findings P0-2/P0-3 and §5.1 accepted as the
evidence basis; no capability contract, evidence class or RFC/ADR
discipline is modified by this document itself.

**Exit gates:** R0 decision document accepted (ADR or RFC per §3);
M22/M23 plan texts amended to this replan's gate structure; ROADMAP
dependency shape updated; `validate-step-0124.ps1` green; the R1
instrumentation landed before any post-replan lowering STEP.

**Non-goals:**

- No claim that M22 S6 is reachable or unreachable (that is what the
  R-phase is for).
- No language-surface change (R0 option (a) is an RFC proposal, not a
  landed change).
- No retirement or demotion of the Rust oracle; no performance claim.
- No re-opening of M24/M26 owner-directed scope decisions.
- No STEP numbers reserved by this document.

## 10. Risks

- **Stop-loss gaming.** A STEP could "advance the frontier" with a
  trivial shape while the hard region stalls. Mitigation: the R-review
  reads coverage growth *and* refusal-frontier identity, and the
  reviewer's judgment stays in the loop; the 4-STEP window is a
  trigger for review, not an automatic verdict.
- **Route B doubles re-baseline work.** Batch 3 lands, then the
  self-host re-freezes. This is the declared cost of breaking the
  feedback loop, and it is bounded: S1/S2 differentials are corpus
  re-runs, not rewrites.
- **Parallel M24 competes for the same evidence host.** The M22 R1
  budget probe and the M24 kickoff inventory both need runner time;
  schedule them in different sessions (the handoff quick-loop already
  documents the GNU flow).
