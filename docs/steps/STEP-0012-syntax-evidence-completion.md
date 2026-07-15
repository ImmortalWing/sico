# STEP-0012: 补齐第二轮语法证据

> - status: complete
> - phase: M0
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

把 A0、B、C 的静态比较、单点错误语料和 AI 离线评测从前四组语义扩展到全部 10 组 P0 语义，为 STEP-0013 的语法决定提供可复现证据；严格区分静态验证、合成 fixture 与尚未获授权的真实模型运行。

## 2. Baseline

- 三套候选各有 54 个镜像案例，但静态指标只覆盖首批 30 个；
- mutation corpus 有 6 类 × 3 候选，只覆盖旧的数字、record、match 和 Result 结构；
- AI v0 有 42 个任务，generation/understanding 只覆盖四组语义，repair 覆盖旧 18 个 mutation；
- 没有正式 parser/type checker，也没有真实模型凭据或成本授权。

## 3. Scope

包含全部 54 个候选案例的同规则静态指标、第二轮 6 类结构 mutation、覆盖 10 组语义的 generation/understanding 任务、覆盖全部 mutation 的 repair 任务、确定性提示包与离线评分器回归。

不包含真实模型调用、模型分数、正式 parser 恢复数据、类型检查成功率或 tokenizer 估算。

## 4. Plan

1. 建立可执行的候选静态指标工具和机器可读快照；
2. 为 capability/resource/using/task/interface/复杂签名增加对齐 mutation；
3. 将 AI 协议升级为 v1，扩展 guide、task、fixture 和校验断言；
4. 生成两次完整提示包并比较 SHA-256；
5. 运行所有仓库离线验证，记录 measured/verified/not-measured 边界；
6. 更新报告、状态、路线图和索引，提交并推送。

## 5. Validation contract

- A0/B/C 各 54 个案例且镜像元数据一致；
- 静态快照可由同一命令逐字重建；
- 每个 mutation 均由合法来源执行恰好一次声明替换得到；
- generation/understanding 每个语义组均覆盖 A0/B/C；
- repair 与 mutation manifest 双向完全一致；
- fixture 只能证明评分路径，不能进入候选成绩；
- 真实模型数据保持 `not measured (external authorization required)`。

## 6. Commit

`test(syntax): [STEP-0012] complete second-round evidence`

## 7. Changes

- 新增静态指标生成/校验器与 162-file 机器快照；
- mutation 从 18 扩展为 36，新增 capability、resource、using、task group、interface 和参数列表关闭错误；
- AI 协议升级为 `sico-ai-eval-v1`，generation/understanding 覆盖全部 10 组 P0，repair 覆盖全部 mutation；
- 更新三套 syntax guide、run format、fixture、离线校验器和文档；
- 新增 [`syntax-evidence-v1`](../reports/syntax-evidence-v1.md) 报告。

## 8. Validation

```text
SYNTAX_METRICS_OK metric=files/lines/bytes/tokens/punctuation/close-label-coverage A0=54/518/10067/2229/836/0 B=54/563/13217/2668/910/1 C=54/488/9990/2494/1266/0
MUTATION_CORPUS_OK entries=36 A0=12 B=12 C=12
AI_EVAL_DATASET_OK protocol=v1 tasks=96 generation=30 understanding=30 repair=36 A0=32 B=32 C=32
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=96 deterministic=true
SEMANTIC_CASES_OK cases=54 valid=25 invalid=29 A0=54 B=54 C=54 programs=162
```

完整提示包连续两次生成 `233750` bytes，SHA-256 均为 `1a32ebda11d7dbeab4f7fb2126f3f68f5670cc6915df4708b64b0a090d411303`。

## 9. Result and boundary

离线证据已覆盖全部 P0：54 cases/candidate、12 mutation intents/candidate、32 AI tasks/candidate。真实模型、模型 token 和 parser 恢复仍未测量；full model run 每模型至少 2,880 次响应，等待用户指定模型、凭据与成本授权，不产生虚构分数。

## 10. Audit links

- report: [`syntax-evidence-v1`](../reports/syntax-evidence-v1.md)
- metrics: [`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)
- mutation corpus: [`syntax-mutations/`](../../syntax-mutations/README.md)
- AI protocol: [`ai-eval/`](../../ai-eval/README.md)
- next: STEP-0013 syntax decision RFC
