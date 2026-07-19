# M10 plan: Runtime observability and debugging

> - status: planned; STEP-0095 next
> - created: 2026-07-19
> - expanded: 2026-07-19
> - phase: M10
> - reserved steps: STEP-0095–0102
> - entry requirement: M9 GO with frozen `sico.execution-plan.v0` and explicit source-map/debug limits
> - successor: M11 Secure HTTP Provider and Automation SDK, only after M10 GO

## 1. Outcome

Turn M9's shell-free execution plan into a diagnosable and interruptible execution path. A user, editor or bounded automation client must be able to:

- bind exact source bytes to the exact compiler and Component that executed;
- receive structured lifecycle, stdout, stderr, cancellation and fault events without parsing human stderr;
- map Runtime traps, limits and Host failures back to stable Sico source coordinates;
- send Ctrl+C or a protocol cancellation request through one typed cancellation path;
- debug the explicitly supported subset through DAP only after real breakpoint, stack and variable evidence exists.

M10 is an observability/debugging milestone, not a language-syntax expansion. It must preserve the M9 authority model and the rule that AI tooling receives bounded data but no execution authority.

## 2. M9 baseline and problem statement

M9 already provides:

- `sico.execution-plan.v0` with direct argv, `shell: false`, 1 MiB capture bounds and client-owned process-tree termination;
- compile-time UTF-8 source coordinates;
- structured runner outcomes and cancellation exit 123 for cancellation paths the runner owns;
- persistent runner isolation, bounded watch and REPL sessions;
- an honest `sico.debug` refusal because Runtime locations and DAP hooks do not exist.

The remaining gap is identity and control continuity. A compiler span cannot currently prove which Runtime instruction failed; killing a child tree cannot be reclassified as typed cancellation; stdout/stderr is not a versioned event stream; and there is no supported way to pause, inspect or continue a real Component.

## 3. Scope and non-goals

### 3.1 In scope

- versioned source, compiler, Component and debug-map identities;
- deterministic compiler-produced debug maps;
- typed Runtime fault records and source frames;
- OS signal/client request to runner `CancelToken` bridging;
- bounded execution lifecycle/log/fault event protocol;
- a minimal, explicitly enumerated DAP subset;
- LSP/editor and data-only AI consumption of the same contracts;
- security, performance and actual-platform exit evidence.

### 3.2 Non-goals

- new language syntax, top-level shorthand or declaration-capable REPL cells;
- HTTP TLS/proxies/credentials or other M11 provider work;
- parallel Task scheduling, `select`, races or task collection;
- unrestricted process/shell, filesystem or network authority;
- arbitrary native debugging, memory editing, expression evaluation or hot code replacement;
- public registry deployment, mobile Runtime completion or unsupported-platform claims;
- parsing stderr, Wasmtime wording or platform exception strings as a protocol.

## 4. Ownership and module boundaries

| Concern | Owner | Forbidden dependency/duty |
|---|---|---|
| source spans, HIR/IR function identity | `sico-source`, HIR/IR crates | no Runtime/process dependency |
| debug-map construction and Component binding | `sico-codegen-wasm` | no DAP, socket or signal handling |
| schema/value validation shared by clients | a narrow tooling/observability contract crate | no process launch or Host authority |
| trap/outcome/frame production | `sico-runner` / Runtime boundary | no parser or source re-read as authority |
| OS signal and cancellation ownership | CLI/runner process boundary | no compiler dependency on OS signals |
| DAP framing and session state | tooling adapter/binary | no authority beyond the launched execution |
| LSP commands and AI summaries | language server / AI tools | plans and bounded observations only |

The compiler may emit data consumed by the Runtime, but it must not depend on runner, DAP or platform crates. Runtime and tooling communicate through versioned serialized contracts, not shared mutable compiler state.

## 5. Versioned contracts to freeze in STEP-0095

### 5.1 Identity chain

`sico.debug.identity.v0` binds:

- SHA-256 of the exact accepted source bytes, with no post-digest normalization;
- stable source document ID and optional display path that is never an authority-bearing path;
- compiler package/version and compiler executable digest;
- language semantics and IR identity;
- final Component SHA-256;
- debug-map SHA-256 and schema identity;
- adapter/WIT identities when composition is involved.

