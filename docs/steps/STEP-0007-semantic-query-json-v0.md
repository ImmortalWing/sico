# STEP-0007: 建立语义索引与查询 JSON v0

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

为 Sico Semantic Index 和 `outline/describe/slice/impact/flow` 建立统一、版本化、低 token、可预算且可离线校验的 JSON v0，使 AI 能按稳定语义 ID 获取最小充分上下文，并明确区分作者声明、编译器证明和工具推断。

## 2. Context and evidence

- SEM-130 要求索引包含符号、类型、调用、数据流、错误、效果、能力、状态、契约、Component、测试与源码范围；
- SEM-131 要求 `declared`、`verified`、`inferred` 不得混淆；
- SEM-132 要求五类查询共享 ID/JSON 类型并返回最小充分上下文；
- SEM-133 允许不完整程序产生部分索引，但事实必须标记完整性和阻塞诊断；
- 10 个代表性程序覆盖状态、契约、资源、异步、Component、revision 和 UI；
- STEP-0006 已固定诊断 code/key 与 JSON 协议，可供部分索引引用阻塞诊断；
- 当前没有 parser/type checker，因此本步骤验证协议和 fixtures，不声称已从 `.sico` 自动生成索引。

## 3. Scope

包含：

- semantic ID、snapshot、source range、事实依据和完整性模型；
- 索引 JSON v0 的模块、符号、关系、契约、测试和诊断引用结构；
- 五类查询的统一 request/response envelope；
- 深度、items、bytes、source 和 transitive 预算；
- 10 个代表性程序的覆盖 manifest；
- 五类合法 response fixture 与预算/可信度/快照非法 fixture；
- JSON Schema、离线校验器、RFC、报告与状态更新。

不包含：

- 实现 parser、type checker、索引构建器或查询服务；
- 把样本注释或 SPEC 内容标记为 `verified`；
- 固定 CLI 文本渲染、缓存数据库、LSP 扩展或网络服务；
- 证明程序切片算法最小、影响分析完整或语义哈希真实稳定；
- 枚举全部跨 Component、跨包和动态加载影响。

## 4. Options and decision

### 每种查询独立设计 JSON

局部简单，但 ID、范围、可信度、预算和分页会分裂，AI 需要五套解析规则。

### 返回通用图查询结果

结构统一，但 `describe`、状态流和影响路径都退化为低层节点/边，token 更多且不易直接使用。

### 决定：统一 envelope 与核心类型，操作专用 result

五类查询共享 request、snapshot、budget、reference、fact、semantic ID 和 completeness；每个 operation 保留紧凑的专用 result。自动化按结构化字段读取，不解析摘要文本。

`verified` 必须有 compiler analysis/semantic rule 证据；当前手工 fixtures 只能使用 `declared` 或 `inferred`。任何截断、阻塞诊断或未知边都必须把相应 result 标记为 `partial`，禁止“截断但 complete”。

## 5. Plan

1. 审计语义规则、工程文档和 10 个代表性样本；
2. 编写 RFC，固定 ID、snapshot、provenance、completeness 和预算；
3. 创建索引/请求/响应 Schema 与协议 manifest；
4. 编写五类合法 fixtures、代表性样本覆盖表和非法 fixtures；
5. 实现离线校验器并运行正反验证；
6. 更新根文档、状态、路线图和审计报告；
7. 运行全量回归，提交并推送。

## 6. Changes

- 接受 [`RFC-0002`](../rfc/RFC-0002-semantic-index-query-v0.md)，固定 semantic ID、snapshot、fact provenance、completeness、预算与五类 operation；
- 新增 [`semantic-index/protocol.json`](../../semantic-index/protocol.json) 和 index/query/response 三个 JSON Schema；
- 新增 10 个代表性程序的 [`sample-matrix.json`](../../semantic-index/sample-matrix.json)，覆盖五类 operation 和 AI01–AI07；
- 新增包含 10 个模块、24 个 symbols、13 条 relations 的 index fixture，详细展开 order-state 与 account-transfer；
- 新增五个合法 request/response pair 与 stale snapshot、伪 verified、虚假 complete 三个非法 response；
- 新增 [`validate-semantic-query.ps1`](../../tools/validate-semantic-query.ps1)，验证真实 UTF-8 source range、证据、完整性、预算和 operation 专用约束；
- 更新方向、工程、语义、README、RFC/report 索引、路线图和项目状态；
- 新增 [`semantic-query-json-v0`](../reports/semantic-query-json-v0.md) 可复现报告。

## 7. Validation

已运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-query.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-diagnostics.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-cases.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
```

核心结果：

```text
SEMANTIC_QUERY_OK modules=10 symbols=24 relations=13 samples=10 operations=5 fixtures=8 accepted=5 rejected=3 max_result_bytes=3630
DIAGNOSTICS_OK catalog=24 cases=29 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=58
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
MUTATION_CORPUS_OK entries=18 A0=6 B=6 C=6
AI_EVAL_DATASET_OK tasks=42 generation=12 understanding=12 repair=18 A0=14 B=14 C=14
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=42 deterministic=true
```

JSON、PowerShell 语法、Markdown 链接、Git whitespace 和工作区状态在提交前复核。

## 8. Metrics

实际登记 3 个 Schema、10 个模块、24 个 symbols、13 条 relations、10 个样本覆盖项和 8 个查询 fixtures。五类合法 canonical result 最大为 3630 UTF-8 bytes；24 个 symbol range 均与真实样本源码 byte/line/column 一致。

当前没有索引器或真实 AI 调用，因此查询延迟、真实 token、slice 最小性、impact 召回率和 flow 准确率均为 not measured。

## 9. Risks and follow-ups

- semantic ID 的 package/version 身份需与未来包格式对齐；
- 语义哈希算法必须通过实现与等价变换测试后才能稳定；
- 部分索引和跨 Component 影响分析需要真实 compiler/runtime 元数据；
- STEP-0008 应进入 `Int`/Decimal 等独立技术原型，不把协议 fixture 当作实现。

## 10. Audit links

- RFC: [`RFC-0002`](../rfc/RFC-0002-semantic-index-query-v0.md)
- report: [`semantic-query-json-v0`](../reports/semantic-query-json-v0.md)
- commit subject: `docs(semantic-query): [STEP-0007] establish semantic query JSON v0`
- next: STEP-0008 numeric representation prototypes
