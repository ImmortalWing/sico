# M22 选项 C 立项草案：拆小 `parser.sico:scalar_ir`（draft，待 owner review）

- 状态：**draft / 立项草案，未开工、未排 STEP、未冻结合同**
- 日期：2026-11-19
- 起草依据：owner 就 [`m22-fuel-wall-decision-brief-2026-11-19.md`](../reports/m22-fuel-wall-decision-brief-2026-11-19.md) 裁决「接受（选项 B 落地）+ 选项 C 立项」；本文件是选项 C 的立项草案
- 关联：[`STEP-0639`](../steps/STEP-0639-m22-gate7-component-axis-option-b-narrowing.md)（选项 B 落地，闭合门 7 预存红）

## 1. 为什么拆 `scalar_ir`（问题）

`parser.sico:scalar_ir` 是 M22 燃料墙 + 组件轴不可测的**单一咽喉点**：

- **closure**：165 fn / 1,049,726 B，超 1 MiB closure fixture bound ⇒ 25 个站点恒 `closure-too-large`。
- **IR**：canonical IR ≈ 9.1 MB，超 guest stdin 注入通道（8 MiB）⇒ 组件轴**不可测量**（抬 bound 只得注入 panic）。
- **燃料**：历史登记 331/335 燃料耗尽热点的所在函数；发射成本 ~ O(单函数指令数²)（STEP-0606/0611/0618：`cgs_dispatcher_slot` 逐指令全 body 重扫，已砍到 O(I²)，剩余二次项集中在**最大函数的指令数**）。

选项 B（STEP-0639）已诚实收窄组件轴口径，但**不闭合**这 336 个站点（25 not-comparable + 311 fuel-exhausted）。唯一既**不抬预算**又**不收窄语义**的真闭合路径 = **降低单函数指令数** = 拆小 `scalar_ir`。

## 2. 目标（可验证终点）

把 `scalar_ir` 从单个巨型函数拆成一组内聚的小函数，使得：

1. `scalar_ir` 自身 closure **< 1 MiB closure bound**（25 站点脱离 `closure-too-large`，可被组件轴测量）。
2. 拆分后各函数的 canonical IR 均能**注入**（< 8 MiB 通道）。
3. 燃料墙实质下降：原先 FuelExhausted 的站点中，一个可测比例转为 `component-equal`（具体比例开工时实测定，不预设）。
4. **逐字节恒等守卫全绿**：整份 `parser.sico` 的 whole-file IR pin `sico_guest_arena_accommodates_full_parser_selfhost_input` + 全部既有 `*_byte_exactly` 组件 pin 不变。

## 3. 方法（草案，开工时按实测细化）

- `scalar_ir` 是**路由敏感**的分发器（STEP-0430 实锤：按 `if_count`/`match_count`/`while_count` 语句计数路由到 general_while / control_while / 单 match 等机器）。拆分**必须保持路由定律不变**——这是最大风险点。
- 候选拆分轴（按内聚性，开工时用 ddmin/调用图实测确认）：
  - 按**机器族**拆：`gw_*`（general_while 族）、`control_*`（if/while 族）、`straight_*`（直通族）、`construct_*`/`record_*`（构造族）、`checked_*`（算术族）各自的下沉发射逻辑抽成独立函数。
  - 按**阶段**拆：语句扫描 / 路由判定 / 各机器发射 三段。
- 纪律：**会失败-先行**——先写一个「`scalar_ir` closure 超 bound」的失败断言/探针，再拆；每拆一片复跑 whole-file IR pin + 组件 pin，逐字节不变才推进。
- §10.2：`selfhost/parser.sico` 单写入方；每次开工/收尾记 HEAD、diff、SHA。

## 4. 风险

- **路由定律回归**：拆分改语句计数/函数边界可能触发 STEP-0430 的路由敏感门（换机器 → 不同门）。缓解：每片跑 whole-file IR pin（字节级）+ match-site ledger 复测。
- **燃料暂升**：函数拆分引入调用开销，短期个别站点燃料可能先升后降；需实测曲线，不预设单调。
- **工程量大**：`scalar_ir` 是 guest 前端最大函数，重构跨多片。

## 5. 排期建议

- 不占当前 M22 出口队列（选项 B 已诚实收窄口径，门 7 已绿）。
- 建议作为独立工作线，开工前按 §10.2 确认 `parser.sico` 单写入方 + 冻结一段稳定窗口。
- 入口门槛：owner 批准本草案 + 冻结窗口就绪。

## 6. 开工前需 owner 确认

1. 批准本草案方向（拆 `scalar_ir` 为选项 C 主攻）。
2. 是否先做一次性**规模侦察片**（不改动、只测拆分可行性 + 预期收益曲线）再决定是否全量投入。