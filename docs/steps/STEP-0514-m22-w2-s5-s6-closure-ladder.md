# STEP-0514：M22 S5/S6 闭合阶梯（owner 规划片）

> - 阶段：M22 S5/S6 规划（**无实现**）
> - 日期：2026-10-22
> - 执行：owner 指示"只负责规划后续步骤，尽量细化，执行是另一个对话"
> - 产物：[M22 计划 §3.3](../plans/M22-compiler-self-host.md)
> - 状态：planning complete（**不推进任何出口门；M22 仍为 NO-GO**）

## 1. 本片目的与结论

本片**不含任何实现**。owner 在 2026-10-22 会话中确认了三项决策，并要求把 M22 的剩余工作细化成可直接执行的阶梯，交给另一个会话执行。

- **决策 1（同意）**：允许插入一个**纯测量片**并前置到 37 语料梯级之前，用途是把"1 MB 级组件发射是否可行"这个可能推翻里程碑的唯一未知量提前暴露。
- **决策 2（同意）**：授权在**合格跑之前**重新基线化 arena / fuel / timeout 限额（ADR-0015 决策 8 允许事前固定限额；失败后上调作废该次 attempt）。
- **决策 3（同意）**：`origin/dev` 并行冲突历史的裁决**推迟到门 6 收尾**统一处理。
- **追加授权（2026-10-22，会话末）**："全权拍板" ⇒ 本会话把剩余全部待决项（canary 次序、判读阈值、探针落点、并行规则、降级规则、工具面、喂入定界、耗量口径、冲突裁决方式、交付物清单）**一次性裁决为 D4–D15**，登记进计划 §3.3.2 与本记录 §4.2；**唯一触及的合同** = [ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)（组件级 canary 次序重排，只改次序、不改门与 `A == B == C`）。

**结论**：M22 剩余工作已收敛为 **7 个 slice（W1–W7）、约 75–125 片**，关键路径 = `W1 → W2 → W3 → W5 → W6 → W7`。阶梯全文写入 [M22 计划 §3.3](../plans/M22-compiler-self-host.md)，是执行会话的唯一权威入口。

## 2. 本片确立的量化基线（实测）

| 项 | 实测值 | 取值方式 |
|---|---|---|
| `A = R(S)` 组件总量 | **1,024,613 B** | 解析 Rust 后端**生产输出** `target/tmp/step0506/compiler.component.wasm` 的 component/core-module 段 |
| 其中 code 段 / data 段 / type 段 | 1,003,250 / 16,487 / 3,685 B | 同上（core sections 10 / 11 / 1） |
| 冻结源集 S | 9 文件 / 29,088 行 / 1,299,939 B | `selfhost/*.sico` |
| S 内宿主内建 | **20 个族**，`text.concat` 2,882 次最多 | `selfhost/*.sico` 全文统计 |
| guest 已闭合内建 / 已发射最大组件 | **1 个** / **1,025 B** | STEP-0513 §5.1 与新钉 |
| 冻结语料 | **37 accepted / 178 refused** | `selfhost/script-build-corpus-v0.json` |
| 拒绝诊断分布 | LEXICAL 116 / SCRIPT_ABI 28 / 散码 34 | 同上 |

⇒ **固定点落差约 1000 倍**（guest 要发射 ~1.02 MB，而它发射过的最大组件是 1,025 B）。

## 3. 阶梯结构（摘要）

