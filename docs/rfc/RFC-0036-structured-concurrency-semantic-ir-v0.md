# RFC-0036: structured-concurrency semantic and IR contract v0

> - status: accepted
> - date: 2026-07-29
> - authors: autonomous-agent
> - target phase: M11
> - supersedes: -
> - superseded-by: -

## Summary

把 M9 已接受的 Task/Future/Stream 源码表层提升为显式、可验证的语义与 IR 契约：task scope 具有静态身份，Task handle 是 scope 内 affine 值，`spawn` 是 eager-start 结构化创建，`await`/`collect_tasks` 是确定性消费，借用不得跨越 suspension point，`sico.ir.v0` 增加 canonical task-scope 表与四个操作。本 RFC 不引入任何新源码拼写；race/select 拼写仍被拒绝，直到独立 RFC 接受。

本 RFC 冻结的是语义、诊断、IR 与顺序执行投影；Runtime 调度器（STEP-0105+）必须实现同一契约，不得在代码里隐式改变 spawn 语义。

## 1. Source surface (unchanged)

接受的拼写完全来自 M9：`async function`、`task group:`/`end task`、`spawn f(args)`、`await x`、`await collect_tasks([...], order: input)`、`using`/stream 既有形式。`collect_tasks` 的 `order` 只接受 prelude `TaskOrder` 字面量 `input`；其他值是普通类型错误。

## 2. Spawn semantics: eager-start

`spawn f(x)` 要求 `f` 是 `async function` 且返回 `Future[T]`；结果为 `Task[T]`。语义为 **eager-start**：task 在 spawn 点按源码顺序立即开始执行；在 v1 sequential profile 中 task 没有 guest 可见 suspension，因此 spawn 等价于立即求值并产生已解决的 `Task[T]` handle。effect 顺序因此严格等于源码顺序，与 M9 (STEP-0087) 实测行为逐字节一致。

选择 eager-start 的理由与撤销条件见 [`STEP-0104`](../steps/STEP-0104-semantic-ir-structured-concurrency.md) §4.1。ADR-0010 的 canonical order（creation order、FIFO）与之兼容：eager-start task 按创建顺序完成，collect 返回创建顺序。

## 3. Scope and affine rules

1. 每个 `task group:` 块创建一个静态 task scope；scope 可嵌套，静态深度不得超过 64。
2. `spawn` 只能出现在至少一个 open task scope 内，handle 归属于最内层 scope；否则 E5104 `TASK_DETACHED`。
3. `Task[T]` handle 是 affine：必须且只能被其所属 scope 内的一个 `await`（或作为元素被一个 `collect_tasks` list）消费一次；重复消费仍是 E5101 `FUTURE_CONSUMED`。
4. scope 关闭时仍有未消费 handle → E5103 `TASK_NOT_CONSUMED`，指向未消费的 binding。
5. `Task[T]` 不得离开其 scope：作为 `return` 值、作为调用实参、或存入逃逸出 scope 的 aggregate → E5102 `TASK_ESCAPES_SCOPE`（判定由 depth 启发式升级为 scope 身份）。
6. `await` 是 suspension point；在其同一函数内处于 live 状态的 resource `borrow` 不得跨越该点 → E5003 `BORROW_ACROSS_SUSPENSION`。
7. Future（非 spawn 的 async 调用结果）规则不变：consume-once（E5101），不受 scope 约束。
8. spawn 不增加任何 effect/capability：task 结构的 effect 闭包与无 task 结构的等价函数必须逐项相同；被 spawn 的 callee 声明 effect 必须是 caller 声明 effect 的子集。

## 4. collect_tasks

`collect_tasks(tasks: List[Task[T]], order: input) -> List[T]` 是 prelude intrinsic（RFC-0007 已登记），本 RFC 赋予其实现语义：

- 消费 list 中每个 handle（各计一次消费）；
- 结果顺序为 task 创建顺序（`order: input`），不是 wall-clock 完成顺序；
- 空 list 合法，返回空 list；
- 在 sequential profile 中 `Task[T]` 与已解决的 `T` 表示相同，collect 即按序构造结果 list；Component 边界上的 `Task`/`Future`/`Stream` 类型仍按 RFC-0013 typed-refused。

## 5. IR contract (`sico.ir.v0` extension)

### 5.1 Task-scope table

