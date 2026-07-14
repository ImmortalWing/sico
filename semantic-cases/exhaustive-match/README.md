# Exhaustive matching

本组验证 SEM-031 和 SEM-064：

- 封闭 enum 的匹配必须完整；
- 携带数据的变体可以显式忽略载荷；
- 多个封闭类型的乘积必须覆盖所有组合；
- `_` 不能把当前剩余变体与未来新增变体混在一起；
- 被前序分支完全覆盖的分支必须拒绝。

## 必须接受

| Case | 规则 |
|---|---|
| [`MATCH-001`](./valid/all-variants.sico) | 显式覆盖 enum 全部变体 |
| [`MATCH-002`](./valid/ignore-payload.sico) | 忽略已命名变体的载荷，不忽略变体本身 |
| [`MATCH-003`](./valid/all-product-cases.sico) | 显式覆盖两个封闭 enum 的所有组合 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`MATCH-101`](./invalid/missing-variant.sico) | `NON_EXHAUSTIVE_MATCH` | missing case: Blue |
| [`MATCH-102`](./invalid/wildcard-sealed-enum.sico) | `SEALED_MATCH_WILDCARD` | wildcard hides sealed variants: Green, Blue |
| [`MATCH-103`](./invalid/wildcard-sealed-product.sico) | `SEALED_MATCH_WILDCARD` | wildcard hides sealed state/event combinations |
| [`MATCH-104`](./invalid/unreachable-branch.sico) | `UNREACHABLE_MATCH_ARM` | match arm is already covered: On |

`MATCH-003` 当前显式列出全部四个组合。候选 B/C 可以提出更短的“拒绝当前剩余组合”写法，但该写法必须在 enum 新增变体后重新触发检查，不能等价于永久 `_`。
