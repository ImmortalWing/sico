# Sico 开发手册

本手册面向编译器、Runtime、Host、工具链和平台适配器的开发者。只想编写和运行 Sico 程序，请从根目录的[用户手册](../../README.md)开始。

> - 当前开发版本：`0.0.2-dev`
> - 已发布归档：`v0.0.1`
> - 工具链：`1.98.0-x86_64-pc-windows-gnu`
> - 当前结论：仓库本地轨已闭环；跨平台生产产品仍为 `blocked-external-evidence`

## 开发状态

项目已完成 M0–M5。M6 host-side 契约与测试已完成，但 Android SDK/NDK、构建工程和 runner 均缺失，退出结论仍为 NO-GO。M7 STEP-0062–0069 的仓库本地实现与洁净室发布演练已闭环，但真实第三方、生产发布、真实模型和目标平台证据仍缺失，因此不能宣称产品或跨平台支持已经完成。

| 范围 | 状态 | 证据边界 |
|---|---|---|
| 编译器、Sico IR、WebAssembly Component | 本地验证完成 | Windows workspace 全量测试 |
| `.sapp`、Runtime、Host、签名与安全更新 | 本地验证完成 | Windows/Wasmtime 与本地签名生态 |
| LSP 与 compiler-backed AI 工具协议 | 本地验证完成 | 离线工具；真实模型调用为 0 |
| Windows Desktop Host | runtime-verified | 安装、打开、执行结果 `42`、卸载 |
| macOS、Linux | contract-verified | 无目标系统原生 Runtime/Host runner 证据 |
| Android | blocked/deferred | 无完整 Gradle/JNI/APK、SDK/NDK、模拟器或真机证据 |
| HarmonyOS/OpenHarmony | proposed/deferred | 无已接受平台实现、HAP 或 runner 证据 |
| production publisher/public registry | external-gated | 本地 policy/registry 仅为 fixture |

权威状态见 [STATUS](../STATUS.md)、[ROADMAP](../ROADMAP.md) 和 [M7 退出审计](../reports/m7-exit-audit.md)。

## 架构入口

- [模块边界与命令所有权](./MODULE-BOUNDARIES.md)
- [Windows 手动发布](./RELEASING.md)
- [完整架构与开发设计](../../DEVELOPMENT.md)
- [方向与非目标](../../DIRECTION.md)
- [语义草案](../../SEMANTICS.md)
- [语法基线](../../SYNTAX.md)
- [执行计划](../plans/README.md)
- [ADR](../adr/README.md)
- [RFC](../rfc/README.md)
- [STEP 执行记录](../steps/README.md)
- [验证报告](../reports/README.md)
- [任务交接](../TASK-HANDOFF.md)

## 仓库结构

| 路径 | 用途 |
|---|---|
| `crates/` | 编译器、语言 CLI、应用 CLI、LSP、AI 工具、Runtime、Host 与生态实现；归属由模块契约冻结 |
| `examples/` | 设计步骤历史与编译器回归基线，不随发布试点改写 |
| `pilots/` | 真实工作流式洁净室试点；不等于独立第三方证据 |
| `syntax-candidates/` | 已接受 B 语法和历史候选语料 |
| `semantic-cases/` | 语义正反例与精确诊断预期 |
| `tests/` | 端到端、平台、生态、安全和性能证据 |
| `docs/` | STEP、ADR、RFC、计划、报告、用户及平台手册 |
| [`tools/`](../../tools/README.md) | 用户入口、发布编排、可重复验证与测量脚本 |

## 构建与全量验证

```powershell
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
.\tools\validate-module-boundaries.ps1
.\tools\validate-step-0069.ps1
```

本仓库某些 Windows 测试可被系统的 installer-name heuristic 拦截；审计命令在需要时显式使用 `__COMPAT_LAYER=RunAsInvoker`，它不会提升权限。Android、Harmony 和 Linux 的开发/验收路径见[平台手册](../platforms/README.md)。

## 开发约束

- 非平凡工作必须分配不可复用的 STEP 编号，并同步状态、计划和验证证据。
- 协议或兼容性变化先进入 RFC；跨组件架构决定先进入 ADR。
- `examples/` 保存设计历史；新的端到端应用工作流放入 `pilots/` 或专用测试目录。
- development signature、fixture publisher 和本地 registry 永远不得升级为 production 证据。
- 没有目标平台 runner 时，只能声明 `contract-verified`，不能声明 runtime support。
- 不得用离线 oracle、synthetic fixture 或 compiler result 冒充真实模型成绩。

## 开发文档与用户文档边界

根 [README](../../README.md) 和 [用户手册目录](../user-guide/README.md) 描述已实现、用户可执行的命令。本开发手册及 ADR/RFC/STEP 可以包含设计历史、内部证据开关和未来方案；用户文档不得把这些内容包装成稳定功能。
