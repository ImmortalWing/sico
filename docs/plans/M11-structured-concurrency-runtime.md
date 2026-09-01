# M11 plan: Bounded structured-concurrency Runtime

> - status: STEP-0103 ADR-0010 accepted-design; STEP-0104 semantic/IR contract complete; STEP-0105 scheduler core complete (2026-08-31); STEP-0106 cancellation/race/select complete (2026-09-01); STEP-0107 next
> - created: 2026-07-19
> - phase: M11
> - reserved steps: STEP-0103–0110
> - design entry: STEP-0098 accepted with one typed cancellation path
> - implementation entry: M10 GO with task-aware bounded events and exact DAP claims
> - successor: M12 Secure HTTP Provider and Automation SDK

## 1. Outcome

Replace M9's deliberately sequential `parallelism = 1` executor with a bounded structured-concurrency Runtime whose task lifetime, cancellation, resource ownership and observation rules are explicit. A Script must be able to:

- create child tasks only inside a lexical/runtime task scope;
- await, join, cancel and collect those children without detached work;
- race or select bounded operations with one deterministic terminal winner;
- move affine resources between tasks without concurrent aliasing;
- apply task, queue, memory, time and Host-operation budgets to the whole run;
- observe task relationships through M10 events and debugging without granting new authority.

M11 changes Runtime scheduling, not the capability lattice, and does not require a new source-language surface by default. Parallelism is an internal execution mechanism: a child task receives at most the parent run's already accepted grants, cannot request a new capability, and cannot turn a resource quota into authority.

## 2. Why this milestone now

M9 proved Task/Future/Stream lowering and real asynchronous Host calls but intentionally executed them through one sequential scope. M10 adds the typed cancellation and task-aware event substrate needed to explain concurrent outcomes. Leaving the scheduler implicit while adding secure HTTP would force the provider to invent its own cancellation, ownership and in-flight request model, creating two competing Runtime truths.

Public rollout, production identity, live-model credential and mobile-runner gates still require external inputs that are not present in the repository. M11 closes a known internal architecture gap while those gates remain unavailable; it does not count as evidence that any external gate is complete. STEP-0110 must recheck those inputs and resume newly available tracks separately instead of extending internal work by inertia.

## 3. Baseline and invariants inherited from M8–M10

M11 starts from constraints, not a blank scheduler design:

- M8 creates a fresh Wasmtime `Store` for every top-level run and treats Engine/Component/Linker—not Store—as reusable persistent-runner state.
- M8 canonical ABI lowering/lifting and `post-return` cleanup define strict arena and borrowed-memory lifetimes.
- M8/M9 resources are affine and Store-owned; handles cannot survive Store teardown or silently cross runs.
- M9 freezes `parallelism = 1`, bounded streams, explicit Task ownership, E5101/E5102 refusal paths and cancellation edge 123.
- M9 watch/REPL generations are isolated even when the runner process persists.
- M10 STEP-0098 supplies one terminal-winner cancellation path; STEP-0099 supplies task-aware bounded events.

Any M11 design that weakens fresh-Store isolation, canonical ABI cleanup, affine resource flow, deterministic capability closure or bounded queues is invalid even if it improves throughput.

## 4. STEP-0103 architecture decision

The first M11 artifact is an ADR, not scheduler code. It compares and decides between:

1. one Store with a cooperative bounded scheduler; and
2. multiple Stores with isolated workers and explicit inter-Store messaging.

The planned v1 decision is **one fresh Store per top-level run with a cooperative scheduler inside that Store**. STEP-0103 must accept or reject that decision with evidence. Until it does, no implementation may make the choice implicitly.

### 4.1 Why single-Store cooperative scheduling is the v1 candidate

- it preserves M8's identity that one top-level run owns one Store and one capability set;
- canonical ABI resources remain inside their creating Store;
- deterministic task IDs, cancellation propagation and total-run budgets have one owner;
- it avoids pretending that Wasm memories, resource handles or borrows can be shared safely across Stores;
- cooperative yield points make fairness and suspension auditable without introducing preemptive guest data races.

