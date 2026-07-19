# Sico audit documentation

本目录记录 Sico 从设计到发布的执行过程、阶段状态、决策和验证证据。根目录的方向、语义、语法和开发文档仍是正式设计入口；本目录负责回答每一步“为什么、做了什么、如何验证、下一步是什么”。

## Current state

- [用户手册](../README.md)
- [详细用户手册目录](./user-guide/README.md)
- [Sico 开发手册](./development/README.md)
- [当前任务交接](./TASK-HANDOFF.md)
- [M9 STEP-0094 可执行接手文档](./handoffs/M9-STEP-0094.md)
- [当前状态](./STATUS.md)
- [阶段路线图](./ROADMAP.md)
- [步骤记录](./steps/README.md)
- [阶段执行计划](./plans/README.md)
- [M8 Script Profile 计划](./plans/M8-script-profile.md)
- [M9 streaming/async/interactive 计划](./plans/M9-streaming-async-interactive.md)
- [M10 Runtime observability/debugging 计划](./plans/M10-runtime-observability-debugging.md)
- [M11 Secure HTTP Provider/Automation SDK 计划](./plans/M11-secure-http-automation-sdk.md)
- [M9 exit audit](./reports/m9-exit-audit-v0.md)
- [M7 与项目退出审计](./reports/m7-exit-audit.md)
- [Android、鸿蒙与 Linux 平台开发手册](./platforms/README.md)

## Decision and evidence records

- [架构决策 ADR](./adr/README.md)
- [语言与接口 RFC](./rfc/README.md)
- [实验与验证报告](./reports/README.md)

## Templates

- [步骤模板](./templates/STEP.md)
- [ADR 模板](./templates/ADR.md)
- [RFC 模板](./templates/RFC.md)
- [报告模板](./templates/REPORT.md)

## Rules

- 非平凡工作使用不可复用的 `STEP-xxxx` 编号；
- 关键工程决定使用 ADR，语言/WIT/诊断决定使用 RFC；
- 实验数据必须进入 report，并记录可复现方法；
- 文档状态必须与仓库真实状态一致；
- 被取消或替代的记录保留历史，不直接删除。
