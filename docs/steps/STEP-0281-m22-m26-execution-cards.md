# STEP-0281: M22–M26 单步执行卡与过时 M22 队列修正

> - status: complete / documentation-only handoff; no gate moved, no capability claimed
> - phase: M22–M26 governance (handoff surface between sessions/models)
> - date: 2026-09-25
> - evidence class: planning/handoff documentation; validators below were executed locally

## 1. Scope and decision

Owner 指令「继续」后的交接收口：工作树携带 STEP-0279（M14–M26 审查）与
STEP-0280（M22 W1 C/D 本机实现）两批未提交修改，且短期内由不同会话/模型
接力。为避免接手方按过时队列或整表推进，本 STEP 只做三件事，不改任何
gate、合同或能力声明：

1. **建立 [`M22–M26 execution cards`](../plans/M22-M26-execution-cards.md)**：
   每个里程碑拆成单张执行卡（22-A..22-D、23-A..23-C、24-A..24-D、
   25-A..25-C、26-A..26-D），每卡固定入口证据、唯一出口输出、停手规则和
   六行审查记录格式；附「发给单个执行模型的固定输入」模板。未来实现
   STEP 编号不预留。
2. **重写 [`handoff-m22.md`](../handoff-m22.md)** 到 STEP-0280 本机基线：
   两批未提交内容的清单、本机已跑证据与边界（Windows GNU 真实 runner，
   msys2-binutils PATH）、下一条有界工作（W1 CI 待裁决 → W2 canary 重钉
   → R3 计数规则）与恢复命令。旧「本机不可重建 runner」的说法作废。
3. **修正 M22 计划 §3.2 的过时执行队列**：`item` 到 `repeat_indent` 的
   区域已通过 canary，不再列为待实现；队列头改为 W1 出口与 W2 基线，
   之后是剩余 formatter 尾部（`Map[Text,U64]` 区、`close_code`/
   `direct_close`/`opener_close`、`normalize_source`、`main`）、其余
   selfhost 源码盘点差分、S5、S6。已退役的 `lexer.sico`/`tokens.sico`/
   `declaration_parser.sico` 明确标注为历史证据，不是队列项。

同步项：[`handoffs/README.md`](../handoffs/README.md) 指向当前
`handoff-m22.md`；[`plans/README.md`](../plans/README.md) 登记 execution
cards 行并刷新 M18/M21/M22/M23/M25 状态摘要；
[route replan §8](../plans/M22-M26-route-replan-v1.md) 的下一步动作表
从「首个动作」改为「STEP-0279 后的当前动作」。

## 2. What this STEP does not do

- 不宣告 W1 关闭：STEP-0278/0280 的独立 CI 仍未取得，M22 保持 NO-GO。
- 不为任何里程碑创建实现 STEP 编号；执行卡只是边界，不是合同或裁决。
- 不修改语言、Runtime、Host、PDF 或 UI 能力接口与语料。
- 不追改 M14–M21 的历史 STEP 裁决。

## 3. Executable evidence

- `./tools/validate-step-0124.ps1 -SelfTest`：正例与五个负例通过（本 STEP
  收尾时干净复跑）。
- `git diff --check`：通过（新增文件以 `rg '[ \t]+$'` 复核行尾）。
- 未运行完整 Cargo 工作区：本 STEP 只改文档与索引，不触及编译产物。

## 4. Gate accounting

无 gate 变化。W1（M22）= 待独立 CI；M22 = NO-GO（S3/S4/S5 partial，
S6/S7 未进入）；M23–M26 = planned，可并行盘点。R3 四步计数从 W2 S3/S4
实现 STEP 起算；本 STEP 为文档交接，不计。下一项有界工作：取得
STEP-0278/0280 的 CI 结果（22-A），或先做 22-B 测量基线；均见
[execution cards](../plans/M22-M26-execution-cards.md)。
