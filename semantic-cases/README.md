# Sico P0 semantic cases

> - 状态：P0 设计判定集 v0
> - 语义基线：[`SEMANTICS.md` 0.1-draft](../SEMANTICS.md)
> - 用途：语言判定标准、诊断设计输入和未来编译器一致性测试

## 1. 这些文件是什么

每个 `.sico` 文件只验证一个核心语义结论：

- `valid/` 中的程序必须被接受；
- `invalid/` 中的程序必须被拒绝；
- 非法程序文件头给出预期诊断键；
- 一个非法文件只保留一个主要根因，避免级联错误干扰结果。

文件头格式：

```sico
// case: MATCH-003
// expect: reject(NON_EXHAUSTIVE_MATCH)
```

非法源码头保留可读的语义诊断键；正式稳定编号、参数和默认短消息由 [`diagnostics/catalog.json`](../diagnostics/catalog.json) 与 [`semantic-case-map.json`](../diagnostics/semantic-case-map.json) 统一管理。机器不得解析 README 或自然语言消息来判断错误类型。

## 2. A0 语义记法

案例源码暂时使用接近候选 A 的 **A0 语义记法**。它只要求读者能识别定义、类型和控制流，不代表以下表层形式已经定稿：

- `fn`、`return` 和 `end`；
- `newtype`、`record`、`enum`；
- `ok`、`error`、`try`、`map_error`；
- `match`、`=>` 和模式；
- 泛型的 `<...>`；
- 契约和 invariant 的写法。

候选 B、C 必须重新表达同一案例，并保持 accept/reject 结果、诊断根因和语义摘要一致。

第一轮 B、C 源码位于 [`syntax-candidates/`](../syntax-candidates/README.md)。

## 3. P0 案例组

| 顺序 | 案例组 | 合法 | 非法 | 入口 |
|---:|---|---:|---:|---|
| 1 | 数字与单位 | 4 | 4 | [`numbers-units/`](./numbers-units/README.md) |
| 2 | 名义类型与不变量 | 3 | 5 | [`nominal-invariants/`](./nominal-invariants/README.md) |
| 3 | 完整模式匹配 | 3 | 4 | [`exhaustive-match/`](./exhaustive-match/README.md) |
| 4 | Result 错误映射 | 3 | 4 | [`result-mapping/`](./result-mapping/README.md) |
| 5 | 效果与能力 | 2 | 2 | [`effects-capabilities/`](./effects-capabilities/README.md) |
| 6 | affine 资源 | 2 | 2 | [`affine-resources/`](./affine-resources/README.md) |
| 7 | Future/Task | 2 | 2 | [`future-task/`](./future-task/README.md) |
| 8 | Stream | 2 | 2 | [`stream/`](./stream/README.md) |
| 9 | Component 调用 | 2 | 2 | [`component-call/`](./component-call/README.md) |
| 10 | revision | 2 | 2 | [`revision/`](./revision/README.md) |

合计 54 个最小程序：25 个必须接受，29 个必须拒绝。机器可读组计数位于 [`manifest.json`](./manifest.json)。

## 4. 案例质量要求

每个案例必须满足：

1. 不依赖网络、文件、时钟或 UI；
2. 除目标规则外，其余结构均合法；
3. 非法案例只有一个主要诊断；
4. 诊断不得建议会改变业务含义的修复；
5. 同一案例在不同后端得到相同判定；
6. 将来能够直接进入 parser、type checker、IR 和 Component 一致性测试。

## 5. 当前边界

这些案例是语义设计判定，不是编译器实测。它们不决定 Decimal 精度、正式效果分类、完整资源借用/生命周期推导、Component async ABI 或 WIT/WASI 版本。

案例中的 `Int` 遵循 SEM-021 的任意精度草案；`Float64.from_int`、`using`、`task group`、`ComponentCall` 等名称只用于展示语义，最终表层或标准库名称仍可改变。
