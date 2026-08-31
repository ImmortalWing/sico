# Affine resources

本组验证 SEM-020、SEM-061、SEM-090—092：资源默认不可复制，移动或关闭会消费所有权，所有受控退出路径执行已登记清理。

## 必须接受

| Case | 规则 |
|---|---|
| [`RES-001`](./valid/close-once.sico) | 资源只关闭一次 |
| [`RES-002`](./valid/scoped-cleanup.sico) | `using` 在 Result 返回路径清理资源 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`RES-101`](./invalid/use-after-move.sico) | `RESOURCE_MOVED` | reader was moved to owned |
| [`RES-102`](./invalid/use-after-close.sico) | `RESOURCE_CLOSED` | reader was already closed |
| [`RES-103`](./invalid/borrow-across-await.sico) | `BORROW_ACROSS_SUSPENSION` | borrow of reader is live across await |

## 语义摘要

- `borrow self` 只在调用期间借用资源；
- `close(self)` 消费资源；
- `move` 转移唯一所有权；
- `using` 登记确定性清理，但不会吞掉函数返回的 Result。

## Component/WIT 映射

`Reader` 映射为 Canonical ABI resource handle。移动只转移 handle 所有权，不复制宿主对象；`using` 的正常和错误退出都触发 resource drop。跨 Component 或任务传递仍需资源类型明确许可。
