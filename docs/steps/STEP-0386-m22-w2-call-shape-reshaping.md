# STEP-0386 — M22: CALL-SHAPE 簇源码整形七户翻绿——嵌套调用/const 实参提升 + bytes.length 入 gw 调度，census 90→97/128、0 DIFF，CALL-SHAPE 37→27

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，CALL-SHAPE 簇源码整形片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**门族定位**（CALL-SHAPE 簇逐户微复现，闭包级 extract+cmp_probe 对拍）：
- `gw_deferred_tail_json`：`to_text(u64_list_at(...))` 嵌套用户调用作 concat
  实参 → 三处提升为 let cell。
- `nested_intrinsic_return_ir`：`to_text(U64.literal(0))` const 实参形态
  gw concat 打包器不收（param/cell 收）→ const 提升为函数顶 let cell
  （nir_zero）。parser.sico 全库仅此 1 处 const 实参形态。
- `argument_instruction_count` 族（aic/argument_values/argument_segment_kind/
  argument_region_instruction_count 四户连带）：
  `let __h = text.length(user_call(...))` 提升（let 形态，0385 只收 set 形态）；
  `set x = next_index(matching_close(...), ...)` 嵌套调用拆 let+set。
- `gw_intrinsic_call_packed` 本体：嵌套 concat 实参提升、
  read_local_instruction_json 嵌套用户调用实参提升、
  `bytes.length` 入 gw_rhs_packed 调度（委托 gw_intrinsic_call_packed，
  ret u64）+ `encode` 实参提升——该户门从 CALL-SHAPE 推进至
  RETURN-COUNT（R3 族，下一片）。

**修改面**：parser.sico 全部源码整形（无一处机器手术除 bytes.length 一行
调度）；七户翻绿、两户推进、零回归。

## 2. 前沿更新

- census **90/128 → 97/128、0 DIFF**；CALL-SHAPE 37→27；新现
  RETURN-COUNT 2 户（R3 族回到台前）+ NON-DOMAIN 1 户（casm trap，
  0365 已登记缺陷类）；GWPACK-OTHER 残余 1 户（control_if_function_ir）。
- 翻绿七户：gw_deferred_tail_json、nested_intrinsic_return_ir、
  argument_instruction_count、argument_region_instruction_count、
  argument_values、argument_segment_kind、environment_call_arguments。
- 下一刀主线：RETURN-COUNT 族（R3 回潮，2 户）+ CALL-SHAPE 27 户
  （checked_arithmetic_* / user_call_expression_json / while_call_rhs_packed
  等）+ control_if_function_ir（GWPACK-OTHER 独苗）。

## 3. 事故与登记

- extract 闭包必须每次从当前 parser.sico 重新生成（陈旧提取文件两次
  误导定位）——extract_closure.py 已入 target/tmp 常备。
- 固件闭包清单两度缺被调（is_declaration_start / decimal_at_most /
  is_canonical_magnitude / is_string_token）——oracle E2031 当场抓获；
  纪律：固件闭包清单以 extract_closure.py 输出为准，不手敲。

## 4. 验证与预算

- census 终值：flipped=97 diffs=0 total=128；HIST CALL-SHAPE 27 /
  GWPACK-OTHER 1 / NON-DOMAIN 1 / RETURN-COUNT 2；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_argument_instruction_count_closure_byte_exactly`
  （14 函数闭包字节全等）。
- `--test selfhost_compiler`：**74/74 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （74/74 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
