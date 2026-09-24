# STEP-0265: M22 restart assessment registration

> - status: complete
> - phase: M22 planning (R0 input)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: planning-only (measured inventory, no executable claim)

## 1. Objective

按 owner 指令（2026-09-24「查看M22已有步骤，是不是需要回退重新开始。
规划好之后重新实现M22」）对 M22 已有产物做分层盘点并判定回退范围，
产出 R0 架构决策（STEP-0264 重划片 §3）的输入文档
[`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md)。

核心判定：**不做 git 级回退；分层重启**——语料/harness/验证器/Rust
oracle 与 S1 formatter、S2 checker 子集无条件保留；S3/S4  lowering 机器
（general-while 帧栈、guard chain、gw 回退路由等 STEP-0260–0262 落地
的通用机器）保留其逻辑、源层数据模型（SOA 并行数组 + JSON 串）的重写
与否挂在 R0 决策上；`lexer.sico`/`tokens.sico`/`declaration_parser.sico`
与 integrated 链的重复（审查 P2）登记为两种 R0 选项下都要偿还的合并债。

拒绝整体回退的三条证据：S1/S2 闭合是关于冻结语料的差分事实，与源层
组织无关；语料 + harness + oracle 是任何重启第一天就需要的最贵资产；
"形状枚举机"判定测于 acb5113（09-20），此后 STEP-0260/0261/0262 已转
向通用机器（formatter 覆盖 16→17→22/30），整体回退恰好丢弃审查要求
的泛化成果。

工作树在途 WIP（`parser.sico` +228/−34：`parameters_ir` 扩展
`Map[Text,U64]` 参数面 + 括号深度感知逗号计数；`selfhost_compiler.rs`
+7：`dump_nm_region_ir` oracle 探针）经审读确认为连贯的下一前沿工作，
按 AGENTS.md §1 原样保留、未提交；文档登记其处置要求（W1/W2 动
`parser.sico` 之前须落地为独立 STEP 或吸收其价值）。

## 2. Context and evidence

- 实测盘点：`wc -l selfhost/*.sico`（2026-09-24 工作树，总计 16,931 行，
  `parser.sico` 12,739 行占 75%）；`git log --oneline -- selfhost/`；
  `git diff` 对在途 WIP 的逐段审读。
- 证据基础：[`m22-quality-review-2026-09-20`](../reports/m22-quality-review-2026-09-20.md)
  §3.2（双前端/形状枚举）、§4 P0-2/P0-3、§5.1、§7 建议 2/3。
- 前置：STEP-0264（M22–M26 route replan，R 阶段收敛门）。

## 3. Scope

包含：新建 `docs/plans/M22-restart-assessment-v1.md`（资产清单 /
三层判定 / R0 两选项成本表 / 合并债 / 分波重启计划 §6）；steps 索
引、plans 索引、docs 索引登记；M22 计划状态头与 STATUS next-step 行
追加 R0 输入指针。不含：任何代码或语料改动；R0 决策文档本身（下一
动作，需 owner 拍板）；对在途 WIP 的提交或回滚。

## 4. Options and decision

- 整体回退（放弃 STEP-0178→0262 重来）：拒绝，理由见 §1 三条证据。
- 原地继续（不回退也不重排）：拒绝——审查 P0-2（SOA 无出口判据）与
  P2（重复/指纹规则）是无期债务，必须在 R0 里终结。
- 分层重启（保留资产、R0 决策后重授权层、偿还合并债、止损门保持
  武装）：采纳，即评估文档 §3/§6。

## 5. Plan

1. 实测盘点 + WIP 审读；2. 起草评估文档；3. 索引与状态同步；
4. `validate-step-0124.ps1` + `git diff --check`。

## 6. Changes

- 新增 `docs/plans/M22-restart-assessment-v1.md`；
- `docs/steps/README.md`：本记录行（并延续 STEP-0264 的索引补齐）；
- `docs/plans/README.md`、`docs/README.md`：评估文档索引行；
- `docs/plans/M22-compiler-self-host.md` 状态头：`M22-restart-assessment-v1`
  登记为 R0 输入；
- `docs/STATUS.md` next-step 行：R0 决策指针。

## 7. Validation

- `powershell tools/validate-step-0124.ps1`：通过（见本节执行记录）。
- `git diff --check`：通过。
- 未运行：编译/测试套件——纯文档变更（AGENTS.md §8 最小验证）。

## 8. Metrics

selfhost 行数实测（评估文档 §2 表）：合计 16,931；`parser.sico`
12,739（75%）；integrated 链（compiler_lexer/compiler_parser/
compiler_semantics/checker）2,848；formatter 827；compiler 430 + driver 49；
legacy lexer/tokens/declaration_parser 947。

## 9. Risks and follow-ups

- 沉没成本辩护风险：缓解 = W2 波次必须移动 canary，否则切换选项
  （评估文档 §6 止损衔接）。
- 下一步：R0 决策文档（owner 按评估 §4 成本表 + R1 数字拍板：
  records RFC 或 SOA 冻结 ADR）；R1 仪器化（canary 覆盖率上报 +
  1k/4k/8k 消耗燃料预算探针）可与 R0 起草并行。
