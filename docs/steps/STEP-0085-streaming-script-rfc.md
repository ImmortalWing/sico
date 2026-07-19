# STEP-0085: streaming Script WIT/resource/cancellation RFC

> - status: complete
> - phase: M9
> - started: 2026-07-18
> - completed: 2026-07-18
> - owners: autonomous-agent

## 1. Objective

Fix the M9 streaming contract before any implementation: versioned InputStream/OutputStream resources, ownership, backpressure, close/error/cancel semantics, and the failure/cancellation matrix mapping every case to the runner's typed outcomes.

## 2. Deliverables (all contract-level)

- [`RFC-0030`](../rfc/RFC-0030-script-streaming-v0.md) (proposed): `sico:script/streams@0.1.0` with `input-stream.read(max) -> result<list<u8>, stream-error>`, `output-stream.write/flush`, and `stdin/stdout/stderr` handle constructors; stream-error = io/cancelled/closed/resource-limit.
- WIT draft merged into `wit/script-profile-v0/world.wit` (same `sico:script@0.1.0` package as the M8 fs amendment); parses with wit-parser 0.253.0 (`repository_wit_contracts_parse` green), `ScriptAbi` boundary unchanged.
- Semantics fixed: affine handles with drop-is-close; read clamp 64 KiB/call, empty-ok-is-EOF; write all-or-error ≤ 1 MiB/call with blocking backpressure; ≤ 64 live handles per run; per-Store lifetime (no cross-run retention); maps to the existing `script.stdio` capability.
- Failure/cancellation matrix (EOF vs error vs closed vs resource-limit vs cancel-during-blocked-call → `Cancelled` 123), and the M8 buffered/streaming mixing rule (one style per direction per run, typed refusal otherwise).

## 3. Exit evidence

- RFC-0030 written with goals/non-goals, interface, semantics, bounds, matrix, M8 compatibility and positive/negative cases;
- WIT parses; codegen test suite green (`cargo test -p sico-codegen-wasm`, 2026-07-18);
- no implementation claims: intrinsic names (`sico.stream.*`) are registered as surface direction only, implementation starts at STEP-0086.

## 4. Links

- [`M9 plan`](../plans/M9-streaming-async-interactive.md) (§5 streaming model)
- [`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md) (language-level resource/async mapping)
- [`RFC-0030`](../rfc/RFC-0030-script-streaming-v0.md)
