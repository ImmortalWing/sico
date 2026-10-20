# STEP-0406 — M22: census 燃料修正（20B→60B，7 户脱 FuelExhausted 面具）+ 6 站点嵌套链摊平，census 115/132、0 DIFF 维持（门推进无翻绿，partial 如实登记）

> - 状态：implementation complete locally（partial 如实登记；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，探针基建修正 + 嵌套链片）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**census 探针燃料修正**：scratch_census_probe.rs 20B→60B（注释登记）——
0404/0405 后 7 户（gw_starts_with/gw_condition/gw_slice_subject/
gw_match_open/general_while/control_while/fixed_guard_chain/scalar_ir）
被 20B 燃料面具为 FuelExhausted；60B 后全部显真身（CALL-SHAPE 10 /
GWPACK-OTHER 3 / NON-DOMAIN 4）。FuelExhausted 不再出现在 histogram。

**6 站点 `matching_close(words, next_index×N(...))` 深嵌摊平**
（gtc×5 + stc×1；N=6 实测——首版按 5 解析致 MissingCallClose，按
`next_index(` 计数通用化修复；脚本生成后 host check + build 双重闸）。
门连锁：gw_text_concat_packed CALL-SHAPE→STATEMENT。

## 2. 前沿更新

- census **115/132、0 DIFF 维持**（无翻绿无回归；门形推进如实登记）。
- 真前沿（燃料修正后）：CALL-SHAPE 10 / GWPACK-OTHER 3 / NON-DOMAIN 4
  = 17 户，逐户手工模式续做（0404 纪律）。

## 3. 事故与登记

- MissingCallClose 两轮（N 数错算/v0 含逗号）——嵌套链脚本必须以
  出现计数 + host check 双闸，不得按假设层数生成。

## 4. 验证与预算

- census 终值：flipped=115 diffs=0 total=132；HIST CALL-SHAPE 10 /
  GWPACK-OTHER 3 / NON-DOMAIN 4；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
