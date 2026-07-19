# M10 plan: Runtime observability and debugging

> - status: in progress; STEP-0095–0096 complete, STEP-0097 next
> - created: 2026-07-19
> - expanded: 2026-07-19
> - phase: M10
> - reserved steps: STEP-0095–0102
> - entry requirement: M9 GO with frozen `sico.execution-plan.v0` and explicit source-map/debug limits
> - successor: M11 bounded structured-concurrency Runtime; STEP-0103 design may start after STEP-0098, implementation and GO require M10 GO

## 1. Outcome

Turn M9's shell-free execution plan into a diagnosable and interruptible execution path. A user, editor or bounded automation client must be able to:

- bind exact source bytes to the exact compiler and Component that executed;
- receive structured lifecycle, stdout, stderr, cancellation and fault events without parsing human stderr;
- map Runtime traps, limits and Host failures back to stable Sico source coordinates;
- send Ctrl+C or a protocol cancellation request through one typed cancellation path;
- debug the explicitly supported subset through DAP only after real breakpoint, stack and variable evidence exists.

M10 is an observability/debugging milestone, not a language-syntax expansion. It must preserve the M9 authority model and the rule that AI tooling receives bounded data but no execution authority.

### 1.1 Why this internal milestone comes before an external gate

The remaining public rollout, production identity, live-model credential and mobile-runner gates require inputs that are not present in this repository and cannot be manufactured by implementation work. M10 is scheduled now because M9 exposed concrete internal debt—identity discontinuity, untyped interruption and unbounded ad-hoc observation—that would make every later provider and concurrent execution path harder to audit.

This ordering is not evidence that an external product gate has closed. STEP-0102 must recheck whether real deployment, third-party, model or platform inputs have arrived; if they have, those tracks resume under their own evidence and authority rules. Internal observability evidence never substitutes for production, mobile or external-user evidence.

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
- HTTP TLS/proxies/credentials or other M12 provider work;
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

## 5. Versioned contracts frozen by STEP-0095

### 5.1 Identity chain

`sico.debug.identity.v0` binds:

- SHA-256 of the exact accepted source bytes, with no post-digest normalization;
- stable source document ID and optional display path that is never an authority-bearing path;
- compiler package/version and compiler executable digest;
- language semantics and IR identity;
- executable Component-code digest computed before `sico.debug-*` custom sections;
- final Component SHA-256;
- debug-map SHA-256 and schema identity;
- adapter/WIT identities when composition is involved.

A missing, stale or mismatched link fails closed. A display URI may help a client locate a document, but only the source digest proves identity. Debug metadata must not contain source text, environment values, secrets or absolute paths by default.

### 5.2 Debug map

Accepted `sico.debug-map.v0` records sorted, non-overlapping mappings from Core module/function/instruction locations to:

- stable Sico function/declaration ID;
- UTF-8 half-open source span;
- optional call-site span and inline parent ID;
- generated/synthetic marker when no user span exists.

Accepted hard limits from STEP-0095 / RFC-0035:

- map file at most 16 MiB;
- at most 256 source documents and 100,000 functions;
- at most 1,000,000 mapping entries;
- at most 256 frames per fault/stack;
- all counts, offsets and lengths checked before allocation;
- canonical ordering, duplicate rejection and unknown-field rejection.

The compiler uses UTF-8 byte coordinates. LSP/DAP converts to UTF-16 or one-based line/column only at its protocol edge using the exact bound source bytes.

### 5.3 Runtime fault and frame records

`sico.runtime-fault.v0` has stable classes rather than engine text:

- domain error;
- cancelled;
- timeout;
- fuel, memory and other resource limits;
- trap;
- Host provider failure;
- incompatible artifact;
- launch failure;
- external termination when a process dies without a runner-observed typed cancellation;
- internal invariant failure.

Each record carries one terminal class, stable code/key, run/generation identity, bounded frames, optional provider identity and a redacted human message. Raw engine text is diagnostic detail only and is excluded from equality, routing and AI classification.

### 5.4 Execution event stream

The execution-event stream carries strict `sico.execution-event.v0` frames with monotonically increasing sequence IDs. Event kinds are:

