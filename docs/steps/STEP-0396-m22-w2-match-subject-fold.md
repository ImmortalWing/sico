# STEP-0396 — M22: match 主语形 token 尾折叠（69 站点全库）——checked_arithmetic_match_ir 翻绿，census 103→104/129、0 DIFF，CALL-SHAPE 3→1

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，主语形折叠片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**69 站点全库折叠**：`let __enc_FN_N = encode(ARG)` + `(set|let) __hX =
bytes.length(__enc)` + `match checked_add(off, __hX)` 三段式折叠为单一
`match checked_add(off, bytes.length(encode(ARG)))` 主语（0395 纪律的
机械化；casm/cam/casm_fb/gow 等机器函数的全部 sweep2 残留一次清完）。

**首版事故与修复**：折叠删除 `let __hX` 声明致 60 处 set-without-let；
修复脚本首版误对 `entries` 等真变量插零初始化（E2001 类型错抓获）——
**重做：零初始化仅限 `__h\d+` scratch**（5 处），build 过。

**casm 门槛推进实证**：casm 经折叠后 typed 门全清，走进 0365 登记的
wasm trap 缺陷类（上游值损坏族）——与 0365 预测一致，该户收口转入
casm trap 专项。

## 2. 前沿更新

- census **103/129 → 104/129、0 DIFF**；**CALL-SHAPE 3→1**（独苗
  gw_ok_wrap_rhs_packed）；checked_arithmetic_match_ir 翻绿；
  NON-DOMAIN 2→3（casm 等入 trap 类）；无回归。
- 下一刀主线：gw_ok_wrap（末户 CALL-SHAPE）+ casm trap 专项（0365
  登记）+ RETURN-COUNT 19 户机器族本体 + GWPACK-OTHER 2 户。

## 3. 事故与登记

- 机械折叠必须先盘点被消变量的声明职责（首声明折叠删除 → 下游 set
  悬空）；零初始化修复仅限 scratch 命名（`__h\d+`），真变量一律人检。
- 教训重申：sweep 类全库变换后立即 build + census 双对拍。

## 4. 验证与预算

- census 终值：flipped=104 diffs=0 total=129；HIST RETURN-COUNT 19 /
  CALL-SHAPE 1 / GWPACK-OTHER 2 / NON-DOMAIN 3；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_checked_arithmetic_match_ir_closure_byte_exactly`
  （31 函数闭包字节全等；清单 extract 生成）；full-source 前沿守卫改钉板
  0365 trap 类（UnreachableCodeReached 断言 + 注释登记"0365 修复后恢复
  typed 拒绝断言"）——折叠后全源走查在 casm 族 typed 门全清处触既定缺陷，
  守卫语义 = 前沿必须是该类 trap（任何他类 trap/typed 拒绝/干净退出均失败）。
- `--test selfhost_compiler`：**80/80 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （80/80 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
