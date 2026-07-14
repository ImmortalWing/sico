# Report: AI 评测协议与离线执行器 v0

> - status: complete
> - date: 2026-07-14
> - related-step: STEP-0004
> - environment: Windows NT 10.0.26100.0, Windows PowerShell 5.1.26100.8655

## 1. Question

能否在没有模型凭据、parser 和 type checker 的条件下，先固定 Sico 候选语法的生成、理解和修复任务，生成不泄露答案的确定性提示包，并机械验证真实运行必需的元数据与离线评分流程？

## 2. Method

建立协议 `sico-ai-eval-v0`，将评测分为三层：

1. 仓库内固定任务、语法指南和提示组合规则；
2. 仓库外模型适配器，负责请求、重试和保存未经修改的原始输出；
3. 仓库内离线评分器，负责验证覆盖、模型元数据、提示包哈希、token/成本记录和确定性 oracle。

任务集包含：

| 类别 | Pair/错误类型 | A0 | B | C | 总数 |
|---|---:|---:|---:|---:|---:|
| generation | 4 | 4 | 4 | 4 | 12 |
| understanding | 4 | 4 | 4 | 4 | 12 |
| repair | 6 | 6 | 6 | 6 | 18 |
| total | — | 14 | 14 | 14 | 42 |

generation 和 repair 对去除测试元数据后的规范源码做精确匹配。understanding 对顶层字段逐项计分，对象键顺序和作为集合使用的数组顺序不影响结果。正式模型比较要求完整 42 项、每项至少 30 次，并记录模型精确版本、日期、采样参数、seed 支持、原始输出、token、成本、速率限制与提示包 SHA-256。

## 3. Reproduction

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-ai-eval.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/prepare-ai-eval.ps1 -OutputPath prompt-packets.json
```

评分真实运行时还必须提供其提示包：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/score-ai-eval.ps1 `
  -RunPath run.json `
  -PromptPacketPath prompt-packets.json `
  -OutputPath score.json
```

## 4. Raw evidence

- 协议和使用说明：[`ai-eval/`](../../ai-eval/README.md)；
- 任务：[`generation.json`](../../ai-eval/tasks/generation.json)、[`understanding.json`](../../ai-eval/tasks/understanding.json)、[`repair.json`](../../ai-eval/tasks/repair.json)；
- 运行记录格式：[`run-format.md`](../../ai-eval/run-format.md)；
- 工具：[`validate-ai-eval.ps1`](../../tools/validate-ai-eval.ps1)、[`prepare-ai-eval.ps1`](../../tools/prepare-ai-eval.ps1)、[`score-ai-eval.ps1`](../../tools/score-ai-eval.ps1)、[`test-ai-eval.ps1`](../../tools/test-ai-eval.ps1)；
- synthetic fixture：[`fixtures/`](../../ai-eval/fixtures/README.md)；
- 真实运行存储策略：[`runs/`](../../ai-eval/runs/README.md)。

在上述环境生成的完整提示包为 81,224 bytes，SHA-256：`1cdfcd9a7c68b6ea8380fd828c212021f82b6a8e80222117145e17edfc2b8b6f`。该哈希是本次环境与仓库版本下的可复现证据，不是跨工具版本的永久协议标识。

## 5. Results

| 检查项 | 结果 | 证据等级 |
|---|---:|---|
| 固定任务 | 42 | verified |
| generation / understanding / repair | 12 / 12 / 18 | verified |
| A0 / B / C | 14 / 14 / 14 | verified |
| repair 与 mutation manifest 一致 | 18 / 18 | verified |
| 连续两次提示包哈希一致 | yes | measured |
| pass fixture | score 1，3/3 fully correct | measured synthetic |
| fail fixture | score 0，预期失败分类全部命中 | measured synthetic |
| 缺失模型元数据 | rejected | verified |
| 完整模型记录路径 | accepted，42 attempts | verified with transient self-test |
| fixture 被标记为正式比较 | no | verified |
| 真实模型调用 | 0 | not measured |
| 模型生成/理解/修复率 | 无 | not measured |
| token、成本、延迟 | 无真实值 | not measured |
| parser/type checker 指标 | 无 | not measured |

自测输出：

```text
AI_EVAL_DATASET_OK tasks=42 generation=12 understanding=12 repair=18 A0=14 B=14 C=14
AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=42 deterministic=true
```

## 6. Interpretation

STEP-0004 已把“如何评测”从自然语言要求变成可执行、可拒绝无效记录的离线协议。任何供应商适配器只要保持提示和原始输出，就能复用同一评分器；synthetic fixture 无法被误标为正式候选比较。

当前精确源码匹配只衡量是否生成规范参考文本。它适合在编译器不存在时建立严格基线，但不能判断另一段代码是否语法合法、类型正确或语义等价。

## 7. Limitations

- 尚未运行真实模型；
- 规范源码精确匹配可能低估语义正确但格式不同的输出；
- understanding oracle 只有 4 组语义，主要覆盖当前首批 P0 案例；
- token 由供应商或外部适配器报告，不同计数方法不能直接比较；
- 真实原始输出不默认进入 Git，需要受控存储和哈希；
- parser、formatter 和 type checker 完成后必须增加相应评分，而不是继续用文本匹配代替。

## 8. Decision impact

- A0/B/C 已具备同模型、同参数、同任务的正式评测入口；
- 没有模型凭据不再阻塞其他 M0 设计工作；
- 当前没有足够数据接受或淘汰任何候选语法；
- STEP-0005 转向剩余 P0 语义案例；真实模型批量运行可在获得凭据和成本授权后作为独立步骤执行。

## 9. Links

- 执行记录：[`STEP-0004`](../steps/STEP-0004-ai-evaluation-protocol-v0.md)；
- 语法评测原则：[`SYNTAX.md`](../../SYNTAX.md)；
- 错误注入报告：[`syntax-error-injection-v0`](./syntax-error-injection-v0.md)。
