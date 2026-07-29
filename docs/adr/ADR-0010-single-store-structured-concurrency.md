# ADR-0010: single-Store cooperative structured concurrency

> - status: accepted-design; implementation gated on M10 GO
> - date: 2026-07-23
> - phase: M11 STEP-0103

## Decision

M11 uses one fresh Wasmtime Store per top-level run and a bounded cooperative scheduler inside that Store. Engine, compiled Component and immutable Linker configuration may be reused; Store, scheduler, task table, arenas, resources, queues, cancellation tree and authority intersection may not cross a run or persistent generation.

Multi-Store execution is deferred. It is a future isolated-worker architecture with copied, versioned bounded messages, not an optimization mode and never a way to share Wasm references or Host handles.

## Ownership graph

```text
runner process
  Engine + immutable Component/Linker cache
  run/generation
    fresh Store + immutable authority intersection
    root scope
      task -> owned continuation/values/resources
      child scope -> child tasks
    bounded ready/completion/event queues
    one cancellation tree + terminal arbiter
```

Every task has one run-local ID, one scope and at most one parent. Scope exit collects or cancels every child; detached tasks are forbidden. Store teardown begins only after all task and Host-operation records are terminal or classified as abandoned under a bounded cleanup deadline.

## Arena and suspension

A task cannot suspend while borrowing canonical ABI memory, a Host-call scratch arena, or a non-suspension-safe resource lease. Values needed after a yield are lifted, copied or moved into task-owned bounded storage first. `post-return` completes after lifted values stop referring to callee memory and before the call arena is reused. Success, fault, cancellation and teardown run the same idempotent cleanup path.

Cancel during lowering refuses publication of a partial call. Cancel during lifting finishes only the bounded lift required to restore ABI ownership, discards the value and runs `post-return`; it never resumes guest work. Allocation and byte accounting are checked before copying.

## Affine resources

A resource belongs to one Store and has one live owner. Moving to a child invalidates the parent handle until an explicit result moves ownership back. A borrow cannot cross suspension unless its provider defines a bounded suspension-safe lease. Concurrent access requires a provider-owned serializing resource; raw handle cloning is forbidden. Cancellation closes child resources in deterministic reverse ownership order. Use-after-move, double close, uncollected terminal resources and cross-Store use are typed failures.

## Scheduling and terminal rules

Task IDs and registration IDs increase monotonically. Runnable tasks use FIFO order. Completions observed in one turn are ordered by readiness class, registration order, then task ID. Collect-all returns creation order; race/select chooses that canonical first item and cancels losers. Parent cancellation propagates down; child cancellation cannot widen authority upward. Each task and the run commit exactly one terminal state.

Host workers publish bounded completion records containing run, generation, task and operation identity. Stale, duplicate, late and cross-generation records are rejected. A worker owns no Script authority and cannot address another task directly.

## Hard limits

| Dimension | Limit |
|---|---:|
| live tasks per run | 1,024 |
| scope nesting | 64 |
| runnable queue | 1,024 |
| Host completion queue | 1,024 records / 16 MiB |
| scheduler and task metadata | 16 MiB |
| select/race operands | 256 |
| task-group children | 1,024 |
| channel capacity | 1,024 items plus an explicit byte budget |
| events | M10: 256 records / 4 MiB |

Fuel, time, memory, Host-operation and capability budgets belong to the run; spawning never multiplies them.

## Adversarial lifetime traces

1. Host await: lift/copy arguments, release scratch borrows, register operation identity, suspend, validate completion identity, resume.
2. Cancellation during ABI work: prevent partial publication, restore ownership, run `post-return`, commit one cancellation terminal.
3. Late completion: generation mismatch or terminal operation ID rejects the record without waking a task.
4. Store teardown: close scopes and resources, expire operation IDs, stop workers under deadline, then drop Store.
5. Next generation: allocate new Store and identity tables; prior tokens, handles, timers, events and completions cannot resolve.

## Authority and debugging

Concurrency is not a capability. Children see an immutable subset/equal view of the accepted run grants. Scheduler controls, task IDs and DAP requests cannot add endpoints, paths, secrets or providers. M11 maps DAP threads only to logical Runtime tasks and uses global safe-pause unless a later version proves selected-task isolation.

## Platform evidence

Windows x64 and Linux x64 must run the same scheduler/lifetime corpus natively before M11 GO. Cross-compilation, WSL absence or protocol fixtures do not count as native Linux evidence. macOS and mobile remain unclaimed without native runners.

## Consequences

The chosen model preserves M8 fresh-Store isolation and affine ownership, and makes cancellation/event ordering auditable. It does not promise simultaneous guest instruction execution. Multi-Store workers, shared Wasm memory, detached tasks and authority-bearing background services remain outside M11.
