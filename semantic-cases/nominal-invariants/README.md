# Nominal types and invariants

本组验证 SEM-030、SEM-032、SEM-042 和 SEM-110：

- 名义类型不因底层表示或字段结构相同而兼容；
- record 构造必须字段完整且不能包含未知字段；
- 可静态证明违反的不变量必须在编译期拒绝；
- 合法不变量进入类型语义，而不只存在于注释。

A0 使用 record 内的 `invariant` 行表示机器可检查的不变量。该写法是语义占位，不代表契约表层语法已经确定。

## 必须接受

| Case | 规则 |
|---|---|
| [`NOM-001`](./valid/nominal-id.sico) | AccountId 只能传给 AccountId 参数 |
| [`NOM-002`](./valid/complete-record.sico) | 完整构造具名记录 |
| [`NOM-003`](./valid/satisfied-invariant.sico) | 常量构造满足 Range 不变量 |

## 必须拒绝

| Case | 诊断键 | 默认短消息 |
|---|---|---|
| [`NOM-101`](./invalid/nominal-confusion.sico) | `TYPE_MISMATCH` | expected AccountId, found UserId |
| [`NOM-102`](./invalid/structural-record-confusion.sico) | `TYPE_MISMATCH` | expected Customer, found User |
| [`NOM-103`](./invalid/missing-field.sico) | `MISSING_FIELD` | missing field: enabled |
| [`NOM-104`](./invalid/unknown-field.sico) | `UNKNOWN_FIELD` | unknown field: admin |
| [`NOM-105`](./invalid/invariant-violation.sico) | `INVARIANT_VIOLATION` | Range invariant is false: min <= max |

`NOM-105` 使用常量，因此编译器可以直接证明不变量为假。运行时输入无法静态证明时如何检查，由后续契约案例单独覆盖。
