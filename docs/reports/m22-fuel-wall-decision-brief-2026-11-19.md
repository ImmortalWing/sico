# M22 燃料墙处置决策简报（供 owner 裁决）

- 日期：2026-11-19
- 范围：`parser.sico` 大闭包的组件轴发射在 800e9 标准燃料预算下耗尽（燃料墙），如何处置
- 状态：**未翻任何 M22 出口门；M22 overall 仍 NO-GO**；证据级别 = internal-fixture
- 触发：owner 本轮指令「先规划尝试全新方法，如果不行就先上调限额」（2026-11-19），与 AGENTS §10.1.4 的 D-B「不上调限额」裁决（2026-11-08）存在先后冲突；本文以实测数据支撑裁决，不凭旧裁决直接否掉，也不凭新授权直接上调。

## 1. 问题口径

站点账本 `selfhost/match-component-coverage-v1.json`（406 站点）当前直方图：

| verdict | 站点数 | 含义 |
|---|---|---|
| `ir-equal+component-other:FuelExhausted` | 311 | 前端 IR 全等，发射端在 800e9 内耗尽（**未闭合**） |
| `ir-equal+component-equal` | 67 | 组件逐字节等同（**真闭合**） |
| `closure-too-large:165fn/1049726B` | 25 | 闭包超出 fixture 上限，**从未被测量** |
| `ir-equal+component-refused-guest` | 3 | 发射端 typed 拒绝（具名残差） |

311 + 25 = 336 个未闭合站点，**全部在 `parser.sico`**。

## 2. 「全新方法」侦察结论：无可行候选

- 燃料墙是**发射器算法属性，不是测量方法论 artifact**：前端对每个耗尽站点都在 800e9 内完成（IR 轴全等），只有发射阶段超预算。
- 超线性项已命名定位：`cgs_dispatcher_slot` 逐指令全 body 重扫 → 每函数 O(I³)，STEP-0618 已砍到 O(I²)；剩余二次项集中在**最大函数的指令数**上（fuel ~ IR²）。
- 逐一评估的候选新方法皆不可用：
  - 合成 per-site shape fixture：基础设施存在，但 tiny fixture 复现不了真实站点的燃料需求（成本按函数指令数，flip 对闭包尺寸非单调）。
  - per-region / 增量发射：不支持；超线性项在单个大函数**内部**，按函数/区域切分仍在大函数上耗尽。
  - 前端便宜切片：前端已证非墙（IR 轴全在预算内完成）。

⇒ 按 owner 两段式授权，落入「先上调限额」这一段。但上调是否值得，取决于 4× 实测产率（见 §3）。

## 3. 上调限额的实测产率（决策关键数据）

历史实测：
- **STEP-0600**：2×（1.6T）全量 331 站点只 flip **5/331 = 1.51%**。
- **STEP-0606**：二次增长下「限额翻倍只多解锁一步 n」⇒ 上调限额买不回站点。
- **STEP-0540**：一个大闭包在 2e12（2.5×）下 67.6s 跑完 ⇒ 发射是 fuel-bound 而非 timeout-bound，故 4× 下很多站点能在 300s 探针超时内跑完 ⇒ 4× 产率可能显著高于 2× 的 1.51%。

本轮新增实测（4× = 3.2e12，走账本组件轴同路径 `guest_emission_with_fuel` 单跑）：
- `scalar_ir`：**无法测量** —— IR 9.1 MB 超 stdin 通道（见 §4），flip=0/0（不可注入）。
- **首个实测数据点（4× = 3.2e12）**：`while_call_rhs_packed`（IR 2.6 MB）在 4× 下**仍然 exhausted**（349 s 跑满）⇒ 连非 `scalar_ir` 的大函数在 4× 下也翻不过来。这直接坐实「上调燃料买不回站点」。
- **第二个实测数据点（4×）**：`gw_compare_packed`（IR 2.9 MB）在 4× 下**仍然 exhausted**（345 s）⇒ 2/2 大函数在 4× 下都翻不过来，进一步坐实上调无效。
- 其余 12 组：测量运行中（每函数 ~345–350s，预计全部 exhausted）；**但结论不依赖它们**，因为即使这 14 个较小函数的 4× 产率高，也覆盖不到主导阻塞 `scalar_ir`（占 25 closure-too-large + 最大燃料热点），且 D-B/STEP-0606 已证二次缩放下上调是差杠杆。数字落地后补记，仅供精度参考。

