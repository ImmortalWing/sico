# STEP-0006: 建立诊断协议 v0

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

为 29 个 P0 非法语义案例的 24 个 provisional reject key 建立可审计的诊断协议 v0：稳定编号、唯一默认短消息、精确源码跨度、相关信息、确定性排序和机器可读 JSON 结构。

## 2. Context and evidence

- [`DIRECTION.md`](../../DIRECTION.md) 要求诊断短、准、稳定、低 token，并禁止工具解析终端文本；
- [`DEVELOPMENT.md`](../../DEVELOPMENT.md) 已提出文本/JSON 双输出、级联抑制和按编号展开；
- [`semantic-cases/`](../../semantic-cases/README.md) 当前包含 29 个非法 case、24 个 provisional key，但编号和消息尚未成为单一事实源；
- STEP-0003 的 parser mutation 仍是设计输入，不冒充真实 parser diagnostic；
- 当前没有编译器，因此本步骤验证协议、目录和 fixtures，不声称验证了实际源码跨度或编译输出。

## 3. Scope

包含：

- 诊断编号分区与 24 个稳定 v0 编号；
- 人类默认文本和 AI/工具 JSON envelope；
- UTF-8 源码跨度、相关跨度、参数、提示、排序、去重和级联规则；
- 29 个非法 case 到正式编号及模板参数的映射；
- JSON Schema、协议示例和离线一致性校验器；
- RFC、报告、索引与项目状态更新。

不包含：

- lexer、parser、type checker 或诊断渲染器实现；
- 为 STEP-0003 mutation 虚构正式 parser 编号或实测跨度；
- LSP 协议适配、诊断本地化和 `sico explain` 内容库；
- 自动修复会改变业务含义的程序。

## 4. Options and decision

### 只稳定自然语言消息

实现最少，但 AI 必须解析英文文本，消息改写也会破坏工具兼容。

### 只输出结构化 JSON

适合自动化，但终端使用和人工审查不便，也不满足默认短诊断目标。

### 决定：稳定 code/key/arguments，文本与 JSON 双视图

`code` 是跨版本身份，`key` 是可读的符号名称，`arguments` 承载类型、字段或变量等变化值；`message` 是由目录模板渲染的短文本，不作为自动化分支条件。默认文本保持一条主行，可选一条 hint；JSON 是 AI、编辑器和测试的正式接口。

编号按诊断阶段/领域分区，编号一经发布不复用。v0 先登记已有语义根因，语法分区只预留，不把尚未实现的 parser 行为写成事实。

## 5. Plan

1. 审计 29 个非法案例、24 个 key 和现有默认短消息；
2. 编写诊断 RFC，确定编号、兼容性和输出规则；
3. 创建机器可读目录、case 映射、JSON Schema 和 fixtures；
4. 实现并运行离线一致性校验器；
5. 更新语义案例、文档索引、路线图和项目状态；
6. 记录可复现报告，提交并推送。

## 6. Changes

- 接受 [`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md)，固定诊断身份、E1xxx–E9xxx 分区、文本/JSON 双视图、UTF-8 坐标、级联与兼容规则；
- 新增 [`diagnostics/catalog.json`](../../diagnostics/catalog.json)，为 24 个 key 分配 E2xxx–E7xxx code 和消息模板；
- 新增 [`semantic-case-map.json`](../../diagnostics/semantic-case-map.json)，记录 29 个 case 的 code、arguments 和 golden message；
- 新增 Draft 2020-12 JSON Schema、1 个合法 fixture 和 3 个单根因非法 fixture；
- 新增 [`validate-diagnostics.ps1`](../../tools/validate-diagnostics.ps1)，校验目录、模板、case/README、envelope、排序、去重、summary 和 fixture 期望；
- 更新方向、工程、语义、案例入口、RFC/report 索引和项目状态；
- 新增 [`diagnostics-protocol-v0`](../reports/diagnostics-protocol-v0.md) 可复现报告。

## 7. Validation

已运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-diagnostics.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-cases.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
```

核心结果：

```text
DIAGNOSTICS_OK catalog=24 cases=29 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=58
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
MUTATION_CORPUS_OK entries=18 A0=6 B=6 C=6
AI_EVAL_DATASET_OK tasks=42 generation=12 understanding=12 repair=18 A0=14 B=14 C=14
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=42 deterministic=true
```

JSON 均可解析；校验器证明目录与 case/README 一致，并对协议正反 fixtures 执行跨字段检查。Markdown 链接、PowerShell 语法、Git whitespace 和工作区状态在提交前复核。

## 8. Metrics

实际登记 9 个分区、24 个稳定 code/key、29 个 case 映射和 4 个 envelope fixture。最长 case 默认消息为 58 UTF-8 bytes，低于 120-byte 上限。

没有编译器，因此诊断 root/跨度准确率、真实级联数量、AI 修复成功率和 token 成本均为 not measured。

## 9. Risks and follow-ups

- v0 的编号身份可以稳定，但部分消息参数需要未来类型检查器提供；
- 源码跨度规则可先稳定，真实生产位置仍需 M1/M2 golden tests 验证；
- parser 编号、warning、note 和本地化目录需在实际前端实现时扩展；
- 下一步应建立语义查询 JSON v0。

## 10. Audit links

- RFC: [`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md)
- report: [`diagnostics-protocol-v0`](../reports/diagnostics-protocol-v0.md)
- commit subject: `docs(diagnostics): [STEP-0006] establish diagnostics protocol v0`
- next: STEP-0007 semantic query JSON v0
