# RFC-0028: AI tooling inspect/fix protocol v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target language/platform version: AI tooling protocol v0
> - supersedes: -
> - superseded-by: -

## Summary

Define bounded compiler-backed `inspect` and `validate_fix` operations for AI tools while preserving the existing provider-neutral 96-task model evaluation protocol.

## Request and budget

One `sico.ai-tool.request.v0` JSON document contains an exact protocol version, bounded request identity, operation, budget and operation-specific input. A request, response or inspect workspace is at most 1 MiB. Inspect accepts at most 16 unique local file URIs, 256 symbols and 100 diagnostics. Requested limits may only reduce these ceilings.

The response is `sico.ai-tool.response.v0` with the same request identity and either a result or stable typed error. Inspect returns source digests/byte counts but never source text. It returns compiler snapshot quality, compiler diagnostics, Semantic Index symbols and explicit total/returned/truncated counters.

## Fix validation

Fix validation binds the original SHA-256 and the exact ordered-as-a-multiset diagnostic keys. This rejects stale buffers, diagnostic confusion and duplicate-diagnostic collapse. The proposed candidate must satisfy the source contract, parser and semantic analyzer. The tool returns a single contiguous edit with at most 256 inserted-plus-removed bytes; unrelated distant changes expand the contiguous edit and fail the cap.

The candidate need not already be canonically formatted. The compiler formatter produces a separate canonical text/digest only after the submitted candidate passes diagnostics. The validated edit always describes original-to-submitted-candidate bytes, so it never silently widens authority to formatter changes.

The tool validates but never applies edits. It has no filesystem mutation, process execution, network, credential or model surface.

STEP-0093 adds `plan_execution` without expanding that authority. It returns shared `sico.execution-plan.v0` data for run/watch/REPL, including direct argv, capture bounds, client cancellation and honest source-map/debug availability. The AI tool still launches nothing.

## Evaluation semantics

Offline compiler-backed metrics and live-model metrics are separate:

- 54 B sources (25 accepted, 29 diagnosed) measure inspect coverage;
- twelve B repair oracle pairs measure fix-validator acceptance;
- 512 invalid metadata/budget requests measure fail-closed protocol behavior;
- the prior 96-task v1 harness remains the model-comparison protocol and passes its synthetic self-tests;
- oracle and synthetic results never become model accuracy, token, latency or cost scores.

Live evaluation requires exact provider/model snapshot metadata, raw output retention, credentials and explicit cost authorization. Absence of either authorization is a required no-call decision, not a failed offline test.

## Limitations

v0 returns compiler definitions/facts but no automatic patch generation. One-contiguous-edit validation intentionally rejects multi-site refactors. It supports the accepted B syntax/compiler baseline; A0 and C remain archived design candidates in the original evaluation corpus. No prompt-injection classifier, network sandbox or provider adapter is included because no model is invoked.

## Links

- [tool protocol](../../ai-eval/tooling-protocol.md)
- [AI evaluation v1](../../ai-eval/README.md)
- [RFC-0001 diagnostics](./RFC-0001-diagnostics-protocol-v0.md)
- [RFC-0002 Semantic Index](./RFC-0002-semantic-index-query-v0.md)
- [STEP-0068](../steps/STEP-0068-ai-tooling-measured-evaluation.md)
