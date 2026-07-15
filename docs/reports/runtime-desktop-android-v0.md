# Report: Runtime desktop/Android feasibility v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0011
> - evidence-cutoff: 2026-07-15

## 1. Question

Sico Host 应如何在桌面和 Android 打开同一个原始 WebAssembly Component，同时保持 Component Model、无 JavaScript、能力隔离、可接受的发布风险和可验证的后续路线？

## 2. Evidence classes

- **verified**：本仓库 STEP-0010 在 Windows x86_64 GNU 上真实运行；
- **official**：上游或平台官方文档明确声明；
- **inference**：由官方事实推导，尚未由本仓库运行；
- **planned**：后续必须实测。

## 3. Runtime comparison

| Runtime | Android/小体积 | Component Model | Sico 结论 |
|---|---|---|---|
| Wasmtime 46 | Android aarch64/x86_64 是 Tier 3；Pulley 可解释执行 | x86_64/aarch64/Pulley 均列出 Component Model 支持；STEP-0010 已验证 resource/async | 主基线；Android 保持 experimental |
| WAMR | 官方支持 Android，解释器/AOT runtime 很小 | 当前官方功能表未列 Component Model（由缺失作出的 inference） | 不能直接承载 Sico v0 WIT/Component；未来观察 |
| WasmEdge | 有 Android 工作与多后端 | 官方 changelog 仍称 Component Model 为 experimental、loader phase only | 当前不足以替代 Wasmtime；未来观察 |

不因 WAMR/WasmEdge 体积优势重做 ABI；只有它们能通过同一 Sico Component/resource/async conformance suite 后才重新评估。

## 4. Platform matrix

| Dimension | Desktop Host v0 | Android Host v0 |
|---|---|---|
| Status | Windows 已 verified；macOS/Linux official primary support | official Tier 3，未实测 |
| Primary backend | Wasmtime + Cranelift | Wasmtime + Pulley，experimental |
| Architecture | x86_64 先行，随后 aarch64 | `arm64-v8a` 真机；`x86_64` 模拟器 |
| Input artifact | raw Component in `.sapp` | 同一个 raw Component in `.sapp` |
| Compilation | 本机编译；可做本地缓存 | raw Component → Pulley bytecode；不接收外部分发 `.cwasm` |
| Native cache | 仅宿主自己生成、版本/配置/CPU 绑定 | v0 默认关闭；以后单独安全评估 |
| UI boundary | Rust native shell or thin platform UI | Kotlin/system UI → narrow JNI → Rust Runtime |
| JavaScript | none | none；不使用 WebView 作为 Runtime |
| Maturity gate | STEP-0010 + future cross-platform CI | Android probe + lifecycle + policy review |

## 5. Why raw Component is the distribution unit

Wasmtime 官方允许预编译 Module/Component，以减少启动、内存和 Runtime 编译器体积；但官方也警告预编译工件不能被充分安全验证，反序列化不可信字节可能导致任意代码执行。因此：

1. `.sapp` 保存架构无关 raw Component；
2. Host 先验证 manifest、签名/哈希、WIT world、能力与大小；
3. 桌面可从已验证 raw Component 在本机生成缓存，缓存键至少包含 Component hash、Wasmtime 版本、target、CPU features 和 engine config；
4. 缓存只能加载 Host 自己产生且完整性匹配的字节；
5. Android 不下载 `.so`/native `.cwasm`，外部 Sico 程序走 VM/解释器路径。

这既保持“一份 `.sapp` 到多平台”，也避免把用户分享文件提升为原生可信代码。

## 6. Android policy and security interpretation

Google Play 当前政策禁止应用从 Play 之外下载 dex、JAR、`.so` 等可执行代码，但明确排除 VM/解释器中运行、间接访问 Android API 的代码。该例外仍要求动态语言代码不能违反其他政策。

对 Sico 的约束：

- Component 只能通过版本化 WIT capability 间接访问 Android；
- 不暴露任意 JNI、任意 native symbol、shell、包安装、辅助功能绕过或其他应用数据；
- 文件从 Storage Access Framework/应用内部或 scoped storage 进入，校验 hash/signature；
- 默认网络、文件、相机、位置等能力均关闭，用户授权与 Android 权限取交集；
- 配置 `StoreLimits`、fuel/epoch、task/stream 上限和输出限额；实例/Store 生命周期有界；
- 商店描述明确 Sico Host 的核心用途是运行用户选择的沙箱 Sico 程序；
- 政策例外不是法律保证，首次 Play 发布前仍需预审和申诉材料。

Android 官方同时指出动态代码有注入/篡改风险，建议避免使用；确有业务需求时应使用可信位置、完整性校验和签名。Sico 的业务本身就是受限程序宿主，因此必须把这些措施作为产品功能，不是附加优化。