This is concurrency, not a promise of simultaneous guest instruction execution. Host operations may complete on bounded workers or engine async facilities, but guest continuation scheduling remains cooperative in v1.

### 4.2 Deferred multi-Store model

Multi-Store execution is deferred as an isolated-worker architecture, not an optimization switch. A future worker must have:

- its own Store, arena, capability/budget intersection and terminal lifecycle;
- no shared Wasm references, borrowed memory, resource handles or ambient Host state;
- messages copied through a versioned bounded canonical ABI/message contract;
- explicit parent/worker cancellation, backpressure and failure propagation;
- separate evidence that persistent runners cannot reuse authority-bearing state.

No M11 exit claim may depend on unimplemented multi-Store behavior.

## 5. Store, arena and resource reconciliation

The STEP-0103 ADR must freeze all of the following rules.

### 5.1 Store and run lifetime

- Every top-level execution creates one fresh Store.
- All tasks in that run are owned by the Store and are terminal before Store teardown completes.
- Persistent watch/REPL/debug runners may reuse Engine, compiled Component and immutable Linker configuration only.
- A new generation receives a new Store, task table, queues, resources, cancellation tree and authority intersection.
- No task, completion, timer, event or Host worker from generation N may publish into generation N+1.

### 5.2 Arena and suspension rule

- A task may not suspend while holding a borrow into an ephemeral canonical ABI or Host-call scratch arena.
- Before a yield point, any value needed later is lifted/copied/moved into bounded task-owned Runtime storage.
- `post-return` runs after the call's lifted values no longer reference callee memory and before another operation can reuse that call arena.
- Task frames and owned continuations have explicit byte/count limits charged to the run, not hidden per-task unbounded heaps.
- Terminal completion, cancellation, panic/trap and Store teardown all execute idempotent cleanup of task-owned arenas.

### 5.3 Affine resources across tasks

- A resource has one owning Store and one live owner at a time.
- Moving a resource to a child invalidates the parent handle until an accepted result transfers ownership back.
- Borrowed use cannot cross a suspension point unless the resource contract explicitly provides a bounded suspension-safe lease.
- Concurrent use requires a dedicated Host resource whose protocol serializes access; cloning a raw handle is forbidden.
- Scope cancellation closes child-owned resources in deterministic reverse ownership order, subject to bounded Host cleanup deadlines.
- Use-after-move, double close, cross-Store handle use and task completion with uncollected affine resources fail with stable typed diagnostics.

## 6. Structured-concurrency semantics

### 6.1 Scope and task lifecycle

Each task belongs to exactly one task scope and has a deterministic run-local `taskId`, optional `parentTaskId` and state:

`created → runnable ↔ suspended → completing → succeeded | failed | cancelled`

The contract must define:

- spawn only while the parent scope is open;
- no detached or daemon task in v1;
- scope exit waits for or cancels all children;
- a Task result is consumed at most once unless its value type is explicitly copyable;
- child failure policy is explicit: propagate, collect as data, or trigger sibling cancellation;
- parent cancellation propagates downward; child cancellation never widens upward authority;
- all terminal races publish exactly one terminal state and one collection result.

### 6.2 Await, task groups and collection

M11 supports the semantics needed for bounded:

- `await task`;
- task groups with a fixed maximum child count;
- collect-all in deterministic task creation order;
- first-success/first-terminal race with cancellation of losing children;
- select over a bounded set of task/stream/timer readiness sources.

Syntax remains subject to the existing language RFC process. Runtime implementation must not invent source spelling before the semantic/IR contract is accepted.

### 6.3 Determinism and tie-breaking

External completion timing is observational and cannot be made globally deterministic. Observable scheduler choices are nevertheless fixed:

- task IDs increase monotonically within one run;
- FIFO ready ordering applies within the same priority class;
- completions observed in one scheduler turn are ordered by readiness class, then registration order, then task ID;
- race/select chooses the first item in that canonical ordering;
- collect-all returns creation order, not wall-clock completion order;
- event sequence IDs record the chosen order.

