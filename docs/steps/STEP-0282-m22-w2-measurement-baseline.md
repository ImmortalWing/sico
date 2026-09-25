# STEP-0282: M22 W2 测量基线冻结（执行卡 22-B）

> - status: complete / measurement-only baseline frozen; no gate moved, M22 stays NO-GO
> - phase: M22 compiler self-host — W2 pre-implementation measurement (execution card 22-B)
> - date: 2026-09-25
> - evidence class: internal-fixture, Windows x64 GNU real runner

## 1. Scope and decision

执行 [M22–M26 execution cards](../plans/M22-M26-execution-cards.md) 卡
**22-B**：在进入任何 W2 S3/S4 实现 STEP 前，冻结 records-surface 测量基线。
卡 22-A（STEP-0278/0280 独立 CI 裁决）因本次无提交/推送授权仍保持待裁决；
22-B 按卡面定义不依赖 W1 裁决，且**不计入 R3 停损窗口**。

冻结产物：[`m22-w2-baseline-2026-09-25.json`](../reports/m22-w2-baseline-2026-09-25.json)
（schema `sico.m22.w2.baseline.v0`），由两个 R1 工具的原始输出程序化生成：

1. `tools/report-m22-canary.ps1` 全量重跑（真实 runner，fuel 5e9）：
   **22/30（73.3%）**，前沿 `nearest_match` /
   `ERR:E-SH-IR-GWPACK-OTHER`，整份 formatter typed refusal exit 122——
   与 STEP-0266/0280 记录完全一致，无漂移。
2. `tools/probe-m22-budget.ps1` 四档阶梯重跑（与 STEP-0266 同口径）：
   1,054/4,128 行消耗燃料 ∈ (1e8,1e9]，8,217/16,395 行 ∈ (1e9,5e9]
   （8k/16k 的 min_cap 饱和在 5e9 阶梯顶），区间与 STEP-0266 一致、
   无悬崖；`PROBE_M22_BUDGET_OK inputs=4`。
3. 当前 tracked `selfhost/*.sico` 盘点：**8 个源、16,390 行**，逐文件
   行数 + SHA256 + 角色入册。`lexer.sico`/`tokens.sico`/
   `declaration_parser.sico` 已由 STEP-0278 退役，不在册、不是队列项。

## 2. Honest gaps（按卡面要求逐项写明）

- **栈高水位：未测得。** runner CLI 不暴露栈高水位（probe 工具证据注记
  原文记录）；可观测上限仍是 StackLimit 结果类。不估算。
- **天然输入 wall-time 未复测**：STEP-0266 的 debug-runner 数值
  （22 函数前缀 45.9 s、全量 formatter 41.9 s）仍为在案记录；
  release runner 的 wall 复测仍开放。
- **本基线为 debug runner + 本机构建缓存产物**：component 按
  STEP-0261 host note 的本机 ABI 变体构建，源树以 SHA256 固定；
  独立 CI 复算时以同一 JSON 的分母与工具重放。
- STEP-0279/0280 未提交修改包含在被测树中（worktree.note 已注明）。

## 3. 六行审查记录（卡面格式）

```text
卡号 22-B / STEP-0282 / 入口 = 卡 22-B（W1 可未裁决，仅测量）
改动 = 本 STEP + 基线 JSON + 索引同步；语料 SHA 见基线 JSON source_tree
前值 → 后值：canary 22/30 → 22/30（无变化）；前沿 GWPACK-OTHER → 同；
燃料区间 (1e8,1e9]/(1e9,5e9] → 同；R3 计数不启动（测量 STEP）
命令 = report-m22-canary.ps1（exit 0）、probe-m22-budget.ps1 -Sizes
@(1024,4096,8192,16384)（PROBE_M22_BUDGET_OK）；未执行 = 独立 CI
gate = W1 待裁决；M22 NO-GO；W2 基线 = 已冻结（本 STEP）
下一张允许领取的卡 = 22-C（需 22-A W1 GO + 本基线，二者缺一不可）
```

## 4. Executable evidence

- `powershell -File tools/report-m22-canary.ps1`：exit 0，JSON 见基线
  `canary` 节（component 由 `sico build --profile script-v0
  selfhost/compiler.sico` 现场构建并实际加载）。
- `powershell -File tools/probe-m22-budget.ps1 -Sizes
  @(1024,4096,8192,16384)`：exit 0，四输入 JSON + `PROBE_M22_BUDGET_OK`。
- 未运行 Cargo 工作区/runner 测试：本 STEP 零产品代码改动，测量工具
  自带构建纪律（cargo build 缓存命中 0.12–0.25 s）。
