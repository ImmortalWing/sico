# STEP-0389 — M22: gw RHS 机器学 `sico.list.empty[U64]()` + gcc 单返回续，census 99→101/129、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，gw 机器族零实参 intrinsic 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**微复现**：`let x = sico.list.empty[U64]()` 在 while 上下文拒 GWPACK-OTHER
（gw_rhs_packed 无调度直落 EXPRESSION）；嵌套用户调用 let 对照收。
该形状全库 41 处（全部机器函数的 cell_locals 初始化），是 GWPACK-OTHER
残余户 user_call_expression_json 的公共门。

**机器扩族**：新打包器 `gw_list_empty_rhs_packed`（+~45 行，镜像
gw_map_empty_rhs_packed 零实参模式：8 词位守卫 `[ U64 ] ( )` 尾零、
emit count=1/value=base/kind=list-u64/pieces=intrinsic_json）+
gw_rhs_packed 一行调度。**首版偏移 bug**：词位按 receiver+1 起算（实应
+5 = `[`），守卫全 trip CALL-SHAPE——probe 对拍当场抓获，修后
IDENTICAL。

**源码整形**：gw_call_chain_packed 函数尾 if/else 双臂 return 改单返回
（0387 gic 模板；门推进至更深 CALL-SHAPE）。

## 2. 前沿更新

- census **99/128 → 101/129、0 DIFF**（+1 = 新打包器入分母）；
  **user_call_expression_json 翻绿**（GWPACK-OTHER 2→1，独苗
  control_if_function_ir）；无回归。
- 下一刀主线：RETURN-COUNT 19 户（机器族本体，双臂 return/多返回 while
  逐户重构）+ CALL-SHAPE 7 户 + control_if_function_ir。

## 3. 事故与登记

- 词位偏移按字符直觉写（receiver+1）而非数 token（+5）——gw 打包器
  守卫一律以 next_index 链数词位，新打包器先写 token 清单再编码。
- bash heredoc 内 `\\n` 被吞为真换行致 Sico 字符串未终结——脚本生成
  含转义的 Sico 源时一律 chr(92) 拼接或 Write 工具直写。

## 4. 验证与预算

- census 终值：flipped=101 diffs=0 total=129；HIST RETURN-COUNT 19 /
  CALL-SHAPE 7 / GWPACK-OTHER 1 / NON-DOMAIN 1；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_gw_list_empty_rhs_byte_exactly`
  （单元形状字节全等 + 元素类型守卫 CALL-TYPE + 拒绝后恢复）。
- `--test selfhost_compiler`：**77/77 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （77/77 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