## 7. Backend trade-offs

### Desktop Cranelift

优点：STEP-0010 已验证；Component/async/resource 完整；性能更接近原生；可以 JIT 或 AOT。代价：Runtime/编译器包体与冷启动更大，需要缓存和版本管理。

### Android Pulley

优点：不需要为 guest 创建原生机器码后直接反序列化；支持 Component Model；可减少平台/JIT 政策耦合。代价：官方明确慢于原生后端，且生成 Pulley bytecode 仍需编译工作，Android Wasmtime 本身是 Tier 3。

### Android Cranelift later

AArch64 Cranelift 和 Component Model 官方支持，但 Wasmtime 需要可执行内存，Android 动态代码安全与商店解释成本更高。只有 Pulley 性能不足且真机/政策验证通过后，才增加可选的 on-device Cranelift；`.sapp` 格式不因此改变。

## 8. Size and packaging

Wasmtime 官方最小化示例表明关闭默认特性和编译器能显著缩小 Runtime；示例数字来自 Linux，不可外推为 Android 实测。Android 必须测最终 App Bundle，而不是静态库文件大小。AAB/配置 APK 可只给设备下发对应 ABI 的 native library，因此 v0 不需要把所有 ABI 同时装到设备。

包体策略：

- `default-features = false`，只开启 runtime、component-model、Pulley、必要 async/limits；
- 不在 Android v0 捆绑完整 CLI、调试器、WASI 全集或 Cranelift；
- 使用 AAB ABI splits；
- 错误码/短消息保留，重型符号和开发诊断放开发构建或外部符号文件；
- 报告 compressed download、installed size、native `.so`、cold RSS 四个不同指标。

## 9. Host/UI boundary

Android UI 使用 Kotlin/系统 API 薄壳，Rust `cdylib` 内持有 Runtime、Component、Store 与 WIT capability state。JNI 只传粗粒度命令、不可变字节和批量 UI event；异步调度留在单侧，避免高频跨 JNI 回调。Android 官方同样建议减少 JNI marshalling 和频率。

这不是用 Kotlin 实现 Sico 语言底层：编译器、Runtime policy、Component binding 和跨平台逻辑仍在 Rust；Kotlin 只负责 Android 生命周期、权限和系统 UI。

## 10. Android minimum probe

M6 前至少完成：

1. `aarch64-linux-android` 真机和 `x86_64-linux-android` 模拟器构建；
2. 通过系统文件选择器打开 STEP-0010 raw Component；
3. Pulley 执行 sync host call、resource、async func；
4. 拒绝错误 hash、超大 Component、未声明 import 和非 raw/precompiled 输入；
5. 测 memory limit、fuel/epoch interrupt、取消和 Host 不崩溃；
6. 测前后台、旋转、进程回收、低内存和重复打开；
7. 记录冷启动、首次运行、热运行、峰值 RSS、`.so`/APK/AAB 大小；
8. 验证 JNI CheckJNI、线程归属和局部/全局引用清理；
9. 形成 Google Play 动态代码、权限和数据安全说明。

## 11. Decision

采用 Wasmtime 作为 Sico Runtime v0 唯一主引擎，保持 raw Component 为跨平台分发格式。桌面以 Cranelift 实现性能路径；Android 先以 64-bit Pulley 实验路径证明正确性与政策可接受性。Android 未实测前不宣称生产支持，也不阻止 M1 编译器前端开发。

## 12. Official sources

- [Wasmtime platform support](https://docs.wasmtime.dev/stability-platform-support.html)
- [Wasmtime support tiers](https://docs.wasmtime.dev/stability-tiers.html)
- [Wasmtime 46.0.1 release](https://github.com/bytecodealliance/wasmtime/releases/tag/v46.0.1)
- [Wasmtime pre-compilation and trust warning](https://docs.wasmtime.dev/examples-pre-compiling-wasm.html)
- [Wasmtime minimal embedding](https://docs.wasmtime.dev/examples-minimal.html)
- [WAMR official repository/features](https://github.com/bytecodealliance/wasm-micro-runtime)
- [WasmEdge changelog](https://github.com/WasmEdge/WasmEdge/blob/master/Changelog.md)
- [Google Play Device and Network Abuse policy](https://support.google.com/googleplay/android-developer/answer/16559646?hl=en)
- [Android dynamic code loading risks](https://developer.android.com/privacy-and-security/risks/dynamic-code-loading)
- [Android NDK ABIs](https://developer.android.com/ndk/guides/abis)
- [Rust Android target support](https://doc.rust-lang.org/rustc/platform-support/android.html)
- [Android JNI guidance](https://developer.android.com/ndk/guides/jni-tips)
- [Android App Bundle format](https://developer.android.com/guide/app-bundle/app-bundle-format)