## 4. 结构性发现：`scalar_ir` 是单一咽喉点

- **311 个燃料耗尽全在 `parser.sico`**，跨 51 个函数。
- **25 个 `closure-too-large` 全在同一个函数 `parser.sico:scalar_ir`**（165fn / 1,049,726B，超出 1MiB 闭包 fixture 上限 1,150B）。
- `scalar_ir` 正是历史登记里 331/335 燃料耗尽热点的所在函数。

⇒ 上调燃料对 `scalar_ir` 这 25 站**根本无效**（它们卡在闭包构建的 fixture 字节上限，还没到燃料测量就跳过）；对其余 311 站取决于 §3 的 4× 产率。

**本轮新增实测（更硬的墙）**：`scalar_ir` 的 canonical IR 文本 = **9,101,116 B（9.1 MB）**，超过 guest stdin 注入通道容量 ⇒ `guest_emission_with_fuel` 直接 panic `the injected IR bounds hold: ChannelBytes(9101116)`，**根本无法注入测量**。这与历史登记的「`parser.sico:scalar_ir` 24 站受 runner 8 MiB stdout 上限阻挡（不得直接上调）」一致。⇒ 对 `scalar_ir`，墙在**燃料之前**：既超闭包 fixture 字节上限（§4 上文），IR 又超注入通道，**上调燃料完全无效**，唯有选项 B（IR-only / not-comparable）或把 `scalar_ir` 拆小（选项 C 的具体形态）可行。

## 5. 附带查出的当前**红**门 7 回归（正交、具体）

`every_match_site_is_measured_on_the_component_axis` 现在 **FAILED**：
- 断言「闭包 fixture 上限已覆盖最大真实闭包（164fn/1,027,015B），任何站点都不得以 `closure-too-large` 跳过」。
- 实际 `scalar_ir` 已长到 **165fn/1,049,726B > 1MiB 上限**，导致 MS-375..399 共 25 站变成 fixture 跳过 ⇒ 断言失败。
- 这是真实回归。**但修法比预想更受约束（本轮订正）**：不能简单「抬高 `MATCH_CLOSURE_BYTE_LIMIT` → 这 25 站变 `FuelExhausted`」。实测证明 `scalar_ir` 的 IR 9.1 MB **超 stdin 注入通道**（`ChannelBytes(9101116)` panic），抬闭包上限只会把失败从 `closure-too-large` 位移成**注入 panic**，不会得到 `FuelExhausted`。
- ⇒ 这 25 站在组件轴上**本质上不可测量**（IR 太大注入不了），诚实归类只能是**具名 `not-comparable`**（比 `not-comparable-at-budget` 更强的 `not-comparable-at-channel`）。⇒ 门 7 回归的修复**依赖选项 B 的语义裁决**（引入具名 not-comparable verdict + 放宽 `every_match_site_is_measured` 接受它），**不是独立小修**。故与燃料决策一并裁决。

## 6. 三个真实选项（待 owner 裁决）

- **选项 A：上调燃料限额**（owner 本轮授权的第二段）。
  - 代价：破坏「标准预算 800e9」语义；D-B 曾正式否决；产率待 §3 实测确认。
  - 若 §3 的 4× 产率高（如 >30%），值得；若仍低（<10%），基本无效。
  - 对 `scalar_ir` 的 25 站无效（§4）。
- **选项 B：接受 IR-only 收窄 gate-2 组件轴**。
  - 对 311 + 25 站，组件轴标 `not-comparable-at-budget`（有 IR witness，无组件判定），与既有 `closure-too-large` 先例一致。
  - 这是**范围收窄**，不是燃料修复；诚实但需 owner 批准收窄 gate-2 语义。
- **选项 C：继续啃硬算法**——让大函数发射变亚二次。
  - 无现成候选；STEP-0636 已摘尽低垂果实（formatter 486.2e9→127.07e9，−73.9%）。
  - 高风险、长周期，但唯一能真正闭合而不收窄/不抬预算的路。

