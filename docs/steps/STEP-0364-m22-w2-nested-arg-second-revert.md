# STEP-0364 — M22: 嵌套调用实参分支复测（第二次回退——span 修复非根因，值编号损坏待专项）

> - 状态：**reverted（第二次）**——0361 span 修复后复测 0358 的
>   `gw_intrinsic_call_packed` 嵌套调用实参分支，**同一字节发散原样复现**
>   （gdt result-60 区域 `?` + `concat args [60,0]` + 值编号跳到 425）；
>   回退后 census 76/127、0 DIFF、histogram 与 STEP-0363 终值逐格一致
> - 里程碑：M22 compiler self-host（S4 lowering 片，负结果复测片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 复测经过与结论

**假设**：0358 的嵌套分支 3 例 DIFF 由 span 缺陷 A/B 引起——0361 已修复
A/B，本片重落地该分支（gic 头放宽 + 嵌套调用实参分支，含 0358 全部
代码）并立即验证。

**结果**：gw_deferred_tail_json 从 CALL-SHAPE 变为 exit 0 但**字节发散
与 0358 完全相同**（read 60 后 `?`、`concat arguments [60,0]`、result 跳到
425）——**span 假设证伪**；根因在嵌套分支的值编号/计数消费与外层
`gw_text_concat_packed` 实参循环的相互作用（隔离形状 c2f byte-exact、
多语句深上下文损坏，与 0363 gfc 的"同位置 set 门"同类：深上下文状态
机未建模形状）。按 0-DIFF 纪律整片回退；净验证 76/127、0 DIFF、
histogram 逐格一致。

**登记（下片专项输入）**：
- 发散特征三件套（`?` = parts 行数不足 / args 含 0 = decimal("?") /
  result 跳号 = gnext 被污染）指向**包行数约定**或 **gic 与 gtc 的值
  交接**在第三道语句后的错位；建议专项以 gdt 截体（保留至第二条
  to_text 语句）二分，对照 oracle 查 gtc_value/gtc_advance 的逐语句
  演进；
- 0358/0364 两次回退同一分支：该分支隔离正确、上下文损坏，下次复测
  前必须先解值编号根因，勿第三次直落。

## 2. 验证

- 回退后：component 构建绿；census 76/127、0 DIFF；HIST CALL-ARGUMENT 8 /
  CALL-SHAPE 14 / EXPRESSION 26 / GWPACK-OTHER 1 / NON-DOMAIN 1 /
  STATEMENT 1 与 0363 终值逐格一致；无残留（gic_callee/gic_call_*
  零残留）；git diff --check 净。
- 本片未增减固件（无落地变更）。
