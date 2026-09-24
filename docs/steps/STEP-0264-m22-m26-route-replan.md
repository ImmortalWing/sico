# STEP-0264: M22–M26 route replan registration

> - status: complete
> - phase: M22–M26 planning
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: planning-only (no executable claim)

## 1. Objective

按 owner 指令（2026-09-24「重新规划路线，审视M22-M26，确保能落实到工程
实现」）登记 M22–M26 路线重划片，核心是把质量审查（
`docs/reports/m22-quality-review-2026-09-20.md` §5.1/§7）的三条结构性结论
变成可执行的调度与门：

1. **M22 R 阶段收敛门**：在任何新 lowering STEP 之前插入 R0 架构决策
   （SOA 冻结 ADR vs `List[record]` RFC 二选一，关闭 P0-2 的悬置状态）、
   R1 仪器化（`formatter.sico` canary 函数覆盖率上报；1k/4k/8k 行输入的
   消耗燃料/栈高水位/wall-time 预算曲线，取代从配置上限推断——关闭
   P0-3）、R2 门内收敛（每个 STEP 必须推进 canary 或移动拒绝前沿）、
   R3 预登记止损（连续 4 个实现 STEP canary 零增长且拒绝前沿不动即宣告
   S6 在当前表面不收敛，记入双实现登记）。
2. **M23 双出口实现门**：Route A（M22 S7 GO 后实现 batch 3）/ Route B
   （M22 R3 止损宣告后立即实现；自举于 richer surface 的 re-baseline 在
   M23 出口审计后重启，ADR-0015 合同不变）。"语言面 churn 不得打断在飞
   闭环"的原始理由在两个出口中都保留：Route B 只在与飞尝试无关时宣告。
3. **M24 即刻并行**：其入口门（M16 GO、M17 gate 1、M7 边界）从不依赖
   M22/M23——开工盘点（计划 §7）与 M22 R 阶段并行启动；§8.6.1 界面库
   工作流固定合同先行顺序（RFC → ADR → 渲染帧语料 → 试点）。

M25 入口门澄清：双实现登记的 M22 行必须显式（GO 或止损登记），缺失不暗
示成功。M26 不变（STEP-0263 门与 2026-09-24 owner 调度规则照旧）。

本步骤只做规划登记（计划文档、M22/M23 计划文本、ROADMAP、STATUS、索
引），不改任何能力合同、gate 数值、证据等级或 RFC/ADR 纪律，不预留任何
实现 STEP 编号。

## 2. Context and evidence

- 证据基础：`docs/reports/m22-quality-review-2026-09-20.md` §3.2（形状枚
  举机）、§4 P0-2（SOA 无通用性出口判据）、P0-3（fuel/栈悬崖未测量）、
  §5.1（当前路径实质阻塞、需重划片）、§7 修复建议 2/3/5；及本会话
  owner 对「自缚」反馈环的确认（贫乏语言面 × 自举重度消费者 × 串行
  M22→M23 门）。
- 治理依据：AGENTS.md §3（计划调整必须更新状态、理由与编号；不得为排期
  好看预留 STEP）；RFC-0033（R0 选项 (a) 走完整 reconsideration gate）；
  ADR-0015（S6 合同不变，Route B 重启仍按 A=B=C）。
- 规划合同：`tools/validate-step-0124.ps1`（13 份计划必备节与
  "no STEP numbers reserved" 子串、ROADMAP 标题顺序与不变量、双索引链
  接、ROADMAP 不得引用无记录的 STEP）。

## 3. Scope

包含：

- 新建 `docs/plans/M22-M26-route-replan-v1.md`（重划片主体；Entry gate /
  Exit gates / Non-goals 齐备；no STEP numbers reserved）；
- `docs/plans/M22-compiler-self-host.md`：状态头登记 R 阶段；§3.2 执行
  队列改述为 R 阶段门内内容（保留全部技术条目与既有子串）；
