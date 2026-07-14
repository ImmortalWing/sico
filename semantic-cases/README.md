# Sico P0 semantic cases

> - 状态：第一批草案
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

诊断键目前是语义名称，不是最终稳定错误编号。后续诊断协议 v0 会为这些键分配编号、JSON 字段和默认短消息。

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

## 3. 第一批案例组

| 顺序 | 案例组 | 合法 | 非法 | 入口 |
|---:|---|---:|---:|---|
| 1 | 数字与单位 | 4 | 4 | [`numbers-units/`](./numbers-units/README.md) |
| 2 | 名义类型与不变量 | 3 | 5 | [`nominal-invariants/`](./nominal-invariants/README.md) |
| 3 | 完整模式匹配 | 3 | 4 | [`exhaustive-match/`](./exhaustive-match/README.md) |
| 4 | Result 错误映射 | 3 | 4 | [`result-mapping/`](./result-mapping/README.md) |

合计 30 个最小程序：13 个必须接受，17 个必须拒绝。

## 4. 案例质量要求

每个案例必须满足：

1. 不依赖网络、文件、时钟或 UI；
2. 除目标规则外，其余结构均合法；
3. 非法案例只有一个主要诊断；
4. 诊断不得建议会改变业务含义的修复；
5. 同一案例在不同后端得到相同判定；
6. 将来能够直接进入 parser、type checker、IR 和 Component 一致性测试。

## 5. 当前边界

本批案例不决定 Decimal 精度、资源借用、效果系统、异步 ABI 或 WIT 版本。这些属于后续 P0 案例组。

案例中的 `Int` 遵循 SEM-021 的任意精度草案；`Float64.from_int` 只用于展示显式转换，最终标准库名称仍可改变。
