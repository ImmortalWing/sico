# STEP-0286: M23 item 2 RFC 起草（执行卡 23-B 续）

> - status: complete / RFC-0049 registered as **draft**; no implementation, no gate moved
> - phase: M23 language v1 batch 3 — per-item RFC phase (execution card 23-B, item 2)
> - date: 2026-09-25
> - evidence class: contract drafting over frozen measurement (STEP-0284) + live probes

## 1. Scope and decision

续卡 **23-B**：起草 M23 item 2（表达式位置条件）的
[`RFC-0049`](../rfc/RFC-0049-language-v1-batch3-let-bound-if-match.md)
（**draft**，owner 接受前不实现）。按计划 §8.2"最小 let 绑定语句形"的
义务，RFC 只收两种形态——`let x = if/else` 与 `let x = match`；臂为
恰好一个表达式；desugar 走 RFC-0046 D2 已落地的
`lower_propagating_let`（cell + 分支写 + 汇合读）机制，**零新 IR
操作**。更大表达式位置（操作数/实参/return/嵌套条件）v0 继续 typed
refusal。

## 2. Measured/recorded facts drafted against

- STEP-0284 普查 `bind_value_match_blocks`：selfhost **248**、应用语料
  **121**——每个都是"只为把一个条件值绑进 cell"的 match 块（set 于各
  臂），即本提案的手工模拟。
- 本 STEP 新增实测：`let x = if …` / `let x = match …` 当前均被 parse
  期拒绝：`MismatchedClose { expected: Function, actual: If|Match }`
  （诚实 typed 拒绝；本 RFC 将其扩为接受，additive widening）。
- D2 先例：`let y = expr?` 的 cell/分支/汇合机制已在
  `crates/sico-ir/src/lower.rs:1894` 落地并有语料。
- else/case/end 语法与 statement `match` 的穷尽规则原样复用；新码
  `E2026`（臂非单表达式）、`E2027`（缺 else），臂类型不匹配走既有
  `E2001`。

## 3. Discipline

- draft：无 owner 接受不写实现；实现另受 M23 §2 双门。
- 与 RFC-0048 独立、可组合、互不依赖接受。
- 自宿 re-baseline：248 个手工位点的改写**不是**自动义务，按 M23 §9
  显式 re-baseline 决策单独登记；M22 差分在采纳前不受影响。

## 4. Executable evidence

- `sico check`（两探针）：`MismatchedClose` parse 拒绝，exit 2；探针为
  临时文件未入库。
- 无 Cargo 工作区/runner 变更；`git diff --check` 与
  `validate-step-0124.ps1 -SelfTest` 通过。

## 5. Gate accounting

无 gate 变化。M23 = planned（item 1/2 RFC draft、item 3 deferred、
item 4 closed）；实现双门不变；R3 不计（合同 STEP）。下一张无需输入
的卡：24-A 盘点。owner 待决：W1 裁决方式、RFC-0048/0049 接受与否。
