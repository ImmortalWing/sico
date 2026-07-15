# Report: Syntax evidence v1

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0012
> - protocol: `sico-ai-eval-v1`

## 1. Question

能否在没有正式 parser/type checker、模型凭据和成本授权的条件下，把 A0/B/C 的静态、mutation 与 AI 离线评测证据补齐到全部 10 组 P0 语义，并为语法决定明确剩余不确定性？

## 2. Evidence classes

- `verified`：仓库脚本可从受版本控制的源文件确定性重建或拒绝漂移；
- `synthetic`：fixture 只验证准备/评分路径，禁止作为模型成绩；
- `not measured`：需要正式 parser、type checker 或真实模型调用；
- `inference`：由结构信息推导的设计取舍，不能冒充实验结果。

## 3. Static metrics

同一工具删除元数据与空行后统计全部 54 × 3 个候选程序：

| Candidate | Files | Lines | UTF-8 bytes | Lexical tokens | Punctuation | Block closes labeled |
|---|---:|---:|---:|---:|---:|---:|
| A0 | 54 | 518 | 10,067 | 2,229 | 836 | 0 / 149 |
| B | 54 | 563 | 13,217 | 2,668 | 910 | 151 / 151 |
| C | 54 | 488 | 9,990 | 2,494 | 1,266 | 0 / 149 |

A0 的 lexical token 最少；C 的 bytes 最少但 punctuation 比 A0 多 51.4%；B 比 A0 多 19.7% lexical token、31.3% bytes，换来每个已统计块结束都显式携带结构名称。`close label` 是静态信息量，不是恢复率。

机器证据在 [`metrics-v1.json`](../../syntax-candidates/metrics-v1.json)，方法与轮次拆分见 [`METRICS.md`](../../syntax-candidates/METRICS.md)。

## 4. Mutation corpus v1

旧 6 类结构 mutation 保持不变，新增：

| ID | P0 group | Error intent | Recovery oracle |
|---|---|---|---|
| MUT-007 | effects/capabilities | missing capability close | next definition |
| MUT-008 | affine resources | missing resource close | next definition |
| MUT-009 | affine resources | missing using close | enclosing function close |
| MUT-010 | Future/Task | missing task-group close | enclosing function close |
| MUT-011 | Component | missing interface close | EOF |
| MUT-012 | revision | missing parameter-list close | function body |

结果：12 类 × A0/B/C = 36 个 mutant；36/36 来源为 `accept` case；36/36 由 manifest 的一次唯一文本替换逐字重建。`recovery` 与 `diagnostic` 仍是设计 oracle，没有 parser 因而没有实际跨度、恢复距离或级联数量。

## 5. AI evaluation protocol v1

任务变化会影响比较，因此协议由 v0 升级为 `sico-ai-eval-v1`：

| Category | v0 | v1 | v1 coverage |
|---|---:|---:|---|
| generation | 12 | 30 | 10 个 P0 pair × A0/B/C |
| understanding | 12 | 30 | 同一 10 个 P0 pair × A0/B/C |
| repair | 18 | 36 | 全部 12 mutation × A0/B/C |
| total | 42 | 96 | 每个候选 32 个 task |

两次独立生成完整提示包均得到：

```text
tasks=96
bytes=233750
sha256=1a32ebda11d7dbeab4f7fb2126f3f68f5670cc6915df4708b64b0a090d411303
```

合成 pass/fail fixture、非法 model metadata 拒绝、96-task model metadata smoke path 与确定性生成均通过。smoke path 使用仓库 oracle 构造瞬时响应，明确不是模型调用。

## 6. Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/measure-syntax-candidates.ps1 -CheckPath syntax-candidates/metrics-v1.json
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-syntax-mutations.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
```

关键输出：

```text
SYNTAX_METRICS_OK ... A0=54/518/10067/2229/836/0 B=54/563/13217/2668/910/1 C=54/488/9990/2494/1266/0
MUTATION_CORPUS_OK entries=36 A0=12 B=12 C=12
AI_EVAL_DATASET_OK protocol=v1 tasks=96 generation=30 understanding=30 repair=36 A0=32 B=32 C=32
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=96 deterministic=true
```

## 7. What remains unmeasured

- 真实模型首次生成、理解和单轮修复率；
- 真实 tokenizer/供应商 token、成本与延迟；
- parser 主要诊断位置、级联数量和恢复距离；
- type-check success 与语义等价率；
- 不同模型家族、温度和上下文长度的稳定性。

真实 full run 每个模型至少需要 96 × 30 = 2,880 个原始响应，必须先有用户提供的模型范围、凭据和成本授权。没有授权时不生成模型分数，但这不阻止基于已声明设计优先级选择 M1 parser 的起始语法；该选择必须在 RFC 中注明可复审条件。

## 8. Decision impact

- A0 在 token 简洁度领先，但通用 `end` 不提供结构类型；
- B 以可量化的 token/byte 成本换取 100% 带名称块结束；
- C 在 bytes/lines 上紧凑、训练语料形态常见，但标点最多且 `}` 不声明结构类型；
- 没有真实 parser/model 数据，任何“更易恢复/更易生成”只能是 inference；STEP-0013 必须选择明确的工程基线和复审门槛，不能宣称已证明模型优胜。

## 9. Links

- step: [`STEP-0012`](../steps/STEP-0012-syntax-evidence-completion.md)
- AI harness: [`ai-eval/`](../../ai-eval/README.md)
- mutation corpus: [`syntax-mutations/`](../../syntax-mutations/README.md)
- static metrics: [`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)
