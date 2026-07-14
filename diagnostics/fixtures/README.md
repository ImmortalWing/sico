# Diagnostic protocol fixtures

这些文件验证 envelope 的核心结构和 RFC 中无法只靠 JSON Schema 表达的跨字段规则。

| Fixture | 期望 | 单一目标 |
|---|---|---|
| [`pass.json`](./pass.json) | accept | 两条已排序 root diagnostics、hint、related 与 summary |
| [`invalid-message-only.json`](./invalid-message-only.json) | reject | 禁止只有 message、没有 code/key |
| [`invalid-range.json`](./invalid-range.json) | reject | byte range 不能逆序 |
| [`invalid-cascade.json`](./invalid-cascade.json) | reject | cascade 必须引用 earlier root/cascade |

fixture 只是协议样本，不是编译器输出。期望结果登记在 [`manifest.json`](./manifest.json)，由 [`validate-diagnostics.ps1`](../../tools/validate-diagnostics.ps1) 离线执行。