- accepted/started;
- generation-published;
- stdout/stderr chunk;
- breakpoint/stopped/continued;
- cancellation-requested;
- terminal outcome/fault;
- truncation or dropped-event marker.

Every event carries `run_id` and `generation_id`. STEP-0095 also reserves bounded optional `task_id`, `parent_task_id` and `scope_id` fields plus a typed causal relation. M10 producers emit one logical task and do not claim parallel execution; reserving these fields prevents STEP-0099 from freezing a schema that M11 would immediately have to replace. Unknown task relationships and cross-run identities fail validation rather than being treated as display-only strings.

Accepted hard bounds:

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

### 5.6 Exact DAP claimed subset

STEP-0095 must accept a machine-readable `dap-claimed-subset-v0.json`. It is the sole allowlist for STEP-0100 and contains exactly one record per request/event, its direction, support state, preconditions, maximum response size and required evidence fixture. Missing, duplicate and unknown claims are validation failures.

The initial claimed request matrix is exact:

| DAP request | M10 v0 state | Required behavior and evidence |
|---|---|---|
| `initialize` | supported | advertises only this matrix; no implied optional capability |
| `launch` | supported | launches one identity-bound Component with existing grants only |
| `setBreakpoints` | supported | unconditional source line breakpoints only; `condition`, `hitCondition` and `logMessage` are typed-refused; each requested line is verified, rejected or reported unbound |
| `configurationDone` | supported | starts/resumes only after the accepted launch sequence |
| `threads` | supported | returns one logical Script execution in M10; M11 may map tasks without changing authority |
| `stackTrace` | supported | bounded source-mapped frames for the selected logical execution |
| `scopes` | supported | fixed read-only locals/arguments/result scopes only when Runtime data exists |
| `variables` | supported | bounded, depth-limited, redacted read-only values; unavailable values say so |
| `continue` | supported | resumes a stopped execution and produces one matching `continued` event |
| `pause` | supported | succeeds only when the Runtime proves a real safe pause; otherwise typed refusal |
| `disconnect` | supported | detaches only by terminating/cancelling the owned launch; no orphan session |
| `terminate` | supported | requests the shared typed cancellation path and one terminal outcome |
| `attach` | refused | arbitrary process/Store attachment is outside the ownership model |
| `next` | refused | M10 v0 makes no source-stepping claim |
| `stepIn` | refused | M10 v0 makes no source-stepping claim |
| `stepOut` | refused | M10 v0 makes no source-stepping claim |
| `evaluate` | refused | no arbitrary guest expression execution |
| `setExpression` | refused | no guest expression mutation |
| `setVariable` | refused | variables are read-only observations |
| `readMemory` | refused | no native memory surface |
| `writeMemory` | refused | no native memory mutation |
| `disassemble` | refused | no native assembly surface |
| `setFunctionBreakpoints` | refused | only unconditional source line breakpoints are claimed |
| `setDataBreakpoints` | refused | only unconditional source line breakpoints are claimed |
| `setInstructionBreakpoints` | refused | Component/native instruction breakpoints are not exposed |
| `setExceptionBreakpoints` | refused | M10 v0 stops only for the accepted breakpoint/fault reasons |
| `restartFrame` | refused | no frame/control-flow rewrite |
| `goto` | refused | no control-flow rewrite |
| `stepBack` | refused | no reverse execution |
| `reverseContinue` | refused | no reverse execution |
| `restart` | refused | persistent generations remain isolated launches |
| `terminateThreads` | refused | M10 has one owned logical execution, not independently terminable tasks |

The initial claimed event matrix is also exact:

| DAP event | M10 v0 state | Required behavior and evidence |
|---|---|---|
| `initialized` | supported | emitted once after capabilities are fixed |
| `stopped` | supported | carries a typed reason and valid logical execution identity |
| `continued` | supported | paired with an accepted resume and correct all-threads semantics |
| `output` | supported | bounded, redacted and derived from the execution-event stream |
| `terminated` | supported | emitted once after the session reaches a terminal outcome |
| `exited` | supported | carries the stable process/runner exit code when one exists |

