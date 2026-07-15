# STEP-0011: Component Runtime 桌面与 Android 可行性

> - status: complete
> - phase: M0
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

基于 STEP-0010 的真实 Component 链路，比较桌面与 Android 的 Runtime 实现、JIT/AOT、包体、沙箱、FFI、发布政策和验证成本，给出 Sico Host v0 的平台基线与分阶段路线。

## 2. Context and evidence

- STEP-0010 已在 Windows 真实执行 Wasmtime 46 同步/async Component；
- Wasmtime 官方将 Windows/macOS/Linux 作为主要 OS 支持，Android aarch64/x86_64 当前是 Tier 3，明确不是生产就绪；
- Cranelift 支持 x86_64/aarch64，Component Model 在两者上受支持，但不支持 32-bit backend；
- Google Play 禁止从 Play 外下载 dex/JAR/.so，但对 VM/解释器中运行的代码设有例外，且动态语言仍必须遵守全部政策；
- Android 官方建议避免动态代码，必须使用可信存储、完整性检查和签名。

官方资料和访问日期记录在 [`runtime-desktop-android-v0`](../reports/runtime-desktop-android-v0.md)。

## 3. Scope

包含官方支持矩阵、Component Model 能力、JIT/AOT/解释器、Android ABI、动态代码政策、宿主 FFI、安全边界、包体和最小实测计划。

不包含完整 Android 应用、UI、应用商店提交、性能承诺或生产沙箱实现。

## 4. Options and decision

接受 [`ADR-0002`](../adr/ADR-0002-runtime-platform-baseline.md)：

- 桌面以 Wasmtime/Cranelift 为基线，开发期直接编译 Component，可使用宿主本地产生的版本化缓存；
- `.sapp` 分发原始、可验证的 WebAssembly Component，不分发架构相关 `.cwasm`；
- Android v0 仅做 64-bit 实验基线：`arm64-v8a` 真机、`x86_64` 模拟器；
- Android 对外部 Component 优先用 Wasmtime Pulley 解释路径，避免把外部输入变成直接加载的原生预编译代码；
- Android 使用 Kotlin/系统 UI 薄壳，经小型 JNI 边界调用 Rust Runtime；不引入 JavaScript/WebView；
- WAMR 保留为未来小体积候选，WasmEdge 保留为观察项，但两者当前不能替代已验证的 Component/async/resource 链。

## 5. Plan

1. 收集 Runtime 与 Android 官方一手资料；
2. 建立桌面/Android 能力与风险矩阵；
3. 区分已验证事实、官方声明、推断和待测项；
4. 给出 Sico Host v0 推荐路线和 Android 最小探针；
5. 更新 ADR、路线图、状态，执行文档检查，提交并推送。

## 6. Changes

- 新增桌面/Android Runtime 可行性报告；
- 新增 ADR-0002，固定原始 Component 分发、桌面 Cranelift 与 Android Pulley 实验路线；
- 明确 `.cwasm` 信任边界、Android 64-bit 范围和 JNI 边界；
- 将 Android 真机探针定义为 M6 前置验证，不把官方可编译性写成仓库实测。

## 7. Validation

- 只采用 Wasmtime、Bytecode Alliance、Rust、Android/Google Play 等官方一手资料；
- 所有日期敏感结论按 2026-07-15 重新检查；
- 与 STEP-0010 的锁定版本和真实结果交叉核对；
- 文档明确标记 verified、official、inference 和 planned，未声称 Android 已运行。

## 8. Metrics

比较 3 个 Runtime、2 类执行后端、4 个 Android ABI、2 个发布工件形态和 9 组 Android 最小探针指标。最终选择 1 个主 Runtime，保留 2 个观察候选。

## 9. Risks and follow-ups

- Android Wasmtime 仍为 Tier 3，必须在 M6 前完成真机、模拟器、生命周期和商店审查探针；
- Pulley 比原生 Cranelift 慢，不能在实测前承诺交互性能；
- Google Play 的 VM/解释器例外不是无限授权，Sico Host 必须限制 WIT 能力、禁止策略违规行为并准备政策说明；
- Android 只支持 64-bit 的 v0 决定需由真实设备覆盖率和性能数据复审；
- Future/Stream、fuel/epoch、内存限制和 JNI 批处理需要进入 Runtime 实现测试。

## 10. Audit links

- ADR: [`ADR-0002`](../adr/ADR-0002-runtime-platform-baseline.md)
- report: [`runtime-desktop-android-v0`](../reports/runtime-desktop-android-v0.md)
- previous: [`STEP-0010`](./STEP-0010-component-runtime-host-call.md)
- commit subject: `docs(runtime): [STEP-0011] choose desktop and Android baseline`
- next: STEP-0012 syntax evidence
