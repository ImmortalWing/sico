# STEP-0370 — M22: gw 机器入口区 match 支持（gw_match_open_pack entry 门摘除），STATEMENT 拒绝类清零

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，机器族自降片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**0368/0369 工件更正（本片起点）**：前两片的 n 系探针全部漏写
`end function` 尾行（mkprobe 实参体末尾遗漏）——oracle 如实 E1001 拒绝、
guest 前端宽容放行并产生误导性 STATEMENT——**"三态谱系"与"入口区
match 门"结论作废**（0365 文档已同步更正；0364 gdt 值损坏与 casm trap
探针显式带 end function，不受影响仍成立）。探针纪律补一条：
**探针文件先过 `sico check` 再驱动结论**。

**真实修复（parser.sico 一处 −3 行）**：`gw_match_open_pack` 的
entry 区显式拒绝（`if is_word(region, "entry"): return STATEMENT`）
摘除——入口区（首个 while 之前）的 checked-arithmetic match 走既有
prologue/臂/join 机制即 byte-exact（**无次道门**：0369 的"次道门"系
工件）。header 区门保留（while 头内 match 仍 typed 拒绝）。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_entry_region_match_byte_exactly`——match→while
（t3）与 while→match（t4）两序字节全等 + 族外 op（bit_and）CALL-TARGET
守卫 + 拒绝后恢复。

## 2. 前沿更新

- 依赖闭包 census **76/127、0 DIFF**；**STATEMENT 拒绝类 1→0 清零**
  （EXPRESSION 26→27 一户推进属健康级联）。
- 入口区 match = 源码函数"先校验参数再进循环"的普适形状（parser 族
  遍布）——本片为该形状族解锁，后续机器本体的 EXPRESSION 门预计随
  之松动。
- 下一刀主线：gw 机器族本体自降（EXPRESSION 27 户）+ 0364 嵌套调用
  实参值编号专项（gdt/casm 两现场对照）+ casm trap 修复。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0363/0364 净态（76/127、0 DIFF）；
  每个子改动立即 census。
- 探针纪律：t3/t4 先 `sico check` 通过再进差分。
- `--test selfhost_compiler`：**70/70 绿**（含新固件两序字节全等）。
- 回归：fmt / validate-step-0262 / validate-step-0245（含 runner
  clippy，借主仓 msys2 gcc）/ report-m22-canary / validate-step-0124
  -SelfTest / git diff --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=76 diffs=0 total=127；HIST
  CALL-ARGUMENT 8 / CALL-SHAPE 14 / EXPRESSION 27 / GWPACK-OTHER 1 /
  NON-DOMAIN 1。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
