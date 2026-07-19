# RFC-0030: Script streaming v0

> - status: proposed
> - date: 2026-07-18
> - authors: autonomous-agent
> - target language/platform version: M9 draft / WASI 0.2-style sync boundary (async upgrade path noted)
> - supersedes: -
> - superseded-by: -

## Summary

Extend the M8 batch Script Profile with a bounded streaming channel: versioned `input-stream`/`output-stream` WIT resources on the `sico:script/streams@0.1.0` interface, with exact ownership, EOF-versus-error, backpressure, close and cancellation semantics. The buffered `script-input`/`script-output` boundary remains the fully supported compatibility profile; a program uses one style per run.

## Problem

M8 transports stdin/stdout/stderr as whole buffers capped at 8 MiB. Representative automation (`Get-Content large.ndjson | sico run transform.sico > result.ndjson`) needs inputs and outputs larger than memory, constant-memory passthrough, and backpressure toward slow consumers — without giving the guest ambient IO authority or letting cancellation hang behind a blocked host call.

## Goals and non-goals

Goals: affine stream handles with exact drop; bounded per-call transport; EOF distinct from error; blocking-with-backpressure instead of unbounded queues; cancellation that reaches blocked host calls within a bounded latency; per-Store resource lifetime with no cross-run retention; a failure/cancellation matrix mapping every case to the runner's typed outcomes.

Non-goals: general WASI 0.3 `async func`/`stream<T>`/`future<T>` adoption at this boundary (the language-level mapping stays RFC-0004; the sync resource boundary here is the M9 v0 transport and can be superseded by a versioned async interface later); listener sockets; child processes; user-defined stream sources; unbounded collect.

## Interface

Single interface `sico:script/streams@0.1.0`, declared in `wit/script-profile-v0/world.wit` alongside the M8 types and fs interfaces, imported by the `program` world. A component imports it only when it calls the streaming intrinsics; M8 artifacts are byte-unchanged.

```wit
interface streams {
  enum stream-error { io, cancelled, closed, resource-limit }

  resource input-stream {
    read: func(max: u64) -> result<list<u8>, stream-error>;
  }
  resource output-stream {
    write: func(bytes: list<u8>) -> result<_, stream-error>;
    flush: func() -> result<_, stream-error>;
  }

  stdin: func() -> input-stream;
  stdout: func() -> output-stream;
  stderr: func() -> output-stream;
}
```

## Semantics

- **Ownership.** `stdin`/`stdout`/`stderr` each return one owned handle per call. Handles are affine: movable into one consumer, droppable exactly once, never copied. Guest-side use after the handle is dropped traps at the Canonical resource table (exit 125); `closed` covers a still-owned handle that has already reached a terminal error.
- **Close.** Dropping the owned handle is close; there is no separate close call. Dropping `output-stream` flushes on a best-effort basis before release; explicit `flush` is the only durability point the guest may rely on.
- **Read.** `read(max)` returns at most `max` bytes; the Host clamps `max` to 64 KiB per call. `read(0)` returns terminal `resource-limit` because `ok([])` is reserved exclusively for EOF. An empty ok list is EOF and is not an error. Any `stream-error` is terminal for that handle.
- **Write.** `write(bytes)` is all-or-error: the Host either accepts the whole payload (blocking while its bounded buffer is full — that is the backpressure) or returns an error. Payloads are at most 1 MiB per call. There is no partial-write report and no silent drop.
- **Lifetime.** Handles live in the run's Store resource table and die with it. A handle cannot outlive the run, move to another Store, or be retained by the persistent runner between executions (STEP-0090 reuses Engine/Linker only, never Stores or tables).
- **Bounds.** Per-run: at most 64 live stream handles; total streamed bytes are unbounded by design (that is the point of streaming), but per-call transport is bounded as above and the guest arena still obeys the 64 MiB M8 ceiling.
- **Capability.** The streams interface maps to the existing `script.stdio` capability: `sico run` auto-grants it for the local invocation, exactly like the buffered stdio channels. Streaming inherits no additional authority.

## Failure and cancellation matrix

| Case | Guest observes | Runner outcome |
|---|---|---|
| EOF on read | `ok([])` | continues |
| Host IO failure | `stream-error.io` (terminal) | guest decides; usually `Domain` 122 |
| Call after a terminal stream error | `stream-error.closed` | guest decides |
| Guest uses a dropped resource handle | Canonical resource-table trap | `Trap` 125 |
| Per-call bound exceeded | `stream-error.resource-limit` | guest decides; exit 125 if propagated as resource-limit |
| Cancel during blocked read/write | `stream-error.cancelled`; the Store epoch deadline also fires | `Cancelled` 123 |
| Store teardown with live handles | — (handles dropped by the table) | normal per other cases |

Cancellation is a control result, not a domain error (RFC-0004): cleanup still runs, and a guest that catches `cancelled` and finishes cleanly is allowed; the runner's deadline enforces termination regardless.

## Relationship to M8

The buffered records (`script-input.stdin`, `script-output.stdout/stderr`) keep their exact 8 MiB semantics. A run that imports streams must not also rely on the buffered channels for the same direction (the runner feeds/consumes each channel exactly once; mixing is a typed refusal at the boundary, not a race). Cache identity names the Script WIT package/version (`sico:script@0.1.0`), unchanged; the streams interface version is carried by the interface import name itself.

## Syntax surface

M9 v0 exposes streams through the closed intrinsic registry (`sico.stream.stdin`, `sico.stream.read`, `sico.stream.write`, `sico.stream.flush`, `sico.stream.close`), keeping Candidate A explicit-`main` scripts unchanged. Source-level Task/Future/Stream syntax stays STEP-0087, and top-level statements stay STEP-0092; this RFC fixes semantics, not surface spelling.

## Positive and negative cases

Positive (to be proven by STEP-0086/0088): 1/16/256 MiB passthrough at bounded RSS; slow-consumer backpressure; EOF vs error distinction; cancel during a blocked read.
Negative (must fail closed): use-after-close; write payload above the bound; cancel storm; handle retained across runs in the persistent runner.
