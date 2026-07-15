# STEP-0018: 实现 parser recovery 与 E1xxx syntax diagnostics

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

让 12/12 B mutation 命中唯一主要根因与预注册 recovery anchor；建立 error/missing node，激活 E1001–E1012，并验证 RFC-0001 text/JSON byte/line/column/related span 与 construct 外最多 1 个级联。

## 2. Boundaries

- 不改变 STEP-0017 的 54 个成功 AST snapshot；
- recovery 只在 next top-level definition、next match arm、function body、function close、EOF 五类 anchor；
- error tree 不可获得可进入 M2 的 semantic AST；
- 不实现 formatter/CLI 或 M2 诊断。

## 3. Plan

1. 为 12 mutation 分配并登记 E1001–E1012；
2. 扩展 parser typed root cause/anchor/related open range；
3. 在 rowan tree 插入 ERROR/MISSING node 并保持 source lossless；
4. 实现 RFC-0001 text/JSON renderer；
5. 运行 12 mutation goldens、54 happy regression、schema/span/cascade 验证；
6. 审查、提交并推送。

## 4. Non-goals

不把 semantic reject 升级为 syntax；不自动修改源码；不猜测多个修复；不通过 catch-all 接受 A0/C。

## 5. Commit

`feat(diagnostics): [STEP-0018] recover B syntax mutations`

## 6. Changes and validation

- parser 新增 12 类 typed root cause、5 类 recovery anchor、related opener range 与零宽 `ERROR/MISSING` node；
- parse error 时 `ast()` 阻止 semantic AST 进入 M2，`recovered_ast()` 仅用于显式编辑器场景；
- diagnostic catalog 激活 E1001–E1012，并新增 12 条 mutation 映射；
- RFC-0001 text/JSON renderer 输出 byte range、1-based Unicode scalar line/column、related range 和 recovery anchor；
- 12 条 golden snapshot、parser/diagnostics tests、catalog/taxonomy、workspace Clippy/tests 与 M0 regression 全部通过。

```text
STEP_0018_OK mutations=12 root_causes=12 diagnostics=12 anchors=5 snapshots=12 cascade_outside_construct_max=0 text_json_spans=pass lossless_recovery=pass semantic_ast_on_error=blocked
```

## 7. Risks and next

恢复范围只覆盖已登记的 12 个单点 B mutation；复合错误、随机输入和深度/资源上限留给 STEP-0021。STEP-0019 只对无 syntax error 的 source 产生 canonical format，不得用格式化掩盖恢复节点。
