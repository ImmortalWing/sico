# M22 S7 出口审计草案（2026-11-06）

> **总裁决：M22 整体仍为 NO-GO。** 本文件是 **S7 出口审计的草案**（STEP-0569，纯文档片），不是权威裁决；权威裁决仍是 [`m22-exit-audit-2026-10-25.md`](./m22-exit-audit-2026-10-25.md)。
> 本草案**不翻任何门**、不修改七门文本与证据等级、不做完成声明。全程只引用已执行的证据（STEP 文件与 STATUS 记载的实测）；不确定处标「**未核实**」。

- 分支 / 树：`codex/m22-w2` @ `a939862`，本片开工时工作树含 1 个非本片的既有脏文件（`selfhost/match-component-coverage-v1.json`，4 处 verdict 由 `component-equal` 改 `FuelExhausted`，**非本片改动，本片不碰**）。
- 证据等级：`internal-fixture`（真实 runner 执行，语料为仓库内冻结集）⇒ 不是 `clean-room-consumer`、不是 `external-pilot`、不是 `production`。
- 门定义权威来源：[M22 计划 §4](../plans/M22-compiler-self-host.md) 与 AGENTS.md §10.9。

## 0. 本草案与既有审计的关系（口径纪律）

| 项 | 权威审计 | 本草案 |
|---|---|---|
| 文件 | [`m22-exit-audit-2026-10-25.md`](./m22-exit-audit-2026-10-25.md) | 本文件（**草案**） |
| 总裁决 | NO-GO | **NO-GO**（不变） |
| 覆盖范围 | 截至 STEP-0543（2026-10-25） | 追加 STEP-0544–0561 的已执行证据（2026-11-05/06），**不覆盖未提交的 STEP-0560 后续收口与 STEP-0566** |

两条纪律：

1. **不改历史结论**：10-25 审计原文保留；本草案是**叠加 W2 最新实测的新草案**，凡与 10-25 冲突处，**以更严格的裁决为准**并显式说明。
2. **上限纪律**：AGENTS.md §10.1 写明「门 4 在 `B(S)` 实测前不得沿用 GO，门 7 在证据问题修复前不得给完成信用」。本草案按该口径执行。**本草案进一步把门 6 的 GO 也标为「不可沿用（已失效）」**，理由见 §3 门 6 —— 这是比 10-25 更严的裁决，因为门 6 的锚点 run 无法覆盖其后的实现改动，且当前活动线的独立 CI 触发面在 STEP-0566 之前是断的。

## 1. 证据矩阵（门 × 当前裁决 × 证据指针 × 剩余缺口）

