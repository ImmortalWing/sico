# Sico

Sico（Simple Coding）是一门面向 AI 理解、生成、检查和修复代码的正规编程语言。

> - 当前版本：`v0.0.1`
> - 版本性质：1.0 前工程里程碑，不承诺稳定语言/ABI
> - 当前结论：仓库本地轨已闭环；跨平台生产产品仍为 `blocked-external-evidence`

项目已完成 M0–M5。M6 host-side 契约与测试已完成，但 Android SDK/NDK、构建工程和 runner 均缺失，退出结论仍为 NO-GO。M7 STEP-0062–0069 的仓库本地实现与洁净室发布演练已闭环，但真实第三方、生产发布、真实模型和目标平台证据仍缺失，因此不能宣称产品或跨平台支持已经完成。

## 当前能力与证据范围

| 范围 | 状态 | 证据边界 |
|---|---|---|
| 编译器、Sico IR、WebAssembly Component | 本地验证完成 | Windows workspace 全量测试 |
| `.sapp`、Runtime、Host、签名与安全更新 | 本地验证完成 | Windows/Wasmtime 与本地签名生态 |
| LSP 与 compiler-backed AI 工具协议 | 本地验证完成 | 离线工具；真实模型调用为 0 |
| Windows Desktop Host | runtime-verified | 安装、打开、执行结果 `42`、卸载 |
| macOS、Linux | contract-verified | 无目标系统原生 Runtime/Host runner 证据 |
| Android | blocked/deferred | 无完整 Gradle/JNI/APK、SDK/NDK、模拟器或真机证据 |
| HarmonyOS/OpenHarmony | proposed/deferred | 无已接受平台实现、HAP 或 runner 证据 |
| production publisher/public registry | external-gated | 本地 policy/registry 仅为 fixture，不是生产身份或公开服务 |

完整结论与外部恢复条件见 [M7 与项目退出审计](./docs/reports/m7-exit-audit.md)。

## 快速验证

仓库固定 Rust `1.97.0-x86_64-pc-windows-gnu`。Windows PowerShell 下可执行：

```powershell
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
.\tools\validate-step-0069.ps1
```

STEP0069 的关键结果应包含：`releases=2 result=42 runtime=true security_rejections=4`。

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
- [洁净室发布试点](./pilots/README.md)
- [Android、鸿蒙与 Linux 开发手册](./docs/platforms/README.md)

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
- M7 [ecosystem and release plan](./docs/plans/M7-ecosystem-release.md) 的 STEP-0062–0069 仓库本地轨已完成；[M7 exit audit](./docs/reports/m7-exit-audit.md) 保留真实第三方、production/public service、live model、Android、Harmony 与 Linux native runner 外部 gate。
- 延后平台的恢复说明已保存在 [Android、鸿蒙与 Linux 平台开发手册](./docs/platforms/README.md)；STEP-0070/0071 文档不代表 APK/HAP、JNI/Node-API、Linux GTK/关联或目标平台 Runtime 已实现。

## 仓库结构

| 路径 | 用途 |
|---|---|
| `crates/` | 编译器、工具、Runtime、Host 与生态实现 |
| `examples/` | 设计步骤历史与编译器回归基线，不随发布试点改写 |
| `pilots/` | 真实工作流式洁净室试点；不等于独立第三方证据 |
| `tests/` | 端到端、平台、生态、安全与性能证据 |
| `docs/` | STEP、ADR、RFC、计划、报告和平台手册 |
| `tools/` | 可重复验证脚本 |