Changing source, compiler, Component or accepted scheduler contract may change task IDs; no cross-build stable ID claim is made.

## 7. Scheduler, Host operations and bounds

### 7.1 Cooperative scheduler

Yield points include:

- explicit await/select/task-group operations;
- bounded stream or channel send/receive;
- asynchronous Host/provider completion;
- timer/deadline wait;
- fuel/epoch/cancellation checkpoints accepted by the Runtime contract.

Pure guest code that never reaches a cooperative yield remains interruptible through the M10 fuel/epoch cancellation mechanism. M11 does not add native guest threads or shared-memory preemption.

### 7.2 Host completion boundary

Blocking Host work may use bounded Runtime-owned workers or engine async support. It returns only a completion record containing run/generation/task/operation identity and bounded typed data. The scheduler rejects stale, duplicate, late or cross-generation completion records. A worker never owns Script authority independently of its run and cannot publish directly to another task.

### 7.3 Candidate hard limits

STEP-0103 must accept or revise these values explicitly:

| Dimension | Candidate hard limit |
|---|---|
| live tasks per run | 1,024 |
| task-scope nesting | 64 |
| runnable queue | 1,024 entries |
| Host completion queue | 1,024 entries and 16 MiB |
| total scheduler/task metadata | 16 MiB per run |
| select/race operands | 256 |
| task group children | 1,024 |
| channel/stream capacity | 1,024 items with an independent byte budget |
| events | inherits M10 item/byte limits |
| cleanup | bounded deadline per Host resource and per Store teardown |

Fairness quantum, cancellation observation latency and scheduling throughput are measured non-SLA goals until the ADR/RFC explicitly promotes a value. Count, byte, fuel, time and Host-operation budgets are cumulative across all tasks in the run; spawning never multiplies a package limit.

## 8. Authority and security model

Parallel execution is Runtime-internal and introduces no capability.

- A child task inherits an immutable subset/equal view of the parent run's accepted capability grants.
- No task API can request, discover or synthesize a new endpoint, path, secret, process or platform privilege.
- Capability checks remain at the same Host/provider boundary and use the same package/Host/invocation intersection.
- A concurrency quota limits consumption; it is not represented as effect authority.
- Task IDs and scheduler controls are not ambient handles and cannot address another run.
- DAP pause/continue/inspection cannot alter capability grants or resource ownership.

The threat corpus must cover spawn bombs, ready-queue starvation, cancellation storms, completion replay, stale generation injection, cross-task resource aliasing, use-after-move, deadlock cycles, event floods, Host workers surviving Store teardown and debugger-induced suspension leaks.

## 9. M10 observability and DAP integration

M11 consumes the `runId`, `generationId`, `taskId`, `parentTaskId`, `scopeId` and causal fields reserved by M10. It adds typed lifecycle events for task creation, suspension, readiness, cancellation propagation and terminal collection without creating a second event transport.

STEP-0103 must decide DAP semantics before implementation:

- `threads` maps logical Runtime tasks only if the adapter can represent their state honestly;
- global pause versus selected-task pause is explicit; v1 should prefer global safe-pause if Store execution cannot isolate one task;
- stack/scopes/variables remain bounded and identity matched;
- stopped/continued events name the affected task set exactly;
- scheduler internals, Host secrets and other tasks' inaccessible values remain hidden;
- task-aware debugging adds observation only and never mutation, evaluation or authority.

If these semantics exceed the exact M10 claimed subset, a versioned DAP claim update and new evidence are required; implementation cannot silently advertise extra capabilities.

## 10. Execution sequence

### STEP-0103: Store/arena structured-concurrency ADR

This documentation-only design step may begin after STEP-0098 while STEP-0099–0102 continue.

Deliver:

- ADR comparing single-Store cooperative scheduling and multi-Store isolated workers;
- accepted v1 model and explicit rejection/defer rationale for the alternative;
- Store/generation/task ownership graph;
- canonical ABI scratch-arena, suspension and `post-return` rules;
- affine resource move/borrow/cleanup rules;
- cancellation tree, terminal-winner and persistent-runner isolation model;
- initial hard-limit and platform-evidence matrix.

