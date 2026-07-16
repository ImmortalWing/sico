# Compiler-backed AI tooling protocol v0

`sico-ai-tool` reads one UTF-8 JSON request from stdin and writes one JSON response. It performs no filesystem write, process launch, network request or model call.

## Inspect

```json
{
  "schema": "sico.ai-tool.request.v0",
  "protocol_version": 0,
  "request_id": "inspect-1",
  "operation": "inspect",
  "budget": {
    "max_files": 16,
    "max_input_bytes": 1048576,
    "max_symbols": 256,
    "max_diagnostics": 100,
    "max_response_bytes": 1048576
  },
  "input": {
    "files": [
      { "uri": "file:///workspace/main.sico", "text": "function main() returns Int:\n  return 1\nend function\n" }
    ]
  }
}
```

The result contains source SHA-256 and byte count, compiler diagnostics, Semantic Index symbols, snapshot quality and used/truncated budget. It never echoes input source.

## Validate fix

`validate_fix` accepts `uri`, `original`, `candidate`, the exact `original_sha256`, an exact diagnostic-key multiset, and `max_changed_bytes` from 1 through 256. It succeeds only when:

1. the digest still matches the original;
2. original compiler diagnostics exactly match the request;
3. the candidate passes parser and semantic diagnostics;
4. the candidate can be represented by one bounded contiguous edit;
5. compiler canonical formatting also remains diagnostic-free; and
6. the response fits its byte budget.

The response includes the validated edit, candidate and canonical digests, canonical text, and before/after diagnostics. Applying an edit remains an editor/user action; this tool only validates and describes it.

## Evidence boundary

The 54-source B corpus and twelve B repair oracle pairs measure the local compiler-backed tool path. They are deterministic fixtures, not model outputs. The existing 96-task `sico-ai-eval-v1` model protocol remains unchanged. A real model run still requires exact model metadata, raw outputs, at least 30 repetitions for a full comparison, credentials and explicit cost authorization.

See [RFC-0028](../docs/rfc/RFC-0028-ai-tooling-inspect-fix-v0.md) and [STEP-0068](../docs/steps/STEP-0068-ai-tooling-measured-evaluation.md).
