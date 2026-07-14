# Syntax candidates

本目录把同一批 P0 语义案例改写为不同表层语法，用于比较 AI 生成、理解、诊断和修复表现。

| 候选 | 风格 | 案例目录 |
|---|---|---|
| A / A0 | 缩进 + 通用 `end` | [`semantic-cases/`](../semantic-cases/README.md) |
| B | Labeled Blocks | [`b/`](./b/) |
| C | Compact Delimited | [`c/`](./c/) |

B、C 目录与 `semantic-cases/` 镜像：相同相对路径、相同 case ID、相同 accept/reject 元数据、相同语义引用。

当前源码是设计样本，没有编译器可以执行。结构检查只能证明案例一一对应，不能替代后续解析与类型检查。

详细规则与评测方法见 [`SYNTAX.md`](../SYNTAX.md)。

第一轮源码体积与结构统计见 [`METRICS.md`](./METRICS.md)。
