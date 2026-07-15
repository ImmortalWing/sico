# Architecture decision records

ADR 记录工程与架构决定，例如 Component Runtime 引擎、`.sapp` 物理格式、构建系统、存储布局和平台集成。

| ADR | 状态 | 标题 |
|---|---|---|
| [ADR-0001](./ADR-0001-sico-host-terminology.md) | accepted | 使用 Sico Host 作为应用宿主产品名称 |
| [ADR-0002](./ADR-0002-runtime-platform-baseline.md) | accepted | Wasmtime 桌面 Cranelift 与 Android Pulley 平台基线 |
| [ADR-0003](./ADR-0003-isolated-storage-wasi-host-v0.md) | accepted | hashed per-app storage identity、reparse-safe root 与显式 WASI grants |

下一可用编号：`ADR-0004`。

创建时使用 [`ADR template`](../templates/ADR.md)，并在本页登记状态和替代关系。
