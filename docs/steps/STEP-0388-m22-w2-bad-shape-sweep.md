# STEP-0388 — M22: 已知坏形状全库清扫——`bytes.length(encode(..))` 与嵌套 concat 实参提升，census 98→99/128、0 DIFF，CALL-SHAPE 28→7（21 户级联至下一门）

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，坏形状清扫片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**清扫两型**（脚本 target/tmp/sweep2.py / sweep1.py，均单行 set/let 限定，
语义等价提升）：
- sweep2（71 处）：`set|let X = sico.bytes.length(sico.text.encode(ARG))` →
  `let __enc_FN_N = sico.text.encode(ARG)` + `set|let X = sico.bytes.length(__enc_FN_N)`
  （bytes.length 调度在 0386 已入 gw，但 intrinsic 实参形状被 arity 守卫拒；
  提升后 cell 实抗原型直过）。覆盖 gw/straight 全部 set/let 上下文
  （过提升无害：oracle 同源自编译，字节恒等）。
- sweep1（53 处）：实参位嵌套 concat（`f(.., concat(A, concat(..)), ..)`）
  内层提升为 let cell。sweep1 census 零翻绿零回归（如实登记：该形状当前
  未卡任何户，保留以降低未来 CALL-SHAPE 面）。

**固件**（selfhost_compiler.rs）：full-source 守卫燃料 40B→120B
（120B 实测 ~4s 达同一 CALL-SHAPE typed 拒绝）；新测试
`sico_compiler_lowers_argument_instructions_closure_byte_exactly`
（25 函数闭包字节全等，sweep 金丝雀翻绿户；闭包清单以
extract_closure.py 输出为准——本片两次手敲清单被 oracle E2031 抓获）。

## 2. 前沿更新

- census **98/128 → 99/128、0 DIFF**；**CALL-SHAPE 28→7**（簇瓦解）；
  21 户级联推进：RETURN-COUNT 19（新主导族，R3 单返回纪律下一片批量适用）、
  GWPACK-OTHER 2、NON-DOMAIN 1（casm trap 登记缺陷）。
- 翻绿：argument_instructions（sweep 金丝雀）。
- 下一刀主线：RETURN-COUNT 19 户（逐户单返回重构，gic 模板已立）+
  CALL-SHAPE 残余 7 户 + GWPACK-OTHER 2 户。

## 3. 事故与登记

- 全库清扫必须配 census 回归对拍（本片三轮均无回归，纪律保持）。
- 手敲固件闭包清单不可靠（两度缺被调）——一律 extract_closure.py 生成。
- **local-bounds 头寸地板**（本片新血泪）：sweep 给 general_while_function_ir
  +5 locals（253/256，破 0343 设的头寸地板 busiest+8≤256）→
  selfhost_local_bounds 守卫当场抓获；按该测试自身纪律（"gw 走查函数只抽
  辅助不长个"）整函数从 sweep 前备份回退（revert_fns.py），census 无翻绿
  损失无回归；教训 = 全库清扫脚本必须跳过 general_while_function_ir（或任何
  近地板函数），sweep 前先看 local-bounds 排名。

## 4. 验证与预算

- census 终值：flipped=99 diffs=0 total=128；HIST RETURN-COUNT 19 /
  CALL-SHAPE 7 / GWPACK-OTHER 2 / NON-DOMAIN 1；探针副本按纪律删除。
- `--test selfhost_compiler`：**76/76 绿**（含新固件 + 燃料更新）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （76/76 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
