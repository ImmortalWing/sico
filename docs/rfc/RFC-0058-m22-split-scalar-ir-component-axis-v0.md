# RFC-0058: M22 拆小 `parser.sico:scalar_ir` 以闭合组件轴 v0

> - status: proposed
> - date: 2026-11-20
> - authors: autonomous-agent
> - target language/platform version: draft
> - supersedes: -
> - superseded-by: -

## Summary

把 `selfhost/parser.sico` 的 `scalar_ir`（单个 1,166 行巨型路由函数）的 **25 个 `match U64.checked_add` 站点物理迁移**到小 helper 函数，使其**包围函数闭包**脱离 1 MiB closure fixture bound，从而让这 25 个 `parser.sico:scalar_ir` 站点（MS-375..399）的组件轴从 `closure-too-large` 转为可逐站点实测——**不抬高任何限额、不收窄任何语义、不改动 `A == B == C` 合同**。

> **2026-11-20 规模侦察订正（决定性，见 §Reconnaissance）**：原立项草案把「闭包」与「燃料」两个目标捆在「拆 `scalar_ir` 成一组内聚小函数」这一个动作上。实测证明该机制**不可达**：函数抽取受**传递闭包守恒**约束，无法缩小 `scalar_ir` 的闭包；且燃料墙 311 站全在**另外 51 个函数**、无一在 `scalar_ir`。本 RFC 已据此订正为**站点迁移**机制，并把燃料目标移出本 RFC 范围（归独立轨道 B）。

## Problem

`scalar_ir` 是 M22 组件轴不可测的单一咽喉点（见 [`M22-option-c-split-scalar-ir-proposal`](../plans/M22-option-c-split-scalar-ir-proposal.md) 与 §Reconnaissance）：

- **closure 轴**：`scalar_ir` 的无界可达闭包 = 174 fn / 1,066,401 B 源（当前 HEAD 实测；冻结 ledger 快照 165 fn / 1,049,726 B 已过期），超 1 MiB closure fixture bound ⇒ 冻结 match-site ledger 中 25 个 `parser.sico:scalar_ir` 站点（MS-375..399）恒判 `closure-too-large`、组件轴不可比。
- **传输轴**：该闭包经 Rust oracle 降为 canonical IR = 9,102,533 B（8.68 MiB）> `MAX_CHANNEL_BYTES` 8 MiB ⇒ 单独编译 `scalar_ir` 的 IR 无法注入 guest stdin 通道。
- **关键结构事实（§Reconnaissance 实测）**：全模块 **只有 `scalar_ir` 一个函数闭包 > 1 MiB**；次大 `general_while_function_ir` 仅 542,749 B（0.518 MiB，约 bound 一半），其余 187 个函数闭包全部 ≤ 542 KB。`closure-too-large` 判定的测量根是**站点所在包围函数**，故只要把站点物理迁移到任一小函数，其闭包立即 < 1 MiB。

[`STEP-0639`](../steps/STEP-0639-m22-gate7-component-axis-option-b-narrowing.md) 的 option-B 已诚实收窄组件轴判据（25 站由整文件 IR pin 间接背书），但**不闭合**这 25 个站点。唯一既**不抬预算**（owner D-B 已裁不上调）又**不收窄语义**的真闭合路径 = **站点迁移**（见 §Reconnaissance 为何「抽函数缩小闭包」不可达）。

## Goals and non-goals

### Goals

1. **组件轴闭合（本 RFC 唯一目标，= 轨道 A）**：把 `scalar_ir` 体内 25 个 `match U64.checked_add` 站点物理迁移到小 helper 函数，使**每个迁移后站点的包围函数闭包 < 1 MiB closure fixture bound**，从而 25 站脱离 `closure-too-large`、进入可测组件轴。
   > **订正**：原表述「`scalar_ir` 自身 closure < 1 MiB」**不可达**（§Reconnaissance 传递闭包守恒）。可达的是「每个**站点**的包围函数闭包 < 1 MiB」，靠**站点迁移**而非缩小 `scalar_ir` 闭包实现。
