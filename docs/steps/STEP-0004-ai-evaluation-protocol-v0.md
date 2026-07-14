# STEP-0004: 建立 AI 评测协议与离线执行器 v0

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

为 A0、B、C 三套候选语法建立可复现的 AI 生成、理解和单点错误修复评测协议，并实现不依赖模型 API 的提示包生成、运行记录校验和离线评分工具。

## 2. Context and evidence

- [`AGENT_GOAL.md`](../../AGENT_GOAL.md) 要求记录模型快照、日期、完整提示、采样参数、重复次数、原始输出、评分器、token、成本和失败分类；
- [`SYNTAX.md`](../../SYNTAX.md) 要求生成、理解和修复使用相同模型与参数，每案例至少重复 30 次；
- [`syntax-mutations/`](../../syntax-mutations/README.md) 已提供 18 个可复现修复输入；
- A0/B/C 当前没有 parser 或 type checker，不能测量真实解析率和类型正确率。

## 3. Scope

包含：

- 固定任务清单与三套候选语法指南；
- generation、understanding、repair 三类任务；
- 模型无关的提示包生成器；
- JSON 运行记录格式、必需元数据和受控原始输出位置；
- 确定性离线评分器、合成 fixture 和自测；
- 明确 measured、verified 与 not measured 的边界。

不包含：

- 调用真实模型或提交凭据；
- 用字符串等价冒充 parser/type checker；
- 产生候选语法优胜结论；
- 实现正式 lexer、parser 或 token 计数器；
- 将合成 fixture 统计成 AI 实测结果。

## 4. Options and decision

### 绑定单一模型 API

可以立即发起请求，但会把供应商、认证、重试与速率限制混入评测核心，也无法在无凭据环境复现。

### 只写自然语言协议

容易阅读，但任务、提示和评分规则会在实际运行时漂移，无法机械确认覆盖率和结果格式。

### 决定：模型无关提示包 + 结构化运行记录 + 离线评分器

仓库只负责确定性准备和评分。外部适配器读取提示包并写入规定 JSON，凭据与请求逻辑留在仓库之外。真实运行必须保存模型精确版本和参数；fixture 使用显式 `synthetic` 标记，只验证工具链。

generation 与 repair 在 parser 出现前只计算规范源码精确匹配和最小修复，不宣称语义正确；understanding 使用结构化期望答案做确定性字段评分。parser/type checker 完成后通过新评分器版本增加真实语法与语义指标。

## 5. Plan

1. 定义协议版本、任务与运行记录格式；
2. 为三类评测建立固定任务集；
3. 编写 A0/B/C 最小语法指南；
4. 实现提示包生成器；
5. 实现运行记录验证和离线评分器；
6. 建立合成通过/失败 fixture，验证评分和失败分类；
7. 编写 v0 报告并更新项目状态；
8. 提交并推送。

## 6. Changes

- 新增 [`ai-eval/`](../../ai-eval/README.md) 协议目录、运行格式、三套语法指南和原始输出存储策略；
- 建立 generation 12、understanding 12、repair 18，共 42 个固定任务；
- 新增任务一致性校验器和确定性提示包生成器；
- 新增离线评分器，验证运行覆盖、模型元数据、提示包哈希、原始输出、token、成本和重复次数；
- 新增 pass、fail、invalid-model fixture 与端到端自测；
- 新增 [`ai-evaluation-protocol-v0`](../reports/ai-evaluation-protocol-v0.md) 报告并更新项目索引。

## 7. Validation

已验证：

- 数据清单 JSON 可解析，任务 ID 唯一且路径存在；
- generation 与 understanding 在 A0/B/C 间一一对应；
- repair 完整覆盖 18 个 mutation；
- 提示包生成结果确定；
- synthetic pass fixture 得到满分且不被标记为模型结果；
- synthetic fail fixture 得到预期失败分类；
- 缺少模型版本、参数或重复次数的真实运行被拒绝；
- Markdown 链接、whitespace 和 Git 状态检查。

核心输出：

```text
AI_EVAL_DATASET_OK tasks=42 generation=12 understanding=12 repair=18 A0=14 B=14 C=14
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=42 deterministic=true
```

## 8. Metrics

完成数据：42 个任务；A0/B/C 各 14 个；完整提示包 81,224 bytes；在 Windows PowerShell 5.1.26100.8655 下连续生成 SHA-256 均为 `1cdfcd9a7c68b6ea8380fd828c212021f82b6a8e80222117145e17edfc2b8b6f`。

真实首次通过率、理解正确率、修复率、token、延迟和成本均为 not measured。

## 9. Risks and follow-ups

- 规范源码精确匹配会拒绝语义等价但非规范的生成结果；需要 parser、formatter 和 type checker 后升级；
- 理解答案是人工制定的结构化 oracle，需要随语义 RFC 变更；
- 不同模型供应商的 token 计数不可直接混用，协议只保存供应商报告值和方法；
- STEP-0005 应继续剩余 P0 语义案例，不因缺少模型凭据阻塞设计工作。

## 10. Audit links

- report: [`ai-evaluation-protocol-v0`](../reports/ai-evaluation-protocol-v0.md)
- commit subject: `test(ai-eval): [STEP-0004] add reproducible offline evaluation harness`
- next: STEP-0005 remaining P0 semantic cases
