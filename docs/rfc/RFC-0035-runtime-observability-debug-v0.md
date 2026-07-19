# RFC-0035: Runtime observability and debug contract v0

> - status: accepted
> - date: 2026-07-19
> - authors: autonomous-agent
> - target language/platform version: M10 contract
> - supersedes: RFC-0034 compile-only Runtime-location/debug refusal after implementation evidence exists
> - superseded-by: -

## Summary

Freeze a bounded, digest-bound and authority-neutral contract for Sico source identity, compiler debug maps, Runtime faults/source frames, execution events, cancellation races and the exact minimal DAP surface. This RFC defines what STEP-0096–0101 must implement; accepting it does not claim that Runtime source frames, typed OS-signal cancellation or DAP already work.

Normative machine assets live in [`observability/`](../../observability/README.md). Human prose never widens a machine allowlist or limit.

## Problem

M9 can execute real Components but its compile spans stop at the compiler boundary. Runtime traps do not prove which exact source bytes produced the executing Component, abrupt process-tree termination cannot honestly become exit 123, stdout/stderr are not a versioned event stream, and `sico.debug` correctly refuses because there are no breakpoint or pause hooks.

Adding debugger code before freezing identity, ownership and bounds would allow stale maps, path/secret disclosure, unbounded queues, misleading success responses and competing terminal outcomes. The contract must be strict before implementation.

## Goals and non-goals

Goals:

- bind exact source bytes, compiler executable, semantics/IR, Component and debug map;
- map typed Runtime outcomes and engine locations to bounded source frames without parsing stderr;
- define task-aware but M10-single-task execution events;
- bridge observed signals/client requests into one cancellation state machine;
- publish one exact DAP request/event allowlist with per-row evidence IDs;
- reject stale, oversized, duplicate, unknown and cross-session data;
- expose observation without adding Script authority.

Non-goals:

- source-language syntax or semantics changes;
- arbitrary native debugging, expression evaluation, memory mutation or attach;
- parallel scheduling (M11), secure HTTP/TLS/secrets (M12), public deployment or mobile completion;
- inferring a source location from engine/stderr text;
- claiming a debugger before STEP-0100 proves real Component pause/breakpoint behavior.

## Contract identities and compatibility

The v0 identities are:

- `sico.debug.identity.v0`;
- `sico.debug-map.v0`;
- `sico.runtime-fault.v0`;
- `sico.execution-event.v0`;
- `sico.cancellation-race.v0`;
- `sico.dap-claimed-subset.v0`.

All serialized objects reject unknown fields. A breaking field/meaning/coordinate change requires a new major identity. New optional behavior cannot be inferred from an unknown field; it requires an accepted schema revision and consumer negotiation. Display text is never a routing or equality key.

## Source and artifact identity

`sico.debug.identity.v0` binds:

- SHA-256 of the exact accepted source bytes, before any newline or Unicode normalization;
- stable document ID and an optional non-authority display URI;
- compiler package/version and compiler executable SHA-256;
- language-semantics and Sico IR schema identities;
- executable Component-code SHA-256 computed before debug custom sections;
- final linked Component SHA-256;
- canonical debug-map sidecar SHA-256;
- sorted adapter and WIT identities.

All hashes are lowercase 64-character hex. A display URI may help a client find a document but cannot prove identity or grant filesystem access. Absolute paths, source text, environment data and secrets are absent by default.

### Sidecar decision

The accepted representation is a canonical JSON `sico.debug-map.v0` sidecar plus a compact Component custom section `sico.debug-link.v0`. Binding is deliberately non-circular:

1. codegen deterministically serializes the executable Component with every `sico.debug-*` custom section absent and computes `component_code_sha256`;
2. the canonical sidecar binds source/compiler identities to `component_code_sha256` and is hashed as `debug_map_sha256`;
3. the compact link section carries those binding digests and `debug_map_sha256`;
4. the complete linked Component is serialized and hashed as final `component_sha256` in `sico.debug.identity.v0`.

The custom section does not embed source text or the full map. Validators reconstruct the code hash by stripping only registered `sico.debug-*` sections, never arbitrary custom sections. STEP-0096 must freeze deterministic bytes for the sidecar/link/stripping algorithm and reject any missing or mismatched edge.

Reasons:

- a sidecar is independently inspectable, cacheable and size-limited;
- stripping debug information can remove the sidecar/link without changing program semantics;
- the compact custom section prevents selecting a plausible map by filename alone and avoids a Component↔map digest cycle;
- embedding a possible 16 MiB map in every Component would inflate normal distribution artifacts.