A missing, stale or mismatched link fails closed. A display URI may help a client locate a document, but only the source digest proves identity. Debug metadata must not contain source text, environment values, secrets or absolute paths by default.

### 5.2 Debug map

Candidate `sico.debug-map.v0` records sorted, non-overlapping mappings from Component function/instruction locations to:

- stable Sico function/declaration ID;
- UTF-8 half-open source span;
- optional call-site span and inline parent ID;
- generated/synthetic marker when no user span exists.

Hard candidate limits to accept or revise explicitly in STEP-0095:

- map file at most 16 MiB;
- at most 256 source documents and 100,000 functions;
- at most 1,000,000 mapping entries;
- at most 256 frames per fault/stack;
- all counts, offsets and lengths checked before allocation;
- canonical ordering, duplicate rejection and unknown-field rejection.

The compiler uses UTF-8 byte coordinates. LSP/DAP converts to UTF-16 or one-based line/column only at its protocol edge using the exact bound source bytes.

### 5.3 Runtime fault and frame records

`sico.runtime-fault.v0` has stable classes rather than engine text:

- guest-domain failure;
- cancelled;
- timeout/epoch deadline;
- fuel/resource/memory limit;
- guest trap;
- Host provider failure;
- incompatible or invalid artifact;
- runner/tool launch failure;
- internal invariant failure.

Each record carries one terminal class, stable code/key, run/generation identity, bounded frames, optional provider identity and a redacted human message. Raw engine text is diagnostic detail only and is excluded from equality, routing and AI classification.

### 5.4 Execution event stream

`sico.execution-events.v0` is an ordered stream with monotonically increasing sequence IDs. Event kinds are:

- accepted/started;
- generation-published;
- stdout/stderr chunk;
- breakpoint/stopped/continued;
- cancellation-requested;
- terminal outcome/fault;
- truncation or dropped-event marker.

Candidate hard bounds:

- each frame/event at most 1 MiB;
- stdout/stderr chunks at most 64 KiB;
- default captured bytes at most 1 MiB per channel;
- queue depth at most 256 events and 4 MiB total;
- exactly one terminal event;
- sequence order stable even when output channels race;
- overflow becomes an explicit marker and never silent loss or unbounded buffering.

Wall-clock timestamps may be optional observations. Sequence IDs and typed outcomes, not timestamps, define behavior.

### 5.5 Cancellation state machine

The state machine has `running → cancellation-requested → terminal`, with guest completion, timeout, Host failure and cancellation racing to one terminal winner. Inputs include:

- Ctrl+C/console control signal;
- DAP disconnect/terminate;
- LSP/editor client cancellation;
- execution-plan process-tree fallback;
- deterministic test timer;
- internal Host/provider cancellation.

The contract must say when exit 123 is guaranteed. If a client can only kill an already-unresponsive process tree, the outcome remains an external termination and must not be forged into typed guest cancellation.

### 5.6 Minimal DAP subset

The planned v0 claim is limited to:

- `initialize`, `launch`, `setBreakpoints`, `configurationDone`;
- `threads` with one logical Script thread unless actual parallelism later exists;
- `stackTrace`, `scopes`, bounded `variables`;
- `continue`, `pause`, `disconnect`, `terminate`;
- stopped/continued/output/terminated events.

Explicitly unsupported: attach to arbitrary processes, native assembly, memory read/write, set variable, evaluate arbitrary expressions, conditional/data breakpoints, reverse execution, hot reload and multi-process debugging. Unsupported requests return typed DAP errors rather than empty success.

## 6. Execution sequence

### STEP-0095: observability/debug/source-identity RFC

Deliver:

- RFC for the identity chain, debug map, runtime fault, execution events, cancellation race and minimal DAP subset;
- JSON Schemas or equivalent strict machine contracts plus positive/negative fixtures;
- exact module ownership and compatibility/versioning rules;
- threat model, redaction policy, hard resource bounds and non-SLA performance goals;
- decision on separate debug-map sidecar versus embedded Component custom section, with tamper binding either way.

Exit evidence:

