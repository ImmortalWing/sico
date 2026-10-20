# STEP-0490 m23 item 3 裸字面量复测：仪表与语料侧准备（AI 腿待 provider 凭据）

> - status: implementation complete locally（准备工作完成；item 3 的 AI 语料腿仍待 owner 凭据，未闭合）
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0284](./STEP-0284-m23-ceremony-census.md)（开工普查）、[M23 计划 §8.3](../plans/M23-language-v1-batch-3.md)、[RFC-0046](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md)（D3）、[M13 live-model 报告](../reports/ai-eval-live-model-v1.md)（provider 身份与成本口径）、[`ai-eval/run-format.md`](../../ai-eval/run-format.md)
> - gate: M23 出口审计 §4 residual #3（item 3 裸字面量复称）——**本片不闭合该门**，只把不依赖凭据的部分做完并钉住

## 1. 定位

M23 计划 §8.3 的 item 3 是**政策项**：在改语法之前，先在**真实生成语料**上重新称量裸字面量的仪式成本，产出 RFC-0046 D3 的决定记录（"D3 维持"或最小 typed-literal RFC）。

语料分两腿：

- **selfhost 腿**（已有，但已过时）：STEP-0284 冻结的是 2026-09-25 的 8 源 / 16,390 行 / 2,169 处字面量构造（3.199/KB）。此后 selfhost 树发生了实质变化（`parser.sico` 前端重写 + arena treadmill + 新增 `codegen.sico`），冻结数字已不代表当前面。
- **AI 生成腿**（缺）：需要 provider 凭据。

本片把两件事做完：**selfhost 腿在固定提交上重测并与普查工具独立对拍**，**AI 腿的语料来源、适配器、计费与冻结机制全部就位**（只差密钥）。

## 2. 交付物

| 项 | 位置 | 说明 |
|---|---|---|
| 普查工具输出参数 | `tools/census-m23-ceremony.ps1` | 新增 `-OutputPath`；默认仍写 STEP-0284 冻结产物（复现契约不变）。**修掉一个真缺陷**：该工具此前硬编码带日期的冻结文件路径，任何一次重跑都会静默覆盖"证据之记录" |
| 裸字面量仪式仪表 | `tools/measure-bare-literal-ceremony.ps1`（新） | 确定性文本计量：字面量构造调用数 / 每 KB / 每百表达式行 / 每百调用点；并给出**假设 typed-literal 形**的同口径对照（把 `X.literal(<digits>)` 改写为 `<digits>` 后重测字节与行）。schema `sico.bare-literal-ceremony.v0` |
| DeepSeek 适配器 | `tools/ai-eval/deepseek_generation_corpus.py`（新，**进版本库**） | M13 的适配器当时放在 `target/ai-eval-experiments/adapter/`（gitignored），已随 target 目录丢失。本片把适配器、计费与语料清单放进仓库，独立方可复跑。纯标准库（无第三方 HTTP 客户端），密钥只从环境变量 `DEEPSEEK_API_KEY` 读，永不落盘 |
| 语料来源 | `tools/prepare-ai-eval.ps1`（既有） | 直接复用冻结的 `sico-ai-eval-v1` 生成提示：`generation` + `A0` 语法 = **10 个任务**（A0 是当前候选语法；B/C 是替代候选语法，不计入当前 Sico 的仪式成本）。10 任务 × N 次重复 = 语料规模 |
| 固定快照自测 | `git archive 2531427` 导出到 `target/tmp/` 后计量 | 9 个 selfhost blob 与该提交**逐字节一致**（`git hash-object` 对拍），排除并发会话在途修改对数字的污染 |

## 3. selfhost 腿重测（固定提交 2531427）

两套**独立实现**的同口径计数一致（字面量构造调用数）：

| 口径 | STEP-0284 冻结（2026-09-25） | 本片（提交 2531427） |
|---|---|---|
| 普查工具：selfhost 源数 / 行 / 字节 | 8 / 16,390 / 694,343 | 9 / 24,517 / 1,082,777 |
| 普查工具：字面量构造调用 | 2,169 | **3,236** |
| 普查工具：每 KB | **3.199** | **3.060** |
| 普查工具：比较谓词 if 行 | 978（59.671/千行） | 1,328（54.166/千行） |
| 普查工具：bind 型 match 块 | 248 | 401 |
| 普查工具：应用语料（`tests/end-to-end`）字面量 | 768（51 文件） | 806（53 文件） |
| 仪式仪表：字面量构造调用（14 文件 = 9 源 + `corpus-w1` 5 条增量） | — | **3,236**（与普查工具一致 ✓） |
| 仪式仪表：每 KB / 每百表达式行 / 每百调用点 | — | 3.055 / 17.638 / 18.710 |
| 仪式仪表：假设 typed-literal 形可省的字节 | — | **42,042 B**（占语料 3.88%） |

