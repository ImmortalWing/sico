# STEP-0359 — M22: concat 实参 `u64.to_text` 分支单落地复测（第二次回退，机制定位至 span 级）

> - 状态：**reverted（第二次）**——单分支（无嵌套支持）复测仍 3 例 DIFF；
>   回退后 census 复测 68/126、0 DIFF、histogram 与 STEP-0357 终值逐格一致
> - 里程碑：M22 compiler self-host（S4 lowering 片，诊断片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 本片内容

**假设检验**：STEP-0358 的 3 例 DIFF 或全部来自嵌套调用分支——本片把
`gw_text_concat_packed` 的 `u64.to_text` 实参分支**单独**重落地（不带
gw_intrinsic_call_packed 嵌套支持），并按 0358 教训**立即**跑 census。

**结果**：DIFF 户与 0358 完全相同（list_get_match_ir / chained_let_function_ir /
straight_let_function_ir）→ 嵌套分支非根因，to_text 分支本体有责。

**机制定位（本片主要产出）**——census 探针增强（DIFF 行带闭包清单）+ 逐字节
对比给出精确线索：

- 三个 DIFF 户的**真实基线是 CALL-ARGUMENT 拒绝**（其函数体含
  to_text-concat 语句，整函数据此拒绝）；分支使其从"拒绝"翻为
  "exit 0 但字节不等"——即分支在深上下文的**发射有 span 级偏差**；
- 首个发散指令 = straight_let_function_ir 的 result 10 write_local：
  oracle start 42178 vs guest 41990（差 −212B，end 一致）；后续 result 11
  起恢复一致——疑似分支把 gcursor/语句消费长度链弄偏 212B（恰约一条
  to_text 语句长度），但隔离形状（c1/c2f）完全 byte-exact，疑点在
  **多语句链上的 gcursor 推进**与分支内 word_span_offset 的相互作用；
- DIFF 户闭包内**无 gw 机器**——是这些函数体语句经 gw 机器降低时命中分支。

**回退净验证**：census 68/126、0 DIFF、四格 histogram 与 0357 终值一致；
无残留；构建绿。

## 2. 登记（下片输入）

- to_text 分支发射在**单语句隔离**正确、**语句链深上下文** span 偏 212B；
  下片首选：以 straight_let_function_ir 截体（保留 result-10 前后语句）
  二分，对照 oracle 定位 gcursor 推进偏差的引入点。
- census 探针已增强（DIFF 行带闭包清单，存于 target/tmp 复用）。
- 方法纪律重申：扩族分支落地后必须立即全量 census；单/双分支分离复测
  是有效的归因手段（本片排除嵌套分支）。

## 3. 验证

- 回退后：census 68/126、0 DIFF、histogram CALL-ARGUMENT 36 / CALL-SHAPE 7 /
  EXPRESSION 14 / STATEMENT 1 与 0357 终值一致；git diff --check 净。
- 本片未增减固件（无落地变更）。
