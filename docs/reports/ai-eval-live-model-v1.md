# Report: authoritative live-model AI evaluation v1 (DeepSeek)

> - protocol: `sico-ai-eval-v1`, scope=full, 96 tasks × 30 repetitions
> - run date: 2026-09-05
> - evidence level: **measured** (`kind: model`, `synthetic: false`,
>   `official_comparison: true` — the first provider-billed run; this
>   supersedes the subagent baselines for ADR-0011 budget checks per
>   M13 plan §5)
> - owner authorization: DeepSeek API key delivered 2026-09-05 with
>   delegation to close out M13 ("你去收口"); numeric budget cap applied
>   by the adapter: **USD 10** (hard stop), owner-visible in this report

## 1. Model identity and run parameters

| field | value |
|---|---|
| provider | `deepseek` (https://api.deepseek.com, OpenAI-compatible chat completions) |
| model name | `deepseek-chat` |
| served version | `deepseek-chat` (the provider reports the alias only; no dated snapshot string is exposed in the API response — recorded exactly as reported) |
| temperature | 0 |
| top_p | 1 |
| seed | not supported by the API → `seed_supported: false`, `seed: null` |
| concurrency | 4 in-flight requests; exponential backoff on 429 (no 429 observed) |
| prompt packets | `target/ai-eval-experiments/prompt-packets.json`, SHA-256 `8e70e0f44c588b92cc7c3623cdce3c79f8bc7c6e8fe7eb2c825a528e757f1eec` (96 tasks; differs from the 2026-07-29 packet hash because STEP-0119 revised the guides) |

## 2. Cost and tokens (provider-reported tokens; adapter-measured cost)

| metric | value |
|---|---|
| input tokens | 2,223,990 |
| output tokens | 162,514 |
| total cost | **USD 0.7791** (adapter-computed from provider-reported tokens at DeepSeek published rates 0.27/1.10 USD per 1M in/out; budget cap USD 10 not approached) |
| latency | p50 632 ms, p95 1043 ms per call; ~9 min wall clock at 4 workers |
| failed calls | 0; dropped attempts: 0 |

## 3. Results (score produced by `tools/score-ai-eval.ps1`; never hand-edited)

Overall **score 0.909544** (2245/2880 fully correct; 6385/7020 points).

| category | A0 | B | C |
|---|---:|---:|---:|
| generation | 0.400 | 0.500 | 0.300 |
| repair | 0.914 | 0.833 | 0.994 |
| understanding | 1.000 | 1.000 | 0.999 |

Failure classes across 2880 attempts: `canonical-source-mismatch` ×540,
`canonical-repair-mismatch` ×93, `understanding-field-mismatch` ×2
(540+93+2 = 635 = 2880−2245, self-consistent).

Determinism: 70/96 tasks byte-identical across all 30 repetitions
(temperature 0 does not make the served snapshot fully deterministic).

Artifacts (gitignored, `target/ai-eval-experiments/`):
`full/run-deepseek-full-2026-09-05.json` SHA-256
`f94080422ea00c297eefb3e44e8ef7bc0a761afd3ce1f7724e313dc65311bbb9`;
`full/score-full.json` SHA-256
`c545555286c060d28ed5803be34f3f47d7e7d1e674f0cc77712e576f3118a2cd`;
smoke run (96×1, score 0.9103) under `smoke/`; adapter
`adapter/live_model_eval.py`; raw outputs retained inside the run files.

## 4. ADR-0011 budget check (authoritative)

| budget | required | measured | verdict |
|---|---|---|---|
| floor: overall ≥ 0.950 | 0.950 | 0.9095 | **FAIL** |
| floor: repair per candidate = 1.000 | 1.000 | 0.914 / 0.833 / 0.994 | **FAIL** |
| floor: understanding ≥ 0.950 | 0.950 | 1.000 / 1.000 / 0.999 | PASS |
| floor: generation A0/B/C ≥ 0.950/0.850/0.550 | — | 0.400 / 0.500 / 0.300 | **FAIL** |
| target: overall ≥ 0.980 | 0.980 | 0.9095 | **NO** |
| target: generation per candidate ≥ 0.950 | 0.950 | 0.400 / 0.500 / 0.300 | **NO** |
| target: repair per candidate = 1.000 | 1.000 | 0.914 / 0.833 / 0.994 | **NO** |
| target: understanding per candidate ≥ 0.980 | 0.980 | 1.000 / 1.000 / 0.999 | PASS |

**Verdict: the authoritative live-model run does not meet the ADR-0011
floor budgets (hard failure) and therefore not the §13 target budgets.**
Per ADR-0011 §4 the budgets are NOT adjusted to pass; any budget change
requires a superseding ADR against a newer measured baseline.

## 5. Interpretation (labeled `inferred`, non-scoring)

- Byte-exact canonical formatting remains the dominant failure
  (`canonical-source-mismatch` 540; generation stays hardest, as in both
  subagent baselines). DeepSeek's outputs are semantically close but not
  canonically formatted.
- Unlike the subagent baselines (repair 1.000), DeepSeek shows a real
  `canonical-repair-mismatch` class (93, concentrated in B-repair),
  i.e. it reformulates rather than reproduces the canonical repair.
- Understanding is essentially solved (0.999–1.000).

## 6. Deviations and honesty notes

- Budget cap USD 10 was set by the adapter under the owner's delegation
  ("你去收口") because the TASK file's numeric-authorization requirement
  had no number supplied; actual spend was USD 0.78, 92% under the cap.
- The served-snapshot capture in `rate_limits` is empty (the adapter
  recorded the alias `deepseek-chat` as `model.version` instead); the
  provider exposes no dated snapshot in the API response, so the alias +
  run date is the closest immutable identity obtainable.
- `observed_ai_frequency` in `error-taxonomy.json` keeps the STEP-0122
  measured frequencies (subagent provenance); refreshing it with this
  run's frequencies is a protocol-semantics change left to the owner.
- No credentials, no raw-output files, and no cost artifacts entered Git.
