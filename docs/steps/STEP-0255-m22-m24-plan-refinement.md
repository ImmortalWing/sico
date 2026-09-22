# STEP-0255: M22–M24 plan refinement — slice status, execution queue, per-item protocols

> - status: complete / planning documentation; no gate or support change; M22 NO-GO unchanged
> - phase: M22–M24 planning support (owner request 2026-09-22 「细化M22-M24文档和step」)
> - completed: 2026-09-22
> - evidence: documentation-only; contract checks via `tools/validate-step-0255.ps1` (which runs the STEP-0124 planning contract and `git diff --check`)

## Objective

Refine the three active-track planning documents (M22 in-progress, M23/M24
planned) into an execution-ready state without reserving future STEP
numbers, changing any entry/exit gate, or widening any declared support
claim.

## Changes

- **M22 plan** (`docs/plans/M22-compiler-self-host.md`): the status line
  now records STEP-0254's harness reuse; new §3.1 freezes the measured
  slice-status table (S0–S7 with closed evidence and remaining bounded
  work each); new §3.2 defines the execution queue from the current typed
  frontier (`item`'s `sico.list.get` result match) through the formatter
  tail, the remaining selfhost sources, S5 widening, S6 bootstrap closure
  and the S7 audit, each with a bounded exit test and the standing
  refusal-first discipline; §5 gains the validation-host toolchain note;
  §7 gains the host-toolchain reproducibility risk.
- **M23 plan** (`docs/plans/M23-language-v1-batch-3.md`): new §8 gives
  each of the four items a per-item work protocol — measured-consumer
  method, RFC proof obligations, bounded exit test — and new §9 fixes the
  sequencing/dependency map (one kickoff inventory, independent per-item
  RFCs, implementation blocked on M22 S7, per-item self-host re-baseline
  entries).
- **M24 plan** (`docs/plans/M24-vision-closure-block-game-pilot.md`): new
  §8 breaks the exit gates into work packages — M17 gate 2 ADR content
  and tolerance contract, M17 gate 4 manifest schema and evidence set,
  roster-package per-package bar, the block-game pilot's observe→plan→
  preview→execute-one→verify/stop capability mapping table, and the
  enumerated fail-closed error-injection corpus.
- Cross-references: `docs/steps/README.md` gains this record's row;
  STATUS records this as the current support step; the ROADMAP M22–M24
  sections gain one-line pointers to the refined sections.

## What this step deliberately does not do

- No future STEP number is allocated in any plan or in the ROADMAP; the
  STEP-0124 planning contract (`no STEP numbers reserved`) stays intact.
- No entry/exit gate, evidence class, non-goal or capability contract is
  added, removed or weakened; §8 in the M22 plan (batch-3 candidates) is
  untouched because M23 consumes it verbatim.
- No language, runtime or platform support claim changes; M22 stays
  NO-GO with the canary frontier unchanged (`E-SH-IR-CALL-TARGET` at
  `item`).

## Executed validation

- `tools/validate-step-0255.ps1`: asserts the new sections and preserved
  invariants in all three plans, re-runs `tools/validate-step-0124.ps1`
  (M14–M25 planning contract, ADR-0015 closure record, ROADMAP ordering
  and step-number reservation check) and `git diff --check`. All green.

## Honest residuals

- The M22 §3.2 queue and the M23 §8/§9, M24 §8 breakdowns are planning
  orderings, not commitments; each work item still takes its own STEP
  with a bounded exit test when its entry gate is satisfied.
- The M23 measured-consumer tables and the M24 frozen corpus plan are
  defined here but measured only in each milestone's kickoff inventory.
- The GNU-toolchain environment repair recorded in the M22 plan §5/§7
  remains open (STEP-0254 residual).
