# STEP-0498 m23 item 3 语料冻结 + RFC-0046 D3 决定记录（item 3 闭合）

> - status: complete（item 3 的 owner-review AI 语料腿已实跑、复审、冻结入库；D3 决定记录已写入 RFC-0046；**M23 整体裁决仍待独立 CI 门**）
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0490](./STEP-0490-m23-item3-bare-literal-preparation.md)（仪表与语料侧准备）、[M23 计划 §8.3](../plans/M23-language-v1-batch-3.md)、[RFC-0046](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md)（D3）、[M13 live-model 报告](../reports/ai-eval-live-model-v1.md)（provider 身份与成本口径）、[`ai-eval/runs/README.md`](../../ai-eval/runs/README.md)（原始输出入库策略）
> - gate: M23 出口审计 §4 residual #1（item 3 的 owner-review AI 语料）——**本片闭合该门**；residual #2（STEP-0328/0329 独立 CI 裁决）已由本片触发的 CI run 取结论，不在本片闭合范围内

## 1. 定位

STEP-0490 把 item 3 不依赖凭据的部分做完（selfhost 腿重测、第二套仪表、
进版库的 DeepSeek 适配器、离线自测），但**没有** provider 凭据，AI 语料腿
未跑。2026-10-21 owner 提供 DeepSeek 凭据，并对两个决策点作明确裁决：

1. **语料复审**：复审通过 → 按原样冻结，并把语料 + manifest + run 文件作为
   item 3 冻结证据入库；
2. **D3 决定记录**：记 "D3 stands" → 写入 selfhost 测量 + AI 腿负结果 +
   类型事实，**不改语法**。

本片执行这两项裁决，并按 M23 计划 §8.3 的输出合同把决定记录追加到
RFC-0046 的 outcome。

## 2. 交付物

| 文件 | 说明 |
|---|---|
| [`ai-eval/runs/item3-2026-10-21/`](../../ai-eval/runs/item3-2026-10-21/) | 冻结证据：`prompt-packets.json` + `run.json` + 100 个语料候选 + `corpus/manifest.json` + 审计说明 README |
| [`ai-eval/runs/.gitignore`](../../ai-eval/runs/.gitignore) | 为该目录开一条**显式入库例外**（默认策略是真实模型原始输出不入库） |
| [`ai-eval/runs/README.md`](../../ai-eval/runs/README.md) | 登记例外及其审计依据（owner 复审决定） |
| [`docs/rfc/RFC-0046-…v0.md`](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md) §3 | D3 的 2026-10-21 复测决定记录（**D3 stands**，无语法改动） |
| [`docs/reports/ai-eval-item3-live-model-2026-10-21.md`](../reports/ai-eval-item3-live-model-2026-10-21.md) | live-model 证据报告（模型身份、成本、语料统计、编译器状态、类型事实、可比性结论） |

## 3. AI 腿实跑（真实计费）

| 字段 | 值 |
|---|---|
| provider / model | `deepseek` / `deepseek-chat`（provider 只暴露别名，无版本快照串） |
| 参数 | temperature 0、top_p 1、`seed_supported: false`、4 并发、429 指数退避（实跑未观测到 429） |
| 调用 | 100 次（10 个 A0 `generation` 任务 × 10 次重复），0 失败 |
| token | 75,830 in / 6,116 out（provider 报告值） |
| 成本 | **USD 0.027196**（按公布费率 0.27/1.10 per 1M 计算；USD 10 硬上限未逼近） |
| 延迟 | p50 875 ms / p95 1281 ms / min 405 ms / max 1906 ms |
| 凭据 | owner 2026-10-21 提供，只从环境变量读，未落盘、未进任何提交物 |

实跑前用本地 mock 做了 **live HTTP 路径预检**（不改仓库）：请求构造、响应
解析、429 退避分支（21 次 HTTP / 20 次调用 = 退避分支确实被触发）、语料落盘
+ 逐文件 SHA、manifest、run 文件形状、成本核算 —— 14/14 全绿。

## 4. 语料复审与冻结

- 语料：100 文件 / 22,091 B / 1,190 行 / 560 表达式行（仓库测量工具口径）/ 350 调用点。
- **temp 0 下 provider 近确定性 → 100 次调用只产出 14 份不同程序**（每任务
  1–2 份）。owner 复审的实质对象就是这 10 个任务的不同程序。
- 冻结哈希：`run.json` `2bc2d768…`；`prompt-packets.json` `438c0bab…`；
  `corpus/manifest.json` `f27b1b16…`；语料文件列表 `dad8fef7…`。
- 入库依据：`ai-eval/runs/README.md` 要求"确有长期回归价值的数据必须经新的
  审计步骤决定才能入库"——**该审计步骤即 owner 本次复审决定**，已在该 README
  与冻结目录 README 中登记。理由：仓库内唯一的 AI 生成 Sico 语料、体量可控、
  逐文件 SHA 可核、且 D3 决定记录以它为对象。

