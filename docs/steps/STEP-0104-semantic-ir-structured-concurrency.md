# STEP-0104: semantic and IR structured-concurrency contract

> - status: complete
> - phase: M11
> - started: 2026-07-29
> - completed: 2026-08-31
> - owners: autonomous-agent

## 1. Objective

Turn the M9 Task/Future/Stream source surface into an explicit, verified semantic and IR contract that the STEP-0105 single-Store cooperative scheduler can consume, without changing any accepted spelling and without regressing the M9 sequential end-to-end evidence.

Verifiable results:

- scope-aware affine Task rules with stable diagnostics E5103/E5104/E5003 alongside existing E5101/E5102;
- `collect_tasks` becomes a typed intrinsic with deterministic creation-order semantics instead of an accidental unknown callee;
- `sico.ir.v0` gains a canonical task-scope table plus `TaskScopeOpen`/`TaskScopeClose`/`Spawn`/`TaskCollect` operations and task-specific verifier rules;
- lowering emits faithful task IR (no more silent spawn/await erasure);
- the sequential codegen profile projects that IR to the exact M9 observable behavior and additionally executes `collect_tasks`, while Task/Future/Stream handles crossing a Component boundary remain refused per RFC-0013.

## 2. Context and evidence

- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md) STEP-0104 deliverables and exit evidence.
- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md): ownership graph, suspension/arena rules, affine resource rules, deterministic tie-breaking (§6.3), hard limits (1,024 tasks/children, scope nesting 64).
- [`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md): Task is language-internal; borrow must not escape across await; completion consumes handle; no unbounded stream collect.
- [`RFC-0007`](../rfc/RFC-0007-prelude-contract-v0.md) / `semantics/prelude-v0.json`: Future/Task/Stream classes and the `collect_tasks`/`input` prelude entries.
- [`RFC-0009`](../rfc/RFC-0009-core-lowering-evaluation-order-v0.md): strict left-to-right evaluate-once lowering.
- [`RFC-0013`](../rfc/RFC-0013-async-backend-support-v0.md): backend async refusal boundary; no mock schedulers.
- Current implementation audit (2026-07-29): spawn/await are erased in `crates/sico-ir/src/lower.rs:620-635`; `Operation::Await`/`StreamNext` are never emitted; `collect_tasks` is refused as a generic unknown call target; E5102 is depth-based, not scope-based; runner executes no tasks at all.

## 3. Scope

In scope:

- semantics: task-scope tracking, consume-once/scope-discipline diagnostics, typed `collect_tasks`, borrow-across-suspension rejection, scope-qualified async facts;
- IR: task-scope table, four operations, verifier discipline and bounds;
- lowering: faithful task emission per RFC-0009 ordering;
- codegen: sequential-profile projection (eager spawn, identity await, in-order collect) plus retained RFC-0013 boundary refusals;
- fixtures (A0 + B mirrors), diagnostic catalog/case-map, semantic index facts, CLI end-to-end regression, validator.

Out of scope (later M11 steps):

- real Runtime scheduler, ready queues, task suspension of guest code, channels/streams runtime (STEP-0105–0107);
- watch/REPL/DAP task integration (STEP-0108);
- any new source spelling, race/select syntax (requires its own RFC);
- cross-Component Task/Future/Stream WIT handles (remains refused).

## 4. Options and decision

### 4.1 Spawn evaluation semantics (the central decision)

- **Candidate A (chosen): eager-start structured spawn.** `spawn f(x)` creates the task and runs it immediately until completion (v1 sequential profile: tasks have no guest-visible suspension), in strict source order. `await` only consumes the resolved handle. Effect ordering is therefore 100% source order, identical to M9's shipped behavior and to RFC-0009.
- Candidate B: deferred-start (child begins at parent's first suspension). More conventional for structured concurrency, but changes M9 observable behavior for parent effects between spawn and await, and would silently alter STEP-0087 evidence.
- Candidate C: unspecified interleaving between spawn and await. Cheapest to implement twice, but violates the project's determinism priority and makes AI reasoning about effect order unreliable.

Decision: A, recorded in [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md). Reversal condition: a future RFC with measured evidence may add a deferred-start profile; eager-start remains valid because the ADR-0010 scheduler may treat spawn as a scheduling action. ADR-0010's FIFO rule is preserved: eagerly started tasks complete in creation order, which is the canonical order used by collect.

### 4.2 Uncollected tasks

- Chosen: reject at compile time with E5103 (every spawned handle must be consumed by `await` or `collect_tasks` before its scope closes). Explicit data flow beats implicit scope-exit collection for AI-generated code; runtime scope-exit cancellation remains for failure/cancellation paths only (ADR-0010).
- Rejected alternative: silently cancel uncollected children at scope exit (hides logic errors; non-explicit).

### 4.3 IR representation

- Chosen: per-function canonical task-scope declaration table (id, optional parent) plus explicit `TaskScopeOpen`/`TaskScopeClose` marker operations and scope-qualified `Spawn`/`TaskCollect`. Verifier enforces canonical ordering, LIFO balance, single-block regions in IR v1, nesting ≤ 64, spawns per scope ≤ 1,024, and affine single-consumption of Task values.
- Rejected alternative: inferring scope lifetime from block dominance (no explicit markers) — harder to verify and easier for malformed IR to smuggle cross-scope handles.

### 4.4 collect_tasks in the sequential profile

- Chosen: fully supported. Under eager-start, a `Task[T]` value is representation-identical to a resolved `T`, so `collect_tasks([a, b], order: input)` lowers to list construction in creation order. This upgrades TASK-002 from lowering-refused to executable and is asserted in the updated STEP-0087 validator.
- Boundary unchanged: Task/Future/Stream as Component import/export types remain refused (RFC-0013).

### 4.5 Detached spawn and borrow across suspension

- `spawn` outside any `task group` scope is rejected with E5104 (no detached/daemon tasks in v1, ADR-0010).
- a `borrow` live across any `await` suspension point is rejected with E5003 (RFC-0004 already froze the rule; this implements it).

## 5. Plan

1. RFC-0036 (semantics, diagnostics, IR ops, sequential profile, limits) — this step's contract.
2. Semantics: scope stack, E5103/E5104/E5003, typed collect_tasks, scope-qualified AsyncState facts; B + A0 fixtures and case-map/catalog updates.
3. IR: scope table, operations, verifier rules/bounds, serialization, mutation tests.
4. Lowering: faithful emission; corpus accounting update (TASK-002 now lowers).
5. Codegen: sequential projection; collect support; boundary refusals retained; STEP-0087 validator updated to execution evidence.
6. Index/tooling: facts readable by outline/Flow queries; no execution authority changes.
7. `tools/validate-step-0104.ps1`, docs registry updates, full workspace + STEP-0087/0090 regression, commit.

## 6. Changes

- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md): full contract (semantics, diagnostics, IR ops, sequential profile, limits), including the async-return resolution verifier rule (§5.3).
- `crates/sico-semantics/src/lib.rs`: task-scope stack with static identity, E5103 `TASK_NOT_CONSUMED` / E5104 `TASK_DETACHED` / E5105 `TASK_SCOPE_LIMIT`, E5003 `BORROW_ACROSS_SUSPENSION`, scope-based E5102 escape detection, typed `collect_tasks` intrinsic (`order: input` only), scope-qualified `AsyncState` facts (`scope-{id}:{name}->pending|awaited|collected`).
- `crates/sico-ir/src/lib.rs`: `task_scopes` canonical table on `Module`, `TaskScopeOpen`/`TaskScopeClose`/`Spawn`/`TaskCollect` operations, `VerifyErrorKind::TaskViolation` rules (LIFO single-block regions, ≤64 nesting, ≤1,024 spawns/collect, affine exactly-once, effect closure), async `Future[T]` return-resolution rule in `verify_terminator`.
- `crates/sico-ir/src/lower.rs`: `TaskScopePlan` pairs `task group`/`end task` via HIR-annotated End tokens; async signatures wrap to `Future[T]`; `spawn`/`await`/`collect_tasks` emit faithful typed IR (no erasure); collect lists are positional `Construct` + scope-qualified `TaskCollect`.
- `crates/sico-ir/tests/{contract,core_lowering,flow_lowering}.rs`: task verifier mutation suites; lowering snapshots extended (TASK-001/TASK-002); corpus accounting now 19 lowered / 6 typed-refused / 33 invalid blocked.
- `crates/sico-codegen-wasm/src/{lib,canonical}.rs`: sequential-v1 projection — `flat_ir_types` unwraps `Task[T]`/`Future[T]` (and collect lists) to the resolved layout; `Spawn` compiles as the eager call, `Await`/`TaskCollect` as slot copies, scope markers as no-ops; collect lists are built in the bounded arena; Component boundary keeps RFC-0013 `AsyncUnsupported` refusals.
- `crates/sico-index/src/lib.rs`: regression test proving scope-qualified `AsyncState` facts ride the existing `compiler_facts` stream.
- Fixtures: `semantic-cases/{future-task,affine-resources}/invalid/{uncollected-task,detached-spawn,scope-nesting-limit,borrow-across-await}.sico` (+ B/C mirrors), `tests/end-to-end/script-task-collect.sico`, `tests/ir/flow-lowering.snap`, `tests/wasm/artifacts.hex` (task component snapshots).
- Registries: `diagnostics/catalog.json` + `semantic-case-map.json` (E5003, E5103–E5105; RES-103, TASK-103–105), `semantic-cases/manifest.json` (58 cases), CLI/AI-tools oracle counts (25/33).
- `tools/validate-step-0087.ps1`: collect_tasks assertion updated from typed refusal to execution evidence, as RFC-0036 §10 authorizes.
- `tools/validate-step-0104.ps1`: this step's validator.

## 7. Validation

Actual (2026-08-31, Windows x64 GNU, pinned toolchain 1.97.0 for validators):

- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --offline --locked`: all green (includes updated 58-case lexer/parser/formatter counts, 25/33 semantic oracle, 19/6 lowering accounting, task verifier mutation suites, new codegen projection and index fact-stream tests).
- `tools/validate-diagnostics.ps1` and `tools/validate-semantic-cases.ps1`: green (catalog 41 codes; case map 33 invalid; manifest 58).
- `sico check` on the four new invalid fixtures: exact E5103/E5104/E5105/E5003, exit 1.
- End-to-end (real Wasmtime runner): `script-task-pair` → `alpha! beta!` exit 0 (M9 behavior byte-exact); `script-task-cancelled` → exit 123 typed cancellation; `script-task-collect` → `alpha! beta!` exit 0 (`collect_tasks` creation-order execution, RFC-0036 §10 upgrade).
- `tools/validate-step-0087.ps1`: green with collect=executed; `tools/validate-step-0090.ps1`: green (generations 1/2/3, exits 0/122/0, warm median 6573 µs).
- `tools/validate-step-0104.ps1`: `STEP_0104_OK diagnostics=E5003/E5103-E5105 task-pair=exact cancel-edge=123 collect=executed step-0087=green`.

## 8. Metrics

None yet; step is correctness-contract work. Compile-time deltas will be measured if the corpus accounting changes materially.

## 9. Risks and follow-ups

- Eager-start must remain the documented semantic until a scheduler RFC says otherwise; STEP-0105 must not silently reinterpret Spawn.
- IR v1 single-block scope regions reject control-flow-spanning task scopes; relaxing needs evidence.
- Cross-Component async handles stay refused until a WIT contract exists.

## 10. Audit links

- [`RFC-0036`](../rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md)
- [`ADR-0010`](../adr/ADR-0010-single-store-structured-concurrency.md)
- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md)
