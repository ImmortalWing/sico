# Report: parser recovery and syntax diagnostics v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0018
> - environment: Windows, Rust 1.97.0

## 1. Question

RFC-0005 B 的 12 个单点结构 mutation 能否各自得到一个稳定主要根因、预注册 recovery anchor、lossless error tree 和符合 RFC-0001 的 E1xxx 文本/JSON 诊断，同时不改变 54 个成功 AST shape？

## 2. Method

parser 按 mutation manifest 中已存在的结构意图识别缺失关闭符或分隔符，在错误位置插入零宽 `ERROR/MISSING` node，并恢复到 next definition、next match arm、function body、function close 或 EOF。测试逐例比较诊断登记、byte/line/column、related opener range、anchor、文本输出和 JSON envelope；错误树不得暴露可进入 M2 的 semantic AST。

## 3. Results

```text
STEP_0018_OK mutations=12 root_causes=12 diagnostics=12 anchors=5 snapshots=12 cascade_outside_construct_max=0 text_json_spans=pass lossless_recovery=pass semantic_ast_on_error=blocked
```

- B mutation 主要根因：12/12，每例恰好一个 parser error；
- 稳定语法诊断：E1001–E1012，与 mutation key/message 一一对应；
- recovery anchor：5 类，均与预注册意图一致；
- construct 外级联：实测最大 0，优于门槛 1；
- error tree：原 source text byte-identical，且含 `ERROR/MISSING` node；
- semantic AST：有 error 时 `ast()` 返回 `None`，仅显式 `recovered_ast()` 可供编辑器结构展示；
- 诊断快照：12/12，覆盖 text、JSON、byte range、Unicode scalar line/column 和 related range；
- happy path：54/54 AST shape snapshot 未改变。

## 4. Interpretation and limits

结果只证明登记的 12 类 B 单点错误。parser 没有猜测复合错误修复，也没有把 29 个语义负例升级为 syntax error。E1xxx 已从保留分区升级为真实 parser 输出；E2xxx 及以后仍只是 M2 设计资产。

## 5. Links

- [`STEP-0018`](../steps/STEP-0018-parser-recovery-syntax-diagnostics.md)
- [`syntax mutation map`](../../diagnostics/syntax-mutation-map.json)
- [`diagnostic snapshots`](../../tests/diagnostics/b-mutations.snap)
- [`parser`](../../crates/sico-parser/src/lib.rs)
- [`diagnostic renderer`](../../crates/sico-diagnostics/src/lib.rs)