Exit evidence:

- adversarial lifetime traces cover await during Host calls, cancel during lifting/lowering, late completion, Store teardown and next-generation startup;
- no rule permits a borrowed arena reference, resource handle or completion record to outlive its owner;
- the ADR states that parallelism adds no capability;
- no scheduler code or support claim precedes ADR acceptance.

### STEP-0104: semantic and IR structured-concurrency contract

Implementation begins only after M10 GO.

Deliver:

- reuse the M9 accepted Task/Future/Stream source surface; any proven missing spelling requires a separate RFC and is not implied by scheduler work;
- HIR/type/effect rules and affine Task/resource flow;
- deterministic task/select semantics and stable diagnostics;
- verified IR operations and canonical lowering contract.

Exit evidence:

- positive/negative fixtures cover move, double-await, escape, uncollected task, illegal borrow across suspension and unsupported detached task;
- effect/capability closure is identical before and after adding task structure;
- malformed IR cannot create unbounded queues, detached tasks or cross-scope handles;
- formatter, diagnostics, semantic index and AI query surfaces understand the accepted syntax without execution authority.

### STEP-0105: single-Store cooperative scheduler core

Deliver:

- bounded task table, scope tree and FIFO ready queue inside one fresh Store;
- deterministic run-local identities and lifecycle transitions;
- fuel/epoch/cancellation checkpoints and bounded Host completion ingress;
- cumulative run-level accounting across tasks.

Exit evidence:

- 1, 2, 16, 256 and 1,024-task workloads execute within accepted bounds;
- task 1,025 and every queue/metadata limit+1 case fail with stable typed outcomes;
- same-turn readiness follows the accepted canonical order;
- stale/duplicate/cross-run completion records fail closed;
- repeated runs show no task, handle or RSS growth across Store teardown.

### STEP-0106: cancellation, timeout, race and select

Deliver:

- downward cancellation tree integrated with M10's one terminal-winner path;
- bounded task groups, collect-all, first-success/first-terminal and select;
- deterministic loser cancellation and resource cleanup;
- deadlock/no-runnable detection where the Runtime can prove it.

Exit evidence:

- completion-vs-cancel, timeout-vs-cancel, parent-vs-child failure and simultaneous-ready matrices have one result;
- losing race branches cannot emit a second terminal result or retain Host work;
- nested cancellation of 1,024 tasks remains within queue/time/memory bounds;
- collection ordering matches the contract under repeated adversarial scheduling.

### STEP-0107: bounded channels, streams and backpressure

Deliver:

- task-aware bounded channel/stream producer and consumer resources;
- explicit close, drop, cancellation and error propagation;
- item and byte accounting shared with task/run budgets;
- fairness behavior for slow producer/consumer combinations.

Exit evidence:

- capacity 0/1/max/max+1, slow consumer, early close, producer failure and cancellation fixtures;
- backpressure prevents queue growth and busy spinning;
- resource move/use-after-close/double-close cases fail deterministically;
- large payload streams keep RSS independent of total stream size apart from accepted buffers.

### STEP-0108: persistent runner, watch, REPL and DAP task integration

Deliver:

- generation-isolated scheduler lifecycle in persistent processes;
- task-aware M10 execution events and exact DAP claim update where needed;
- watch/REPL cancellation and replacement rules;
- bounded editor and AI summaries of task causality.

Exit evidence:

- replacing a watch generation cancels and joins all old tasks before the new Store publishes;
- REPL cells cannot retain tasks/resources into a later Store unless a separately accepted persistent-state contract exists;
- pause/disconnect/terminate cannot strand tasks or workers;
- 100 sequential sessions with changing grants show no state, event, authority, handle or RSS leak;
- AI/tooling remains a bounded data consumer and cannot spawn/control tasks directly.

### STEP-0109: cross-platform native runner parity

Deliver:

