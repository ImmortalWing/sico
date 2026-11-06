# STEP-0361 — M22: concat 实参 `u64.to_text` 落地 + 两处潜伏 span 缺陷正解修复，census 72/127、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，机器族自降片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**承接 STEP-0360 登记的正解方向，三件套落地**（parser.sico，+~150 行）：

- **缺陷 A 正解**：gw 机器 `gcursor` 改从声明冒号后起（
  `find_text_from(":", fn_start)` 的下一个字节）——语句关键字搜索不再命中
  函数名内子串（straight_let/chained_let 的 "let"、while_call_rhs_packed
  等的 "while" 同类全消）；0360 已证该改动对既有冻结语料零行为变化；
- **缺陷 B 正解**：新 helper `gw_find_code_text`（逐命中扫描 + 字符串/
  `//` 注释跟踪 + 反斜杠转义回溯），替换**全部 26 处**
  `find_text_from(src, "end function", fn_start)`——机器体内
  `find_text_from(src, "end function", fn_start)` 自身源代码文本的字面量
  不再使 fn_end 提前（STEP-0360 的 token 走线方案已证伪弃用；本方案对无
  字面量函数零行为变化〔首个命中即返回〕）；
- **to_text 门重落地**：`gw_text_concat_packed` 实参派发新增
  `sico.u64.to_text` 分支（镜像 utf8_decode 委托
  `gw_intrinsic_call_packed`；STEP-0358/0359 隔离 byte-exact 已证）。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_concat_to_text_and_span_fixes_byte_exactly`——
`probe_let_span`（名含 "let" + 体内含 "end function" 字面量 + to_text
concat 实参，一片三钉）字节全等 + to_text 多余操作数 CALL-SHAPE 守卫 +
拒绝后恢复。

## 2. 前沿更新

- 依赖闭包 census **68/126 → 72/127、0 DIFF**（分母 +1 = 新 helper；
  CALL-ARGUMENT 36→7、GWCOND/GWPACK-OTHER 清零、EXPRESSION 14→32 健康
  级联）；三户原 CALL-ARGUMENT 拒绝户（list_get_match_ir /
  chained_let_function_ir / straight_let_function_ir）全部推进至各自
  真实下一门（EXPRESSION），不再以 DIFF 形式半开。
- 下一刀主线：gw 机器族自降——三户 if 条件用户调用形状的 EXPRESSION 门
  （0358 登记的嵌套调用实参随后按同法复测）+ CALL-ARGUMENT 7 户 +
  CALL-SHAPE 14 户。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0357 终值（68/126、0 DIFF）后动刀；每个
  子改动立即 census（0360 教训）。
- 微复现：probe_let_span（三缺陷同体）byte-exact；c1/c2f 隔离形状
  复核不变。
- `--test selfhost_compiler`：**68/68 绿**（含新固件）。
- 回归：fmt / validate-step-0262 / validate-step-0245（含 runner clippy，
  借主仓 msys2 gcc）/ report-m22-canary / validate-step-0124 -SelfTest /
  git diff --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=72 diffs=0 total=127；HIST
  CALL-ARGUMENT 7 / CALL-SHAPE 14 / EXPRESSION 32 / NON-DOMAIN 1 /
  STATEMENT 1。
- R3 连续停滞数保持 0（parser 覆盖 68→72 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
