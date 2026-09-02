# STEP-0114: streaming request and response bodies

> - status: complete
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Result

`crates/sico-http-provider/src/framing.rs`: strict HTTP/1.1 transfer framing under the RFC-0037 §7 budget model.

- `decide_framing`: one framing decision from strict header views — duplicate `Content-Length` values are refused even when equal (no smuggling via normalization), mixed CL+TE refused, non-canonical lengths (`+N`, `0N`) refused, declared length checked against the direction budget before acceptance.
- `ChunkedReader`: incremental chunked-body reader with bounds-before-allocation — hex-only sizes, extensions refused (strict v1), 64 KiB chunk cap, leading-zero and `0x` forms refused, budget gate per chunk, data-after-terminal-zero and second-size-lines refused.
- `validate_trailer`: bounded trailer sections (≤64 lines, no obs-fold, control-free, bounded name/value).
- 18/18 provider tests green including the smuggling/budget/amplification corpus (validate-step-0111 chain).

Integration into the live guest transport (WIT resources wired through the runner) lands with STEP-0117's connection lifecycle; this step freezes and proves the parser/policy layer the transport will consume.

## 2. Audit links

- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) §7
- [`STEP-0111`](./STEP-0111-secure-http-rfc.md)
