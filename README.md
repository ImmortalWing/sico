# Sico

Sico（Simple Coding）是一门面向 AI 理解、生成、检查和修复代码的正规编程语言。

项目已完成 M0–M5。M6 STEP-0054–0061 的 Android-neutral Host、Intent、permission、lifecycle、native UI contract、Desktop/Mobile parity 与审计已完成，但退出结论为 NO-GO：当前缺少 licensed Android SDK/NDK、ADB、emulator/device，尚无 Android Runtime 与原生 UI 真机证据。下一步是恢复 STEP-0060 device validation；M7 计划已形成但仍被 M6 GO 门槛阻塞。

## 文档

- [自治 Agent 项目目标提示词](./AGENT_GOAL.md)
- [项目状态与审计记录](./docs/README.md)
- [方向与设计目标](./DIRECTION.md)
- [核心语义草案](./SEMANTICS.md)
- [M1 语法基线、候选与评测方法](./SYNTAX.md)
- [语法错误注入集](./syntax-mutations/README.md)
- [AI 生成、理解与修复评测](./ai-eval/README.md)
- [诊断协议与稳定错误目录](./diagnostics/README.md)
- [语义索引与 AI 查询 JSON v0](./semantic-index/README.md)
- [开发与架构文档](./DEVELOPMENT.md)
- [候选语言示例](./examples/README.md)
- [P0 语义正反例集](./semantic-cases/README.md)

## 已确认技术路线

```text
Sico 源码
    ↓
AST、类型检查与 Sico IR
    ↓
WebAssembly Component
    ↓
.sapp
    ↓
Sico Runtime / Sico Host
```

- 第一代编译器、Runtime 和 Host 核心使用 Rust；
- WebAssembly Component 是正式执行与分发格式；
- WIT 定义组件和宿主能力接口；
- 通用系统能力优先复用 WASI；
- 不依赖 JavaScript 或 TypeScript；
- M0 设计与技术基线已通过退出审计；`Int`/Decimal、resource/async 和真实 Component/Runtime host-call 原型已完成，Runtime v0 选择 Wasmtime，M1 表层语法选择 B Labeled Blocks。
- M1/M2 已完成 lossless frontend、formatter、稳定诊断、25/25 valid 与 29/29 exact invalid semantic oracle、Semantic Index/query；
- M3 已完成 typed IR/verifier、lowering、deterministic Core/Component、WIT Result/record/resource、Future/Stream Runtime contract，以及 raw Component `build/run`；
- M4 已完成 canonical `.sapp`、strict loader、development signature/trust、capability closure、isolated storage/WASI、Runtime limits/faults、package CLI/cache 与 security/property baseline；结论见 [M4 exit audit](./docs/reports/m4-exit-audit.md)。
- M5 已完成 Windows runtime-verified Desktop Host 与 macOS/Linux contract artifacts；结论见 [M5 exit audit](./docs/reports/m5-exit-audit.md)。
- M6 [Android Host plan](./docs/plans/M6-android-host.md) 的 host-side work 已执行至 STEP-0061；按 [M6 exit audit](./docs/reports/m6-exit-audit.md) 提供 Android runner 后恢复 STEP-0060。raw Component 仍只作为 compiler regression boundary，不是应用分发格式。
- M7 [ecosystem and release plan](./docs/plans/M7-ecosystem-release.md) 已规划 STEP-0062–0069，但在 M6 GO 前不开始实现。
