# Report: 诊断协议 v0 目录与离线验证

> - status: complete
> - date: 2026-07-14
> - related-step: STEP-0006
> - environment: Windows NT 10.0.26100.0, Windows PowerShell 5.1.26100.8655, Python 3.14.6 (no jsonschema package)

## 1. Question

能否把 29 个 P0 非法语义案例的 24 个 provisional key 收敛为稳定、短、机器可读且可离线校验的诊断协议，同时保持现有语义、mutation 和 AI 评测资产不回归？

## 2. Method

验证分为五层：

1. **目录身份**：检查 9 个编号分区连续且不重叠，24 个 code/key 唯一、升序并位于 active 分区；
2. **模板参数**：检查 `{placeholder}` 与 `required_arguments` 完全一致，消息单行且不以句号结尾；
3. **case 覆盖**：逐一读取 29 个非法 `.sico` 的 `case` 与 `expect: reject(KEY)`，与 code、arguments、渲染消息和组 README 对照；
4. **envelope 行为**：对合法 fixture 和三个单根因非法 fixture 执行结构、range、cascade、排序、去重、summary 与消息渲染检查；
5. **仓库回归**：重新运行语义 case、syntax mutation、AI dataset 和 AI harness 校验。

[`diagnostics-v0.schema.json`](../../diagnostics/schema/diagnostics-v0.schema.json) 是发布的 Draft 2020-12 结构契约。当前环境没有 `jsonschema` 包，因此本报告没有声称使用第三方 JSON Schema validator；PowerShell 校验器直接验证 RFC 的核心字段及 Schema 无法表达的跨字段规则，并静态检查 Schema 身份与版本。

## 3. Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-diagnostics.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-cases.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
```

## 4. Raw evidence

- 规范：[`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md)；
- 目录：[`catalog.json`](../../diagnostics/catalog.json)；
- case oracle：[`semantic-case-map.json`](../../diagnostics/semantic-case-map.json)；
- schema 与 fixtures：[`diagnostics/schema/`](../../diagnostics/schema/diagnostics-v0.schema.json)、[`diagnostics/fixtures/`](../../diagnostics/fixtures/README.md)；
- 校验器：[`validate-diagnostics.ps1`](../../tools/validate-diagnostics.ps1)；
- 执行记录：[`STEP-0006`](../steps/STEP-0006-diagnostics-protocol-v0.md)。

## 5. Results

| 检查项 | 结果 | 证据等级 |
|---|---:|---|
| 编号分区 | 9 | verified |
| active / reserved 分区 | 6 / 3 | verified |
| 正式诊断 code/key | 24 / 24 unique | verified |
| 非法语义 case 映射 | 29 / 29 | verified |
| 未使用 catalog 项 | 0 | verified |
| 无 catalog 的 reject key | 0 | verified |
| 最长 case 默认消息 | 58 UTF-8 bytes | measured |
| 消息上限 | 120 UTF-8 bytes | accepted contract |
| envelope fixtures | 4 | measured |
| accept / expected reject | 1 / 3 | verified |
| 语义 case 回归 | 54 cases / 162 programs pass | verified |
| syntax mutation 回归 | 18 pass | verified |
| AI dataset / harness 回归 | 42 tasks / harness pass | verified |
| 真实 compiler diagnostics | 无 | not measured |
| 真实 AI 修复率/token | 无 | not measured |

核心输出：

```text
DIAGNOSTICS_OK catalog=24 cases=29 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=58
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
MUTATION_CORPUS_OK entries=18 A0=6 B=6 C=6
AI_EVAL_DATASET_OK tasks=42 generation=12 understanding=12 repair=18 A0=14 B=14 C=14
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=42 deterministic=true
```

## 6. Interpretation

现有非法设计判定不再依赖散落的自然语言：每个 key 能唯一解析为稳定 code，每个变化值有结构化 argument，每条 case 消息都能从 catalog 确定渲染。AI 可以按 code/key 分支并直接读取 missing、expected、found 等参数。

本结果只证明协议资产内部一致、正反 fixtures 有效、旧数据未回归。它不证明任何 `.sico` 程序已被 parser/type checker 检查，也不证明未来实现会自动选择正确跨度或抑制全部级联。

## 7. Limitations

- 目前只有语义诊断 E2xxx–E7xxx；E1xxx 等待真实 parser 和 mutation golden tests；
- Schema 已发布但未在此环境用独立 Draft 2020-12 引擎交叉验证；
- fixture 的 line/column 与 byte 只验证顺序和边界，不读取对应真实源码重算 Unicode 坐标；
- v0 不稳定机器可应用 edits，只有可选单行 hint；
- 没有多文件真实编译、LSP 转码、本地化或恶意输出压力测试；
- 没有真实模型调用，不能推导修复成功率或 token 节省比例。

## 8. Decision impact

- M0 诊断协议 v0 从 planned 变为 complete for design contract；
- M1/M2 必须让编译器输出与 catalog、schema 和 case oracle 形成 golden tests；
- AI 工具不得解析终端 message；
- 下一依赖项可以进入语义索引与 `outline/describe/slice/impact/flow` JSON v0；
- 自动 edit、parser E1xxx 和 LSP adapter 仍需后续 RFC/实现证据。

## 9. Links

- 方向：[`DIRECTION.md`](../../DIRECTION.md)；
- 工程架构：[`DEVELOPMENT.md`](../../DEVELOPMENT.md)；
- P0 语义案例：[`semantic-cases/`](../../semantic-cases/README.md)；
- AI 评测：[`ai-eval/`](../../ai-eval/README.md)。