- every candidate schema rejects unknown fields, duplicate identities, integer overflow, stale digests and over-limit inputs;
- at least two independently generated Components demonstrate distinct identities;
- no implementation or debugger claim before the RFC is accepted.

### STEP-0096: deterministic compiler debug-map artifact

Deliver:

- stable mapping from verified IR functions/operations to source spans and emitted Component locations;
- canonical serialization and Component/debug-map digest binding;
- build/cache/package integration without changing non-debug program semantics;
- explicit synthetic mappings for adapters/generated trampolines.

Exit evidence:

- repeated builds are byte-identical;
- representative control flow, calls, match, loops, helper functions and Script intrinsics map to exact spans;
- malformed, reordered, duplicated, truncated and mismatched maps fail closed;
- debug-disabled artifacts remain supported and honestly lack Runtime locations.

### STEP-0097: structured Runtime faults and source frames

Deliver:

- runner converts typed outcomes and engine traps into `sico.runtime-fault.v0`;
- bounded stack walking and source lookup using the exact bound map;
- text and JSON CLI presentation generated from the typed record;
- Host/provider frames separated from guest frames.

Exit evidence:

- real divide/overflow/domain errors where applicable, explicit traps, infinite-loop timeout, fuel/memory limit, malformed guest and Host failure map to stable classes;
- wrong/missing debug maps never produce plausible but false source spans;
- at most 256 frames and bounded messages under adversarial recursion.

### STEP-0098: OS signal and client cancellation bridge

Deliver:

- Windows console control and portable signal abstraction owned by CLI/runner;
- cancellation request protocol for persistent watch/debug sessions;
- one terminal-winner implementation shared by timer, signal, DAP and Host cancellation;
- process-tree termination retained only as bounded fallback.

Exit evidence:

- Ctrl+C reaches blocked read, blocked write, HTTP wait, guest busy loop and watch generation;
- completion-vs-cancel, timeout-vs-cancel and double-cancel races produce exactly one terminal record;
- typed exit 123 appears only when the runner observed and classified cancellation;
- no orphan runner, worker or temporary Component remains.

### STEP-0099: bounded execution events and logs

Deliver:

- runner event producer and client decoder for `sico.execution-events.v0`;
- bounded stdout/stderr chunking, ordering, truncation and backpressure;
- lifecycle events for one-shot, watch, REPL and debug runs;
- mandatory redaction hook before events leave the runner boundary.

Exit evidence:

- binary output, invalid UTF-8, 1 MiB exact, limit+1, slow consumer and disconnected client cases;
- queue never exceeds item/byte limits; overflow emits a stable marker;
- terminal event remains deliverable after log truncation;
- replayed captured events are deterministic apart from explicitly observational timing fields.

### STEP-0100: minimal DAP implementation

Deliver:

- bounded DAP framing/server and launch adapter over the accepted Runtime control hooks;
- verified breakpoint binding to exact source/component identity;
- stack/scopes/variables for values that can be represented safely and honestly;
- typed refusal for every unsupported DAP feature.

Exit evidence:

- real Component stops at entry and at least one user breakpoint, continues and terminates;
- nested calls produce source-mapped frames in correct order;
- scalar/local values are bounded and type-correct; unavailable/optimized values are labeled unavailable;
- stale source, stale Component, invalid breakpoint, oversized message and disconnect races fail closed;
- a debugged trap cannot poison the next session.

If Wasmtime/runtime hooks cannot support real pause/breakpoint semantics without an unacceptable engine fork, STEP-0100 must record a NO-GO for breakpoints and may ship a fault-inspection adapter only. It must not call that reduced surface a debugger.

### STEP-0101: editor and AI execution feedback integration

Deliver:

- LSP commands return launch/debug plans referencing the versioned contracts;
- editor adapter owns bounded execution and DAP lifecycle;
- AI tools consume redacted fault/event summaries and source frames as data only;
- docs describe exact client responsibilities and unsupported features.

Exit evidence:

- paths/metacharacters remain direct argv values with `shell: false`;
- capture, cancellation and debug identity survive real LSP/DAP framing;
- AI protocol cannot launch, read arbitrary files, access secrets or widen capabilities;
- malformed/stale event streams and diagnostic confusion fail closed.