| slice | 内容 | 出口判据摘要 |
|---|---|---|
| W1 | 1 MB 级发射的规模可行性测量（**只测量**） | 1 MiB 输出的 (wall, fuel, 峰值内存) 与增长曲线；三档判读（≤6× 可行 / ≤16× 临界 / >16× 不可行） |
| W2 | 20 个宿主内建族的逐字节编解码 | 每族：IR 逐字节等同 **且** 整组件逐字节等同 **且** 过真实 `wasmparser::Validator` **且** 前沿探针已翻转 **且** 独立 CI 绿 |
| W3 | **组件级广度 rung**：W3-a `formatter.sico` 组件级 → W3-b 37 条 accepted 语料组件级差分 | formatter 组件逐字节等同；每条语料 == Rust 生产输出 == 冻结 `component_sha256` |
| W4 | 178 条拒绝一致性 | (verdict, diagnostic id, span) 三元组逐条一致 |
| W5 | **组件级尺度 rung**：5 文件 bundle（≈1,024,613 B）+ 9 文件全集组件级全等 | bundle 组件逐字节等同 |
| W6 | ADR-0015 自举闭环 | `A == B == C` + M7 `.sapp` + hostile 语料 + 预算实测 + 证据清单 |
| W7 | 门 6/7 收尾 | 快照 + 独立 CI + 全量 M0–M21 + 新出口审计 + 冲突裁决 + 门 4 补强 + 支持面登记表 |

**W1 的三档判读是本阶梯的决策枢纽**：判读为"不可行"即触发新增止损触发器 T-R3-a，停止 W2/W3 的顺序推进并转入 ADR / RFC 修订流。

## 4. 决策登记与文档差异裁决

**4.1 owner 三决策（2026-10-22 授权）**：① 允许插入纯测量片并前置；② 授权合格跑之前重基线化限额（失败后上调作废该 attempt）；③ `origin/dev` 冲突历史推迟到门 6 收尾。

**4.2 追加授权"全权拍板" ⇒ D4–D15 全部裁决关闭**（owner 2026-10-22；执行会话直接遵守，不再回头请示）：

| 编号 | 决定 | 落点 |
|---|---|---|
| D4 | 组件级 canary 次序冻结为 `formatter 组件级 → 37 语料组件级 → bundle 组件级 → harness`（**先广度后尺度**） | **[ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)**（新增修订段，status 行标注 A1）；STEP-0486 §2.4 加对齐注记（原句保留以便追溯） |
| D5 | W1 探针**不进 `selfhost/`**，放 runner 测试侧独立 fixture；探针组件**不得进入 A/B/C 链** | 计划 §3.3.4 |
| D6 | W1 判读阈值固定（≤6× 可行 / ≤16× 临界 / >16× 不可行） | 计划 §3.3.4 |
| D7 | 不可行分支方案优先级：二进制输出通道 > 流式/分块发射 > 降中间表示成本 > 提高限额 | 计划 §3.3.4 |
| D8 | 限额重基线候选值：arena 4 GiB / host cap 5120 MiB / fuel 1.6T / timeout 180 s（2× 现有值；**不得无证据上调**） | 计划 §3.3.2 / §3.3.4 |
| D9 | **W2 期间禁止并行开族**（共享 `selfhost/parser.sico` / `selfhost/codegen.sico`） | 计划 §3.3.5 |
| D10 | 族降级后若 S 内该族仍有未覆盖调用点形状 ⇒ 不得降级，必须闭合 | 计划 §3.3.5 |
| D11 | 语料一致性工具面 = `tools/probe-m22-corpus-parity.ps1`（只读）+ 冻结期望 `selfhost/corpus-parity-v0.json` | 计划 §3.3.7 |
| D12 | S 喂入：单块 ≤ 64 KiB / 总量 ≤ 2 MiB / 超限 fail-closed；`A` 产出固化为独立脚本 + SHA | 计划 §3.3.8 / §3.3.9 |
| D13 | 无 fuel 实际耗量读数出口时 ⇒ 登记为**上界**并显式标注，**不阻塞 GO** | 计划 §3.3.9 / §3.3.10 |
| D14 | `origin/dev` **不 rebase 不 merge**；M22 完成判据不含与 `origin/dev` 合并；冲突历史转 M25 集成议题 | 计划 §3.3.10 |
| D15 | 门 7 之前必须产出**支持面登记表**（族 × 子集边界 × 语料覆盖 × 未覆盖形状） | 计划 §3.3.5 / §3.3.10 |