An embedded full map and an unbound filename-only sidecar were rejected. A debug-disabled Component remains valid but honestly has no Runtime source locations.

## Debug map

`sico.debug-map.v0` uses UTF-8, zero-based, half-open byte coordinates. It records sorted source documents, functions and non-overlapping Component instruction ranges. Each mapping contains:

- Component function index and half-open instruction range;
- stable Sico function ID;
- source document ID and half-open source range;
- optional call-site range and inline-parent function ID;
- `generated` marker for synthetic/adaptor code.

Mappings sort by Component function, instruction start/end and stable function ID. Documents and functions have unique IDs. A mapping must reference known IDs, remain inside the bound source byte length, and not overlap a previous mapping in the same Component function. Protocol integers are limited to JSON's exactly interoperable range `0..=9,007,199,254,740,991`; implementation-specific `u64` overflow is rejected before allocation.

Accepted hard limits:

| Dimension | Limit |
|---|---:|
| canonical map bytes | 16 MiB |
| source documents | 256 |
| functions | 100,000 |
| mappings | 1,000,000 |
| Runtime/source frames | 256 |
| document/display/function/string field | 64 KiB UTF-8 |

## Runtime fault and frame records

`sico.runtime-fault.v0` contains exactly one terminal class:

- `domain-error`;
- `cancelled`;
- `timeout`;
- `resource-limit.fuel`;
- `resource-limit.memory`;
- `resource-limit.other`;
- `trap`;
- `host-provider-failure`;
- `incompatible-artifact`;
- `launch-failure`;
- `external-termination`;
- `internal-invariant`.

`external-termination` is intentionally distinct: when an unresponsive process can only be killed externally, the Runtime must not forge `cancelled` or exit 123. Each record has one run/generation identity, stable code/key, optional provider ID, a redacted bounded message and at most 256 frames. A source frame is emitted only when the Component/debug/source identity chain matches exactly; otherwise the frame remains generated/unavailable rather than plausible.

Raw Wasmtime/platform wording may appear only as redacted diagnostic detail and is excluded from equality, routing and AI classification.

## Execution events

`sico.execution-event.v0` is one event in a monotonically increasing per-run sequence. Event kinds are:

- `accepted`, `started`, `generation-published`;
- `stdout`, `stderr`;
- `breakpoint`, `stopped`, `continued`;
- `cancellation-requested`;
- `terminal`, `fault`;
- `truncated`, `dropped`.

Every event contains `run_id`, `generation_id`, `sequence`, kind and redacted payload. Optional `task_id`, `parent_task_id`, `scope_id` and typed causal relation are reserved now. M10 emits one logical task; M11 may populate multiple task identities without replacing the transport. Task/generation identities cannot cross runs.

Hard limits:

- one serialized event or DAP frame: 1 MiB;
- stdout/stderr chunk: 64 KiB;
- captured stdout and stderr: 1 MiB each by default;
- queued events: 256 and 4 MiB total;
- message/value string: 64 KiB UTF-8;
- exactly one terminal event;
- overflow emits a bounded `truncated`/`dropped` marker and never silently grows memory.

Sequence IDs, not wall-clock timestamps, define order. Timestamps are optional observations.

## Redaction and value policy

Data leaving the runner crosses one mandatory redaction hook. The allowlist is typed lifecycle fields, stable identities/codes, source coordinates, bounded stdout/stderr bytes and explicitly inspectable values from the identity-matched launched run.

The deny/redact set includes secret values and known encodings, ambient environment, ungranted paths, authorization headers, provider credentials, Host internals, raw process command lines and values from another run/task scope. Absolute paths become non-authority display URIs or are omitted. DAP values are read-only, maximum depth 8, maximum 256 children per container and subject to the 1 MiB frame cap. Redaction happens before queueing, logging, DAP framing or AI summarization.

## Cancellation state machine

The normative matrix is [`cancellation-race-v0.json`](../../observability/contracts/cancellation-race-v0.json). States are `running → cancellation-requested → terminal`. Guest completion/failure, timeout, observed signal/client cancellation, Host failure and external fallback race through one atomic terminal-winner operation.

The first transition successfully committed by the Runtime wins; wall-clock timestamps do not retroactively change it. Tests control observation order and must cover both orders for each race. Repeated cancellation is idempotent.

Exit 123 is guaranteed only when the runner observes a typed cancellation request and commits `cancelled`. If the client only kills an unresponsive process tree, the record is `external-termination`, `typed_cancel = false`, with no forged stable guest exit.

## Exact DAP v0 claim

