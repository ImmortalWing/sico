# STEP-0385 — M22: GWPACK-OTHER 簇瓦解——`sico.text.length` 入 gw RHS 调度，census 89→90/128、0 DIFF，GWPACK-OTHER 34→1、CALL-SHAPE 5→37 级联

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，gw 机器族 RHS 扩族片 2）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**门族定位**：GWPACK-OTHER 簇主公共门 = gw set/let RHS 的 `sico.text.length`
无任何调度（gw_rhs_packed 直落 EXPRESSION）。微复现两探针（while 上下文）：
`set m = sico.text.length(cell)` 拒、`set m = sico.text.length(user_call(...))`
拒。孤立形状对照（cell concat / user-call let）全绿，锁定缺的是
text.length 本身的调度。

**修改**（parser.sico）：
- `gw_rhs_packed` 加一行调度：`text.length` → 既有
  `gw_intrinsic_call_packed`（param 零指令直引 / cell read_local+intrinsic
  双指令；ret_kind "u64"）——零新打包器。
- 实参提升两处源码绕行（gw_intrinsic_call_packed 的 arity 守卫只收单
  token param/cell 实参）：fixed_literal_argument 与
  straight_environment_literal_count 的
  `set __h = sico.text.length(user_call(...))` 拆为
  `let tmp = user_call(...)` + `set __h = sico.text.length(tmp)`。

**固件**（selfhost_compiler.rs）：新测试
`sico_compiler_lowers_gw_text_length_rhs_byte_exactly`——text.length cell
形状字节全等 + call 实参守卫（CALL-SHAPE）+ 拒绝后恢复 +
straight_environment_literal_count 闭包（13 函数）字节全等；同片更新
full-source 前沿断言 GWPACK-OTHER→CALL-SHAPE（40B 燃料手动实测 typed
拒绝非 trap）。

## 2. 前沿更新

- census **89/128 → 90/128、0 DIFF**；**GWPACK-OTHER 34→1**（簇瓦解）、
  **CALL-SHAPE 5→37**（大片级联推进：text.length 解锁后各户走到各自下一道
  真实门）；straight_environment_literal_count 翻绿；无回归。
- 下一刀主线：CALL-SHAPE 37 户门族细分（新主导族）+ 残余 GWPACK-OTHER 1 户。

## 3. 事故与登记

- 陈旧闭包提取文件误导定位（修完后对拍旧 extract 仍 CALL-SHAPE）——
  纪律：extract 必须每次从当前 parser.sico 重新生成（extract_closure.py
  已入 target/tmp）。
- 固件首跑缺 fixed_operation_operand_width（闭包清单漏被调）——oracle
  E2031 当场抓获，补全。

## 4. 验证与预算

- census 终值：flipped=90 diffs=0 total=128；HIST CALL-SHAPE 37 /
  GWPACK-OTHER 1；探针副本按纪律删除。
- `--test selfhost_compiler`：**73/73 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （73/73 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
