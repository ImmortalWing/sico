# Sico

Sico（Simple Coding）是一门面向 AI 理解、生成、检查和修复代码的正规编程语言。

项目已完成 M0–M3：Rust 编译器可完成 syntax/semantics、typed IR、deterministic Core Wasm/Component，并通过 `sico build/run` 在 Wasmtime 46.0.1 执行同步 scalar `main()`。当前进入 M4 `.sapp` 与安全 Runtime 规划，下一步是 STEP-0038 package threat model/contract。

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
- M3 退出审计已给出 GO；`.sapp` manifest/hash/signature、capability closure、storage、limits 与 package `inspect` 按 [M4 plan](./docs/plans/M4-sapp-runtime.md) 实施，当前不得把 raw Component 视为最终应用包。