2. **逐字节恒等守卫全绿**：整份 `parser.sico` 的 whole-file IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` 不变；全部既有 `*_byte_exactly` 组件 pin 不变；`formatter.sico` 组件 28,459 B 逐字节不变。
3. **路由定律不变**：`scalar_ir` 的语句计数路由（STEP-0430：按 `if_count`/`match_count`/`while_count` 路由到 general_while / control_while / 单 match 等机器）逐站点保持同一机器选择——所有路由判定（计数、`general_route_count` 三级判定、general_while SKIP→control_while 级联、两个内联形状探针）必须留在 `scalar_ir` 入口或原子迁移，不得拆散/换序（§Reconnaissance 风险点清单）。
4. 燃料曲线：迁移后记录 25 站的实际 verdict（`component-equal` / `FuelExhausted` / 其它非 too-large），**实测定、不预设**（小 helper 发射成本低，但站点是否转 `component-equal` 取决于发射成本，需实跑）。

### Non-goals

- **燃料墙（311 站）不在本 RFC 范围**（= 轨道 B，独立立项）：实测 311 个 `FuelExhausted` 站点分布在 **51 个不同函数**，**没有一个在 `scalar_ir`**（前几名：`control_while_function_ir` 23 站、`straight_cell_function_ir` 18、`straight_text_concat_packed` 16、`while_call_rhs_packed` 13、`gw_compare_packed` 13）。O(单函数指令数²) 发射成本是逐函数的，降燃料 = 拆那 51 个大路由/下沉函数，**与 `scalar_ir` 站点迁移无关**。
- **不**把 `scalar_ir` 的内联机器族逻辑「抽成它仍调用的顶层函数」——传递闭包守恒 ⇒ 闭包只增不减、25 站仍卡 `closure-too-large`、燃料不降（311 站在别处），**此路径净收益为零**（§Reconnaissance）。
- 不改动 canonical IR 格式、不引入紧凑 IR 传输编码。
- 不抬高 fuel / 内存 / stdout / closure / 8 MiB 通道任何限额。
- 不改 `A == B == C` bootstrap 合同、不改 Rust oracle、不改任何出口门判据（含 STEP-0639 option-B 判据）。
- 不追求一步闭合全部 336 站点；本 RFC 只闭合 `scalar_ir` 的 25 站。

## Semantics

本 RFC 不改变任何语言语义、诊断身份或 IR 形状。`scalar_ir` 是 `parser.sico`（guest lowering 模块）内部的**路由分发器**；拆分是**纯结构性重构**：

- 拆分前：`scalar_ir(source, ...) -> IR`，内部按语句计数选择机器并内联发射。
- 拆分后：`scalar_ir` 保留为唯一入口与路由判定点，把各机器族的**下沉发射逻辑**抽成独立子函数（如 `scalar_ir_gw_*` / `scalar_ir_control_*` / `scalar_ir_straight_*` / `scalar_ir_construct_*` / `scalar_ir_checked_*`），`scalar_ir` 按原路由定律委派。
- **可观测契约（必须逐字节保持）**：对任意合法输入，拆分前后 `scalar_ir` 产出的 canonical IR **逐指令/逐字节等同**；对任意畸形输入，typed refusal 身份与 span **不变**。

## Reconnaissance（2026-11-20 规模侦察，决定性）

本节固化立项后、开工前的一次**只读规模侦察**结论（复用固件镜像探针 `tools/probe-m22-closure-bounds.py` 的 `sico_functions`/`sico_callees`/`sico_closure` 精确算法，对当前 HEAD `selfhost/parser.sico` 全 188 函数逐一实测）。** coordinator 已独立复跑核验本节数字。**

### R.1 只有 `scalar_ir` 闭包超界

| 函数 | 闭包 | 占 1 MiB bound |
|---|---|---|
| **`scalar_ir`** | **174 fn / 1,066,401 B** | **1.017 MiB（超 +17,825 B）** |
| `general_while_function_ir`（次大） | 107 fn / 542,749 B | 0.518 MiB |
| `fixed_guard_chain_ir` | 94 fn / 410,610 B | 0.392 MiB |
| `gw_match_open_pack` | 91 fn / 408,163 B | 0.389 MiB |
| `control_while_function_ir` | 92 fn / 402,636 B | 0.384 MiB |

全模块 **只有 `scalar_ir` 一个函数闭包 > 1 MiB**；其余 187 个全部 ≤ 542,749 B。

### R.2 传递闭包守恒 ⇒ 抽函数不能缩小 `scalar_ir` 闭包

`closure(scalar_ir) = {scalar_ir} ∪ ⋃_{g ∈ D} closure(g)`（`D` = `scalar_ir` 直接调用集）。把 `scalar_ir` 体内一段内联代码（含对 helper 集 `H` 的调用）抽成新顶层函数 `f` 且 `scalar_ir` 仍调 `f`：新闭包 = 旧闭包 ∪ {f} ∪ closure(f)，而 `closure(f) ⊆` 旧闭包（那段代码本在 `scalar_ir` 体内）。**故新闭包 = 旧闭包 ∪ {f}，严格增大。** ⇒ 原草案「抽内联机器族逻辑成顶层函数以缩小 `scalar_ir` 闭包」**数学上不可达**。

### R.3 25 站脱困靠「站点迁移」，非闭包缩小

`closure-too-large` 的测量根 = **站点所在包围函数**（`probe-m22-closure-bounds.py`：`sico_closure(sources[path], site.function)`；`census-m22-match-sites.py` 按 `current_function` 归属 `match`）。25 站（MS-375..399）是 `scalar_ir` 体内的 `match U64.checked_add(...)`，因其包围函数是 `scalar_ir`（闭包超界）而判 too-large。**只要把站点物理迁移到任一小函数，其包围函数闭包立即 < 1 MiB ⇒ 可测。** 最干净做法：引入包裹 helper（含 `match U64.checked_add`），`scalar_ir` 改调它——例如 21 个「end offset」样板（`scalar_ir` 体内重复 21 次、合计 437 行 / 26,901 B、占 `scalar_ir` 43%）可收进一个 `checked_add_end(...)` helper（其闭包 ≈ 4 fn / ~4 KB）；余 4–5 站各用同型小 helper 包裹。**可行性 100%，改动小、风险低。**

### R.4 燃料墙与 `scalar_ir` 脱钩

311 个 `FuelExhausted` 站点分布在 **51 个不同函数**，**无一在 `scalar_ir`**（前 15：`control_while_function_ir` 23、`straight_cell_function_ir` 18、`straight_text_concat_packed` 16、`while_call_rhs_packed` 13、`gw_compare_packed` 13、`checked_arithmetic_set_match_ir` 13、`gw_ok_wrap_rhs_packed` 12、`gw_text_concat_packed` 12、`control_if_function_ir` 12、`gw_condition_packed` 11、`gw_slice_sub_length_subject_packed` 8、`gw_slice_subject_packed` 8、`gw_rhs_multi_pack` 7、`fixed_guard_chain_ir` 7、`general_while_function_ir` 7）。降燃料 = 拆这 51 个（尤其最大的）路由/下沉函数 = **轨道 B**，独立于本 RFC。

### R.5 需实跑项

- 迁移后 25 站的实际 verdict（`component-equal` 还是落回 `FuelExhausted`）取决于小 helper 的发射成本，**需实跑**组件轴套件。
- ledger 快照（165/1,049,726）已过期（`source_sha256` 不符），**任何结论以 HEAD 实测为准**；本 RFC 数字为当前 HEAD 实测，开工前须重测。

### R.6 开工阻塞发现（2026-11-20，会失败-先行实测）——oracle codegen 对「非 main 的 Result-returning 函数 + match」崩溃

按会失败-先行纪律实现 slice 1（helper + 20 个 end-offset 站点迁移）时，发现**站点迁移要求 helper 返回 `Result`，而工具链对「非 `main` 的 Result-returning 函数 + `match`」有缺口/bug**。三种 helper 形状系统排查（每种跑整文件 IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` 实测）：

