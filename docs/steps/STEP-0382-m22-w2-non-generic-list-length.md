# STEP-0382 — M22: gw 比较机器非泛型 `sico.list.length` 右操作数，EXPRESSION 拒绝类 27→0 清零，census 82/127、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，机器族自降片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**诊断（0381 续）**：`operand_nonparam_kind`（EXPRESSION 户 canary）截体五变体定门——
A（去尾调用）/B（臂 return 改常量）/D（加 while 改路由）/E（纯 U64 调用臂）
均仍拒、F（条件改 `U64.equal(cell, 0)`）**翻绿** → 门 = **if 条件右操作数的
非泛型 `sico.list.length(cell_names)`**——gw 比较机器右操作数族（STEP-0356
落地泛型 `[U64]` 形态）缺裸形态（parser 族守卫的普遍形状）。R3-if 形状的
gw 本体（r3/r3b 探针含用户调用臂 return）经实证**早已 byte-exact**，
0381 的"R3 收口"假设就此修正为操作数族缺口。

**修改**（parser.sico `gw_compare_packed` 右操作数一处，+~45 行）：泛型
`[U64]` 分支的 `[` 守卫新增 else 裸形态——`sico.list.length(<param|cell>)`
直发 intrinsic（名无泛型后缀；参数实参零指令、cell 实参 read+intrinsic
双指令），`gcp_r_done` 门与族外拒绝（CALL-SHAPE/CALL-ARGUMENT）保持。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_compare_non_generic_list_length_byte_exactly`——
`operand_nonparam_kind` 闭包（8 函数、List[Text] 上下文）字节全等 + 多余
实参 CALL-SHAPE 守卫 + 拒绝后恢复；0356 泛型固件的 no-generic 守卫按新
行为改写（裸形态在 U64-list 上下文 oracle 不接受，该片正例为 compile-only，
字节全等由 List[Text] 上下文本固件承担——如实登记）。

## 2. 前沿更新

- 依赖闭包 census **76/127 → 82/127、0 DIFF**（+6 翻绿）；**EXPRESSION
  拒绝类 27→0 清零**（本片即该类最后公共门）；CALL-ARGUMENT 8→27 /
  CALL-SHAPE 14→16 为健康级联（各户推进至真实下一门）。
- 下一刀主线：CALL-ARGUMENT 27 户（gw_rhs_packed/gw_intrinsic_call_packed/
  scalar_ir 等机器本体自降——翻任一即大片连锁）+ CALL-SHAPE 16 户。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0370 终值（76/127、0 DIFF）后动刀；每个
  子改动立即 census。
- 微复现：五变体截体（A/B/D/E/F）+ r3/r3b R3 对照；onk 闭包 oracle
  字节全等。
- `--test selfhost_compiler`：**71/71 绿**（含新固件 + 0356 固件改写）。
- 回归：fmt / validate-step-0262 / validate-step-0245（含 runner clippy，
  借主仓 msys2 gcc）/ report-m22-canary / validate-step-0124 -SelfTest /
  git diff --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=82 diffs=0 total=127；HIST
  CALL-ARGUMENT 27 / CALL-SHAPE 16 / GWPACK-OTHER 1 / NON-DOMAIN 1。
- R3 连续停滞数保持 0（parser 覆盖 76→82 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
