# Future and Task

本组验证 SEM-072、SEM-100—104：调用异步函数先产生 affine Future，并发只能显式 spawn 到可见任务作用域，Future/Task 只能消费一次。

## 必须接受

| Case | 规则 |
|---|---|
| [`TASK-001`](./valid/await-once.sico) | Future 被显式等待一次 |
| [`TASK-002`](./valid/structured-pair.sico) | 任务在作用域内按明确输入顺序收集 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`TASK-101`](./invalid/await-twice.sico) | `FUTURE_CONSUMED` | pending was already awaited |
| [`TASK-102`](./invalid/task-escapes-scope.sico) | `TASK_ESCAPES_SCOPE` | task cannot leave its task group |

## 语义摘要

- 调用 `async fn` 只创建 Future，不自动并发执行；
- `await` 消费 Future；
- `spawn` 创建属于当前 `task group` 的 Task；
- `await collect_tasks(..., order: input)` 明确结果顺序并消费任务句柄。

## Component/WIT 映射

Future/Task 优先映射 Component async 能力，但本案例不绑定 ABI 版本。领域 `Result` 仍位于 Future 的普通完成值中；取消和 trap 走独立控制/隔离通道。
