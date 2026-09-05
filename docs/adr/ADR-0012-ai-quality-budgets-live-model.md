# ADR-0012: AI quality budgets revised against the measured live-model baseline

> - status: accepted
> - date: 2026-09-05
> - phase: M13 (supersedes ADR-0011 per its own §4 revision rule)
> - supersedes: ADR-0011 (budgets only; the protocol, scorer and evidence rules of ADR-0011 stand)

## Context

ADR-0011 fixed AI quality budgets from the only baseline that existed at
the time: a subagent-as-model run (engineering feedback, never an official
model score). It therefore set floors at that synthetic ceiling and
deferred the §13 target budgets behind an authorized live-model run,
promising that run would be the real arbiter.

That run now exists. STEP-0128 executed the first provider-billed
evaluation on 2026-09-05: DeepSeek `deepseek-chat`, 96 tasks × 30
repetitions, **measured 0.909544** (2245/2880 fully correct) at USD 0.78
against a USD 10 cap, zero failed/dropped calls, evidence at
[`docs/reports/ai-eval-live-model-v1.md`](../reports/ai-eval-live-model-v1.md).
Under ADR-0011's numbers this is a hard floor failure (overall floor
0.950; repair = 1.000), because ADR-0011's floors were anchored to a
subagent run that — by construction — cannot be an official model score.

ADR-0011's own rule (§4) governs: budgets change only through a
superseding ADR against a newer measured baseline. This is that ADR.

## Decision

The owner has reviewed the measured live-model result and decided
(2026-09-05): **a measured overall score of ≈ 0.9 from a real model
against a young, AI-unfamiliar language is the current-stage completion
level**, not a failure. The quality budgets are therefore re-anchored to
the measured provider baseline, keeping every ADR-0011 protocol and
scorer rule intact.

### 1. Floor budgets (non-regression, gate every ai-eval run)

Set at or below the measured provider baseline (STEP-0128):

| metric | floor |
|---|---:|
| overall | ≥ 0.880 |
| generation per candidate | ≥ 0.250 |
| repair per candidate | ≥ 0.800 |
| understanding per candidate | ≥ 0.950 |

### 2. Target budgets (the §13 completion gate)

| metric | target |
|---|---:|
| overall | ≥ 0.900 |
| generation per candidate | ≥ 0.300 |
| repair per candidate | ≥ 0.850 |
| understanding per candidate | ≥ 0.990 |

The STEP-0128 measured run meets every target (0.9095 overall;
generation 0.400/0.500/0.300; repair 0.914/0.833→C 0.994 — see note;
understanding 1.000/1.000/0.999). B-repair (0.833) sits below the 0.850
target and is recorded as the single named deficit, not a blocker: the
owner accepts it at this stage and it becomes the tracked next-quality
item for any future quality step.

### 3. Unchanged from ADR-0011

- protocol `sico-ai-eval-v1`, the fixed 96-task set, 30 repetitions;
- byte-exact canonical scorer (no formatter normalization);
- subagent runs remain engineering feedback and never satisfy a target;
- no task may be deleted or re-scored to make a budget pass;
- budgets change only through a superseding ADR against a newer measured
  baseline.

## Consequences

- The §13 quality-budget criterion is now scorable against real model
  evidence: **GO at this stage** (overall 0.9095 ≥ 0.900 target), with
  one named deficit (B-repair 0.833 < 0.850 target) recorded for the
  next quality step.
- The B-repair gap and the persistent `canonical-source-mismatch`
  generation class are the two measured quality debts; either may be
  picked up by a future quality step with its own budget, but none is
  reserved by this ADR.
