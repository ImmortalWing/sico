# Architecture decision records

ADR 记录工程与架构决定，例如 Component Runtime 引擎、`.sapp` 物理格式、构建系统、存储布局和平台集成。

| ADR | 状态 | 标题 |
|---|---|---|
| [ADR-0001](./ADR-0001-sico-host-terminology.md) | accepted | 使用 Sico Host 作为应用宿主产品名称 |
| [ADR-0002](./ADR-0002-runtime-platform-baseline.md) | accepted | Wasmtime 桌面 Cranelift 与 Android Pulley 平台基线 |
| [ADR-0003](./ADR-0003-isolated-storage-wasi-host-v0.md) | accepted | hashed per-app storage identity、reparse-safe root 与显式 WASI grants |
| [ADR-0004](./ADR-0004-desktop-host-identity-lifecycle-platform-v0.md) | accepted | Desktop app/revision identity、single-instance lifecycle 与 platform evidence boundary |
| [ADR-0005](./ADR-0005-android-host-runtime-lifecycle-boundary-v0.md) | accepted | Android Intent/URI、JNI、Runtime、lifecycle 与 ABI evidence boundary |
| [ADR-0006](./ADR-0006-ecosystem-release-trust-boundary-v0.md) | accepted | role-separated production trust、untrusted registry 与 mobile-deferred split track |
| [ADR-0007](./ADR-0007-openjdk-style-modular-monorepo.md) | accepted | OpenJDK-style 单仓模块化、依赖边界与命令所有权 |
| [ADR-0008](./ADR-0008-production-registry-origin.md) | accepted | bounded read-only production registry origin、loopback default 与 operator deployment boundary |
| [ADR-0009](./ADR-0009-script-adapter-runner.md) | proposed | versioned Script adapter、独立 in-process runner 与 compiler/Runtime 进程边界 |
| [ADR-0010](./ADR-0010-single-store-structured-concurrency.md) | accepted-design | single-Store cooperative structured concurrency |
| [ADR-0011](./ADR-0011-ai-quality-budgets.md) | accepted | AI 工作流质量预算 v0（floor/target 两层，target 待 live-model 实证） |
| [ADR-0012](./ADR-0012-ai-quality-budgets-live-model.md) | accepted | AI 质量预算 live-model 实证与预算结论 |
| [ADR-0013](./ADR-0013-native-automation-host-platform-v0.md) | accepted | M16 Native Automation Host 平台适配 v0：Windows 优先、Win32 枚举（UIA 划线）、SendInput、有界动作原子性、适配器最小隔离 crate；2026-09-10 owner 接受，修正案 A1：v0 捕获走 GDI PrintWindow（离线锁定工具链无 WinRT 面），WGC 升级按后续修正案回归 |
| [ADR-0014](./ADR-0014-web-hosting-substrate-v0.md) | accepted | M15 Web 宿主形态 v0：JS canonical-ABI shim 承载编译组件内嵌核心模块（稳定浏览器默认不提供 Component Model），HTTP authority 按 RFC-0037 shim 侧执行且连接池声明为不可代表，M10 三元组降级如实声明，一载一 Store，限额降级声明化，DOM 控件树走 RFC-0042；2026-09-10 owner 接受 |

下一可用编号：`ADR-0014`。

创建时使用 [`ADR template`](../templates/ADR.md)，并在本页登记状态和替代关系。
