# STEP-0263: M26 PDF document processing milestone registration

> - status: complete
> - phase: M26
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent

## 1. Objective

按 owner 指令（2026-09-24 「确认，要使用sico原生UI库」）登记新里程碑
M26「PDF document processing」：以 versioned package 提供 PDF 结构化
阅读、合并与旋转能力，并以 Sico 语言原生界面库（M24 计划 §8.6.1）编写
GUI PDF 工具试点。同日追加 owner 指令（2026-09-24「如果原生UI库不完善
那就完善M24」）登记为 M26 GUI 调度规则：开工盘点实测界面库不足以承载
PDF 工具时，缺口先回 M24 §8.6.1 工作流完善（其 RFC+ADR 先行纪律不变），
M26 包切片独立推进、GUI gate 等待、任何时刻不回退 web UI。本步骤只做
规划登记（计划文档、ROADMAP、索引、规划校验器、STATUS），不预留任何
实现 STEP 编号、不改动任何编译代码或 gate。

## 2. Context and evidence

- 规划草案经 owner 确认（2026-09-24），并追加指令：GUI 部分必须使用
  Sico 语言原生界面库（WinUI 3-like Fluent 风格，M24 计划 §8.6.1
  工作流），不得使用第三方或 web 工具包。
- 架构依据：AGENTS.md §4（能力不进核心语法；标准库助手只收窄不扩权）、
  §7（未受信输入 fail-closed）、M17 gate-1 先例（versioned 二进制经
  M7 包边界承载，STEP-0166）、M14 求解器 oracle 先例（纯 Sico 实现 +
  冻结 oracle，STEP-0138）。
- 标准依据：ISO 32000-2（合同 RFC 需钉住接受子集并 typed 拒绝其余）。
- 规划合同校验器：`tools/validate-step-0124.ps1`（M14–M25 计划文件存在、
  三段必备节、不预留 STEP 编号、ROADMAP 标题顺序、双索引链接）。

## 3. Scope

包含：

- 新建 `docs/plans/M26-pdf-document-processing.md`（Entry gate /
  Non-goals / Exit gates 等必备节；no STEP numbers reserved）；
- `docs/ROADMAP.md` 新增 M26 条目并扩展依赖图（M25 → M26）；
- `docs/plans/README.md`、`docs/README.md` 索引行；
- `tools/validate-step-0124.ps1` 扩展到 M26；
- `docs/STATUS.md` roadmap decision 行登记 owner 指令。

不包含：任何 PDF 实现、RFC/ADR 起草、语料生成、实现 STEP 编号预留。

## 4. Options and decision

- 编号与位置：候选 (a) 插在 M24/M25 之前进入 v1.0 范围——会推迟 v1.0
  发布审计；候选 (b) 排在 M25 之后为 M26——v1.0 结论不被新能力推迟。
  决定：(b)，已在规划草案中向 owner 明示推荐，owner 确认（2026-09-24）。
  撤销条件：owner 显式要求 PDF 能力进入 v1.0 范围。
- GUI 技术路线：owner 指令直接指定 Sico 语言原生界面库（STEP-0257 已
  登记该库为 M24 前置工作流），无候选比较；试点 authority 沿用 M24
  格式转换器先例（本地应用 + 用户选定路径 + 既有文件能力）。
- 范围诚实声明：页面光栅化不在范围内，GUI 只展示结构化信息与承载
  文件操作；加密文档打开为 typed 拒绝而非支持目标。

## 5. Plan

1. 起草 M26 计划文档（六段式 + kickoff inventory 要求）；
2. ROADMAP 增加 M26 章节与依赖图补边；
3. 双索引与规划校验器同步；
4. STATUS.md 登记；
5. 运行 `validate-step-0124.ps1` 与 `git diff --check`。

## 6. Changes

- 新增 `docs/plans/M26-pdf-document-processing.md`；
- `docs/ROADMAP.md`：header updated 行、current phase 行、M25 章节后
  新增 `## M26: PDF document processing`、依赖图（M14–M25 shape 扩展
  为含 M26）；
- `docs/plans/README.md`、`docs/README.md`：M26 行；
- `tools/validate-step-0124.ps1`：plans 数组、headings、invariants、
  末尾 OK 行扩展至 M26；
- `docs/STATUS.md`：roadmap decision 行追加 M26 登记（含 2026-09-24
  同日「完善M24」调度规则）；
- `docs/plans/M24-vision-closure-gui-application-pilot.md` §8.6.1：登记
  M26 GUI 为依赖方消费者与缺口归属（交叉引用，不改 M24 任何 gate）。

## 7. Validation

- `powershell tools/validate-step-0124.ps1`：通过（见本节执行记录）。
- `git diff --check`：通过。
- 未运行：编译/测试套件——本步骤为纯文档变更，不可能影响编译代码
  （AGENTS.md §8 允许的最小验证）。

## 8. Metrics

无（纯文档登记步骤）。

## 9. Risks and follow-ups

- M26 GUI gate 依赖 M24 §8.6.1 界面库合同与真实渲染证据；若界面库
  延迟，包切片可先关闭、GUI gate 如实 deferred 并注明外部门名称，
  不回退 web UI。
- 下一步（owner 决定启动时）：按 M26 计划 §7 开工盘点 STEP
  （STEP-0129/0142 模式），随后起草 PDF 合同 RFC 与限额/拒绝合同 RFC。

## 10. Audit links

- 计划：[`M26 PDF document processing`](../plans/M26-pdf-document-processing.md)
- 前置指令：STEP-0257（Sico 语言原生界面库登记）、STEP-0258（GUI 试点
  形态先例：本地应用 + 用户选定路径 + typed fail-closed 语料）
- 先例：STEP-0141（M14 GO）、STEP-0166（versioned 二进制承载）、
  STEP-0138（纯 Sico + 冻结 oracle 先例）