- the same scheduler, cancellation, stream, Store-isolation and DAP corpus on Windows x64 and Linux x64 native runners;
- platform-specific worker/signal notes without changing abstract outcomes;
- reproducible environment and raw evidence for each claimed platform;
- macOS evidence only when an actual native runner is available.

Exit evidence:

- Windows x64 and Linux x64 both pass the hard correctness/resource corpus;
- cross-compilation, schema fixtures and generated launch files do not count as native Runtime evidence;
- no macOS, Android or Harmony Runtime claim is made without native execution;
- platform differences are recorded as typed, bounded behavior rather than ignored.

Absence of a Linux x64 native runner makes M11 NO-GO; Windows-only success is insufficient.

### STEP-0110: M11 security, performance and platform exit audit

Deliver:

- aggregate validator for STEP-0103–0109 plus M0–M10 regression;
- Store/arena/resource security review and adversarial scheduler corpus;
- throughput, latency, fairness, cancellation, RSS and handle report;
- Windows x64/Linux x64 actual-platform matrix;
- explicit GO/NO-GO, residual risks, external-input recheck and M12 decision.

Exit evidence is defined in sections 11–13.

## 11. Performance and resource evidence

The minimum performance matrix includes:

- sequential baseline versus 2/16/256 runnable tasks;
- 1/16/256/1,024 mostly suspended tasks;
- small and maximum bounded task results;
- slow Host completion and slow stream consumer backpressure;
- simultaneous completion, cancellation storm and deep scope teardown;
- 100 fresh Stores and 100 generations in persistent runner mode;
- ready/completion/event queue saturation with exact peak RSS and handle counts.

Reports record raw samples, sample count, machine/OS/runtime versions and host-load caveats. Throughput/fairness goals remain non-SLA unless promoted in STEP-0103. Hard count/byte/time/security limits and absence of leaks are pass/fail gates.

## 12. Platform evidence policy

- Windows x64 and Linux x64 native runner execution are mandatory for M11 GO.
- The same accepted Component fixtures and semantic outcomes run on both platforms.
- OS signal and Host-worker mechanics may differ, but cancellation and terminal records remain protocol-equivalent.
- macOS is claimed only after native execution of the same corpus; otherwise it remains unclaimed.
- Android/Harmony remain separate Host/Runtime tracks and inherit no desktop result.
- CI labels, cross-compilation and unit-only simulation are supporting evidence, never native runner evidence.

## 13. M11 exit gate

M11 is GO only when:

1. STEP-0103 accepts the Store/arena ADR before scheduler implementation;
2. one fresh Store per run and persistent-generation isolation remain intact;
3. no task suspends with an illegal scratch-arena borrow and `post-return` cleanup is correct;
4. affine resources move, close and cancel without alias, leak or cross-Store use;
5. spawn/await/group/select/race semantics have one bounded deterministic outcome;
6. cumulative task, queue, memory, fuel, time and Host-operation limits resist limit+1 and stress cases;
7. M10 task-aware events/DAP integration is bounded, identity matched and authority neutral;
8. cancellation joins or abandons Host work under an explicit bounded policy with no stale publication;
9. Windows x64 and Linux x64 native runners pass the same correctness/security/resource corpus;
10. the complete M0–M10 regression is green and the external-input recheck is recorded.

Any detached task, capability widening, arena borrow across suspension, cross-Store resource sharing, unbounded queue, double terminal result, stale generation publication or Windows-only exit claim is an automatic NO-GO.

## 14. Handoff to M12

M12 may begin only after M11 GO freezes task ownership, Store/arena lifetime, cancellation and in-flight Host-operation accounting. The planned successor is [`M12 Secure HTTP Provider and Automation SDK`](./M12-secure-http-automation-sdk.md), which must use the Runtime scheduler rather than create a provider-local concurrency model.

At STEP-0110, the project must first recheck public deployment, production identity, third-party, live-model and mobile-runner inputs. If one is available, that external track may resume in parallel under its existing gate; M12 remains internal engineering evidence and cannot substitute for it.
