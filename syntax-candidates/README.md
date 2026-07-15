# Syntax candidates

本目录把同一批 P0 语义案例改写为不同表层语法，用于比较 AI 生成、理解、诊断和修复表现。

| 候选 | 风格 | M1 状态 | 案例目录 |
|---|---|---|---|
| A / A0 | 缩进 + 通用 `end` | not selected; retained control | [`semantic-cases/`](../semantic-cases/README.md) |
| B | Labeled Blocks | accepted baseline by [`RFC-0005`](../docs/rfc/RFC-0005-labeled-block-syntax-baseline.md) | [`b/`](./b/) |
| C | Compact Delimited | not selected; retained control | [`c/`](./c/) |

B、C 目录与 `semantic-cases/` 镜像：相同相对路径、相同 case ID、相同 accept/reject 元数据、相同语义引用。

当前每套候选包含 54 个设计样本，覆盖 10 组 P0 语义。没有编译器可以执行；结构检查只能证明案例一一对应，不能替代后续解析与类型检查。

“accepted baseline” 表示 M1 只实现 B，不表示真实 parser 恢复或模型正确率已经测量；复审条件见 RFC-0005。A0/C 不删除、不改写为 B，继续用于可重复对照。

详细规则与评测方法见 [`SYNTAX.md`](../SYNTAX.md)。

全部 54 个案例的可复现静态指标见 [`METRICS.md`](./METRICS.md) 与机器快照 [`metrics-v1.json`](./metrics-v1.json)。
