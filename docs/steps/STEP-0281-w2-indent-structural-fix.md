# STEP-0281: W2 片 1 — indent structural 修复（顶格体语句按语句处理）

> - status: complete（guest 修复 + 豁免收窄已落地；runner 测试仓本机不可
>   重建——gcc/dlltool 缺失——集成链以 CI 裁决）
> - phase: M22 compiler self-host — W2（guest re-authorization；R3 止损计数自本 STEP 起算）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机：checker 组件 235 条全量探针 235/235；CI：集成链全量）

## 1. Objective

关闭 STEP-0280 登记的 indent diverge：把 guest semantics 第二遍对「函数体
内顶格非声明行」的处理从「静默跳过」改为「按语句跑 body 检查」
（structural 方向；typed refusal 已在 STEP-0280 §3 否决）。

## 2. Changes（`selfhost/compiler_semantics.sico`，第二遍主循环）

- 新增 per-iteration 标志 `process_body_line`（I64）：
  - 缩进行（`else` 支）：函数内一律置 1（行为同旧）；
  - 0-indent 行（`same(raw, line)` 支）：函数头 / `end function` 处理后，
    若仍在函数内且本行非函数头 → 置 1（**新**：顶格语句进入 body 检查）。
- 原活跃守卫下的整个 body 检查块（E7002 payload 检查、effects/
  capabilities 声明、close/read/move/borrow、await/collect/spawn、task
  depth、next/collect Stream 检查、return 线 E4001/E4002/E6002/E7001、
  if 行守卫登记与 end if 弹栈）**搬出 if/else**，置于循环尾
  `if I64.equal(process_body_line, 1)` 单点执行；声明区判别
  （`same(raw, line)`）语义不变（函数头/E6001/end function/E5103 仍在
  原支）。
- 顺序影响核对：body 检查块现位于 match/try/E2001-E2020 检查块（仍
  缩进行限定）**之后**；两块的 return 诊断在同一行不重叠（payload/
  effects 检查线 vs 构造器/try 检查线），235 全量探针零回归证实。

parser 侧（compiler_parser.sico）未动：顶格行的**语法**检查（E1002
match 分隔、task 计数等）仍只覆盖缩进行——登记为后续 declared-subset
候选，不影响本片关闭的豁免。

## 3. 结果（五个 indent 语料，guest 修复后）

| 条目 | oracle | 修复前 guest | 修复后 guest |
| --- | --- | --- | --- |
| IND-001/002（valid） | accept | accept | accept ✅ |
| IND-003 | E1001 | E1001 | E1001 ✅ |
| IND-004 | E2031（清单首码）/ E4001 / E4002 | accept ❌ | **E4001**（emit 首码，valid oracle 码）|
| IND-005 | E7002 | accept ❌ | **E7002** ✅ |

- **IND-005 移出豁免** → strict semantic 桶（E7002 identity 关闭）。
- **IND-004 保留豁免**：guest 现以 E4001 拒绝（优于 false-accept），但
  清单 `diagnostic_ids` 按 ordinal 排序首码 = E2031，first-diagnostic
  identity 未闭——guest 补 E2031（unresolved call target）后移出。
- `DIVERGENCE_WAIVER` 10 → 9；分区 (116, 37, 72, 10) → **(116, 38, 72, 9)**。

## 4. Validation（本机实测）

- `sico check selfhost/checker.sico` ok；`sico build --profile script-v0`
  成功。
- **checker 组件 235 条全量探针：235/235 pass**（约 208 s）——搬块重排
  零回归。
- 本机不可运行（已知宿主限制）：runner 测试仓 cargo 构建（gcc/dlltool），
  集成链全量 + canary 以 CI 裁决。

## 5. Follow-ups（W2 继续）

- records 重授权层（W2 主片，关余 8 条 records 豁免）。
- IND-004 的 E2031 first-identity（可并入 records 片或独立小片）。
- 候选 declared-subset：parser 顶格行语法检查；match/try/E2001-E2020
  块（缩进行限定）是否扩到顶格行。
