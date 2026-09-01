# STEP-0120: AI quality-budget ADR (ADR-0011)

> - status: complete
> - phase: M13 (parallel support track)
> - started: 2026-09-01
> - completed: 2026-09-01
> - owners: autonomous-agent

## 1. Objective

Fix the numeric quality budgets that `AGENT_GOAL.md` §11/§13 require once a real baseline exists, as `ADR-0011`: per-category minimum scores, evaluation scope and repetition count, the authoritative scorer mode, and the explicit split between subagent-run non-regression floors and the completion-gate targets that only an authorized live-model run can prove.

## 2. Context and evidence

- [`STEP-0119`](./STEP-0119-ai-generation-quality-baseline.md): re-measured baseline (96 tasks × 30 repetitions, byte-exact scorer, subagent-as-model) — generation A0 1.000 / B 0.900 / C 0.600, repair 1.000 ×3, understanding 1.000/0.982/1.000, overall 0.974359. Scorer normalization was rejected; the byte-exact scorer is authoritative.
- [`M13 plan`](../plans/M13-ai-tooling-closure.md) STEP-0120 deliverable and exit evidence: budgets reference the STEP-0119 baseline, stated as pass/fail thresholds usable by STEP-0123; anything the subagent baseline cannot justify defers to the live-model run.
- `AGENT_GOAL.md` §11: 数值预算在有真实基线后通过 ADR 确定。不得为了漂亮数据删除困难案例。 §13 AI 工作流: 结果达到项目正式确定的质量门槛。

## 3. Scope

In scope: the ADR, its registry entries, a validator proving ADR/baseline consistency (floors ≤ measured baseline; targets honestly marked unproven), registry updates.

Out of scope: changing the scorer or task set (frozen by STEP-0119), any live-model claim (no credentials), guide iteration on the residual failures (STEP-0121+ or a later guide step).

## 4. Decision shape

Two-tier budgets, recorded in the ADR:

- **Floor budgets (non-regression floors)**: set at or below the measured subagent baseline so any future subagent run that drops below them is a hard failure. These gate every ai-eval run from now on.
- **Target budgets (completion gate)**: the thresholds that satisfy `AGENT_GOAL.md` §13's "项目正式确定的质量门槛". Where the measured baseline does not meet a target (generation B 0.900 vs 0.95, C 0.600 vs 0.90), the target is asserted as the goal but its *proof* is deferred to the authorized live-model evaluation per M13 §5 — the subagent run cannot promote it.

## 5. Plan

1. This step doc + `docs/adr/ADR-0011-ai-quality-budgets.md`.
2. `tools/validate-step-0120.ps1`: ADR presence/consistency, floor-vs-baseline arithmetic, deferral honesty check.
3. Registries (adr/README, steps/README, STATUS, M13 plan), validation, commit.

## 6. Changes

- `docs/adr/ADR-0011-ai-quality-budgets.md` (new, accepted): two-tier budgets. Authoritative configuration frozen: `sico-ai-eval-v1`, 96 tasks × 30 repetitions, byte-exact scorer, recorded model identity per run, subagent runs never count as official. Floor budgets (non-regression, gate every run): overall ≥ 0.950, generation A0 ≥ 0.950 / B ≥ 0.850 / C ≥ 0.550, repair = 1.000, understanding ≥ 0.950 — all at or below the measured STEP-0119 baseline. Target budgets (the AGENT_GOAL §13 completion gate): overall ≥ 0.980, generation per candidate ≥ 0.950, repair = 1.000, understanding ≥ 0.980 — the subagent baseline does not meet them, so their proof is deferred to the authorized live-model evaluation; until then §13's quality criterion reports `blocked-external-evidence`. No task deletion/re-scoring to pass a budget; budget changes need a superseding ADR against a newer baseline.
- `docs/adr/README.md`: ADR-0011 registered; next free number ADR-0012.
- `tools/validate-step-0120.ps1` (new): ADR presence/acceptance, floor-vs-baseline arithmetic, deferral honesty (no target may be claimed met from subagent runs), registry consistency.
- `docs/steps/README.md`, `docs/STATUS.md`, `docs/plans/M13-ai-tooling-closure.md`: registration and status.

## 7. Validation

Executed 2026-09-01: `tools/validate-step-0120.ps1` green (`STEP_0120_OK adr-0011=accepted floors<=baseline targets=deferred-live-model registry=consistent`). Documentation-only step: no code changed, no compiler/runtime behavior affected; the M11 mainline claims are untouched.

## 10. Audit links

- [`M13 plan`](../plans/M13-ai-tooling-closure.md)
- [`STEP-0119`](./STEP-0119-ai-generation-quality-baseline.md)