| helper 形状 | 实测结果 |
|---|---|
| ① `returns Result[U64, Text]` + match-case 内 `return ok(sum)` / `return error("ERR:...")` | 前端 typed 拒 **`ERR:E-SH-IR-TYPE-MISMATCH`** —— `U64.checked_add` 实际返回 `Result[U64, NumericError]`（`sico-semantics/src/lib.rs:3826-3828`），error 载荷类型 Text≠NumericError |
| ② `returns Result[U64, NumericError]` + 直返 `return U64.checked_add(a, b)` | 前端 typed 拒 **`ERR:E-SH-IR-EXPRESSION`** —— return 位的宿主内建路由只认 `sico.bytes.length`（STEP-0609 窄路由），不认返回 Result 的内建 |
| ③ `returns Result[U64, NumericError]` + match-case 内 `return ok(sum)` / `return error(e)` | **前端通过**（scalar_ir 降级成功），但 **oracle codegen panic**：`sico-codegen-wasm/src/lib.rs:2258` `index out of bounds: len is 2 but index is 2`，位于 `Operation::Project` 的 `base.types[*index]`（`case ok(sum):` 从 Result 提取 ok 载荷触发的 Project 指令，其 local layout 的 `fields`/`types` 不自洽） |

**判定**：形状 ③ 证明 **guest 前端（scalar_ir）能降级 Result-returning helper**（这是新前沿能力的正面证据），但 **Rust oracle codegen 在该形状上 panic**——oracle 是永久参照，此 panic 是真缺陷（`Operation::Project` 对非 `main` 的 Result-returning 函数 + match 的 local layout 计算越界）。原 parser.sico **从未用过 Result 返回**（0 处），故此形状是 parser.sico 新前沿，恰好踩中 oracle 此 bug。