**4.3 文档差异 D1–D3 的裁决结果**：

| 编号 | 差异 | 裁决 |
|---|---|---|
| D1 | ADR-0016 决策 6 与 STEP-0486 §2.4 的 canary 次序不一致 | **✅ 关闭（ADR-0016 A1）**：组件级次序冻结为 `formatter 组件级 → 37 语料组件级 → bundle 组件级 → harness`；STEP-0486 §2.4 已加对齐注记；**阻断纪律不变** |
| D2 | 2026-09-28 审计记 formatter 29/30；2026-10-20 审计记 30/30 CLOSED | **✅ 关闭**：取更晚且有证据的 30/30（IR 面）；组件面未闭合归 W3-a |
| D3 | 整文件自举未关闭（`codegen.sico` 的 `List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`） | **✅ 关闭**：定为**固定点的必要条件**，写入 W5 第 3 项，不得降级为可选项 |

## 5. 诚实性

- 本片**没有**新增任何 guest 能力、**没有**推进任何出口门、**没有**产生新的运行时或性能主张；M22 仍为 **NO-GO**（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO，2026-10-20 出口审计口径）。
- 本片**触及了一处合同**：[ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) 把决策 6 的组件级 canary 次序按"先广度后尺度"重排（owner 2026-10-22 授权）。该修订**只改次序**，不改任何出口门、不改 `A == B == C`、不改证据等级、不改支持面主张；STEP-0486 §2.4 的原表述**保留**并加对齐注记以便追溯。
- 阶梯里的规模数字（75–125 片）是**规划估计，不是承诺**；三处最大不确定源已在计划 §3.3.13 逐条列明（W1 可能改写 W2 的实施方式；W3-b 的脚本 ABI 组宽度未知；W4 的 116 条 LEXICAL 可能是成片系统性偏差）。
- 本片**不预留任何 STEP 号**：阶梯只用 slice 标识（W1–W7 / T1–T20 / G1–G2 / R1–R3），执行会话按 §3.2 的取号规则在**开工时**顺取（取号前核对 `docs/steps/README.md` 与 `handoff-m22.md`；M24 占 STEP-0330–0337、M23 已占 STEP-0489/0490）。
- 本片编号 **STEP-0514** 为规划片自身；执行线下一片从 **STEP-0515** 起顺取（若并行会话已占用 0514/0515，按本线既有先例顺延并在 handoff 登记）。
- 全部证据等级 `internal-fixture`；`contract-verified` 与 runtime 支持继续分离。

## 6. 验证记录（2026-10-22）

| 验证 | 命令 | 结果 |
|---|---|---|
| 规划合同 | `.\tools\validate-step-0124.ps1` | `STEP_0124_OK`（计划保留 `Entry gate` / `Non-goals` / `Exit gates` 与 `no STEP numbers reserved`；门 5 ADR-0015 行未动） |
| 空白纪律 | `git diff --check` | 净 |
| 合同修订 | [ADR-0016 A1](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) + [STEP-0486 §2.4 对齐注记](../../steps/STEP-0486-m22-w2-s5-codegen-wave.md) | 决策 6 组件级 canary 次序 = `formatter 组件级 → 37 语料组件级 → bundle 组件级 → harness`；原句保留可追溯 |
| 组件分段基线 | 解析 `target/tmp/step0506/compiler.component.wasm` | 1,024,613 B；core 1,024,161 / code 1,003,250 / data 16,487 / type 3,685 |
| 内建族统计 | `selfhost/*.sico` 全文 `sico.<ns>.<fn>` 统计 | 20 族，`text.concat` 2,882 次最多 |
| 语料分布 | `selfhost/script-build-corpus-v0.json` | 37 accepted / 178 refused；LEXICAL 116 / SCRIPT_ABI 28 / 散码 34 |

未运行：全工作区 `cargo test` / `clippy` / `fmt`（本片为纯文档变更，不触及编译单元）；独立 CI（**未提交未推送**）。
