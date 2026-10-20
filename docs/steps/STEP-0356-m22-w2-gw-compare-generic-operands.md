# STEP-0356 — M22: gw 条件右操作数泛型 `sico.list.length[U64]` + 用户调用分支，gw_unreachable_terminator 翻绿，GWCOND-EXPRESSION 类清零

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，gw 机器族自降片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**诊断**：0355 登记的两个新辅助的实测门微复现——`gw_unreachable_terminator`
拒 GWCOND-EXPRESSION（while 头条件右操作数 `sico.list.length[U64](gunreach)`
泛型宿主调用落在族外：`gw_compare_packed` 右操作数只收非泛型
`sico.text.length`〔且限 string 参数〕/参数/cell/字面量）；`gw_deferred_tail_json`
拒 EXPRESSION（if 条件右操作数用户调用 `u64_list_at(...)` 无分支）。
oracle 转储钉死目标：intrinsic 名携带泛型（`sico.list.length[U64]`）、参数
实参零指令直引（`"arguments":[0]`）。

**修改**（parser.sico `gw_compare_packed` 右操作数一处，+~130 行）：

- 新泛型分支：`sico.list.length [ U64 ] ( <param|cell> )` 右操作数识别
  （族外泛型类型 CALL-TYPE；无泛型 Falls through 如实拒〔while 头路径映
  GWCOND-EXPRESSION，Fixture 按实登记〕；cell 实参走 read_local+intrinsic 双
  指令、参数实参零指令——`gw_intrinsic_call_packed` 内部硬编 +5 走位不支持
  泛型接收者，故本分支直接发射）；`gcp_r_done` 门保护既有 width 收尾不被
  泛型/调用形状误算；
- 新用户调用右操作数分支：镜像左操作数既有路径（`function_declaration_index`
  → `while_call_rhs_packed` → `call_instruction_json`，尾部 `)` 校验）；
- 左操作数、非泛型 text.length、param/cell/字面量路径零行为变化。

**途中诊断发现（登记下片线索）**：callee-last 模块中机器上下文 words 截断
（探针在 callees 后时 `sico.list.length(words)`=86 ≈ 单函数量，被调声明不
可见；同一探针置于模块首位时 =207 全量、被调解析成功）。该上下文行为同时
影响左/右用户调用操作数的 callee 解析（`gw_deferred_tail_json` 与 cc* 微复
现同征），非本片引入（0355 基线同征），根因待下片用 words 来源审查定位。

> **STEP-0357 更正（2026-10-12）**：上述"words 截断"发现为**探针脚本工件
> 误判**——`mkprobe2.sh` 的 sed 改写未生效导致 S1*/cc* 系列探针文件实际
> 只含探针本体、未带 callees 前缀（probe_cc9x.sico 实测 257B 仅探针），
> "被调声明不可见"由此产生，非机器行为。scalar_ir 与机器全程喂全量 words
> （scalar_ir 入口插桩实测 n=86 对应 stdin 257B 的探针本体）。0356 本片
> 的真实遗留 = 用户调用右操作数分支漏置 `gcp_r_done`，共享 width 尾巴对
> 调用形状重跑拒 EXPRESSION（0357 已修复并字节实证）。文档与证据不一致
> 处以本更正为准。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_condition_generic_list_length_byte_exactly`——
`gw_unreachable_terminator` 原文 + `u64_list_at`/`next_index` 闭包字节全等 +
两拒绝守卫（`length[Text]` → CALL-TYPE、无泛型 → GWCOND-EXPRESSION）+ 拒绝
后恢复正例。

## 2. 前沿更新

- **gw_unreachable_terminator 翻绿**（微复现 + census 闭包双证实）；依赖闭包
  census **66→67/126、0 DIFF**。
- **GWCOND-EXPRESSION 1→0 清零**。现 histogram = CALL-ARGUMENT 34 /
  EXPRESSION 24 / STATEMENT 1（gut 翻绿后 EXPRESSION 户原位由级联补齐，
  净数不变属健康流动）。
- `gw_deferred_tail_json` 仍拒 EXPRESSION——callee-last words 截断上下文
  （见 §1 诊断发现）为其实际下一门；下片先攻 words 上下文根因（预计一次性
  解锁大片区：EXPRESSION 24 户中大量机器族 if 条件用户调用形状）。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0355 终值（66/126、0 DIFF）后动刀。
- `--test selfhost_compiler`：**66/66 绿**（含新固件正例 + 两拒绝守卫 + 恢复）。
- 回归：fmt / validate-step-0262 / validate-step-0245（含 runner clippy，借
  主仓 msys2 gcc）/ report-m22-canary / validate-step-0124 -SelfTest / git
  diff --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=67 diffs=0 total=126；HIST
  CALL-ARGUMENT 34 / EXPRESSION 24 / STATEMENT 1。
- R3 连续停滞数保持 0（parser 覆盖 66→67 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
