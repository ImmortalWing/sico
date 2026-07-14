# ADR-0001: 使用 Sico Host 作为应用宿主产品名称

> - status: accepted
> - date: 2026-07-14
> - owners: repository-owner, autonomous-agent
> - supersedes: Sico Player terminology in pre-ADR documents
> - superseded-by: -

## Context

Sico 需要一个安装在桌面和 Android 上的原生产品，用于打开、安装、授权、运行和管理 `.sapp`。早期文档称其为 `Sico Player`，但 Player 容易让用户理解为影音播放器或被动内容查看器，不能准确表达应用宿主职责。

底层 `Sico Runtime` 已有清晰职责：加载和执行 WebAssembly Component、实施权限与资源限制、提供 WIT 宿主能力。需要为包含 Runtime、应用管理和用户界面的上层产品确定正式名称。

## Decision drivers

- 准确表达承载和运行应用；
- 与 Runtime 职责清楚区分；
- 不产生影音、浏览器或 OCI 容器误解；
- 可用于桌面与 Android；
- 适合代码模块、架构文档和开发者交流；
- 不限制面向普通用户的品牌展示名。

## Considered options

### Player / Viewer

简短，但强烈暗示影音播放或只读查看，拒绝。

### Launcher

能表达启动，却不能覆盖安装、权限、生命周期、宿主接口和应用管理，拒绝。

### Container

能表达承载，但容易与 OS/OCI 容器和沙箱实现混淆，拒绝。

### Environment

范围足够，但名称抽象，无法清楚指代用户安装的原生程序，拒绝。

### Host

准确表达宿主进程和宿主能力，可自然形成 Desktop Host 与 Android Host，接受。

## Decision

正式使用：

- `Sico Runtime`：底层 Component 执行、安全、权限和宿主能力引擎；
- `Sico Host`：面向用户、包含或调用 Runtime 的应用宿主产品；
- `Sico Desktop Host`：Windows、macOS、Linux 平台构建；
- `Sico Android Host`：Android 平台构建。

面向普通用户的图标、安装名称或商店名称可以直接显示 `Sico`；`Host` 是正式架构与开发名称。

## Consequences

正面：

- 产品职责更准确；
- Runtime 与用户界面边界更清楚；
- 桌面和 Android 使用统一术语；
- 代码 crate 可采用 `host-*`，避免未来迁移。

代价：

- 需要修改全部早期文档；
- 外部早期讨论中可能仍有 Player 旧称；
- Host 一词在 WIT host interface 语境中需要使用完整限定名避免歧义。

## Validation

通过全仓旧称搜索验证术语迁移。该决定只改变名称，不改变 Runtime、Component 或 `.sapp` 行为，因此不需要执行代码测试。

## Revisit conditions

只有在真实用户研究表明 Host 仍造成明显误解，或产品品牌需要完全不同的正式开发名称时重新评估。品牌展示名变化本身不要求推翻本 ADR。

## Links

- Step: [`STEP-0002`](../steps/STEP-0002-sico-host-terminology.md)
- Direction: [`DIRECTION.md`](../../DIRECTION.md)
- Development: [`DEVELOPMENT.md`](../../DEVELOPMENT.md)