| 门 | 名称 | 当前裁决 | 证据指针（已执行） | 剩余缺口 |
|---|---|---|---|---|
| **1** | L1 formatter 差分 + checker 声明子集 | **GO（限声明子集）** | [10-25 审计 §1 门1](./m22-exit-audit-2026-10-25.md)：formatter canary 30/30、`full_source_exit: 0`、`FORMATTER-CLOSED`（`tools/report-m22-canary.ps1`）；checker 语料电池 `tools/validate-step-0245.ps1` exit 0（bootstrap_bundle 6/6、selfhost_checker 11/11） | 拓宽声明子集需独立的 declared-contract 更新；10-25 后 W2 未触碰 L1 ⇒ 维持 |
| **2** | L2 冻结语料 / 组件级字节全等 | **PARTIAL** | 组件轴 **36 / 396** 逐站全等（[STATUS 2026-11-05 STEP-0558 条](../STATUS.md)、[STEP-0551](../steps/STEP-0551-m22-w2-b2-value-slot-index.md)）；残差 **331 `FuelExhausted` + 24 8MiB trap + 5 refused-guest**；W4 拒绝语料 verdict parity **147 / 150**（3 燃料墙，[STATUS 2026-11-05 STEP-0558 条](../STATUS.md)、[STEP-0561](../steps/STEP-0561-m22-w2-w4-verdict-parity.md)） | 见 §3 门 2；fuel 墙根因未定位、精确 (id, span) parity 未达、37 accepted / 178 refused 的组件级/三元组差分未做、bundle 未做 |
| **3** | ADR-0015 `A == B == C` + M7 `.sapp` 信任链 | **NO-GO** | [10-25 审计 §1 门3](./m22-exit-audit-2026-10-25.md)：guest 尚未产出自编译组件 B；三阶段字节证据、`.sapp` 打包、`sico-app` 执行自编译、hostile manifest / limit+1 均未演示 | 整条自举闭环未进入；fuel 墙是 B(S) 的前置硬阻塞（见 §2） |
| **4** | 自编译预算（`B(S)` 完整记录） | **NO-GO（不得沿用 GO）** | [10-25 审计 §3](./m22-exit-audit-2026-10-25.md)：现有只有 **IR 层**观测（全源 parser.sico 降低 25.93 s；fuel 800e9 为配置上限非耗量读数）；`B(S)` 未实测 ⇒ cold/warm wall、fuel 读数、峰值内存、hostcall 计数、timeout/cancellation、M19 对照**全部缺** | `B(S)` 完整预算记录 + M19 吞吐对照；A 路装配已测（[STEP-0516](../steps/STEP-0516-m22-w2-s5-w1-scale-feasibility.md)、[STEP-0548](../steps/STEP-0548-m22-w2-p0-assembly-scale-precheck.md)）但**只覆盖 A 路不覆盖 B(S)** |
| **5** | 双实现登记 | **GO** | [10-25 审计 §1 门5](./m22-exit-audit-2026-10-25.md)：Rust 编译器保持**唯一 byte oracle**（`crates/sico-codegen-wasm` 未被 W2 改动）；Sico 自举为第二实现；证据车 = census 闭包对拍 + 全源差分 + canary + 固件钉 | 无（两实现均未被删除或降格） |
| **6** | M0–M21 全量回归 + 独立 CI | **GO 不可沿用（已失效）** | 旧锚点 = 独立 CI **run #52**（`head_sha 9f0b7a3`，`completed/success`，57m34s；[10-25 审计 §2](./m22-exit-audit-2026-10-25.md)）；**触发面缺口** = 该 workflow 的 `push` 只挂 `codex/m22-w1-ci`，该分支已从远端删除（`git ls-remote` 实测，[STEP-0566 §2.1](../steps/STEP-0566-m22-ci-workflow-trigger.md)） | 见 §3 门 6 —— 当前活动线 `codex/m22-w2` 在 STEP-0544–0561 全部实现改动上**无任何独立 CI run**；STEP-0566 已修触发面但**未提交未推送 ⇒ 当前无新 run** |
| **7** | 声明 ↔ 执行证据映射 | **不得给完成信用（NO-GO）** | [10-25 审计 §1 门7](./m22-exit-audit-2026-10-25.md) + AGENTS.md §10.11 未闭合项：语料覆盖 0 条、整文件自举未关闭、`M-g3′`/`M-g2`/`M-g3` 未闭合、W4 精确 parity 未达、**4 个既有红**（[STEP-0559](../steps/STEP-0559-m22-w2-callshape-u64-list-get-frontend-gap.md)） | 全部 §10.11 未闭合项关闭 + 支持面登记表（D15）覆盖全族后才可给信用 |

**总裁决：NO-GO。** 门 1/5 GO；门 2 PARTIAL；门 3 NO-GO；门 4 NO-GO；门 6 GO 已失效；门 7 不得给完成信用。

## 2. 贯穿性事实：fuel 墙根因未定位（同时压门 2 / 门 3 / 门 4）

最近三片实测**证伪**了「fuel 墙在文本累加 / 索引 / 游标重扫」三条假设，静态可读路径解释不了 2.4e9 fuel：

