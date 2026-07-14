# Numbers and units

本组验证 SEM-003、SEM-021、SEM-024、SEM-032 和 SEM-042：

- `Int` 普通算术不发生固定宽度溢出；
- Int、Float64、Text 之间不隐式转换；
- 显式转换在源码中可见；
- 相同底层表示的单位类型仍不能混用。

## 必须接受

| Case | 规则 |
|---|---|
| [`NUM-001`](./valid/int-arbitrary-precision.sico) | Int 保持任意精度整数结果 |
| [`NUM-002`](./valid/explicit-int-to-float.sico) | 显式 Int → Float64 转换 |
| [`NUM-003`](./valid/typed-bytes.sico) | 显式构造并传递 Bytes |
| [`NUM-004`](./valid/typed-currency.sico) | 同一货币领域类型可以传递 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`NUM-101`](./invalid/implicit-int-to-float.sico) | `IMPLICIT_NUMERIC_CONVERSION` | expected Float64, found Int; convert explicitly |
| [`NUM-102`](./invalid/text-as-int.sico) | `TYPE_MISMATCH` | expected Int, found Text |
| [`NUM-103`](./invalid/mixed-unit.sico) | `TYPE_MISMATCH` | expected Bytes, found Seconds |
| [`NUM-104`](./invalid/cross-currency.sico) | `TYPE_MISMATCH` | expected Cny, found Usd |

`Bytes`、`Seconds`、`Cny`、`Usd` 在这些最小程序中使用 `newtype` 表示。是否由标准库提供更完整的 `Size`、`Duration`、`Money<Currency>` 不影响“不允许隐式混用”的判定。
