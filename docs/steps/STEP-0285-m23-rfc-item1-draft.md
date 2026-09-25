# STEP-0285: M23 逐项 RFC 起草（执行卡 23-B，item 1）+ item 4 收口 + item 3 缺口

> - status: complete / RFC-0048 registered as **draft**; item 4 closed per recorded rule; item 3 deferred on external gap; no implementation, no gate moved
> - phase: M23 language v1 batch 3 — per-item RFC phase (execution card 23-B)
> - date: 2026-09-25
> - evidence class: contract drafting over frozen measurement (STEP-0284) + live probes

## 1. Scope and decision

执行 [M22–M26 execution cards](../plans/M22-M26-execution-cards.md) 卡
**23-B**：在 STEP-0284 冻结普查之上起草首项 RFC。四项独立推进，本 STEP
处理 item 1/3/4；item 2（表达式位置条件）的 RFC 另行起草。

1. **Item 1 → [`RFC-0048`](../rfc/RFC-0048-language-v1-batch3-infix-comparison-logical-and.md)
   （draft）。** 提案最小封闭集 = 五个有序/不等中缀比较
   `< <= > >= !=` 加 `&&`，全部 desugar 到既有 `EqualFixed`/`LessFixed`
   两个 IR 操作与既有分支结构（无新 IR 操作、无新内建）；`||` 与前缀
   `!` 因普查零实测消费被排除。交换形操作符（`>`/`<=`）加原子操作数
   限制并以新 `E2025` typed refusal 强制；`&&` 以嵌套分支定义短路。
2. **Item 4 → 按计划 §8.4 既有决策规则收口。** 规则的"否则"支臂触发：
   runner 不暴露 call/allocation 计数（STEP-0284 在案），"measurably
   better" 无法成立 → 组合式 `text.length`/`char_at` 记录为最终形态，
   项关闭；重开需新运行时测量证据 + 完整 RFC-0033 门。
3. **Item 3 → 缺口登记，deferred。** selfhost 侧测量已冻结（2,169 处、
   3.199/KB）；owner 审核的 AI 生成语料是未满足腿，决策记录推迟，
   不改语法。

## 2. Measured/recorded facts drafted against

- 普查：978 比较谓词 if 行、79 嵌套比较 if 位点、按谓词 call sites
  （less_than 693 / equal 502）。
- 本 STEP 新增实测（写入 RFC §2）：`if x < 4:` 当前是词法错误
  （`UnexpectedCharacter('<')`）；`if x <= 4:` 当前 **check 零诊断通过、
  build 才以** `unsupported call target x<=U64.literal` **拒绝**——
  `LessEqual` 记号存在于词法表但无任何阶段消费，构成 check/build 分裂。
  该缺口的方向由本 RFC 裁决（接受），拒绝语料行随实现 STEP 落矩阵。
- IR 事实：定宽比较操作只有 `EqualFixed`/`LessFixed`；typed dispatch
  只有 `equal`/`less_than`（`crates/sico-ir/src/lower.rs:2971` 一带）；
  RFC-0044 的 `==` 在 `LineKind::If` 顶格记号切分处 lowering。
- 诊断码占用（append-only 核对）：E2001、E2002、E2010、E2011、
  E2020–E2024、E2031；RFC-0048 取下一号 `E2025`。

## 3. Discipline

- RFC-0048 为 **draft**：没有 owner 接受不写实现；实现本身另受 M23 §2
  双门（Route A = M22 S7 GO，Route B = R3 止损）约束。本 STEP 不把草案
  写成支持。
- 四项独立：item 1 草案与 item 4 收口互不依赖；item 2 RFC 待起草；
  历史裁决不追改。
- 自宿 re-baseline：guest lexer 对 (`== <= < >= > != &&`) 的镜像只在
  采纳片落地（RFC §9），当前 selfhost 源零中缀使用（` == ` 出现 0 次，
  已实测），M22 差分不受影响。

## 4. Executable evidence

- `sico check --json`（`<=` 探针）：diagnostics 空、exit 0；
  `sico build --profile script-v0`（同文件）：typed refusal
  `unsupported call target x<=U64.literal`。`<` 探针：unregistered
  lexical error `UnexpectedCharacter('<')`。探针为临时文件，未入库。
- 无 Cargo 工作区/runner 变更；`git diff --check` 与
  `validate-step-0124.ps1 -SelfTest` 通过。

## 5. Gate accounting

无 gate 变化。M23 = planned（item 1 RFC draft、item 2 RFC 待起草、
item 3 deferred-external、item 4 closed）；实现双门不变。R3 不计
（合同/测量 STEP）。下一张允许领取的卡：item 2 RFC（23-B 续）、
24-A 盘点；22-C 仍等 W1 裁决。
