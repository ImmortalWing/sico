# AI tooling measured evaluation v0 review

> - status: complete-offline / live-model-not-authorized
> - step: STEP-0068
> - date: 2026-07-16

## Result

The new `sico-ai-tools` crate and `sico-ai-tool` stdin/stdout binary implement bounded compiler-backed inspect and fix validation. Eight Rust tests pass. Inspect covers all 54 accepted-B compiler sources: 25 are diagnostic-free and 29 produce their compiler diagnostics. All twelve B repair oracle pairs from the existing AI task manifest pass digest/diagnostic/edit/compiler validation. All 512 invalid schema or budget variants fail closed.

The pre-existing provider-neutral evaluation remains intact and passes:

```text
AI_EVAL_DATASET_OK protocol=v1 tasks=96 generation=30 understanding=30 repair=36 A0=32 B=32 C=32
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=96 deterministic=true
```

## Security review

- Request, workspace and response byte budgets are enforced independently.
- Inspect returns digests and structured facts, not input source.
- Original SHA-256 prevents applying a fix to stale source.
- Exact diagnostic multisets prevent code/key confusion and duplicate collapse.
- Candidate parser and semantic diagnostics must be empty.
- One contiguous edit and a 256-byte cap reject unrelated wide rewrites.
- Canonical formatting is compiler-produced after candidate acceptance and does not widen the validated edit.
- The tool cannot write files, execute commands, access credentials, call models or use a network.

## Live-model gate

The local environment check found zero configured common model API credentials, and the user has not granted a cost budget for live calls. Therefore no live request was made and no model accuracy/token/latency/cost score exists. This is the required evidence boundary; synthetic fixtures and compiler oracles are not model measurements.

## Residual limits

The tool validates proposed patches but does not generate them. v0 intentionally rejects multi-site refactors. Only accepted B syntax is compiler-measured; A0/C remain historical comparison data. Long-term model evaluation still needs an authorized provider adapter, immutable model version, raw run storage and at least 30 repetitions for official full comparisons.

## Evidence

`STEP_0068_OK tests=8 mutations=512 inspect=54 clean=25 diagnosed=29 fixes=12 ai_tasks=96 model_runs=0 credentials=0 cost_authorization=absent external_side_effects=0 next=STEP-0069`

## Links

- [RFC-0028](../rfc/RFC-0028-ai-tooling-inspect-fix-v0.md)
- [STEP-0068](../steps/STEP-0068-ai-tooling-measured-evaluation.md)
- [tool protocol](../../ai-eval/tooling-protocol.md)