**阻塞结论**：本 RFC 的轨道 A（站点迁移）**当前不可落地**——它要求 helper 返回 Result，而 oracle codegen 在该形状上 panic。落地轨道 A 的前置 = **先修 oracle codegen 的 `Operation::Project` 越界 bug**（改动永久参照、影响全部既有 pin、回归风险高）。这超出「小改动、低风险」的原估算，**须 owner 裁决**：(a) 立项修 oracle codegen bug 再做轨道 A；或 (b) 轨道 A 暂缓，25 站继续由 option-B（STEP-0639）整文件 IR pin 间接背书。**本片未遗留任何 parser.sico 改动**（三种形状均已 revert，parser.sico 恢复 HEAD `01d21ed2`）。

## Syntax candidates

不涉及表层语法。此节记录**站点迁移轴**的两个候选（原「按机器族/阶段拆 `scalar_ir`」的候选已被 §R.2 传递闭包守恒否决——抽成 `scalar_ir` 仍调用的函数只会增大闭包、25 站仍卡 too-large）：

- **候选 A（推荐）— 按样板簇迁移**：把 `scalar_ir` 体内重复的「end offset」样板（21 处、437 行 / 26,901 B、占 `scalar_ir` 43%，逻辑近乎逐字相同、仅变量名不同）收进单一包裹 helper `checked_add_end(...)`（含 `match U64.checked_add`），`scalar_ir` 改调它 ⇒ 21 站物理落入小 helper；余 4–5 站各用同型小 helper 包裹。改动最小、helper 闭包最小、最内聚。
- **候选 B — 按站点逐个包裹**：为每个 `match U64.checked_add` 站点单独引入一个最小 helper。粒度最细、每 helper 闭包最小，但 helper 数量多、`scalar_ir` 调用点改写分散。

开工时用调用图实测确认每个 helper 的闭包确 < 1 MiB；可 A/B 混用，但每一片须过 §Validation 守卫（whole-file IR pin + 既有组件 pin 逐字节不变）。**关键约束**：迁移只动 `match U64.checked_add` 站点的**包围函数**，**不得触碰 §R 风险点清单里的路由判定块**（计数、`general_route_count` 三级判定、general_while SKIP→control_while 级联、两个内联形状探针）——否则触发 STEP-0430 换机回归。

## Positive and negative cases

- **正例（必须保持）**：任意合法 Sico 源经 `scalar_ir` 降为 canonical IR，拆分前后逐字节等同；`formatter.sico`、`parser.sico` 整文件 IR pin 不变。
- **正例（本 RFC 目标）**：`scalar_ir` 站点组件轴测量从 `closure-too-large` 转为可比（`component-equal` 或具名 `FuelExhausted`/其它非 too-large verdict）。
- **负例（必须 typed 拒绝、身份不变）**：畸形源在拆分前后触发同一 diagnostic id 与 span；不得因拆分引入新的接受面或新的拒绝面。
- **负例（回归即失败）**：任何既有 `*_byte_exactly` pin、whole-file IR pin、match-site ledger 既有 verdict 出现非本 RFC 预期的漂移。

## AST and IR

不新增 IR 节点、不改 canonical JSON 序列化。拆分只改变 guest `parser.sico` 的函数边界，不改 IR 生产逻辑。冻结序列化字节不变是验收硬条件。

## Component/WIT mapping

不涉及 Component/WIT 表面。组件轴测量（match-site ledger `component_sha256`）的**比较口径不变**：拆分后各子函数组件仍与 Rust oracle 同源比对。本 RFC 不得改变 ledger 的 verdict 语义（`component-equal` / `FuelExhausted` / `closure-too-large` 判定边界不变）。

