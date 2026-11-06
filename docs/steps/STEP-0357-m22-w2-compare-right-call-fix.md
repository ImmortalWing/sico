# STEP-0357 — M22: 右操作数用户调用分支闭环（gcp_r_done 漏置修复）+ 0356"words 截断"发现更正，chained_operand_check 翻绿

> - 状态：implementation complete locally（本机全量回归见 §3；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，诊断纪律片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**0356 发现更正（文档-证据一致性修复）**：STEP-0356 登记的"callee-last 模块
机器上下文 words 截断"经本片根因审查判定为**探针脚本工件误判**——
`mkprobe2.sh` 的 sed 改写未生效，S1*/cc* 系列探针文件实际只含探针本体、
未带 callees 前缀（probe_cc9x.sico 实测 257B 仅探针；scalar_ir 入口插桩
实测 n=86 恰对应 257B 输入的全量词数）。scalar_ir 与全部机器全程喂全量
words，无截断行为。误判根源 = 探针生成脚本未校验输出文件内容；本片起
探针脚本恢复显式 `cat callees.inc` 前缀并对输出文件做字节数 sanity。

**真实修复（parser.sico `gw_compare_packed` 一行）**：0356 落地的用户调用
右操作数分支**漏置 `gcp_r_done = 1`**——分支成功设置 `gcp_close_idx` 后未
关门，共享 width 收尾（`if gcp_r_done == 0` 守卫内）对调用形状重跑
`fixed_operation_operand_width`（对 `u64_list_at` 原子返回 1 → close_idx
指到 `(` → 形状校验拒 EXPRESSION）。补 `set gcp_r_done = U64.literal(1)`。
0356 census 未捕获（无翻绿固件行走该分支）；本片微复现 ccR（右操作数
用户调用条件）EXPRESSION → 修复后 **byte-exact**，census 级联证实。

**固件**（selfhost_compiler.rs，同 scoped 变更）：新测试
`sico_compiler_lowers_gw_compare_user_call_right_operand_byte_exactly`——
`U64.less_than(n, u64_list_at(rs, b, U64.literal(0)))` 右操作数调用条件
字节全等 + 四参畸形 CALL-ARITY 拒绝守卫（实按传播身份登记）+ 拒绝后恢复。

## 2. 前沿更新

- **chained_operand_check 翻绿**（0356  EXPRESSION 户）；依赖闭包 census
  **67→68/126、0 DIFF**。
- histogram 健康流动：EXPRESSION 24→14、CALL-ARGUMENT 34→36、**CALL-SHAPE
  0→7**（argument_* 族 6 户 + environment_call_arguments 从 EXPRESSION 推进
  至真实下一门）。
- `gw_deferred_tail_json` 推进至 CALL-ARGUMENT（json concat 发射链的下一门，
  真实机器缺口，非工件）。
- 下一刀主线：gw 机器族自降（gw_rhs_packed/gw_condition_packed 等
  CALL-ARGUMENT 34 户被调链）+ argument_* 族 CALL-SHAPE 门。

## 3. 验证与预算

- 基线：本片开工 census 复跑 0356 终值（67/126、0 DIFF）后动刀。
- 微复现：ccR 右操作数调用条件修复前 EXPRESSION → 修复后 byte-exact
  （oracle 直比）；scalar_ir 入口插桩 86=257B 全量词数（工件实锤）。
- `--test selfhost_compiler`：**67/67 绿**（含新固件）。
- 回归：fmt / validate-step-0262 / validate-step-0245（含 runner clippy，
  借主仓 msys2 gcc）/ report-m22-canary / validate-step-0124 -SelfTest /
  git diff --check 本片重跑全绿（数字见 STATUS）。
- census 终值（探针按纪律删除）：flipped=68 diffs=0 total=126；HIST
  CALL-ARGUMENT 36 / EXPRESSION 14 / CALL-SHAPE 7 / STATEMENT 1。
- R3 连续停滞数保持 0（parser 覆盖 67→68 增长）；M22 整体 NO-GO 不变
  （S6/S7 未进入）。
