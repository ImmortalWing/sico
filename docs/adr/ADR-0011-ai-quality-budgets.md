# ADR-0011: AI workflow quality budgets v0

> - status: accepted
> - date: 2026-09-01
> - phase: M13 STEP-0120

## Context

`AGENT_GOAL.md` §11 requires numeric quality budgets fixed by ADR once a real baseline exists, and §13 makes "结果达到项目正式确定的质量门槛" a completion criterion. The baseline exists: STEP-0119 re-measured the 96-task × 30-repetition corpus under the byte-exact scorer (subagent-as-model, engineering feedback only — subagent runs never produce official model scores per `docs/STATUS.md` §6).

Measured baseline (v2 run, 2026-08-04):

| category | A0 | B | C |
|---|---:|---:|---:|
| generation | 1.000 | 0.900 | 0.600 |
| repair | 1.000 | 1.000 | 1.000 |
| understanding | 1.000 | 0.982 | 1.000 |

Overall 0.974359 (2700/2880 attempts; 6840/7020 points).

## Decision

### 1. Authoritative evaluation configuration

- protocol `sico-ai-eval-v1`, the fixed 96-task set, 30 deterministic repetitions per task;
- scorer: byte-exact canonical comparison, no formatter normalization (STEP-0119 rejected normalization: non-uniform across candidates and it would weaken the canonical-source property that `validate_fix` digests rely on);
- every run records model identity/version and raw responses; subagent-as-model runs are engineering feedback and never satisfy a target budget.

### 2. Floor budgets (non-regression, gate every ai-eval run)

Set at or below the measured subagent baseline; a run dropping below any floor is a hard failure:

| metric | floor |
|---|---:|
| overall | ≥ 0.950 |
| generation A0 | ≥ 0.950 |
| generation B | ≥ 0.850 |
| generation C | ≥ 0.550 |
| repair (per candidate) | = 1.000 |
| understanding (per candidate) | ≥ 0.950 |

### 3. Target budgets (the §13 completion gate)

| metric | target |
|---|---:|
| overall | ≥ 0.980 |
| generation (per candidate) | ≥ 0.950 |
| repair (per candidate) | = 1.000 |
| understanding (per candidate) | ≥ 0.980 |

The subagent baseline does not meet the generation targets (B 0.900, C 0.600) nor the overall target (0.974359 < 0.980). Per M13 §5, **target budgets can only be proven by the authorized live-model evaluation**; until owner-delivered credentials produce that run, the §13 quality criterion is reported as `blocked-external-evidence`, never inferred from subagent runs. Guide/scorer improvements (e.g. a C formatter settling the C layout failure class) may raise the subagent baseline meanwhile, but no subagent run promotes a target to "met".

### 4. Rules

- No task may be deleted or re-scored to make a budget pass (AGENT_GOAL §11); corpus changes require their own step and a re-measured baseline.
- Budget changes require a superseding ADR referencing a newer measured baseline.
- The floor budgets bind every future ai-eval run regardless of model identity.

## Consequences

STEP-0123 can score the §13 quality criterion mechanically: floors gate engineering runs; targets gate the completion claim and are currently `blocked-external-evidence`. The C-generation gap (0.600) is now a named, budgeted deficit instead of an anecdote.