| 片 | 假设 | 实测结果 |
|---|---|---|
| [STEP-0555](../steps/STEP-0555-m22-w2-b3-cursor-and-module-index.md)（B3 游标化 + 模块索引） | 游标化省 fuel ⇒ 更多站点回绿 | **证伪**：`FuelExhausted` 331 → 344（+13）、component-equal 36 → 23（−13）；0 站点回绿、13 站点回退 ⇒ 已回退 |
| [STEP-0556](../steps/STEP-0556-m22-w2-b4-c-linear-assembly-negative.md)（B4-C 线性装配） | 循环内文本累加拼接是主要 fuel 墙 | **证伪**：三处累加器改 `list.append` + 一次 `join`，字节等价 28/28 全过，但四臂 fuel 逐档不变 ⇒ 已回退 |
| [STATUS 2026-11-05 B4-C 复核条](../STATUS.md)（原语 O 度 + 穷举静态路径分析） | `cgj_member` 字段重扫 / core-helper O(n²) | **证伪**：`sico.text.join` 线性、`list.append` 摊还 O(1)、`bytes.length`/`at` O(1)；模块级重扫合计 ≈ 81K ops = 2.4e9 的 0.01% ⇒ 静态可读路径无法解释 2.4e9（≈1.2e6 ops/输出字节） |

**判读**：fuel 墙**不在** guest 发射器的 IR 走查算法，也**不在**文本累加。下一刀需仪器化 profile（wasmtime epoch/opcode）或结构预索引。**在 fuel 墙定位并修复前**：331 个 `FuelExhausted` 站点无法回绿（门 2 卡在 36/396）、guest 无法在 800e9 主 fuel 预算内产出整文件组件 B（门 3 前置硬阻塞）、`B(S)` 无法实测（门 4）。**这三门的卡点是同一堵墙。**

## 3. 逐门剩余缺口（通往 GO 的有限清单）

### 门 2（PARTIAL）

1. **fuel 墙根因未定位**（§2）⇒ 331 `FuelExhausted` + 24 8MiB trap 无法回绿；组件轴停在 36/396。
2. **W4 精确 (id, span) parity 未达**：guest 的 44 个 `E-SH-IR-*` 码全在 `parser.sico`，与 Rust `LEXICAL`/`E-code` 命名空间不相交 ⇒ 需诊断映射契约（[STATUS 2026-11-05 STEP-0561 条](../STATUS.md)）。28 `SCRIPT_ABI` 明确排除（`check=accepted` / `script_build=refused`，guest 前端够不到 ABI 检查）。
3. **3 个燃料墙拒绝项**（`scope-nesting-limit.sico` ×3，FuelExhausted exit 125）与 verdict 正交但未闭合。
4. **组件级差分未做**：37 条 accepted 语料、178 条 refused 的 `(verdict, id, span)` 三元组逐条一致（W3-b / W4）、5 文件 bundle 与 9 文件全集（W5）均未做；**语料覆盖仍 0 条**。
5. **发射词表**：STEP-0560 补掉 STEP-0542 漏掉的第 7 项集合 helper `sico.list.length[U64]`，但**组件轴逐站差分未跑** ⇒ 那 7 个站点是否真闭合**未核实**（发射能力 ≠ 站点闭合，[STATUS 2026-11-06 STEP-0560 条](../STATUS.md)）。

### 门 6（GO 已失效 —— 本草案相对 10-25 的主要收紧）

- **旧锚点 run #52 覆盖不到当前实现**：run #52 的 SHA `9f0b7a3` 早于 STEP-0544–0561 的全部 W2 改动（fuel 归因、B3/B4 三刀、发射词表补刀、W4 parity、声音洞修复）。一个证明的是「10-25 时点的干净提交回归绿」，**不构成**「当前候选回归绿」。
- **触发面在 STEP-0566 之前是断的**：[STEP-0566 §2.1](../steps/STEP-0566-m22-ci-workflow-trigger.md) 以 `git ls-remote` 实测证明远端已无 `codex/m22-w1-ci`，而 workflow 的 `push` 只挂该分支 ⇒ 活动线 `codex/m22-w2` 与 `dev` 的 push **均不产生 CI run** ⇒ STEP-0544–0561 期间**没有任何 exact-SHA 独立 CI 可取**。
- **STEP-0566 的修复尚未生效**：触发面已改为含 `codex/m22-w2` + `workflow_dispatch`，但该改动**未提交未推送** ⇒ **当前树没有任何 SHA 上的独立 CI run**。
- **剩余缺口**：在干净、唯一、已提交的候选 SHA 上重跑 `tools/run-ci.ps1` + runner 全工作区 + 版本化 M22 电池 + 全部站点/语料探针 + 模块边界检查；取得该 SHA 的独立 `completed/success`（run id + head_sha + 日志）后才可重建门 6 的 GO。**本草案无法代替这一步。**

