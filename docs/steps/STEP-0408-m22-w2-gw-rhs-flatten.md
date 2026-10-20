# STEP-0408 — M22: gw_rhs 单函数 18 处 fall-through 展开（span 限定实证）——门未清，深层结构待 0404 式 elif 手术，census 115/132、0 DIFF 维持

> - 状态：implementation complete locally（partial 如实登记；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌手术片 2）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**gw_rhs_packed 单函数 flatten_if.py 应用**（18 处 fall-through 展开；
0405 灾难后新纪律全步照办：单函数作用域 + diff hunk 落点审查（全部在
gw_rhs span 9710-10439 内，首 hunk @9851）+ host check/build 双闸 +
census 复拍）。

**结果**：gw_rhs 仍 STATEMENT——其门根**非 fall-through eligible ifs**，
而是 elif 链 / 嵌套 ERR-guard / header 区结构（0404 gca 已证的下一层
手术）。18 处展开为后续 elif 抽辅助铺平了地基（消除一层嵌套）。

## 2. 前沿更新

- census **115/132、0 DIFF 维持**；HIST GWPACK-OTHER 12 / NON-DOMAIN 4 /
  CALL-SHAPE 1 不变；无回归。
- 下一刀主线：gw_rhs elif 链 / 嵌套 ERR-guard 逐段抽辅助（0404 方法
  在 724 行函数上的系统应用，预计多片）；GWPACK-OTHER 12 户随 gw_rhs
  逐段解锁。

## 3. 事故与登记

- Windows python 不识别 MSYS /tmp（Span 审查首跑 FileNotFoundError）
  ——span 审查改用 bash diff hunk 行号比对（落点全部 ≥ span 起点）。

## 4. 验证与预算

- census 终值：flipped=115 diffs=0 total=132；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 5. STEP-0409 追加侦察（同日，源码净 = 0408 终态）

- census 探针燃料再升 60B→120B（与固件 full-source 守卫同档）；5 个
  FuelExhausted 户中 1 个显真身，残余 4 户（gw_condition /
  general_while / fixed_guard_chain / scalar_ir）在 256MiB arena +
  120B 燃料下仍为 **wasm trap**——最深端真身（512MiB arena 需求或
  真缺陷，逐户重定）。
- **16 红户完全收敛为三桶**：9 户 STATEMENT（gw 机器族深嵌，gw_rhs
  为共享主门）+ 3 户 GWPACK-OTHER（control_if/control_while/scc）+
  4 户 trap（最深 walk 函数）。下片顺序：gw_rhs 巨臂抽辅助（解锁 9
  户）→ control_if/scc → 4 户 trap 重定。
