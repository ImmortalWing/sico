# STEP-0284: M23 开工仪式普查（执行卡 23-A）

> - status: complete / measurement-only census frozen; no gate moved, M23 stays planned
> - phase: M23 language v1 batch 3 — kickoff inventory (execution card 23-A)
> - date: 2026-09-25
> - evidence class: internal-fixture, deterministic text census

## 1. Scope and decision

执行 [M22–M26 execution cards](../plans/M22-M26-execution-cards.md) 卡
**23-A**：M23 四项语言人体工学（中缀比较/逻辑运算符、表达式位置条件、
裸字面量、`text.chars`）的统一开工测量，按 M23 计划 §7（STEP-0129/0142
模式）在任何语法改动前冻结消费证据。纯测量，不改 grammar/IR，不计 R3。

冻结产物：[`m23-ceremony-census-2026-09-25.json`](../reports/m23-ceremony-census-2026-09-25.json)
（schema `sico.m23.ceremony.v0`），由新增工具
`tools/census-m23-ceremony.ps1` 生成。工具为纯 ASCII、无时间戳、逐字段
文档化正则/行模式（沿用 `census-ec4-records.ps1` 先例），**两次运行字节
一致（已实测 cmp）**，独立方可复算。

## 2. Measured results（2026-09-25，本工作树）

分母与 [STEP-0282 W2 基线](../reports/m22-w2-baseline-2026-09-25.json)
一致：selfhost 8 源、16,390 行。应用语料 `tests/end-to-end`：51 文件、
3,795 行（EC-4 的 39 文件 + RFC-0047 批新增 12 个 record fixture）。

| 指标（plan §8 对应项） | selfhost | application |
|---|---|---|
| checked 算术 match 块（item 1c） | 263 | 42 |
| 其中单目标 bind 型 match（item 2 代理） | 248 | —（字段保留） |
| 比较谓词 if 行（item 1a 需求代理） | 978（59.7/千行） | 36 |
| 比较谓词 return 行 | 已入库（逐文件） | 同 |
| 嵌套比较 if ≥2 层位点（item 1b 代理） | 79 | 已入库 |
| literal 构造器调用（item 3 selfhost 侧） | 2,169（3.199/KB） | 768 |
| `text.length`/`char_at` 调用（item 4 组合式） | 153 | 已入库 |
| formatter `word_kind` 区域 chars 调用 | 0 | — |

## 3. Honest gaps（按卡面写明，不估算）

- **item 3 的 AI 生成语料：未测。** owner 门控外部门（计划 §8.3 要求
  owner 审核的 AI 生成 Sico）；本 STEP 只冻结 selfhost/应用侧 literal
  密度。裸字面量裁决（"D3 维持"或 typed-literal RFC）在语料补齐前不可
  作出。
- **item 4 的运行时计数：未测。** runner CLI 不暴露 call/allocation
  计数器；`text.chars` 内建按其决策规则需要"可测地更优"的运行时证据，
  当前仅有组合式调用位点计数。item 4 在证据补齐前**不能翻内建**，
  组合式保持为记录在案的最终形态候选。
- 比较谓词行、嵌套 if 位点是**文本行代理**（逐字段模式已写进 JSON 与
  工具头注），不是语法解析计数；RFC 阶段（23-B）须以解析级核对再引用。

## 4. Interpretation discipline

本普查是需求信号，**不构成任何运算符/语法决定**（计划 §8.1："No
operator decision is argued from recollection"——也不从行数argue）。
例如 978 行比较谓词只说明中缀比较的消费面存在；最小封闭集、desugar、
短路语义仍由 23-B 的 RFC 按四证据门决定。实现仍双门：Route A（M22 S7
GO）或 Route B（R3 止损登记）+ 对应 accepted RFC，本 STEP 不改变该门。

## 5. Executable evidence

- `powershell -File tools/census-m23-ceremony.ps1`：exit 0，
  `CENSUS_M23_CEREMONY_OK selfhost_files=8 app_files=51`；连续两次运行
  输出 JSON `cmp` 字节一致（确定性实测）。
- 未运行 Cargo 工作区/runner：零产品代码改动。

## 6. Gate accounting

无 gate 变化。M23 = planned（测量/合同可并行，实现等双门）；四项彼此
独立待 RFC。下一张允许领取的卡：23-B（逐项 RFC，需引用本普查的冻结
计数与分母）；或继续 M22 关键路径（22-A CI 裁决）。