- `docs/plans/M23-language-v1-batch-3.md`：状态头 + §2 入口门改双出口
  （Route A / Route B）；
- `docs/ROADMAP.md`：header updated 行、current phase 行、M22 重划片支
  持段、M23 状态/进入条件、M25 进入条件、M14–M26 依赖图重注；
- `docs/STATUS.md`：updated 行、next step 行（R 阶段先行）、roadmap
  decision 行追加 owner 指令登记；
- `docs/plans/README.md`、`docs/README.md`：重划片文档索引行；
- 本 STEP 记录与 `docs/steps/README.md` 登记。

不包含：任何编译代码、语料、校验器行为变更；R0 决策文档本身的起草
（那是重划片后的下一个动作，需单独 STEP）；M24/M25/M26 计划文本的任何
gate 数值改动。

## 4. Options and decision

- 是否动 M22 S6 的合同（ADR-0015）：候选 (a) 放宽为语义等价——拒绝，
  ADR-0015 明确禁止且无 mismatch 先例；候选 (b) 保持字节全等、改调度——
  采纳。重划片只动"何时、以什么判据走"，不动"走通的标准"。
- 止损判据数字：候选 (a) 按时间（如 30 天）——拒绝，形状难度不均，时
  间窗会误杀深形状的正常攻坚；候选 (b) 按 canary 增长计步——采纳，
  4 个连续零增长 STEP 是"边际收益低且近似恒定"的可观测代理
  （审查 §5.1 原话），且每个 STEP 本身有界。
- M23 是否无条件解锁：拒绝。语言 churn 打断在飞闭环的风险真实存在，
  故 Route B 仍要求 M22 先宣告收敛不被证明，而非 owner 或执行者主观
  判断。

## 5. Plan

1. 起草 `M22-M26-route-replan-v1.md`（§1–§10）；
2. 同步 M22/M23 计划文本与 ROADMAP/STATUS；
3. 双索引 + steps 索引登记；
4. 运行 `validate-step-0124.ps1` 与 `git diff --check`。

## 6. Changes

见 §3 Scope 列表；全部变更可在 `git diff` 中按文件核对。

## 7. Validation

- `powershell tools/validate-step-0124.ps1`：通过（见本节执行记录）。
- `git diff --check`：通过。
- 未运行：编译/测试套件——本步骤为纯文档变更，不可能影响编译代码
  （AGENTS.md §8 允许的最小验证）。

## 8. Metrics

无（纯规划登记步骤）。重划片的可观测指标在 R1 落地后才存在
（canary 覆盖率、预算曲线、止损计数）。

## 9. Risks and follow-ups

- **止损博弈**：STEP 可用琐碎形状"推进前沿"拖延判据。缓解：R 评审同时
  读覆盖率与拒绝前沿身份，4-STEP 窗是评审触发器而非自动判决。
- **Route B 的 re-baseline 成本**：batch 3 落地后自举前端需 re-freeze；
  有界（S1/S2 为语料重跑非重写），且在双实现登记中留痕。
- **并行资源争用**：M22 R1 预算探针与 M24/M23 开工盘点共用证据机
  runner 时间，需分会话调度（handoff 手稿已记录 GNU 快速环）。
- 下一步：R0 决策文档（SOA 冻结 ADR 或 `List[record]` RFC，先合同后实
  现），以及 M23/M24 开工盘点 STEP——三者互不阻塞，可并行。

## 10. Audit links

- 计划：[`M22–M26 route replan v1`](../plans/M22-M26-route-replan-v1.md)
- 证据基础：[`m22-quality-review-2026-09-20`](../reports/m22-quality-review-2026-09-20.md) §3.2/§4 P0-2/P0-3/§5.1/§7
- 合同：ADR-0015（A=B=C 不变）、RFC-0033（R0 选项 (a) 的 gate）、
  [`M22 plan §3.2`](../plans/M22-compiler-self-host.md)、
  [`M23 plan §2`](../plans/M23-language-v1-batch-3.md)
