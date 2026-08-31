# STEP-0119: AI generation-quality attribution, guide/prompt correction and re-measured baseline

> - phase: M13 (parallel support track)
> - status: complete
> - date: 2026-08-04
> - plan: [`M13 AI tooling closure`](../plans/M13-ai-tooling-closure.md)

## Result

Attribution over the 2026-07-29 hy3 run (2,880 attempts, 810 failures) showed `canonical-source-mismatch` ×750 = 25 unique generation tasks × 30 deterministic repetitions, composed of: 150 attempts (20%) formatting-only differences, ~540 (~72%) surface token deviations induced by prompt wording and guide under-specification, and 60 (8%) hard generation errors. A formatter-normalization experiment proved scorer-side normalization could recover at most 1 B task (A0/C have no formatter), and one fixture (`syntax-candidates/b/result-mapping/valid/total-error-map.sico`) was itself non-canonical.

Decision (recorded per M13 §4): **the byte-exact scorer stays unchanged**. Normalization would recover only 20%, cannot be uniform across candidates, and would weaken the canonical-source property that `validate_fix` digests and deterministic builds rely on. Instead the dominant prompt/guide bucket was fixed:

- `ai-eval/tasks/generation.json`: pseudo-type phrasing that models copied verbatim (`Result of Int or ParseError`, `Stream of Int or ReadError`, `List of Int`, `spawn compute(1) as first`) rewritten as plain prose with named-payload and no-body-member hints; match-arm qualification stated explicitly.
- `ai-eval/guides/{a0,b,c}.md`: added canonical-layout sections (2-space indent, one blank line between top-level declarations, no comments, single trailing newline, byte-for-byte comparison warning), member-signature rules (`field` keyword in B records, bare no-body members in capability/resource/interface), qualified match patterns and payload construction, spawn/await binding forms, and one complete example program per candidate. The B example was verified to parse and to be byte-identical under `sico format`.
- `syntax-candidates/b/result-mapping/valid/total-error-map.sico`: made canonical (`function (problem)` per the formatter); `syntax-mutations/b/MUT-005-missing-enum-close.sico` re-synced to preserve the single-point-mutation invariant; `syntax-candidates/metrics-v1.json` regenerated (the snapshot was already stale on HEAD — `expected 54 A0 files, found 58` — the regeneration absorbs both this fix and the in-flight STEP-0104 corpus additions, so STEP-0104 must regenerate it again if its corpus changes further).

Re-measured baseline (subagent-as-model, scope=full, 96 tasks × 30 repetitions, `official_comparison=true`, model version `subagent-runtime-2026-08-04`, deterministic: one answer per task replicated across 30 repetitions, same methodology as the hy3 run):

| syntax | category | hy3 score | v2 score |
|---|---|---:|---:|
| A0 | generation | 0.000 | **1.000** |
| B | generation | 0.200 | **0.900** |
| C | generation | 0.300 | **0.600** |
| A0/B/C | repair | 1.000 | 1.000 |
| A0 | understanding | 0.982 | **1.000** |
| B | understanding | 1.000 | 0.982 |
| C | understanding | 1.000 | 1.000 |
| overall | | **0.884615** | **0.974359** (2700/2880, points 6840/7020) |

Residual failures (6 unique tasks × 30 = 180 attempts, all `canonical-source-mismatch` except the last):

- `GEN-B-RESULT-002`: lambda parameter typing (`function (problem: ParseError)` vs untyped) and positional payload construction — guide does not pin lambda parameter typing;
- `GEN-C-MATCH-001`, `GEN-C-RES-002`, `GEN-C-STREAM-002`: C has no formatter, so one-line vs multi-line enum layout is guide-text ambiguity (the guide permits `enum Color { Red, Green, Blue }` but cannot state when canonical form chooses it);
- `GEN-C-RESULT-002`: enum-variant construction qualification (`MissingInput` vs `AppError.MissingInput`) — guide wording under-specifies qualified construction;
- `UNDERSTAND-B-TASK-002`: `spawned_calls` field shape ambiguity (call expressions vs bound task names).

These are registered as follow-up candidates for the M13 track (guide iteration needs one more re-run; a C formatter would settle the C layout class permanently). They are not hidden by regrading: the scorer stayed byte-exact.

## Evidence

- dataset changes: `ai-eval/tasks/generation.json`, `ai-eval/guides/{a0,b,c}.md`, `syntax-candidates/b/result-mapping/valid/total-error-map.sico`, `syntax-mutations/b/MUT-005-missing-enum-close.sico`, `syntax-candidates/metrics-v1.json`;
- validators: `tools/validate-ai-eval.ps1` (`AI_EVAL_DATASET_OK tasks=96`), `tools/test-ai-eval.ps1` (`AI_EVAL_TEST_OK deterministic=true`), `tools/measure-syntax-candidates.ps1 -CheckPath` (`SYNTAX_METRICS_OK`), `cargo +1.97.0-x86_64-pc-windows-gnu test -p sico-format`/`-p sico-parser` pass;
- run artifacts (gitignored): attribution `target/step-0119-analysis/analyze.py`, `fmt_experiment.py`, `cases/`; packets `target/step-0119-analysis/prompt-packets-v2.json` (SHA-256 `8e70e0f44c588b92cc7c3623cdce3c79f8bc7c6e8fe7eb2c825a528e757f1eec`); raw outputs `target/step-0119-analysis/rerun-v2/worker-00..11.json`; `run-v2.json`; `score-v2.json` (run SHA-256 `ce90670a0e109990af5ba91e06124eb0b5edabf56da40a116a319e870d646bd7`).

Note: `tools/validate-step-0017.ps1` / `validate-step-0019.ps1` fail in this environment because the rust-toolchain-pinned `1.97.1-x86_64-pc-windows-gnu` toolchain lacks its std component (`can't find crate for std`); the same test suites pass on `1.97.0-x86_64-pc-windows-gnu`. This is a pre-existing environment issue unrelated to this step.

## Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/prepare-ai-eval.ps1 -OutputPath target/step-0119-analysis/prompt-packets-v2.json
python target/ai-eval-experiments/build_run.py --results-dir target/step-0119-analysis/rerun-v2 --packets target/step-0119-analysis/prompt-packets-v2.json --out target/step-0119-analysis/run-v2.json --scope full --repetitions 30 --run-id subagent-v2-2026-08-04 --date 2026-08-04 --model-version subagent-runtime-2026-08-04
powershell -NoProfile -ExecutionPolicy Bypass -File tools/score-ai-eval.ps1 -RunPath target/step-0119-analysis/run-v2.json -PromptPacketPath target/step-0119-analysis/prompt-packets-v2.json -OutputPath target/step-0119-analysis/score-v2.json
```

Expected scorer line begins with `AI_EVAL_SCORE_OK run=subagent-v2-2026-08-04 ... score=0.974359`. Re-deriving worker outputs requires re-running the 12 subagent workers (fresh instances may drift slightly; the recorded score is reproducible exactly from the stored worker files).

## Evidence boundary

This step produces subagent-measured engineering evidence only. It is not a live-model result: the authoritative evaluation waits for owner-delivered DeepSeek credentials per M13 §5, and ADR-0011 (STEP-0120) must treat budgets the subagent baseline cannot justify as deferred. No M10/M11 contract was touched; the scorer, task identities and protocol version are unchanged (packet SHA change comes from prompt/guide content, recorded per run).