### 门 3 / 门 4（NO-GO）

- 门 3：`A = R(S)`、`B = run(A,S)`、`C = run(B,S)` 三者组件校验且 `A == B == C`；M7 `.sapp` canonical 打包、显式本地信任、发布演练、`sico-app` 自编译、hostile manifest / 未知字段 / 重复路径 / 穿越 / 截断 / 尾随 / 溢出 / limit+1 全部 fail-closed —— **全部未进入**（[10-25 审计 §1 门3](./m22-exit-audit-2026-10-25.md)）。
- 门 4：`B(S)` 完整预算记录（cold/warm wall、fuel 耗量读数、峰值 guest 内存、hostcall 计数、I/O 字节、timeout/cancellation、M19 吞吐对照）—— **全部缺**。IR-only 的 25.93 s 不是 `B(S)` 的证据（[10-25 审计 §3](./m22-exit-audit-2026-10-25.md)）。fuel 墙（§2）是 `B(S)` 的前置硬阻塞。

### 门 7（不得给完成信用）

- AGENTS.md §10.11 的未闭合项逐项关闭后才可给信用：**4 个既有红**（`step0507/0519/0520/0526`，[STEP-0559](../steps/STEP-0559-m22-w2-callshape-u64-list-get-frontend-gap.md) 定性为 guest 前端能力缺口 `List[U64]` Result-match 三层堵死，非 codegen 语法重塑可修，归 M14/M15）、语料覆盖 0 条、整文件自举未关闭、`M-g3′`(15)/`M-g1`(354)/`M-g2`(46)/`M-g3`(21) 站点未闭合、W4 精确 parity 未达。
- **支持面登记表（D15）**需在 GO 前覆盖全部族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）。

## 4. 本草案不主张什么（诚实收窄）

- 本草案**不**主张 M22 完成；**不**主张门 2 / 3 / 4 / 7 达成；**不**重建门 6 的 GO。
- 门 1 / 门 5 的 GO **沿用 10-25 审计**，本草案**未重跑** `tools/report-m22-canary.ps1` 与 `tools/validate-step-0245.ps1` ⇒ 对这两门只做指针引用，**未独立复验**（诚实标注）。
- 门 2 的 36/396、331/24/5 残差、W4 的 147/150 均引自 STATUS 与 STEP 的**已执行**记录，本草案未重跑 396 站点 ledger（历史全量 ≈6114 s）⇒ **未独立复测组件轴**。
- 组件轴的 331 个 `FuelExhausted` 站点**不是**闭合，只是「从被拒推进到燃料耗尽」；24 个 8MiB trap 是**未闭合**（runner 结果上限，与 fuel 正交，不得直接上调上限）；5 个 refused-guest 是**未闭合**。
- W4 的 147/150 只覆盖**verdict 半边的 check 轴**，**不覆盖**精确 (id, span) parity，也**不覆盖** 28 条 `SCRIPT_ABI`（明确排除）。
- 门 6 的「已失效」是**本草案基于 STEP-0566 实测的推断性收紧**，不是 owner 已裁决的翻门；权威裁决仍以 owner 复审为准。
- 本草案对 3 个燃料墙拒绝项只登记现象（exit 125 / FuelExhausted），**未做归因**。

**证据等级**：`internal-fixture`（真实 runner 执行，但语料为仓库内冻结集）⇒ **不是** `clean-room-consumer`、**不是** `external-pilot`、**不是** `production`。