# Sico diagnostics protocol v0

本目录是诊断编号、默认短消息和机器协议的唯一事实源。规范性行为见 [`RFC-0001`](../docs/rfc/RFC-0001-diagnostics-protocol-v0.md)。

## Files

- [`catalog.json`](./catalog.json)：编号分区、24 个正式 code/key、消息模板和必需参数；
- [`semantic-case-map.json`](./semantic-case-map.json)：29 个非法 P0 case 到 code、arguments 和期望消息的映射；
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
- 结构或坐标含义的破坏性变更需要新协议主版本和 RFC。

## Validate

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-diagnostics.ps1
```

这只验证设计资产的一致性。仓库尚无 compiler，不能把输出解释为 `.sico` 已经实际产生这些诊断。
