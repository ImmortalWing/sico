# Stream

本组验证 SEM-090、SEM-100、SEM-105：Stream 是 affine、异步、带错误与背压的资源，不是可以隐式同步遍历或无界收集的 List。

## 必须接受

| Case | 规则 |
|---|---|
| [`STREAM-001`](./valid/pull-one.sico) | `next` 被 await，并保留 Result 与 Option 两层含义 |
| [`STREAM-002`](./valid/bounded-collect.sico) | 收集 Stream 时提供显式数量上限 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`STREAM-101`](./invalid/unbounded-collect.sico) | `UNBOUNDED_STREAM_COLLECT` | stream collection requires an explicit limit |
| [`STREAM-102`](./invalid/missing-await.sico) | `ASYNC_VALUE_REQUIRES_AWAIT` | stream.next returns a Future; await it |

## 语义摘要

- `next()` 返回 `Future<Result<Option<T>, E>>`；
- `None` 是正常结束，`Error` 是预期流错误，取消不塞入 `E`；
- 消费者每次拉取形成背压；
- `collect(limit: n)` 的限额是 API 请求上限，不能扩大 Runtime policy。

## Component/WIT 映射

Stream 优先映射 Component async stream/future 形状；`T` 与领域错误 `E` 保持类型化。资源 drop/cancel 关闭生产端，具体 ABI 版本留待技术原型决定。
