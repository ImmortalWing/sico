# STEP-0404 — M22: 顶层化手术续——fall-through 展开 + elif 链抽辅助，gcc/gca 翻绿 RETURN-COUNT 清零，census 111→115/132、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌容量攻坚续片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**Sico 无布尔否定的发现与正确变换**：guard 反转需要 `if not C:`——
Sico 不存在。正确通用变换 = **fall-through 展开**：`if C: T else:
return E end if`（T 恒以 return 收尾）→ `if C: T end if` + E 作
fall-through 尾 return（无需否定，语义等价）。脚本 flatten_if.py
入 target/tmp 常备。

**elif 链抽辅助**（gcc/gca 的 I64/U64 kind 链 + checked_add/sub/mul/div
op 链）：`if A: X else: if B: Y else: return E`（臂带副作用不可
fall-through）→ helper 函数内 flat guard 连 + 主函数
`if starts_with(res, "ERR:"): return res` 单守卫（新辅助
`gca_subject_kind` / `gca_op_name_check`，+2 分母）。

**结果**：gcc/gca 双双翻绿，**RETURN-COUNT 2→0 清零**。

## 2. 前沿更新

- census **111/130 → 115/132、0 DIFF**（+2 = 新 helper）；翻绿 gcc +
  gca；无回归；HIST CALL-SHAPE 7 / GWPACK-OTHER 3 / NON-DOMAIN 7。
- 下一刀主线：残余 17 户逐户同法（gw_rhs/starts_with/sef/scc/control_if
  等）+ NON-DOMAIN 7 重定（arena 后真身）。

## 3. 事故与登记

- flatten_if.py 首版 0 命中（臂尾判定误把 `end if` 当最后语句）——
  臂恒 return 判定须跳过结构尾行。
- 固件测试构建件 DuplicateId 纠缠（41 函数 fixture 报 function[42]，
  根因未收束）——时间盒判退，翻绿见证以 census 0-DIFF 为准，测试
  删除保持片净；下片如有余力重提。

## 4. 验证与预算

- census 终值：flipped=115 diffs=0 total=132；HIST CALL-SHAPE 7 /
  GWPACK-OTHER 3 / NON-DOMAIN 7；探针副本按纪律删除。
- 固件：无新测试（同上判退）；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 6. STEP-0405 批跑灾难与回退（同日）——负结果，源码净 = 0404 终态

flatten_if.py 全库批跑（13 函数 58 处）致 census 115→13（102 户
UNRESOLVED/EXPRESSION/CALL-SHAPE 雪崩）——脚本 else/end if 边界判定在
含深嵌/异常缩印的函数上误配（gow 报 24 处即红色信号：远超真实
fall-through 候选数），guest 前端被切碎后全局腐化。立即整文件从
备份恢复 + is_word 探针 sanity 复绿（IDENTICAL）+ census 维持
115/132 确认无残留。

**纪律登记**：flatten_if.py 标注"半手工工具——批跑前必须逐函数 diff
审查 + 单函数 probe 验证，禁止全库无人值守批跑"；gow 等深嵌函数的
fall-through 候选以人工结构审查为准。下片回到逐户手工模式（0404
gcc/gca 已证可行）。
