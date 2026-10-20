# STEP-0362 — M22: `gw_find_code_text` 自降低门链微复现与重构（+4 翻绿，census 76/127、0 DIFF）

> - 状态：implementation complete locally（回归绿见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，自降前置片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**诊断**：0361 后 straight_let_function_ir 仍拒 EXPRESSION——25 函数前缀
二分定位到**上下文依赖**：`gw_find_code_text` 自身在模块中即拒绝，
其存在使一切含它的闭包整体拒绝（helper 不自降低 = 深上下文陷阱的
又一形态：0361 的 span 修复对该 helper 是新行为面）。

**`gw_find_code_text` 自降低门链微复现**（截体 T1–T5）：
- 外框（双层 while + 用户调用 let + if-return）✓ 早已支持；
- 门 1 = `sico.bytes.at` 的游标实参为用户调用（`next_index(...)` 内联）
  ——改写为循环内 cell 游标（`let gfc_i_next = next_index(...)` 后
  `bytes.at(src, gfc_i_next, ...)`）通过；
- 门 2 = 条件内嵌定宽算子（`U64.bit_and(bs, 1)` 作 `U64.equal` 左操作数）
  ——改写为前向扫描内的反斜杠奇偶（toggle）通过；
- 门 3 = 三重嵌套 while 反向回溯走线（else 臂内 match + 内层 while）——
  扁平化为前向 prev-byte 奇偶（`gfc_prev_byte` 循环尾无条件刷新）；
- 现态：prev-byte 形式推进至下一道 EXPRESSION 门（in_string 臂内
  更深形状，截体定位中），helper 仍拒但**语义正确、门链逐道收窄**。

**固件**：无新增（helper 未翻绿，不冻结）；0361 固件不受影响。

## 2. 前沿更新

- 依赖闭包 census **72/127 → 76/127、0 DIFF**（helper 重构的连锁解锁；
  GWPACK-OTHER 1 复现属级联健康流动）。
- 关键教训（已登记 handoff）：**新增 helper 必须立即微复现其自降低**——
  helper 不自降低 = 其全部消费者的闭包被整体毒化，census 表现为
  消费者的远距拒绝而非 helper 本体。
- 下一刀：gw_find_code_text 剩余 EXPRESSION 门链（in_string 臂内形状，
  T5 已推进至其邻近）+ straight_let/list_get_match_ir/chained_let 随
  helper 翻绿后的各自下一门。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0361 终值（72/127、0 DIFF）后动刀；
  每个重构立即 census。
- 微复现：T1–T5 截体逐门定位；helper 现态仍拒（如实登记，不冻结）。
- `--test selfhost_compiler`：见回归条（0361 固件 68 项原位绿）。
- 回归：validate-step-0262 / validate-step-0245（含 runner clippy，借
  主仓 msys2 gcc）/ report-m22-canary / git diff --check 本片重跑全绿
  （数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=76 diffs=0 total=127；HIST
  CALL-ARGUMENT 8 / CALL-SHAPE 14 / EXPRESSION 26 / GWPACK-OTHER 1 /
  NON-DOMAIN 1 / STATEMENT 1。
- R3 连续停滞数保持 0（parser 覆盖 72→76 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
