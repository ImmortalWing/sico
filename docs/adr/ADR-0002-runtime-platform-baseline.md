# ADR-0002: 采用 Wasmtime 并区分桌面 Cranelift 与 Android Pulley 路线

> - status: accepted
> - date: 2026-07-15
> - owners: repository-owner, autonomous-agent
> - supersedes: -
> - superseded-by: -

## Context

STEP-0010 已证明 Wasmtime 46 能在 Windows 原生 Rust Host 中执行 Sico 所需的 WIT import/export、resource 和 async Component。Sico 仍需同时覆盖桌面与 Android，但 Wasmtime 官方把 Android 列为 Tier 3；移动端还有 64-bit ABI、动态代码、包体和 Google Play 政策约束。

## Decision drivers

- 保持同一 `.sapp` 跨平台；
- 不引入 JavaScript；
- 保留 Component Model/WIT/resource/async；
- 不把不可信分享文件当原生代码加载；
- 桌面性能与 Android 发布风险分别优化；
- 避免在 M0 重写 Runtime 或 ABI。

## Considered options

### 所有平台 Wasmtime + Cranelift

能力和性能一致，但 Android 仍是 Tier 3，并增加可执行内存、包体与商店解释风险。桌面接受，Android 暂不作为默认。

### 所有平台 Wasmtime + Pulley

行为更统一、便携，但官方明确慢于原生后端，会无理由牺牲桌面性能。只用于 Android v0 风险收敛。

### Android 使用 WAMR

Android 与小体积优势明显，但当前官方功能表没有足够的 Component Model/resource/async 证据。切换会迫使 Sico 自造 adapter 或降低已验证语义，拒绝作为 v0 主线。

### Android 使用 WasmEdge

有 Android 工作，但 Component Model 仍标记 experimental/loader phase only，拒绝作为 v0 主线。

### 分发预编译原生 `.cwasm`

启动快、Runtime 可去掉编译器，但工件架构/配置绑定，Wasmtime 官方明确警告不可信预编译字节不能安全反序列化；Android 外部分发原生执行代码也增加政策风险，拒绝作为 `.sapp` 格式。

## Decision

- Wasmtime 是 Sico Runtime v0 唯一主引擎；
- `.sapp` 只分发 raw WebAssembly Component；
- Desktop Host 使用 Wasmtime/Cranelift，可从已验证 Component 生成宿主本地缓存；
- Android Host v0 只支持 64-bit 实验目标，外部 Component 默认走 Wasmtime/Pulley；
- Android UI 是 Kotlin/系统 API 薄壳，Rust `cdylib` 通过窄 JNI 边界提供 Runtime；
- 不可信或外部分发的 Wasmtime precompiled artifact 永不进入 `deserialize`；
- WAMR/WasmEdge 只有通过同一 Sico conformance suite 后才可成为替代引擎。

## Consequences

正面：不改变语言/包 ABI；保持无 JavaScript；桌面获得原生性能；Android 先收敛动态代码与信任风险；替代 Runtime 必须以测试而非宣传进入。

代价：Android 性能可能不足；Wasmtime Android 仍需大量验证；桌面与 Android 后端不同，需要确定性与一致性测试；Rust/JNI/Kotlin 形成三层构建边界。

## Validation

桌面 Wasmtime/Cranelift 已由 STEP-0010 在 Windows 验证。Android/Pulley 目前只有官方能力依据，必须按报告的 Android minimum probe 完成后才能升级为生产支持。

## Revisit conditions

- Android Pulley 无法满足真实 UI/计算性能；
- Wasmtime Android 升至更高支持 tier 或 Cranelift JIT 获得明确平台/商店验证；
- WAMR/WasmEdge 完整支持 Sico 使用的 Component resource/async 并通过 conformance suite；
- Google Play 动态代码政策改变；
- 包体、内存或安全测试超过 M6 门槛。

## Links

- Step: [`STEP-0011`](../steps/STEP-0011-runtime-desktop-android-feasibility.md)
- Report: [`runtime-desktop-android-v0`](../reports/runtime-desktop-android-v0.md)
- Prototype: [`component-host-call`](../../prototypes/component-host-call/README.md)
