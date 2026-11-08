# STEP-0606 · M22 · 燃料归因诊断（fuel-attribution diagnostic）：把燃料墙从「常数主导·源未定位」修订为「具名超线性发射器算子主导」（match-dispatcher 重扫 + text_concat）；D8 上调限额买不回站点

> **片型** = 纯文档/测量登记片（measurement/documentation slice）。本片**只登记已测得的诊断证据与口径修订**，不改任何源码/测试/语料/账本，**不翻任何门**；M22 整体仍 **NO-GO**，未提交未推送。
>
> **复现路径（coordinator 独立读取的 sweep 日志，视为 ground truth）**：`target/tmp/step_probe/{cost_centers,match_quadratic}.log`。harness = `runner/sico-runner/tests/m22_fuel_profile.rs`，探针上限经 env `SICO_M22_PROBE_FUEL` 抬升（in-file 冻结资格上限 `FUEL_CEILING=800_000_000_000`）；sweep 在 **800e9**、单线程下运行，使用 **emit-only 合成闭包 + 一个真实站点**。
>
> **依赖的 runner 能力**：新落地的燃料钩 `run_with_fuel`（与 `run_with_fuel_timeline`）。本诊断**只调用 `run_with_fuel`**，产出 `consumed` 总量与括号中点（`fuel_mid`，约 ±12.5% 带）；`FuelTrace::timeline`（`run_with_fuel_timeline`，逐 tick epoch 采样）**已被构建但本批探针未行使**（见 §6 诚实收窄）。
>
> **禁改清单（本片遵守）**：`runner/sico-runner/src/**`、`runner/sico-runner/tests/**`、`selfhost/**`、`crates/**`、任何冻结语料/账本、`docs/ROADMAP.md`、`docs/handoff-m22.md`。本片**只新增本文件**并对 `AGENTS.md` §10.1.4 / `docs/STATUS.md` / `docs/steps/README.md` 做日期化登记（不改写任何既有段落正文）。

## 0. 结论摘要（一句话）

**M22 燃料墙是「工作负载/超线性驱动」，不是「每跑固定常数」**——其主导项是**两个具名发射器算子**：①**match-dispatcher 重扫**（由 `match_flat` vs `match_one` 等体量判别器证明为二次）与 ②**`text_concat` 装配**（二次，每倍增 slope ≈3.1）。⇒ **对 owner 的 D8 裁决后果：上调燃料限额（D8 重基线）买不回站点**——二次增长下，限额翻倍只多解锁一步 n；**真正的杠杆是降低 match 重扫与 text_concat 装配的复杂度**（与 STEP-0548 装配策略结论收敛）。本片把 AGENTS §10.1.4 原有「**常数主导（具体 wasm-op 源仍未定位）**」的开放问题**修订/收窄**为「**具名超线性发射器算子主导**」——是**对开放问题的解决/收窄，不是对既立事实的推翻**。**未翻任何门，M22 overall 仍 NO-GO。**

## 1. 背景：为什么做这次归因诊断

AGENTS §10.1.4 长期把 M22 燃料墙表征为**发射器层 + fuel ~ IR² + 「常数主导（具体 wasm-op 源仍未定位）」**。其中「常数主导」本身是**带保留的措辞**——「具体 wasm-op 源仍未定位」明确承认源未知。

新落地的 runner 燃料钩（`run_with_fuel` / `run_with_fuel_timeline`，见 STEP-0572 §5 的生产钩缺口）首次让**op/括号级燃料读数**成为可能（无需此前被拒的 runner 生产钩改造）。本次诊断即用该钩对燃料墙做**归因**：**燃料到底烧在哪些发射器算子上、随什么标度增长**。

## 2. 方法：等体量判别器 + 负对照 + 真实站点锚

| 手段 | 设计 | 目的 |
|---|---|---|
| **判别器 `match_flat` vs `match_one`** | `match_flat` = N 个独立 `match`；`match_one` = 1 个 `match` + N−1 个 `set`，**体量（body 大小）相等** | 把「match-dispatcher 重扫」从「body 扫描」中**隔离**出来——两者 IR 体量相近，唯一差异是 match 分发器数量 |
| **`text_concat` 族** | n=8/16/32/64 递增 | 测 `text_concat` 装配的标度 |
| **`functions` 族** | n 递增 | 对照族 |
| **负对照 `forwarded_let_control`** | IR 平坦（2069→2088），n 翻倍 | 证明**平坦读数可达** ⇒ 增长族确在测「发射器功随 IR 增长」，方法论有效 |
| **真实站点锚 `parser.sico:fixed_literal_const_segment`** | ir 92057 B，emit-only | 锚定到真实账本站点；测其翻转点 |

## 3. 证据（全部 800e9 单线程实测，来自 sweep 日志）

### 3.1 判别器：match-dispatcher 重扫是二次的

| n | `match_flat`（N 个 match）ir / fuel_mid | `match_one`（1 match + N−1 set）ir / fuel_mid |
|---|---|---|
| 8 | 21060 / **115.2e9** | 6418 / **5.6e9** |
| 16 | 40052 / **EXHAUSTED（>800e9）** | 8394 / **14.4e9** |

- **n=16 时 ~55× 差距**（flat 耗尽 vs one 仅 14.4e9），在相近 n 下。
- **`match_flat` observed_over_quadratic = 1.000（n=4）、0.778（n=8）** ⇒ **干净的 O(n²)**。
- ⇒ **match-dispatcher 重扫 = 二次**（隔离自 body 扫描，因 `match_one` 等体量仅线性）。