**读法（不构成任何语法决定）**：绝对调用数 +49%，但**密度下降 4.5%**（3.199 → 3.055/KB），比较谓词密度同样下降（59.671 → 54.166/千行）。自举树的增长来自与字面量无关的代码（arena treadmill 机器）。按 M23 计划 §8.1 的纪律，这只是需求信号，不从行数或回忆argue任何算子/语法决定；D3 是否维持**必须**等 AI 腿。

## 4. AI 腿的准备状态

| 环节 | 状态 |
|---|---|
| 端点可达性 | ✅ 实测 `https://api.deepseek.com` 返回 401（网络通、缺凭据） |
| provider 身份与参数 | ✅ 钉死为 M13 同款：`deepseek` / `deepseek-chat` / temperature 0 / top_p 1 / seed 不支持 / 4 并发 / 429 指数退避 |
| 成本口径与硬上限 | ✅ 按 provider 报告的 token × DeepSeek 公布费率（0.27 / 1.10 USD per 1M in/out）计算，默认硬上限 USD 10（M13 同口径） |
| 语料来源 | ✅ 冻结协议的 10 个 A0 生成任务，`prepare-ai-eval.ps1` 可复算（SHA 已记录） |
| 适配器离线自测 | ✅ fixture 模式端到端跑通：20 次调用 → run 文件（SHA 记录）+ 20 个语料候选 + `manifest.json`（逐文件 SHA）+ 成本 $0.0140；`--dry-run` 输出完整调用计划 |
| **provider 凭据** | ❌ **不在本机**：M13 的 key 是 2026-09-05 会话内交付，未进 Git、未落盘；本机会话存储只有 2026-10，检索不到任何 DeepSeek key 或旧适配器 |
| owner 复审腿 | ❌ 计划 §8.3 要求语料经 owner 复审后才冻结计数 |

## 5. 诚实性

- 本片**不**闭合 item 3，**不**改 RFC-0046 的 D3 决定记录，**不**改任何语法面；
- 本片**不**产生任何模型分数或 AI 质量主张——fixture 是自造的管道自测，不是模型输出；
- 上表的 selfhost 数字是**固定提交**上的测量，但工作树在并发 M22 会话下仍在变动，因此本片记录的是"提交 2531427 上的面"，不是"当前工作树的面"；
- 新普查产物 `docs/reports/m23-ceremony-census-2026-10-21.json` 的 `repo_root` 字段记的是测量时所在的固定快照路径，`step` 字段沿用工具身份（STEP-0284 的普查契约）——**运行身份是 STEP-0490**（见本记录片与 steps 索引）；
- 普查工具此前会覆盖冻结产物的缺陷是**本片发现并修掉的**（本片第一次重跑确实覆盖了冻结文件，已用 `git checkout` 还原为 STEP-0284 的提交态）；
- 证据等级 `internal-fixture`；未跑独立 CI（本片提交后另行触发）。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 普查工具（新输出参数） | `tools/census-m23-ceremony.ps1 -RepoRoot <pin> -OutputPath docs/reports/m23-ceremony-census-2026-10-21.json` | `CENSUS_M23_CEREMONY_OK selfhost_files=9 app_files=53`，exit 0；冻结产物 `m23-ceremony-census-2026-09-25.json` 未再被改写 |
| 仪式仪表 | `tools/measure-bare-literal-ceremony.ps1 -CorpusPath <pin>/selfhost` | `BARE_LITERAL_CEREMONY files=14 lines=24600 kb=1059.349 expression_lines=18347 call_sites=17291 literals=3236 per_kb=3.055 per_100_expr=17.638 typed_bytes_saved=42042`，exit 0 |
| 两工具交叉对拍 | 字面量构造调用数 | 普查 3,236 = 仪表 3,236 ✓ |
| 固定快照完整性 | `git hash-object` 逐文件对拍 | 9/9 selfhost blob 与提交 2531427 一致 ✓ |
| 适配器离线自测 | `deepseek_generation_corpus.py --fixture-dir … --repetitions 2` | run 文件 + 20 个语料候选 + manifest + 成本 $0.0140，exit 0 |
| 适配器计划模式 | `deepseek_generation_corpus.py --dry-run` | 10 包 × N 次 = 调用计划、端点、预算上限全部打印，exit 0 |

未跑：任何 provider 实调用（无凭据）、独立 CI（本片提交后触发）、live-model 评分（不在本片范围）。

## 7. 下一步（owner 输入）

1. **给凭据**（一行即可）：把 DeepSeek key 放进环境变量 `DEEPSEEK_API_KEY`，或直接告知。我按 M13 同款参数跑 10 任务 × N 次（默认 10 次 = 100 个 AI 生成程序，成本量级 $0.03），产出语料 + run 文件 + SHA 清单。
2. **owner 复审语料**：我把语料与逐文件 SHA 交给你复审；复审通过后才冻结计数。
3. **出决定记录**：在冻结语料上跑仪式仪表，把结果写进 RFC-0046 的 D3 决定记录（"D3 维持"或最小 typed-literal RFC 的完整 RFC-0033 证据）。
4. 若 owner 选择**显式豁免**该外部门，则按 M23 出口审计原文，M23 可整体 GO（豁免须显式记录，不得隐含）。