The sole allowlist is [`dap-claimed-subset-v0.json`](../../observability/contracts/dap-claimed-subset-v0.json). It contains exactly one record per named request/event, a unique evidence ID, support state, direction, preconditions, maximum payload and required behavior.

Supported requests are exactly:

`initialize`, `launch`, `setBreakpoints`, `configurationDone`, `threads`, `stackTrace`, `scopes`, `variables`, `continue`, `pause`, `disconnect`, `terminate`.

Supported events are exactly:

`initialized`, `stopped`, `continued`, `output`, `terminated`, `exited`.

The machine file also enumerates typed-refused requests. Every other request uses the declared `unsupported-request` error policy; an unclaimed event must never be emitted. `setBreakpoints` accepts only unconditional source line breakpoints: condition, hit condition and log-message modes are refused. M10 claims no stepping, evaluation, mutation, native memory/assembly, reverse execution, attach or independent thread termination.

Support in this RFC is an implementation target, not current evidence. STEP-0100 iterates every machine row against real Components. If the selected Runtime cannot prove safe pause and source breakpoints without an unacceptable engine fork, M10 records DAP NO-GO rather than renaming post-mortem inspection.

## Module ownership

| Concern | Owner | Forbidden dependency/duty |
|---|---|---|
| source/function/IR identities | source, HIR and IR crates | Runtime/process access |
| deterministic map/link construction | `sico-codegen-wasm` | DAP, signal or socket handling |
| serialized schema/value validation | future narrow observability contract crate | process launch or Host authority |
| fault/frame/event production | `sico-runner` / Runtime | source re-read as authority |
| signal/cancellation terminal winner | CLI/runner process boundary | compiler/platform reverse dependency |
| DAP framing/session state | tooling adapter | capabilities beyond owned launch |
| editor and AI summaries | LSP/AI tools | execution or secret authority |

The compiler emits data consumed by Runtime/tooling but never depends on them. Contract assets at repository root are data, not a reason to move Host or debugger implementation into the language frontend.

## Security and privacy

The validator and later implementation must fail closed on:

- unknown fields/schema versions, duplicate IDs or mappings and unsafe integers;
- stale source/compiler/Component/debug-map binding;
- path, source, environment, secret or cross-session leakage;
- oversized/recursive frames, variables, events and DAP messages;
- DAP requests that imply unsupported capability;
- double terminal outcomes, replayed completions and disconnected paused sessions;
- AI consumers treating Runtime text as instructions.

Debugger requests never change package, Host or invocation grants. Session teardown is idempotent and removes only session-owned state.

## Performance goals

The following are measured non-SLA goals until implementation evidence promotes them:

- cancellation token observed within 100 ms at an accepted polling/async boundary;
- P95 source lookup at most 1 ms over 100,000 mappings;
- 100 sequential sessions without RSS or handle growth;
- pause/continue latency reported separately from process launch;
- debug build size/time overhead reported against identical source with debug disabled.

Missing a non-SLA latency goal is reported; violating count, byte, identity or security bounds is a correctness failure.

## Alternatives

1. **Parse stderr/engine text.** Rejected: unstable, non-typed and incapable of proving source identity.
2. **Embed the full map.** Rejected for v0: distribution size and independent inspection/cache costs; a digest-bound sidecar plus compact link is sufficient.
3. **DAP advertise-then-refuse.** Rejected: clients infer capabilities from initialization; the machine allowlist must match evidence.
4. **Treat process kill as cancellation.** Rejected: it fabricates exit 123 without runner observation.
5. **Implement debugger in compiler crates.** Rejected: violates module direction and mixes execution authority into the language frontend.

## Validation and acceptance criteria

STEP-0095 is accepted when:

- JSON schemas and manual strict validator agree on all normative identities/limits;
- positive fixtures validate;
- unknown field, duplicate mapping, unsafe integer, stale digest and over-limit fixtures reject with their exact reason;
- DAP supported/refused/event sets equal the fixed lists and every row has a unique evidence ID;
- cancellation cases and both-order race fixtures produce one declared winner;
- docs and local links validate;
- no runner/compiler implementation or debugger support claim is introduced.

STEP-0096 implements only the identity/debug-map/link portion. Runtime frames, signal cancellation, events and DAP remain later numbered gates.

## Links

- [`M10 plan`](../plans/M10-runtime-observability-debugging.md)
- [`STEP-0095`](../steps/STEP-0095-observability-debug-contract.md)
- [`RFC-0034`](./RFC-0034-tooling-execution-plan-v0.md)
- [`RFC-0018`](./RFC-0018-runtime-limits-fault-taxonomy-v0.md)
- [`observability assets`](../../observability/README.md)
