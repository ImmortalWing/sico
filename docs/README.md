# Sico audit documentation

本目录记录 Sico 从设计到发布的执行过程、阶段状态、决策和验证证据。根目录的方向、语义、语法和开发文档仍是正式设计入口；本目录负责回答每一步“为什么、做了什么、如何验证、下一步是什么”。

## Current state

- [用户手册](../README.md)
- [详细用户手册目录](./user-guide/README.md)
- [Sico 开发手册](./development/README.md)
- [当前任务交接](./TASK-HANDOFF.md)
- [M10 STEP-0098 可执行接手文档](./handoffs/M10-STEP-0098.md)
- [当前状态](./STATUS.md)
- [阶段路线图](./ROADMAP.md)
- [步骤记录](./steps/README.md)
- [阶段执行计划](./plans/README.md)
- [M8 Script Profile 计划](./plans/M8-script-profile.md)
- [M9 streaming/async/interactive 计划](./plans/M9-streaming-async-interactive.md)
- [M10 Runtime observability/debugging 计划](./plans/M10-runtime-observability-debugging.md)
- [M10 observability/debug 机器合同](../observability/README.md)
- [M11 bounded structured-concurrency Runtime 计划](./plans/M11-structured-concurrency-runtime.md)
- [M12 Secure HTTP Provider/Automation SDK 计划](./plans/M12-secure-http-automation-sdk.md)
- [M14 应用就绪语言基线计划](./plans/M14-application-ready-language.md)
- [M15 Web/UI 平台计划](./plans/M15-web-ui-platform.md)
- [M16 Native Automation Host 计划](./plans/M16-native-automation-host.md)
- [M17 视觉与模型包生态计划](./plans/M17-vision-ml-ecosystem.md)
- [M18 代表性 AI 应用与外部试点计划](./plans/M18-ai-application-pilots.md)
- [M19 生产工程与发布就绪计划](./plans/M19-production-engineering.md)
- [M20 跨平台 Runtime、语言 v1 与完成审计计划](./plans/M20-platform-breadth-language-v1.md)
- [M21 开发者体验、标准库第二批与生态激活计划](./plans/M21-developer-experience-and-stdlib.md)
- [M22 编译器自举轨道计划](./plans/M22-compiler-self-host.md)
- [M23 语言 v1 第三批（表达式人机工学）计划](./plans/M23-language-v1-batch-3.md)
- [M24 视觉收口与 GUI 应用试点计划](./plans/M24-vision-closure-gui-application-pilot.md)
- [M25 v1.0 发布——平台广度、完成审计与产品退出计划](./plans/M25-release-v1-completion.md)
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