All other DAP requests and events are unclaimed. Requests receive a typed unsupported error rather than empty success; the adapter must not emit unclaimed events. A supported row is still a NO-GO until STEP-0100 exercises its success path and relevant failure/precondition paths against a real Component.

## 6. Execution sequence

### STEP-0095: observability/debug/source-identity RFC

Status: complete. Contract and evidence: [`RFC-0035`](../rfc/RFC-0035-runtime-observability-debug-v0.md), [`STEP-0095`](../steps/STEP-0095-observability-debug-contract.md) and [`observability/`](../../observability/README.md).

Deliver:

- RFC for the identity chain, debug map, runtime fault, execution events, cancellation race and minimal DAP subset;
- exact `dap-claimed-subset-v0.json` request/event allowlist, its schema and a validator that rejects missing, duplicate or unknown rows;
- JSON Schemas or equivalent strict machine contracts plus positive/negative fixtures;
- exact module ownership and compatibility/versioning rules;
- threat model, redaction policy, hard resource bounds and non-SLA performance goals;
- decision on separate debug-map sidecar versus embedded Component custom section, with tamper binding either way.

Exit evidence:

- every candidate schema rejects unknown fields, duplicate identities, integer overflow, stale digests and over-limit inputs;
- every DAP matrix row has a unique evidence ID, bounded response contract and typed supported/refused state;
- at least two independently generated Components demonstrate distinct identities;
- no implementation or debugger claim before the RFC is accepted.

### STEP-0096: deterministic compiler debug-map artifact

Status: complete. Implementation and evidence: [`STEP-0096`](../steps/STEP-0096-deterministic-compiler-debug-map.md), [`report`](../reports/deterministic-debug-map-v0.md), `sico-observability` and `tools/validate-step-0096.ps1`.

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

Accepted result: `sico build --debug-info` emits an atomic deterministic Component/map/identity triplet; real Core offsets cover calls, matches, back edges, block terminators and Script intrinsic helpers; generated allocator/helper bodies use source-less synthetic mappings; malformed/mixed artifacts fail closed; normal builds remain unlinked; packaging preserves the linked Component without absorbing developer sidecars. Multi-Core Components require explicit `core_module` qualifiers on functions and mappings.

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

- runner event producer and client decoder for `sico.execution-event.v0` frames;
- bounded stdout/stderr chunking, ordering, truncation and backpressure;
- lifecycle events for one-shot, watch, REPL and debug runs;
- reserved task/parent/scope identities and causal relationships that remain valid when M11 adds multiple logical tasks;
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

- the validator iterates every row of `dap-claimed-subset-v0.json`; every supported request/event has real-Component evidence and every refused request has a typed negative fixture;
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

Hard resource limits are contractual and were accepted by STEP-0095; latency figures remain measured non-SLA goals until implementation evidence promotes them.

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
6. every supported row in `dap-claimed-subset-v0.json` is exercised against real Components, every refused row has typed negative evidence, and no unclaimed event is emitted;
7. editor and AI integrations preserve direct argv, redaction and authority boundaries;
8. repeated debug/persistent sessions prove teardown and isolation;
9. the complete M0–M9 regression is green;
10. platform support is limited to actual execution evidence.

Failure of the DAP breakpoint gate may produce an honest partial milestone or M10 NO-GO; it cannot be papered over by renaming post-mortem fault inspection as interactive debugging.

## 11. Handoff to M11 and external-gate recheck

After STEP-0098 freezes the shared typed cancellation path, the documentation-only STEP-0103 ADR may begin in parallel with STEP-0099–0102. This early design lane exists so the single-Store/multi-Store and arena-lifetime decision is settled before scheduler code. M11 implementation, support claims and GO still require complete M10 GO, including task-aware bounded events and the exact DAP matrix.

The planned successor is [`M11 bounded structured-concurrency Runtime`](./M11-structured-concurrency-runtime.md). Secure HTTP moves to [`M12`](./M12-secure-http-automation-sdk.md), after scheduler ownership and cancellation semantics are stable. Neither milestone widens Script authority, and neither counts as evidence for the still-open public rollout, production identity, live-model or mobile-runner gates; STEP-0102 explicitly rechecks those inputs before handoff.
