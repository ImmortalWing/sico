# Order state machine specification

> 状态：代表性程序草案
> 目的：验证封闭状态、显式转换、非法路径和 AI 状态流理解

## 1. 功能

该模块定义一个最小订单生命周期。状态只能通过 `transition` 函数改变，调用者不能构造规范之外的状态跳转。

## 2. 状态

```text
Draft
PendingPayment
Paid
Shipped
Cancelled
```

`Shipped` 和 `Cancelled` 是终止状态。

## 3. 事件

```text
Submit
Pay
Ship
Cancel
```

## 4. 合法转换

```text
Draft          + Submit → PendingPayment
Draft          + Cancel → Cancelled
PendingPayment + Pay    → Paid
PendingPayment + Cancel → Cancelled
Paid           + Ship   → Shipped
```

状态图：

```text
Draft ──Submit──> PendingPayment ──Pay──> Paid ──Ship──> Shipped
  │                    │
  └────Cancel──────────┴────Cancel──────────────> Cancelled
```

除以上组合之外，所有转换均返回 `InvalidTransition`。

## 5. 正式错误

```text
InvalidTransition(status: OrderStatus, event: OrderEvent)
```

错误携带原状态和事件，便于编译器、AI 和调用者定位失败路径。

## 6. 类型、效果与能力

| 定义 | 返回 | 效果 | 能力 |
|---|---|---|---|
| `transition` | `Result<OrderStatus, TransitionError>` | 无 | 无 |

`transition` 是纯函数：不读取存储、不访问网络、不修改外部状态。

## 7. 不变量

- 每次成功转换只产生一个已声明状态；
- `Shipped` 没有任何后续转换；
- `Cancelled` 没有任何后续转换；
- 未列出的状态与事件组合不得被默认为成功；
- 状态转换不产生副作用；
- 相同输入始终产生相同结果。

## 8. 测试矩阵

### 8.1 成功

| 当前状态 | 事件 | 新状态 |
|---|---|---|
| `Draft` | `Submit` | `PendingPayment` |
| `Draft` | `Cancel` | `Cancelled` |
| `PendingPayment` | `Pay` | `Paid` |
| `PendingPayment` | `Cancel` | `Cancelled` |
| `Paid` | `Ship` | `Shipped` |

### 8.2 失败

至少验证：

| 当前状态 | 事件 | 结果 |
|---|---|---|
| `Draft` | `Pay` | `InvalidTransition` |
| `PendingPayment` | `Ship` | `InvalidTransition` |
| `Paid` | `Pay` | `InvalidTransition` |
| `Shipped` | `Cancel` | `InvalidTransition` |
| `Cancelled` | `Submit` | `InvalidTransition` |

完整测试最终应覆盖全部 `5 × 4 = 20` 个组合，其中 5 个成功、15 个失败。

## 9. AI 语义概要目标

编译器应能生成：

```text
module: order-state
purpose: validate order status transitions
states: Draft, PendingPayment, Paid, Shipped, Cancelled
events: Submit, Pay, Ship, Cancel
transitions: 5
terminal states: Shipped, Cancelled
errors: InvalidTransition
effects: none
capabilities: none
```

AI 不读取函数体也应能回答：

- 一个订单从草稿到发货要经过哪些状态；
- 哪些状态可以取消；
- 哪些是终止状态；
- 增加退款事件会影响哪些类型、转换和测试；
- 状态转换是否访问外部能力。

## 10. 候选 A 暴露的开放问题

- 是否使用普通 `enum + match` 表达状态机，还是提供专门状态机声明；
- 元组及元组模式的正式语法；
- 枚举变体是否必须使用类型名限定；
- 通配分支是否会隐藏新增状态缺少专门处理；
- 是否需要编译器区分“有意拒绝”与“遗漏转换”；
- `test` 和 `expect` 是否属于语言、标准库还是外部测试 DSL；
- 多行表达式如何续行，是否完全由语法上下文判断；
- `Result` 值的相等性规则；
- 编译器如何从普通函数生成可信状态图；
- 终止状态是否需要正式声明；
- 状态机变更如何进入语义哈希和影响分析。

这些问题需要与账户转账、异步任务和 UI 状态等后续样本共同决定。