每个 function 可声明 canonical `task_scopes` 表：`{ scope: u32, parent: u32 | null }`，按 id 升序、parent 必须先声明、parent 链深度 ≤ 64。无 task 结构的 function 不携带该表（序列化形状不变）。

### 5.2 Operations

- `task-scope-open { scope }` / `task-scope-close { scope }`：标记 scope region；必须 LIFO 平衡；IR v1 中 open 与 close 必须位于同一线性 block（跨控制流 region 拒绝为 `TaskViolation`，留待有证据后放宽）；
- `spawn { scope, callee, args } -> Task[T]`：scope 必须 open；callee 必须存在且返回 `Future[T]`；callee 声明 effect ⊆ caller 声明 effect；
- `await`（既有操作，现在开始真实 emission）：操作数 `Task[T]`/`Future[T]` → `T`；
- `task-collect { scope, tasks } -> List[T]`：`tasks` 为 `List[Task[T]]`；scope 必须 open。

### 5.3 Verifier rules

新 `VerifyErrorKind::TaskViolation` 覆盖：未声明/未 open 的 scope 引用、非 LIFO 或跨 block region、parent 链深度超限、scope 内 spawn 数超过 1,024、Task 值被消费零次或多次、Task 值在其 scope region 外使用、spawn callee 非 async 或 effect 越界、`task-collect` 操作数类型错误。既有 `Limit` 继续覆盖计数上限。malformed IR 因此无法创建 unbounded 队列、detached task 或 cross-scope handle。

async 函数返回解析：IR 中 async function 声明 `Future[T]` 返回类型；其 `Return` 终结符的操作数可以是已计算的 `T`（eager-start 下 return 即解析该 future）或一个已为 `Future[T]` 的值（async 透传）。两种形状在 sequential profile 下表示相同，verifier 均接受；其他类型错配仍是 `TypeMismatch`。

### 5.4 Sequential codegen profile

`sequential-v1` profile 的 canonical 投影：`task-scope-open/close` 无运行时代价；`spawn f(args)` = 在 args 按 RFC-0009 顺序求值后直接调用 `f`；`await x` = identity；`task-collect` = 按 list 顺序构造结果。`Task[T]`/`Future[T]` 的运行时表示与 `T` 相同。任何把 `Task`/`Future`/`Stream` 类型暴露到 Component import/export 的 function 仍触发 RFC-0013 `AsyncUnsupported`。

## 6. Diagnostics

| code | key | 含义 |
|---|---|---|
| E5003 | `BORROW_ACROSS_SUSPENSION` | live borrow 跨越 `await` suspension point |
| E5103 | `TASK_NOT_CONSUMED` | scope 关闭时存在未消费的 spawned handle |
| E5104 | `TASK_DETACHED` | `spawn` 出现在任何 task scope 之外 |
| E5105 | `TASK_SCOPE_LIMIT` | task-scope 嵌套、单 scope spawn 数或 collect 长度超过 §8 上限（arg: `dimension`） |

E5101/E5102 保持不变；E5102 的判定从 depth 启发式升级为 scope 身份，对外诊断文本与 span 策略不变。catalog、semantic-case-map 与 fixture 同步登记。

## 7. Tooling and index

semantics 为 spawn/await/collect 发射 scope 限定的 `AsyncState`/`StreamOperation` facts（名称格式 `{scope}:{name}->pending|->awaited|->collected`）。outline/Flow 查询把它们作为既有 fact 流暴露；formatter 无新语法；LSP/AI 工具不获得任何执行或 task 控制权限。

## 8. Limits (aligned with ADR-0010)

| Dimension | Limit |
|---|---:|
| task-scope 静态嵌套 | 64 |
| 每 scope spawn 数 | 1,024 |
| collect list 长度 | 1,024 |
| IR v1 scope region | 单一线性 block |

## 9. Non-goals

- Runtime 调度器、ready queue、guest suspension（STEP-0105）；
- race/select 源码拼写与语义（独立 RFC）；
- bounded channel/stream runtime（STEP-0107）；
- watch/REPL/DAP task 集成（STEP-0108）；
- 跨 Component Task/Future/Stream WIT handle（继续拒绝）。

## 10. Compatibility

TASK-002 (`collect_tasks`) 从 lowering-refused 升级为可执行，这是本 RFC 显式授权的语义变化；STEP-0087 的历史验证器随之更新为执行证据。其余所有既有合法/非法案例的判定不变。eager-start 把 M9 的实现行为正式化，因此无运行行为迁移成本。
