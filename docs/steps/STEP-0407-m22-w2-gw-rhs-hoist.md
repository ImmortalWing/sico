# STEP-0407 — M22: gw_rhs 六处 to_text 提升——CALL-SHAPE 族近乎清空（10→1），前沿整体换挡 GWPACK-OTHER 12，census 115/132、0 DIFF 维持

> - 状态：implementation complete locally（partial 如实登记；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，CALL-SHAPE 族续片 2）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**gw_rhs_packed 六处 `concat(.., to_text(next_index×N(..)))` 提升**
（手工定界脚本，函数域内 diff 审查；0405 批跑灾难后新纪律首用：作用域
限定单函数 + host check/build 双闸 + census 复拍）。

**前沿换挡**（本片最重要观测）：CALL-SHAPE 10→1（gw_rhs 提升解锁 9
户），但 9 户整体涌入 GWPACK-OTHER（3→12）——gw_rhs 的下一道门
（其自身闭包显 STATEMENT）成为 12 户公共守门人。**修复 gw_rhs 本体的
STATEMENT（724 行深嵌结构手术）= 下一次大片级联的单点钥匙**。

## 2. 前沿更新

- census **115/132、0 DIFF 维持**；HIST GWPACK-OTHER 12 / NON-DOMAIN 4 /
  CALL-SHAPE 1；无回归。
- 下一刀主线：gw_rhs_packed 本体顶层化手术（0399/0404 方法，724 行
  逐段）；预计解锁 ~12 户。

## 3. 事故与登记

- 首版脚本文件拼接 bug（lines[start:end+1] = out 错位）致
  UnexpectedTopLevel——整文件恢复后修正为全量重建；函数域脚本必须
  全量重建写回，禁止切片赋值。

## 4. 验证与预算

- census 终值：flipped=115 diffs=0 total=132；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
