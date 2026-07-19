# Sico AI evaluation v1

> - status: verified offline harness
> - protocol: `sico-ai-eval-v1`
> - related step: [`STEP-0004`](../docs/steps/STEP-0004-ai-evaluation-protocol-v0.md)
> - extended by: [`STEP-0012`](../docs/steps/STEP-0012-syntax-evidence-completion.md), [`syntax evidence v1`](../docs/reports/syntax-evidence-v1.md), [`STEP-0068`](../docs/steps/STEP-0068-ai-tooling-measured-evaluation.md)

本目录定义 A0、B、C 候选语法的可复现 AI 生成、理解和错误修复评测。评测核心不绑定模型供应商：仓库生成固定提示包，外部适配器调用模型并保存原始输出，仓库中的离线评分器只读取结果文件。

## Workflow

```text
task manifests + syntax guides
             ↓ prepare
      prompt-packets.json ──→ external model adapter
                                  ↓ raw outputs
                              run.json
                                  ↓ score
                              score.json
```

1. 校验协议和任务：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
   ```

2. 生成不含答案的提示包：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/prepare-ai-eval.ps1 -OutputPath prompt-packets.json
   ```

3. 外部适配器按照 [`run-format.md`](./run-format.md) 保存模型原始输出。

4. 离线评分：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/score-ai-eval.ps1 -RunPath run.json -PromptPacketPath prompt-packets.json -OutputPath score.json
   ```

5. 运行仓库自测：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
   ```

## Task set

| 类别 | 数量 | 当前确定性评分 |
|---|---:|---|
| generation | 30 | 去除案例元数据后的规范源码精确匹配 |
| understanding | 30 | 顶层字段及结构化值匹配 |
| repair | 36 | 精确恢复对应合法来源程序 |
| total | 96 | — |

generation 和 understanding 使用相同的 10 个语义 pair，覆盖全部 P0 语义组，并分别覆盖 A0/B/C。repair 完整覆盖 [`syntax-mutations/`](../syntax-mutations/README.md) 的 36 个单点错误。

## Evidence levels

- `verified`：任务清单、路径、覆盖、一一对应关系、提示包确定性和评分器 fixture；
- `measured`：只有真实模型运行产生的原始输出、token、成本和评分；
- `not measured`：当前仓库没有真实模型运行，也没有 parser/type checker 指标。

[`fixtures/`](./fixtures/) 中的数据具有 `synthetic: true`，只用于证明工具能识别正确和错误输出，严禁计入模型比较。

## AI-oriented error taxonomy

[`error-taxonomy.json`](./error-taxonomy.json) 将当前设计失败语料分成 12 类，精确覆盖 24 个稳定语义诊断、E1001–E1013、29 个语义负例、12 类/36 个语法 mutation 与 STEP-0092 top-level rejection。每类记录 compiler prevention 与 AI repair constraint。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-error-taxonomy.ps1
```

taxonomy 的 `observed_ai_frequency` 固定为 `not-measured`，且禁止排名。它是未来真实模型错误的分类 schema，不是“这些错误在 AI 中已经常见”的频率证据。

## Compiler-backed tooling v0

[`tooling-protocol.md`](./tooling-protocol.md) 增加本地、无副作用的 `inspect` 与 `validate_fix` JSON 协议。它复用正式 compiler diagnostics、Semantic Index 和 canonical formatter，并以 source digest、精确诊断 multiset、单段 edit 和 256-byte change cap 防止 stale 或宽泛修复。

该工具已离线检查 54 个 B source（25 clean、29 diagnosed）并验证 12 个 B repair oracle；这些是工具链测量，不是模型分数。A0/C 继续作为设计历史与 v1 比较任务保留。真实模型评测仍必须遵循本页的 provider metadata、raw output、30 次重复、凭据和成本授权门槛。

## Versioning

任务、提示或评分语义变化时必须增加协议版本。只修正文档拼写或不影响生成字节的工具缺陷可以保留当前版本，但报告中必须记录工具提交。不同协议版本的分数不能直接合并。
