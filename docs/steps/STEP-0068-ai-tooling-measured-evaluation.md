# STEP-0068: AI tooling protocol and measured evaluation

> - status: complete-offline / live-model-not-authorized
> - phase: M7
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Provide bounded structured inspect/fix APIs and measure their complete local compiler path without turning fixtures into model claims.

## 2. Scope

Included: compiler-backed inspect; source-digest and diagnostic-bound fix validation; one bounded edit; canonical formatter output; stdin/stdout binary; 54-source inspect corpus; twelve B repair oracles; 512 protocol mutations; regression of the 96-task AI v1 harness; explicit live-model authorization gate.

Excluded: live model calls, credentials, provider adapter, automatic patch generation, multi-site refactors, file writes and model accuracy claims.

## 3. Plan

1. Preserve the existing AI evaluation v1 protocol and historical A0/B/C tasks. — complete
2. Freeze bounded inspect/fix request and response semantics. — complete
3. Implement compiler diagnostics, Semantic Index and formatter integration. — complete
4. Bind fixes to source digest, exact diagnostics and byte-limited edit. — complete
5. Measure B corpus and repair-oracle paths. — complete
6. Regress the 96-task offline harness and audit live authorization. — complete
7. Add corpus, RFC, report, validator and full regression. — complete

## 4. Changes and validation

Added `sico-ai-tools` and `sico-ai-tool`, the detailed tooling protocol and 24 machine cases. Eight tests pass. Inspect processed 54/54 B sources (25 clean, 29 diagnosed); validate-fix accepted 12/12 B repair oracles; 512/512 invalid protocol variants were rejected. The existing 96-task v1 dataset and deterministic synthetic harness still pass.

No model was called: zero common API credentials were present and no cost authorization was granted. Model attempts, accuracy, tokens, latency and cost remain `not measured`.

Result: `STEP_0068_OK tests=8 mutations=512 inspect=54 clean=25 diagnosed=29 fixes=12 ai_tasks=96 model_runs=0 credentials=0 cost_authorization=absent external_side_effects=0 next=STEP-0069`.

## 5. Risks

Fix validation is deliberately narrower than refactoring. Compiler-backed oracle success measures the tool, not an AI. Future live runs must preserve raw provider outputs and exact model metadata; they cannot be merged with synthetic or compiler-oracle results.

## 6. Links

- [RFC-0028](../rfc/RFC-0028-ai-tooling-inspect-fix-v0.md)
- [review](../reports/ai-tooling-measured-evaluation-v0.md)
- [tool protocol](../../ai-eval/tooling-protocol.md)
