# STEP-0259: M18 audit-addendum registration — no rework, flip protocol for portfolio item 4

> - status: complete / planning documentation; no gate or support change; M18 stays GO 4/5 (external pilot + portfolio item 4 NO-GO) until the registered addendum runs
> - phase: M18 planning support (owner confirmation 2026-09-22 of the assessment that the pilot re-designation does not require redoing M18)
> - completed: 2026-09-22
> - evidence: documentation-only; contract checks via `tools/validate-step-0259.ps1` (which re-runs `tools/validate-step-0124.ps1` and `git diff --check`)

## Objective

Answer the owner's question 「M18 修改了，M18 要重新完成吗？」 with a
registered decision: **no rework**. The 2026-09-22 re-designation
(STEP-0258) changed only the target of portfolio item 4 — a NO-GO item
that was never completed — so the delivered four-pilot evidence and the
STEP-0165 exit audit remain valid. What M18 needs is a bounded
exit-audit **addendum** after the M24 format-converter pilot closes, not
a re-completion.

## Registered protocol (added to M18 plan §7)

- When the M24 format-converter pilot passes M24 §3 gate 5, an M18
  exit-audit addendum is executed against the M18 plan §4 acceptance path:
  builds/runs without core patches, M19 release-bundle install, frozen
  corpus byte-stability, fail-closed refusal corpus.
- The addendum flips portfolio item 4 from NO-GO to GO, re-runs the
  standing regression (M18 §8 gate 7 at addendum time), updates the
  ROADMAP/STATUS M18 lines, and is recorded as its own STEP.
- The external pilot NO-GO is untouched: it is an owner-gated external
  gate outside M24 and outside this protocol.

## Changes

- **M18 plan** §7: the addendum-registration paragraph above.
- **ROADMAP**: the M18 status line records the registration.
- **STATUS**: `updated` and `current support step` record this STEP.
- Cross-reference: `docs/steps/README.md` gains this record's row.

## What this step deliberately does not do

- No M18 gate text, evidence class or status change: M18 stays GO 4/5
  until the addendum actually runs with evidence.
- No anticipation of the addendum's outcome: it may close item 4 or
  record an honest deferral; nothing here pre-commits the verdict.
- No STEP number is reserved for the addendum itself; it takes its own
  number when executed.

## Executed validation

- `tools/validate-step-0259.ps1`: asserts the M18 §7 addendum markers
  (no-rework rationale, flip protocol, external-pilot exclusion), the
  ROADMAP/STATUS/steps-index pointers, then re-runs
  `tools/validate-step-0124.ps1` and `git diff --check`. All green.

## Honest residuals

- The addendum is registered but unscheduled beyond its trigger (M24
  §3 gate 5); the M24 UI-library and codec-contract prerequisites still
  gate that trigger.
- Until the addendum runs, portfolio item 4 remains an explicit NO-GO
  and M18 remains GO 4/5.