## 0. 裁决与执行结果（2026-11-19 owner）

- **Owner 裁决**：「接受，不等」——接受本简报建议（不上调燃料 → 选项 B 收窄 + 随 B 修门 7 + 选项 C 立项），不等剩余 12 组 4× 数字。
- **已执行（门 7 修复，选项 B 落地）**：`every_match_site_is_measured_on_the_component_axis` 从「任何站点都不得 `closure-too-large`」**重定范围为**「每个站点要么组件轴已测量，要么是**具名、whole-file IR pin 背书**的 not-comparable 残差——无 pin 的 `closure-too-large` 视为静默 fixture skip 并失败」。25 个 `scalar_ir` 站点（IR 9.1 MB 超 stdin 通道，组件轴不可测量，IR 由 whole-file pin `sico_guest_arena_accommodates_full_parser_selfhost_input` 见证）据此被诚实接纳。**该测试现 GREEN**；clippy 干净；测量路径（`sico_compiler_reproduces_the_frozen_match_site_ledger`）未改动、不受影响。
- **选项 C（拆小 `scalar_ir`）登记为后续真杠杆**：把巨型 `scalar_ir`（165 fn）拆成多个小函数以降单函数指令数、从根上降 O(I²) 发射成本——唯一既不抬预算又不收窄语义的闭合路径，属语言/前端重构，需单独立项（未开工）。
- **未 commit**：改动待 owner 明确授权提交（`selfhost_compiler.rs` 的门 7 重定范围 + 两个 `#[ignore]` 测量钩 + 本简报）。

## 7. 建议（基于结构证据，不依赖待测的 14 组数字）

**不采纳选项 A（上调燃料）作为主路径。** 理由（三条，均不依赖 14 组待测数字）：
1. **主导阻塞 `scalar_ir` 上调无效**：25 个 closure-too-large 卡在闭包 fixture 字节上限，且其 IR 9.1 MB 超 stdin 注入通道 ⇒ 在燃料之前就不可测量/不可发射（§4）。上调燃料对它零效果。
2. **历史+理论证伪上调杠杆**：STEP-0600 实测 2× 仅 1.51% flip；STEP-0606 证二次缩放下「翻倍只多解锁一步 n」。4× 即便对较小函数产率略高，也只覆盖 311 里的少数（14 组代表的多为中小函数），对 `scalar_ir` 族无效。
3. **破坏标准预算语义**：上调即放弃「800e9 标准预算」口径，与 D-B（2026-11-08 正式否决上调）一致；owner 本轮授权的是「全新方法不行就上调」，但数据表明上调**买不回主导站点**，属「不行的上调」，故如实回报、不建议采纳。

**建议路径：选项 B（IR-only / not-comparable 收窄 gate-2 组件轴）为主，选项 C 登记为后续真杠杆。**
- 对 `scalar_ir` 族（25 closure-too-large + 其燃料耗尽站点）：组件轴标 `not-comparable-at-budget`（有 IR witness，无组件判定），与既有 `closure-too-large` 先例一致。诚实、可复现、不虚增。
- **修 §5 门 7 回归**（与选项 B 绑定，非独立）：因 `scalar_ir` IR 9.1 MB 不可注入，抬闭包上限得不到 `FuelExhausted`；须随选项 B 引入具名 `not-comparable` verdict 并放宽 `every_match_site_is_measured` 接受它，才能既诚实又让该测试转绿。
- **选项 C（拆小 `scalar_ir`）登记为真正能闭合（而非收窄）的后续投入**：把巨型 `scalar_ir` 拆成多个小函数，降低单函数指令数，从根上降 O(I²) 发射成本。这是唯一既不抬预算、又不收窄语义的闭合路径，但是语言/前端重构，工作量与风险大，需 owner 单独立项。

**请 owner 裁决**：① 是否接受「不上调燃料、改走选项 B 收窄 + 先修门 7 回归」；② 是否为选项 C（拆 `scalar_ir`）单独立项；③ 14 组 4× 产率数字落地后是否需要据此微调（默认不需要，结论已由结构证据锁定）。