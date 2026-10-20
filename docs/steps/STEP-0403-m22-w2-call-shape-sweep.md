# STEP-0403 — M22: CALL-SHAPE 族续清——`to_text(next_index(..))` 实参提升，gw_ok_wrap 翻绿，census 110→111/130、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，CALL-SHAPE 族续片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**已知坏形 sweep**（gw_ok_wrap 最后一处 + gw_call_chain 同款一处）：
`set X = concat(.., to_text(next_index(..)))` 实参内嵌用户调用提升为
`let __tt = next_index(..)` + `set X = concat(.., to_text(__tt))`
（gw concat 打包器的 to_text 分支只收原子实参，0361 模板外的新变体）。

**门连锁记录**：gcc/gca 的 CALL-SHAPE 清后暴露 0399 深度 2 族
（顶层嵌套 if 带 return → RETURN-COUNT）；gw_rhs/starts_with 走 STATEMENT。
下一片回到 0400 的顶层化手术（逐户巨大 if 反守卫+抽辅助）。

## 2. 前沿更新

- census **110/130 → 111/130、0 DIFF**；**gw_ok_wrap_rhs_packed 翻绿**
  （0397 遗留末户清零）；CALL-SHAPE 12→7（5 户级联至 RETURN-COUNT 2 /
  STATEMENT / trap）；无回归。
- 下一刀主线：残余户逐户顶层化手术（gcc/gca/gw_rhs/starts_with）+
  GWPACK-OTHER 3 + NON-DOMAIN 7。

## 3. 事故与登记

- 无新事故；模式库累积（嵌套实参族：next_index(matching_close)、
  to_text(next_index)、to_text(user_call)、to_text(const)、
  bytes.length 三形）已覆盖本轮全部命中。

## 4. 验证与预算

- census 终值：flipped=111 diffs=0 total=130；HIST CALL-SHAPE 7 /
  GWPACK-OTHER 3 / NON-DOMAIN 7 / RETURN-COUNT 2；探针副本按纪律删除。
- 固件：无新测试（gow 翻绿由 0384 sort/join 测试的 sort 路径 +
  census 字节对拍共同见证）；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
