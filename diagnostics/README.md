# Sico diagnostics protocol v0

本目录是诊断编号、默认短消息和机器协议的唯一事实源。规范性行为见 [`RFC-0001`](../docs/rfc/RFC-0001-diagnostics-protocol-v0.md)。

## Files

- [`catalog.json`](./catalog.json)：编号分区、49 个正式 code/key、消息模板和必需参数；
- [`syntax-mutation-map.json`](./syntax-mutation-map.json)：12 个 B mutation 到 E1001–E1012、默认消息和 recovery anchor 的映射；
- [`top-level-syntax-map.json`](./top-level-syntax-map.json)：STEP-0092 两种 rejected top-level candidates 到 E1013 的映射；
- [`semantic-case-map.json`](./semantic-case-map.json)：41 个非法 case 到 code、arguments 和期望消息的映射（33 个设计态 P0 case + 8 个实现态 case）；
- [`implementation-cases/`](./implementation-cases/manifest.json)：实现态增量语料——用**当前可执行语法**写的最小拒绝源，由 `tools/validate-step-0489.ps1` 用真实编译器 `sico check --json` 实跑，证明目录模板、编译器 `arguments` 与默认短消息逐字节一致；
- [`schema/diagnostics-v0.schema.json`](./schema/diagnostics-v0.schema.json)：JSON envelope 的结构约束；
- [`fixtures/`](./fixtures/README.md)：通过和失败协议样本；
- [`tools/validate-diagnostics.ps1`](../tools/validate-diagnostics.ps1)：目录、案例、README 和 fixtures 的离线校验器。

## Consumer rule

自动化使用 `code`、`key`、`arguments` 和 `range`。`message` 用于显示，不用于分支或提取数据。

默认文本：

```text
E2001 user.sico:18:10 expected User, found Option<User>
hint: handle none before using the value
```

机器输出示例见 [`pass.json`](./fixtures/pass.json)。位置的权威坐标是 0-based UTF-8 半开 byte range；line/column 是 1-based Unicode scalar value 坐标。

## Change rule

- 已发布 code 不复用；
- 新诊断必须先登记 catalog 和最小 compile-fail case；
- 消息模板占位符必须与 `required_arguments` 完全一致；
- 默认 case 消息不超过 120 UTF-8 bytes；
- v0 不发布机器可应用 edits；不确定或改变业务语义的修复不进入 hint；
- 结构或坐标含义的破坏性变更需要新协议主版本和 RFC；
- **编译器实现与目录必须同形**：编译器发出的 `arguments` 集合等于目录的 `required_arguments`，且默认短消息等于目录模板按同一组参数渲染的结果。只在人读文本里出现的变化值（比较算符、非交换形改写式）必须同时进入 `arguments`，否则机器消费者无法复原该诊断。

## Scope of the catalog

- 目录覆盖**编译器前端**诊断：语法 E1xxx + 语义 E2xxx–E7xxx。
- **E8xxx 不在目录内**：按 RFC-0001 的分区表，`E8000–E8999` 是 reserved 分区；RFC-0039 的模块/包解析码（E8001–E8021）属于 CLI 模块解析通道，不是源码级编译器诊断，登记进目录需要变更分区状态（新协议主版本 + RFC），因此保持在外。
- **E1014–E1016（RFC-0039 语法码）尚未登记**：登记需要扩展 `syntax-mutation-map.json` / `top-level-syntax-map.json`，而这两张表绑定冻结的 mutation 与 candidate 语料；作为独立欠账登记，不在本步静默扩表。

## Validate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-diagnostics.ps1
```

该校验同时验证设计资产、12 个 parser mutation 映射、两种 top-level candidate 映射和实现态增量语料。编译器前端当前产生 E1001–E1013；E2xxx 及以后由语义前端按各步骤证据实现，目录中的设计项不能单独解释为实现声明。

实现态对拍（目录 ↔ 真实编译器逐字节）另由本步的记录校验器承担：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0489.ps1
```
