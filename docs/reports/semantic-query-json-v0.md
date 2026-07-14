# Report: 语义索引与查询 JSON v0 离线验证

> - status: complete
> - date: 2026-07-14
> - related-step: STEP-0007
> - environment: Windows NT 10.0.26100.0, Windows PowerShell 5.1.26100.8655

## 1. Question

能否为 10 个代表性程序建立统一、低 token、可预算、可区分事实可信度的 Semantic Index 与 `outline/describe/slice/impact/flow` JSON v0，并用离线正反 fixtures 阻止 stale snapshot、伪 verified 和虚假完整性？

## 2. Method

验证分为六层：

1. **协议与 Schema**：解析 protocol manifest 和 index/query/response 三个 Draft 2020-12 Schema；
2. **索引身份**：检查 semantic ID 唯一、不含 source offset，module/source 存在，relation 两端可解析；
3. **源码坐标**：按真实 UTF-8 文件复算 symbol byte range 的 line/Unicode-scalar column，并确认 range 选中声明名称；
4. **可信度与完整性**：declared 必须有 source evidence，verified 必须有 compiler/SEM evidence，partial/unknown 必须说明原因或 blocking diagnostic；
5. **查询 pair**：检查 request/response/snapshot/operation/target 关联、operation 专用角色、路径、slice inclusion reason、完整 state rejection 和 canonical byte 预算；
6. **仓库回归**：重跑诊断、语义案例、mutation 和 AI 离线评测工具。

fixtures 的 index 覆盖全部 10 个模块，详细展开 order-state 和 account-transfer。其余 8 个模块只用于 outline 与协议覆盖，不冒充完整索引。

## 3. Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-query.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-diagnostics.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-semantic-cases.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
```

## 4. Raw evidence

- 规范：[`RFC-0002`](../rfc/RFC-0002-semantic-index-query-v0.md)；
- 协议入口：[`semantic-index/`](../../semantic-index/README.md)；
- 样本矩阵：[`sample-matrix.json`](../../semantic-index/sample-matrix.json)；
- index fixture：[`index.json`](../../semantic-index/fixtures/index.json)；
- 查询 fixtures：[`fixtures/`](../../semantic-index/fixtures/README.md)；
- 校验器：[`validate-semantic-query.ps1`](../../tools/validate-semantic-query.ps1)；
- 执行记录：[`STEP-0007`](../steps/STEP-0007-semantic-query-json-v0.md)。

## 5. Results

| 检查项 | 结果 | 证据等级 |
|---|---:|---|
| JSON Schema | 3 | verified metadata/parse |
| 代表性模块 | 10 | verified |
| 详细 semantic symbols | 24 | verified |
| 索引 relations | 13 | verified |
| 样本覆盖 | 10 / 10 | verified |
| operation 覆盖 | 5 / 5 | verified |
| AI01–AI07 覆盖 | 7 / 7 | verified |
| request/response fixtures | 8 | measured |
| accept / expected reject | 5 / 3 | verified |
| 最大合法 canonical result | 3630 UTF-8 bytes | measured |
| symbol source ranges | 24 / 24 match actual UTF-8 source | verified |
| 诊断协议回归 | pass | verified |
| 语义 case 回归 | 54 cases / 162 programs pass | verified |
| syntax mutation 回归 | 18 pass | verified |
| AI dataset / harness 回归 | 42 tasks / harness pass | verified |
| 真实 index/query latency | 无 | not measured |
| slice/impact/flow 准确率 | 无 | not measured |
| 真实 AI token/理解率 | 无 | not measured |

核心输出：

```text
SEMANTIC_QUERY_OK modules=10 symbols=24 relations=13 samples=10 operations=5 fixtures=8 accepted=5 rejected=3 max_result_bytes=3630
DIAGNOSTICS_OK catalog=24 cases=29 partitions=9 fixtures=4 accepted=1 rejected=3 max_message_bytes=58
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
MUTATION_CORPUS_OK entries=18 A0=6 B=6 C=6
AI_EVAL_DATASET_OK tasks=42 generation=12 understanding=12 repair=18 A0=14 B=14 C=14
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=42 deterministic=true
```

## 6. Interpretation

五类查询现在共享同一组稳定概念：semantic ID、snapshot、facets/evidence、source range、completeness、budget、items/edges 和 next query。AI 不必解析 purpose/summary 才能找到签名、错误、契约、影响路径或状态边。

三个非法 fixture 证明离线执行器会拒绝：来自另一 snapshot 的响应、只有 source evidence 的 `verified` 声明，以及被截断却标 complete 的结果。flow fixture 还要求 complete 状态图显式覆盖剩余拒绝组合。

这些结果证明协议和 oracle 自洽，不证明真实 compiler 能生成相同索引，也不证明 slice 最小、impact 无漏项或 flow 来自静态证明。

## 7. Limitations

- 只有 order-state/account-transfer 详细展开；其他样本只有 module outline 和覆盖计划；
- accepted fixtures 不包含 `verified` 事实，因为当前没有 compiler analysis；
- Schema 元数据和核心字段由自有 PowerShell validator 验证，未使用独立 Draft 2020-12 引擎交叉验证；
- snapshot 的生产 hash descriptor 和定义级 semantic hash 尚未通过实现原型固定；
- 没有跨 package、跨 Component、动态加载或 Runtime capability 的真实图；
- byte 预算已确定验证，token budget 尚无 tokenizer 适配实测；
- 没有真实模型调用，不能得出理解率、影响分析召回率或 token 节省比例。

## 8. Decision impact

- M0 语义查询协议 v0 从 planned 变为 complete for design contract；
- M1 可先实现 module/declared signature outline，M2 再接 resolved relations 和查询；
- partial/verified/snapshot 规则成为未来 index golden tests 的硬约束；
- semantic hash、跨 Component impact 和真实 accuracy 进入后续原型；
- 下一步骤进入数值表示原型，不把 JSON fixture 当作编译器实现。

## 9. Links

- 核心语义：[`SEMANTICS.md`](../../SEMANTICS.md)；
- 工程架构：[`DEVELOPMENT.md`](../../DEVELOPMENT.md)；
- 代表性问题：[`examples/ISSUES.md`](../../examples/ISSUES.md)；
- 诊断协议：[`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md)。
