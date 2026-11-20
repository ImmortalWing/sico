# RFC-0058: M22 拆小 `parser.sico:scalar_ir` 以闭合组件轴 v0

> - status: proposed
> - date: 2026-11-20
> - authors: autonomous-agent
> - target language/platform version: draft
> - supersedes: -
> - superseded-by: -

## Summary

把 `selfhost/parser.sico` 的 `scalar_ir` 从单个巨型路由函数拆成一组内聚子函数，使其 closure 脱离 1 MiB closure fixture bound、各子函数 canonical IR 可注入 8 MiB 通道，从而让 `parser.sico:scalar_ir` 的 match 站点组件轴从「不可测/`closure-too-large`」转为可逐站点实测——**不抬高任何限额、不收窄任何语义、不改动 `A == B == C` 合同**。

## Problem

`scalar_ir` 是 M22 燃料墙与组件轴不可测的单一咽喉点（见 [`M22-option-c-split-scalar-ir-proposal`](../plans/M22-option-c-split-scalar-ir-proposal.md)）：

- **closure 轴**：`scalar_ir` 无界可达闭包 = 165 fn / 1,049,726 B 源，超 1 MiB closure fixture bound ⇒ 冻结 match-site ledger 中 25 个 `parser.sico:scalar_ir` 站点（MS-375..399）恒判 `closure-too-large`、组件轴不可比。
- **传输轴**：该闭包经 Rust oracle 降为 canonical IR = 9,102,533 B（8.68 MiB）> `MAX_CHANNEL_BYTES` 8 MiB ⇒ 即便抬 closure bound，单独编译 `scalar_ir` 的 IR 也无法注入 guest stdin 通道。
- **燃料轴**：发射成本 ~ O(单函数指令数²)（STEP-0606/0611/0618：`cgs_dispatcher_slot` 逐指令全 body 重扫，已由优化刀砍到 O(I²)，剩余二次项集中在**最大函数的指令数**）⇒ 单个巨型 `scalar_ir` 是燃料墙热点函数。

[`STEP-0639`](../steps/STEP-0639-m22-gate7-component-axis-option-b-narrowing.md) 的 option-B 已诚实收窄组件轴判据（25 站由整文件 IR pin 间接背书），但**不闭合**这 25 个站点（更不闭合 311 个 fuel-exhausted 站点）。唯一既**不抬预算**（owner D-B 已裁不上调）又**不收窄语义**的真闭合路径 = **降低单函数指令数 / 单闭包尺寸** = 拆小 `scalar_ir`。

## Goals and non-goals

### Goals

1. `scalar_ir` 自身 closure **< 1 MiB closure fixture bound**，使 25 个 `scalar_ir` 站点脱离 `closure-too-large`、进入可测组件轴。
2. 拆分后每个子函数的 canonical IR 均可注入（逐函数 < 8 MiB 通道）。
3. 燃料墙实质下降：原先 `FuelExhausted` 的站点中，一个**实测定**（不预设）的比例转为 `component-equal`。
4. **逐字节恒等守卫全绿**：整份 `parser.sico` 的 whole-file IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` 不变；全部既有 `*_byte_exactly` 组件 pin 不变。
5. **路由定律不变**：`scalar_ir` 的语句计数路由（STEP-0430：按 `if_count`/`match_count`/`while_count` 路由到 general_while / control_while / 单 match 等机器）逐站点保持同一机器选择。

### Non-goals

- 不改动 canonical IR 格式、不引入紧凑 IR 传输编码（那是被否决的备选，见 Alternatives）。
- 不抬高 fuel / 内存 / stdout / closure / 8 MiB 通道任何限额。
- 不改 `A == B == C` bootstrap 合同、不改 Rust oracle、不改任何出口门判据（含 STEP-0639 option-B 判据）。
- 不追求一步到位把全部 336 站点转 `component-equal`；本 RFC 只承诺 `scalar_ir` 闭包面（25 站）转可测 + 燃料曲线实质下降，其余站点归后续片。

## Semantics

本 RFC 不改变任何语言语义、诊断身份或 IR 形状。`scalar_ir` 是 `parser.sico`（guest lowering 模块）内部的**路由分发器**；拆分是**纯结构性重构**：

- 拆分前：`scalar_ir(source, ...) -> IR`，内部按语句计数选择机器并内联发射。
- 拆分后：`scalar_ir` 保留为唯一入口与路由判定点，把各机器族的**下沉发射逻辑**抽成独立子函数（如 `scalar_ir_gw_*` / `scalar_ir_control_*` / `scalar_ir_straight_*` / `scalar_ir_construct_*` / `scalar_ir_checked_*`），`scalar_ir` 按原路由定律委派。
- **可观测契约（必须逐字节保持）**：对任意合法输入，拆分前后 `scalar_ir` 产出的 canonical IR **逐指令/逐字节等同**；对任意畸形输入，typed refusal 身份与 span **不变**。

## Syntax candidates

不涉及表层语法。此节记录**拆分轴**的两个候选：

- **候选 A（推荐）— 按机器族拆**：把 `gw_*`（general_while 族）、`control_*`（if/while 族）、`straight_*`（直通族）、`construct_*`/`record_*`（构造族）、`checked_*`（算术族）各自的下沉发射逻辑抽成独立函数。内聚性最高、与既有命名对齐。
- **候选 B — 按阶段拆**：语句扫描 / 路由判定 / 各机器发射三段。边界更清晰但跨机器耦合、路由定律回归风险更高。

开工时用 ddmin / 调用图实测确认最终拆分粒度；可 A/B 混用，但每一片须过 §Validation 守卫。

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

1. 会失败-先行：先落一个「`scalar_ir` closure 仍超 1 MiB bound」的失败断言/探针，再拆。
2. 每拆一片：whole-file IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` + 全部既有 `*_byte_exactly` 组件 pin **逐字节不变**，方可推进。
3. 终态：`scalar_ir` closure < 1 MiB bound；ledger 重测 `scalar_ir` 25 站脱离 `closure-too-large`（转 `component-equal` 或具名非 too-large verdict）；`--check` 无 unexpected pending。
4. 燃料曲线实测记录（拆分前后对比，方向性）；不预设单调、不预设比例。
5. 冻结工件协调重冻结（`emitter-vocabulary` source_sha256 + match-site ledger parser digest），循既有纪律。
6. 全程未上调任何限额；未改 `A == B == C` 合同、Rust oracle、出口门判据。

满足上述方可登记 option-C 收口 STEP；不满足则诚实收窄并归后续片。**本 RFC 被接受不等于 M22 任何门翻转；M22 overall 仍 NO-GO。**

## Links

- [`M22-option-c-split-scalar-ir-proposal`](../plans/M22-option-c-split-scalar-ir-proposal.md)（立项草案）
- [`STEP-0639`](../steps/STEP-0639-m22-gate7-component-axis-option-b-narrowing.md)（option-B 落地，本 RFC 的前置）
- [`m22-fuel-wall-decision-brief-2026-11-19`](../reports/m22-fuel-wall-decision-brief-2026-11-19.md)（owner 裁决依据）
- [`ADR-0016`](../adr/ADR-0016-selfhost-soa-compilation-unit-v0.md)（`scalar_ir` 单一 lowering 模块边界）
- [`ADR-0015`](../adr/ADR-0015-compiler-bootstrap-closure-v0.md)（`A == B == C` 合同，本 RFC 不弱化）
- M22 计划 §10.1.4（燃料墙归因与 D-B/D-A 裁决）