## 5. 计量结果与诚实性

| 指标 | AI 腿（本片） | selfhost 腿（固定提交 2531427） |
|---|---:|---:|
| 字面量构造调用 | **0** | 3,236 |
| 每 KB | **0.000** | 3.055 |
| 每百表达式行 | **0.000** | 17.638 |
| 假设 typed-literal 形可省 | 0 B（0.00%） | 42,042 B（3.88%） |

两套独立仪表对拍一致（仓库测量工具 + 独立重算），且 run 文件的 `raw_output`
与语料文件逐字节对齐（两侧均 22,091 B）。

**这个 0 是表面语法属性，不是 AI 代码属性**：A0 是 M1 时代**保留作对照**的
候选语法（`syntax-candidates/README.md`：`not selected; retained control`），
其指南**根本没有显式宽度字面量形**，且"没有编译器可以执行"——100/100 文件被
当前前端以 `E1013` 拒绝（冻结提示要求无 `main` 的声明式程序，而当前语言要求
顶层显式 `main`；A0 候选语料里同样一个 `main` 都没有）。因此 **AI 腿与
selfhost 腿不可比**，既不能佐证也不能否证 3.055/KB。

**类型事实（当前编译器实测）**：`Int` 位置的裸整数**已合法**（`return 0` →
syntax ok / type ok / 0 诊断）；`I64` 位置的裸整数被 `E2002` 拒（`expected
I64, found Int; convert explicitly`）；`Int` 位置的 `I64.literal(0)` 被 `E2001`
拒。`Int` 与 `I64` 无隐式转换 ⇒ 显式构造子**只在真正需要 `I64`/`U64` 值时
才被强制**；selfhost 的密度反映的是它自身的宽度型代码属性，不是语言强加的税。

## 6. D3 决定记录（写入 RFC-0046 §3）

记录内容：①selfhost 腿新测量；②AI 腿负结果及其表面语法成因（含"不可比"
的明确结论）；③`Int`/`I64` 类型事实；④结论 —— **D3 stands，不改语法**；
真正待决的问题被收窄为"能否为 `I64`/`U64` 写一个免显式构造子的上下文推断
宽度字面量"，但**本片不就此开 RFC**，任何未来的 typed-literal RFC 仍须带
完整 RFC-0033 证据（显式反糖、source-map 身份、格式化幂等、typed 诊断）与其
自己的实测消费者。

**本片未改任何语法、语义、IR、codegen 或运行时代码。**

## 7. 证据

- 报告：[`ai-eval-item3-live-model-2026-10-21`](../reports/ai-eval-item3-live-model-2026-10-21.md)
- 冻结语料：[`ai-eval/runs/item3-2026-10-21/`](../../ai-eval/runs/item3-2026-10-21/)
- 决定记录：[RFC-0046 §3 D3](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md)
- 归因报告（本会话另一门）：[`ci-arena-ceiling-attribution-2026-10-21`](../reports/ci-arena-ceiling-attribution-2026-10-21.md)

```powershell
# 计量（冻结语料）
powershell -NoProfile -ExecutionPolicy Bypass -File tools\measure-bare-literal-ceremony.ps1 `
  -CorpusPath ai-eval\runs\item3-2026-10-21\corpus
# → BARE_LITERAL_CEREMONY files=100 lines=1190 kb=21.573 expression_lines=560 call_sites=350 literals=0 …

# 编译器状态（全部 100 个候选）
target\debug\sico.exe check --json ai-eval\runs\item3-2026-10-21\corpus\GEN-A0-CAP-001-r01.sico
# → 11 × E1013 SYNTAX_UNEXPECTED_TOP_LEVEL
```

## 8. 残余与下一步

- **item 3 闭合**（本片）：owner-review 语料已实跑、复审、冻结入库；D3 决定
  记录已写入；无语法改动。
- **独立 CI 裁决**：CI 恢复可达后已触发三轮 —— `39454624507`（2531427）、
  `39457030463`（6808569）两次红在**同一处**且根因已确定性归因为 M22 轨的
  `MEMORY_PAGES`(33792 页) vs 宿主上限(32768 页)常量错配（见归因报告）；M22
  轨已于 STEP-0497 修复并推分支，本片触发的 run `39524849306`（`7e5ad3c`）
  的结论待取。**M23 整体裁决不在本片作出**：七门与三项残余须按出口审计口径
  逐门复审。
- **item 3 的可比 AI 腿仍未存在**：若要得到与 selfhost 腿可比的 AI 腿，需要
  以当前语法表面重新生成（= 新提示工程，与计划"不新造提示工程"的默认冲突，
  需 owner 显式豁免）。本片不作此选择。
