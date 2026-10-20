# STEP-0410 — M22: gw_rhs 双臂抽辅助落地——`if gws_multi` 315/65 行两臂各成 helper，census 115→116/134、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌手术片 3——主门首破）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**gw_rhs_packed `if gws_multi:` 双臂抽辅助**（0399/0404 方法的完整应用）：
- main 收缩为两行委托（`if gws_multi: return gw_rhs_multi_pack(ctx) end if`
  + `return gw_rhs_single_pack(ctx)`，11 参上下文）；
- 新 helper `gw_rhs_multi_pack`（315 行臂 + 共享累加器 + 尾装配）与
  `gw_rhs_single_pack`（65 行臂同构）（+2 分母）；
- 跨臂 scratch 变量零初始化修复（`__h218`/`__h229`——抽辅助的隐藏成本
  登记：arm 内 set 的 scratch 若在臂外声明，helper 需自带零初始化 let）。

## 2. 前沿更新

- census **115/132 → 116/134、0 DIFF**（+1 = gw_rhs_packed 本体翻绿，
  +2 分母 = 两 helper）；无回归。
- 门连锁：STATEMENT 桶一户翻绿、一户换挡入 GWPACK（hist GWPACK-OTHER
  12→13）；下一层门 = helper 内嵌套 ERR-guard（`if starts_with(pack,
  "ERR:"): return pack` 居于 param-if 内 = 深度 2）逐段再抽或提升。
- 下一刀主线：gw_rhs_multi_pack 内部 ERR-guard 逐段处理（预计再解 2-3
  层）→ GWPACK-OTHER 桶（13 户）→ 4 户 trap。

## 3. 事故与登记

- 行号硬编码两次错位（0408 后行漂移）——结构手术一律程序化定位
  （特征行 next() 扫描），禁硬编码行号。
- `__`-前缀 scratch 的 set-without-let 报错字节定位 + 函数域 set/let
  配对扫描（含 `__` 变量）定位两例——已入检查清单。

## 4. 验证与预算

- census 终值：flipped=116 diffs=0 total=134；HIST GWPACK-OTHER 13 /
  CALL-SHAPE 1 / NON-DOMAIN 4；探针副本按纪律删除。
- 固件：无新测试（翻绿由 census 字节对拍见证）；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
