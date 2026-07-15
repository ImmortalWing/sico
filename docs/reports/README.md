# Reports

本目录保存可复现的实验、AI 评测、性能、安全、兼容性和 Runtime 对比报告。

现有静态语法指标位于 [`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)。后续新报告使用 [`report template`](../templates/REPORT.md)。

| Report | Status | Related step | 内容 |
|---|---|---|---|
| [`syntax-error-injection-v0`](./syntax-error-injection-v0.md) | complete | STEP-0003 | 18 个单点结构错误变体及可复现性验证 |
| [`ai-evaluation-protocol-v0`](./ai-evaluation-protocol-v0.md) | complete | STEP-0004 | 42 项 AI 任务、运行协议和离线评分器验证 |
| [`remaining-p0-semantic-cases`](./remaining-p0-semantic-cases.md) | complete | STEP-0005 | 10 组、54 个 P0 语义判定及 A0/B/C 镜像 |
| [`diagnostics-protocol-v0`](./diagnostics-protocol-v0.md) | complete | STEP-0006 | 24 个稳定诊断、29 个 case 映射和 JSON v0 fixtures |
| [`semantic-query-json-v0`](./semantic-query-json-v0.md) | complete | STEP-0007 | 10 模块索引、五类查询、可信度/完整性与预算 fixtures |
| [`numeric-representation-v0`](./numeric-representation-v0.md) | complete | STEP-0008 | `Int`/Decimal 运算、规范编码、限额和 WIT 边界原型 |
| [`resource-async-wit-v0`](./resource-async-wit-v0.md) | complete | STEP-0009 | affine resource、Task/Future/Stream 与 WASI 0.3 WIT 原型 |
| [`component-runtime-host-call-v0`](./component-runtime-host-call-v0.md) | complete | STEP-0010 | 真实 Component host call、resource、数值记录与原生 async 往返 |
| [`runtime-desktop-android-v0`](./runtime-desktop-android-v0.md) | complete | STEP-0011 | 桌面/Android Runtime、后端、发布政策与最小探针选择 |
| [`syntax-evidence-v1`](./syntax-evidence-v1.md) | complete | STEP-0012 | 54-case 静态快照、36 mutation 与 96-task AI 离线协议 |
| [`m0-exit-audit`](./m0-exit-audit.md) | complete | STEP-0014 | M0 全门槛、真实 Rust/Component 重跑、递延登记与 M1 GO 结论 |
