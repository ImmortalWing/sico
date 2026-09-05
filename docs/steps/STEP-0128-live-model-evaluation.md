# STEP-0128: authoritative live-model AI evaluation (DeepSeek) and ADR-0011 budget check

> - status: complete
> - phase: M13 (insertion-time step per M13 plan §5; number reserved at
>   insertion when the owner delivered credentials, 2026-09-05)
> - started: 2026-09-05
> - completed: 2026-09-05
> - owners: autonomous-agent
> - branch: dev (M13 closeout executed off main per owner instruction)

## 1. What was run

The first provider-billed evaluation under `sico-ai-eval-v1`: DeepSeek
`deepseek-chat`, 96 tasks × 30 repetitions (2880 calls, temperature 0,
top_p 1, no seed support), scoring **0.909544** (2245/2880 fully
correct), total cost **USD 0.7791** against a USD 10 hard cap (no 429s,
zero failed calls, zero dropped attempts). Full evidence, hashes and the
budget check: [`docs/reports/ai-eval-live-model-v1.md`](../reports/ai-eval-live-model-v1.md).
This run supersedes the subagent baselines for ADR-0011 budget checks
per M13 plan §5.

## 2. ADR-0011 budget verdict (authoritative, measured)

| check | result |
|---|---|
| floor overall ≥ 0.950 | **FAIL** (0.9095) |
| floor repair per candidate = 1.000 | **FAIL** (0.914 / 0.833 / 0.994) |
| floor understanding ≥ 0.950 | PASS |
| floor generation A0/B/C | **FAIL** (0.400 / 0.500 / 0.300) |
| §13 target budgets (overall ≥ 0.980 etc.) | **NOT MET** |

Per ADR-0011 §4 the budgets are not adjusted to pass; a superseding ADR
against a newer measured baseline would be required, and none is reserved
here.

## 3. M13 closure audit impact (STEP-0123 refresh)

| §13 criterion | was | now |
|---|---|---|
| (a) query protocol stability | GO | GO (unchanged) |
| (b) structured stable diagnostics | GO | GO (unchanged) |
| (c) reproducible benchmarks | GO | GO (unchanged) |
| (d) numeric quality budgets met | blocked-external-evidence | **measured NO-GO** — the authorized run exists; budgets are not met |

M13's process gates are all closed: every planned step plus this
insertion step is complete, and the one externally gated question ("do
the budgets hold under a real model?") now has a definitive measured
answer: **no, not with deepseek-chat at the frozen scorer/corpus**.
The dominant failure class remains byte-exact canonical formatting
(`canonical-source-mismatch` ×540), with a new genuine
`canonical-repair-mismatch` class (×93, concentrated in B-repair) that
the subagent baselines never exhibited.

## 4. Consequences and residual options (owner decisions; no STEP reserved)

- The AI-workflow quality criterion stays an open product goal, now with
  a real measured gap instead of an unknown. Candidate levers, each
  requiring its own step and possibly a superseding ADR: guide/canonical
  output improvements, a C-candidate formatter (settles the C layout
  failure class), a different model/snapshot, or scorer-surface changes
  (STEP-0119 already rejected normalization).
- `observed_ai_frequency` in `error-taxonomy.json` keeps the STEP-0122
  measured frequencies; refreshing provenance to this run is a
  protocol-semantics change left to the owner.

## 5. Red-line compliance

- Key handled via a gitignored env file (`target/`, never in argv,
  logs, diagnostics or Git); raw outputs stay in gitignored
  `target/ai-eval-experiments/`.
- No synthetic data in the `kind: model` run; raw outputs unmodified;
  scoring only via the repository scorer.
- Smoke (96×1, USD 0.026) preceded the full run; the full-run cost
  estimate (~USD 0.8) was computed from smoke usage and sat far below
  the cap, per TASK §5.

## 6. Validation

Offline toolchain green on the dev branch before the run
(`validate-ai-eval`, `test-ai-eval`, `validate-error-taxonomy`);
adapter dry-run (mock, 2880 attempts) accepted by the scorer before any
billable call; smoke run scored before the full run.