### STEP-0102: M10 security, performance and platform exit audit

Deliver:

- aggregate validator for STEP-0095–0101 and M0–M9 regression;
- security review, adversarial protocol corpus, performance/RSS report and actual platform matrix;
- explicit GO/NO-GO plus residual-risk register and next-milestone decision.

Exit evidence is defined in sections 8–10 below.

## 7. Threat model and security gates

M10 must defend against:

- stale/tampered debug maps pointing faults at innocent source;
- path/source/secret leakage through maps, frames, variables or logs;
- oversized/recursive DAP and event messages;
- breakpoint or pause requests widening guest capability;
- double terminal outcomes and cancellation races;
- disconnected clients leaving a paused or privileged process alive;
- debug session state/resources leaking into later persistent runs;
- AI tools treating untrusted Runtime text as instructions.

Hard gates:

- all serialized inputs are strict, bounded and versioned;
- debug identity is digest-bound end to end;
- no debugger request changes package/Host capability grants;
- source text and variables are returned only for the launched, identity-matched document/session;
- secrets and ambient environment remain absent or redacted;
- session teardown removes only session-owned processes/files and is idempotent;
- protocol mutations, truncation, replay and cross-session mix-and-match fail closed.

## 8. Performance and resource plan

Hard resource limits are contractual; latency figures are measured non-SLA goals until STEP-0095 accepts them.

| Dimension | Hard limit / candidate goal |
|---|---|
| debug map | ≤16 MiB, ≤1,000,000 mappings, ≤100,000 functions |
| runtime frames | ≤256 frames; bounded message/string fields |
| DAP/event frame | ≤1 MiB |
| event queue | ≤256 events and ≤4 MiB |
| captured output | default ≤1 MiB per channel with explicit truncation |
| cancellation | candidate goal: runner observes a fired token within 100 ms |
| source lookup | candidate goal: P95 ≤1 ms over a 100,000-entry map |
| debug build overhead | measured against the same source with debug map disabled; no hidden SLA |
| pause/continue | measured locally with process spawn separated from in-session latency |
| repeated sessions | at least 100 sequential sessions with bounded RSS and no handle growth |

Every report records raw samples, sample count, environment and host-load caveats. A non-SLA latency miss is reported, not silently converted into a correctness failure; hard memory/count/security bounds remain failures.

## 9. Platform evidence policy

- Windows x64 is the initial Runtime target because M9 has actual runner evidence there.
- Linux/macOS claims require native execution of signal, event, fault-map and DAP cases on those platforms.
- Cross-compilation, generated launch configuration or protocol fixtures count as contract evidence only.
- Android/Harmony remain separate blocked/deferred tracks and are not upgraded by desktop tests.
- Platform-specific signal differences must map to the same abstract cancellation state machine without claiming identical OS mechanics.

## 10. M10 exit gate

M10 is GO only when:

1. source/compiler/Component/debug-map identities are versioned and fail closed on mismatch;
2. compiler maps representative generated code deterministically to exact source spans;
3. real Runtime faults and limits produce stable bounded source frames without parsing stderr;
4. Ctrl+C and client cancellation reach blocked and busy execution with one terminal winner;
5. lifecycle/log/fault transport stays within item/byte bounds under slow/disconnected clients;
6. every claimed DAP feature is exercised against real Components, and unsupported features refuse honestly;
7. editor and AI integrations preserve direct argv, redaction and authority boundaries;
8. repeated debug/persistent sessions prove teardown and isolation;
9. the complete M0–M9 regression is green;
10. platform support is limited to actual execution evidence.

Failure of the DAP breakpoint gate may produce an honest partial milestone or M10 NO-GO; it cannot be papered over by renaming post-mortem fault inspection as interactive debugging.

## 11. Handoff to M11

M11 may start only after M10 GO freezes structured events, redaction and typed cancellation, because secure network operations need those mechanisms for timeout/certificate/DNS/credential diagnostics. M10 must not implement TLS or secrets early. The planned successor is [`M11 Secure HTTP Provider and Automation SDK`](./M11-secure-http-automation-sdk.md).
