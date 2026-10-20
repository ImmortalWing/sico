# STEP-0411 — M22: gw_rhs_multi_pack 首处嵌套 ERR-guard 手术（at-dispatch 抽辅助）——census 116→117/135、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌手术片 4）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**嵌套 ERR-guard 的正确变换定型**（重要方法学）：
- **提升纯调用不可行**（callee 可报错时，提升改变错误传播路径 = 语义
  变化，实锤放弃）；
- 正确变换 = **分支对抽辅助**：`if cond: let p = calleeA / guard / consume
  else: let p = calleeB / guard / consume end if` → helper（内部分支各
  自调用+守卫+消费，四值经四行 pack 串返回），caller 单守卫 + split_lines
  解包。语义严格等价（每分支各调各的、错误原样传播）。

**首应用**：`gw_rhs_at_dispatch` 辅助落地（bytes.at 双变体分支对），
caller（gw_rhs_multi_pack）收缩为单守卫 + 解包 8 行。multi_pack 内
余 7 处同类 guard 待逐片同法。

## 2. 前沿更新

- census **116/134 → 117/135、0 DIFF**（+1 翻绿 +1 分母）；无回归。
- 节奏定型：每片一处 guard 手术 ≈ +1 绿；gw_rhs_multi_pack 余 6 处 +
  GWPACK-OTHER 13 + NON-DOMAIN 4。

## 3. 事故与登记

- 容器 if 的 end if 定位首跑误取内层 guard 的 end if（块断言拦截）——
  容器边界按**同缩进** `end if` 定位，禁 next() 裸扫。

## 4. 验证与预算

- census 终值：flipped=117 diffs=0 total=135；HIST GWPACK-OTHER 13 /
  CALL-SHAPE 1 / NON-DOMAIN 4；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