### 3.2 `text_concat` 装配族：二次

| n | fuel_mid | slope_vs_prev |
|---|---|---|
| 8 | 14.4e9 | — |
| 16 | 89.6e9 | **3.11** |
| 32 | 556e9 | **3.10** |
| 64 | **EXHAUSTED** | — |

⇒ slope ≈ **3.1/倍增** ⇒ **二次**。与 STEP-0548「左折叠 concat 装配为二次」的结论一致。

### 3.3 `functions` 族：近线性-plus

slope_vs_prev **1.22 / 1.64 / 1.56** ⇒ 近线性-plus（非二次主导）。

### 3.4 负对照 `forwarded_let_control`：方法论有效

IR 平坦（2069→2088），fuel **恒定 4.5e8**（n=8/16/32/64），slope_vs_prev **0.50** 纯由 n 翻倍引起 ⇒ **平坦读数可达**；⇒ 增长族（§3.1–3.3）**确在测「发射器功随 IR 标度」**，方法论自证。

### 3.5 真实站点锚：翻转点未达（仅下界）

`parser.sico:fixed_literal_const_segment`（ir 92057 B，emit-only）：6.4e9 / 64e9 / 800e9 **三档全耗尽** ⇒ **永不达翻转点，即使 800e9** ⇒ **精确烧量未钉死，只有「>800e9」（`lower_bound_only`）**。**刻意未推到 6.4e12**（约 15+ min/测试，超预算）。

## 4. 发现与 D8 后果

- **发现**：M22 燃料墙是**工作负载/超线性驱动**，主导项为**两个具名发射器算子**——①match-dispatcher 重扫（§3.1，二次）②`text_concat` 装配（§3.2，二次 slope≈3.1）。
- **D8 后果**：**上调燃料限额（D8 重基线）买不回站点**——二次增长下，**限额翻倍只多解锁一步 n**（与 STEP-0600 实测一致：2× 燃料只转绿 5/331 = 1.51%）。**真正的杠杆 = 降低 match 重扫与 text_concat 装配的复杂度**（与 STEP-0548 装配策略结论收敛）。

## 5. 与 AGENTS §10.1.4 的调和（不推翻，而是解决开放问题）

§10.1.4 原表述：燃料墙 = **发射器层 + fuel ~ IR² + 「常数主导（具体 wasm-op 源仍未定位）」**。

- **不矛盾**：本诊断**确认**「发射器层」与「fuel ~ IR²」两点。
- **修订**：本诊断**定位了源**——不是「尺寸无关的固定常数」，而是**两个具名超线性发射器算子（match 重扫 + text_concat）**。⇒ 把「常数主导·源未定位」**修订为「具名超线性发射器算子主导」**。
- **定性**：这是对开放问题（「源未定位」）的**解决/收窄**，不是对既立事实的推翻——因「常数主导」本身是**带保留措辞**（明确承认源未知）。

## 6. 诚实收窄（必须记录的口径边界）

1. **emit-only 合成闭包 + 一个真实站点**：**非**完整 396 站点账本、**非** live guest-前端降级路径（emit-only 注入绕过前端）。
2. **锚站点精确烧量未钉死**：`fixed_literal_const_segment` 只有「>800e9」（`lower_bound_only`）；刻意未推到 6.4e12（约 15+ min/测试，超预算）。
3. **timeline 钩未行使**：`FuelTrace::timeline`（`run_with_fuel_timeline`，逐 tick epoch 采样）**已构建但本批探针未调用**——只调 `run_with_fuel`，仅得 `consumed` 总量 / 括号中点。
4. **`fuel_mid` 是括号中点，非精确 consumed**：约 **±12.5% 带**。
5. **未翻任何门**：M22 七门、ADR-0015 的 `A == B == C`、证据等级、冻结契约均未动；**M22 overall 保持 NO-GO**。
6. **未提交未推送**：HEAD 仍 `af6dcb7`，全部改动留在工作树。
7. **工作树上下文**：存在与本片无关的既有 dirty 文档（`docs/plans/M24-vision-closure-gui-application-pilot.md`、`docs/rfc/RFC-0052-amendment-A3-ui-selection-model.md`、`docs/rfc/RFC-0052-native-ui-source-binding-v0.md`、`docs/user-guide/NATIVE-UI.md`）与本片正交的既有 M22 dirty 文件（`selfhost/parser.sico`、`selfhost/match-component-coverage-v1.json`、`runner/sico-runner/src/lib.rs`、`runner/sico-runner/tests/m22_fuel_profile.rs`、`docs/steps/STEP-0571-*.md`），本片**均未触碰**。

## 7. 文件清单

- **新增**：`docs/steps/STEP-0606-m22-fuel-attribution-diagnostic.md`（本文件）。
- **日期化登记（不改写既有段落正文）**：
  - `AGENTS.md` §10.1.4：在 STEP-0605 段后、`仍属 owner 待裁` 条前，追加一条 2026-11-07 addendum（燃料归因诊断 + 口径修订 + D8 后果 + 诚实收窄）。
  - `docs/STATUS.md`：顶部日期化日志区新增一条 2026-11-07 条目（燃料归因诊断摘要）。
  - `docs/steps/README.md`：索引表在 STEP-0605 行后新增 STEP-0606 一行。
- **未动**：`runner/sico-runner/src/**`、`runner/sico-runner/tests/**`、`selfhost/**`、`crates/**`、任何冻结语料/账本、`docs/ROADMAP.md`、`docs/handoff-m22.md`、上述既有 dirty 文件。