## AI evaluation

不涉及 live-model 评测。验收以确定性差分为主：

- whole-file IR pin（字节级）；
- 既有 `*_byte_exactly` 组件 pin 全绿；
- match-site ledger 重测：`scalar_ir` 25 站脱离 `closure-too-large`、其余站点无回归；
- 燃料曲线：记录拆分前后可测站点的燃料变化（方向性，非里程碑数字）。

## Compatibility

- **源码身份**：`parser.sico` 源 SHA 变化 ⇒ `emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` 与 match-site ledger 的 parser digest 会漂移，须循 STEP-0605/0625/0627 协调重冻结纪律（`census-m22-*` `--update`）。这是预期的机械重冻结，非语义漂移。
- **无接口/WIT/包格式变化**；无 Rust oracle 变化。
- 并发纪律：`selfhost/parser.sico` 单写入方（§10.2），开工/收尾记 HEAD、diff、SHA。

## Security and privacy

无新增权限、hostcall、I/O 或攻击面。拆分不改变 default-deny 授权模型。畸形输入仍 fail-closed typed refusal。

## Alternatives

- **紧凑 IR 传输（否决）**：把 scalar_ir canonical IR 9.1 MB 压进 8 MiB 通道以直接测组件面。否决理由：需独立传输格式/RFC、触及冻结 IR 序列化口径、且不降燃料墙（只解传输轴不解燃料轴）；owner D-B 已裁不上调限额，紧凑传输与之同属「绕过尺寸墙」而非「消除尺寸墙」。
- **上调 closure/8 MiB 限额（否决）**：owner D-B 明确否决上调限额买回站点（STEP-0600/0606 实测：限额翻倍只解锁极少数站点、且不解决二次燃料增长）。
- **维持 option-B 间接背书（不采纳为终点）**：option-B（STEP-0639）诚实但不闭合站点；本 RFC 是其后续根治路径。

## Validation and acceptance criteria

正式接受门槛（逐条可复现）：

1. 会失败-先行：先落一个「25 个 `scalar_ir` 站点包围函数闭包仍超 1 MiB bound / 仍判 `closure-too-large`」的失败断言/探针，再做迁移。
2. 每迁一片：whole-file IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` + 全部既有 `*_byte_exactly` 组件 pin + `formatter.sico` 28,459 B 组件 pin **逐字节不变**，方可推进。
3. 终态：ledger 重测 `scalar_ir` 25 站（MS-375..399）的**包围函数**全部变为小 helper、闭包 < 1 MiB bound ⇒ 脱离 `closure-too-large`（转 `component-equal` 或具名非 too-large verdict）；`--check` 无 unexpected pending。**注**：`scalar_ir` 自身闭包仍 > 1 MiB（它仍调用 helper）——这是预期的、非失败；判据是**每站点包围函数闭包**，非 `scalar_ir` 自身闭包。
4. 25 站迁移后实际 verdict 实测记录（`component-equal` / `FuelExhausted` / 其它）；不预设。
5. 冻结工件协调重冻结（`emitter-vocabulary` source_sha256 + match-site ledger parser digest + 站点归属函数变化），循既有纪律。
6. 全程未上调任何限额；未改 `A == B == C` 合同、Rust oracle、出口门判据；未触碰 §R 风险点的路由判定块。

满足上述方可登记 option-C 收口 STEP；不满足则诚实收窄并归后续片。**本 RFC 被接受不等于 M22 任何门翻转；M22 overall 仍 NO-GO。**

## Links

- [`M22-option-c-split-scalar-ir-proposal`](../plans/M22-option-c-split-scalar-ir-proposal.md)（立项草案）
- [`STEP-0639`](../steps/STEP-0639-m22-gate7-component-axis-option-b-narrowing.md)（option-B 落地，本 RFC 的前置）
- [`m22-fuel-wall-decision-brief-2026-11-19`](../reports/m22-fuel-wall-decision-brief-2026-11-19.md)（owner 裁决依据）
- [`ADR-0016`](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)（`scalar_ir` 单一 lowering 模块边界）
- [`ADR-0015`](../adr/ADR-0015-compiler-bootstrap-closure-v0.md)（`A == B == C` 合同，本 RFC 不弱化）
- M22 计划 §10.1.4（燃料墙归因与 D-B/D-A 裁决）
