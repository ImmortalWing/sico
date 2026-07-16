# Sico Android Host 详细开发手册

> 状态：`contract-verified / blocked-not-implemented`
> 适用契约：Android Host v0
> 最后复核：2026-07-16

本手册用于在具备 Android SDK/NDK、模拟器与真机的环境中，把仓库已有 Kotlin 适配器补齐为可构建、可安装、可验收的 Android Host。它不构成 Android Runtime 已完成的声明。

## 1. 当前真实状态

仓库已经具备：

- `android/host/AndroidManifest.xml`；
- `IntentAdapter.kt`、`PermissionAdapter.kt`、`LifecycleAdapter.kt`、`UiAdapter.kt`、`NativeBridge.kt`；
- 共享 `sico-mobile-host-core` 桥接 schema 与主机侧测试；
- Android Host ADR、Intent/JNI RFC、M6 计划与 NO-GO 审计。

仓库仍缺少：

- Gradle settings、root/module build、wrapper 与锁定后的 AGP/Gradle/JDK 组合；
- `HostActivity.kt` 和 Activity Result/file-picker 编排；
- Rust Android JNI `cdylib`、导出符号、panic/exception 防线和 `.so` 打包；
- 可安装 APK/AAB、x86_64 emulator 与 arm64 device 运行证据；
- 真实触摸、IME、TalkBack、后台/进程死亡和 Runtime backend 证据。

所以 JVM 或 Rust 主机测试只能证明契约，不可写成 `runtime-verified`。

## 2. 规范与冻结基线

实现前先阅读：

- [ADR-0005 Android Host Runtime/lifecycle](../adr/ADR-0005-android-host-runtime-lifecycle-boundary-v0.md)；
- [RFC-0021 Android Intent/JNI contract](../rfc/RFC-0021-android-intent-jni-contract-v0.md)；
- [M6 Android Host 计划](../plans/M6-android-host.md)；
- [STEP-0060 parity 恢复点](../steps/STEP-0060-desktop-android-parity-app.md)；
- [移动平台共用检查表](./MOBILE-SHARED-CHECKLIST.md)。

v0 冻结值如下：

| 项目 | 值 | 说明 |
|---|---:|---|
| `minSdk` | 28 | Android 9 起；改变需 ADR |
| `compileSdk` | 36 | 编译基线 |
| `targetSdk` | 36 | 行为与商店策略基线 |
| NDK | r27d / `27.3.13750724` | 项目选择 LTS；实现前再次查官方页 |
| ABI | `arm64-v8a`, `x86_64` | arm64 真机 + x86_64 模拟器 |
| Rust targets | `aarch64-linux-android`, `x86_64-linux-android` | 只构建这两个目标 |
| bridge envelope | 64 KiB | 超限在解码前拒绝 |
| `.sapp` 输入 | 64 MiB | 流式复制并做上限加一探测 |
| 文本事件 | 4 KiB | Kotlin 与 Rust 两侧都检查 |

Android Gradle Plugin 与 Gradle/JDK 的兼容组合会变化。创建工程当天必须按 [AGP compatibility](https://developer.android.com/build/releases/about-agp) 和 [AGP roadmap](https://developer.android.com/build/releases/gradle-plugin-roadmap) 选择同一组稳定版本，写入 version catalog 与 wrapper，并把选择记录到设备证据中；不得只写 `latest`。当前官方 NDK 页面仍列 r27d 为最新 LTS，同时已有更新的 stable NDK，项目保持 r27d，除非新的 ADR 和双 ABI 回归批准升级。

## 3. 环境准备与审计

需要 Android Studio、受支持 JDK、Android SDK Platform 36、Build Tools、Platform Tools、NDK r27d、CMake（仅在引入 C/C++ glue 时需要）、Rust stable 与两种 Android target。SDK/NDK 许可必须由使用者阅读并接受；自动化不得替所有者接受条款。

PowerShell 审计示例：

```powershell
java -version
rustc --version --verbose
cargo --version
rustup target list --installed
adb version
adb devices -l
Get-ChildItem Env:ANDROID_HOME,Env:ANDROID_SDK_ROOT,Env:ANDROID_NDK_HOME -ErrorAction SilentlyContinue
```

缺 target 时，在已安装且与 `rust-toolchain.toml` 一致的 toolchain 上执行：

```powershell
rustup target add aarch64-linux-android x86_64-linux-android
```

然后通过 Android Studio SDK Manager 安装 API 36 和 NDK `27.3.13750724`。若企业环境使用离线镜像，要把镜像来源、校验值和实际 `source.properties` 保存进 `environment.json`。

## 4. 目标工程布局

最终建议布局如下；已有文件迁入标准 `src/main` 目录时保持 Git 历史：

```text
android/
  settings.gradle.kts
  build.gradle.kts
  gradle/libs.versions.toml
  gradle/wrapper/gradle-wrapper.properties
  gradlew
  gradlew.bat
  host/
    build.gradle.kts
    proguard-rules.pro
    src/main/AndroidManifest.xml
    src/main/kotlin/.../HostActivity.kt
    src/main/kotlin/.../{Intent,Permission,Lifecycle,Ui,NativeBridge}Adapter.kt
    src/main/jniLibs/arm64-v8a/libsico_android_host.so
    src/main/jniLibs/x86_64/libsico_android_host.so
    src/test/...
    src/androidTest/...
crates/sico-android-jni/
tools/build-android-host.ps1
tools/test-android-host.ps1
tests/platform/evidence/android/<date>/<run-id>/
```

不要把本机 SDK 绝对路径、keystore、设备序列号或 release secret 提交到仓库。`local.properties` 必须忽略；wrapper、version catalog 和依赖锁应提交。

## 5. Gradle 模块约束

模块至少应使用下列约束；插件版本由第 3 节审计后锁定，而不是从本示例猜测：

```kotlin
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "org.sico.host.android"
    compileSdk = 36
    ndkVersion = "27.3.13750724"

    defaultConfig {
        applicationId = "org.sico.host.android"
        minSdk = 28
        targetSdk = 36
        versionCode = 1
        versionName = "0.0.1-dev"
        ndk { abiFilters += setOf("arm64-v8a", "x86_64") }
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    sourceSets["main"].jniLibs.srcDir("src/main/jniLibs")
    buildFeatures { buildConfig = true }
}
```

Debug 包应使用默认 debug 签名且名称显式带 `dev`。Release 构建从环境变量或 CI secret store 读取 signing config；Gradle 文件只引用变量名，不保存值。使用 `packaging` 冲突排除前，必须确认没有隐藏重复 Rust 动态库。

## 6. Rust JNI crate

新建 workspace crate `sico-android-jni`，只承担 JNI 所需的薄适配，不复制 package、permission、lifecycle 或 UI 规则：

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
sico-mobile-host-core = { path = "../sico-mobile-host-core" }
```

JNI 辅助 crate 的版本应在实现时锁定到 `Cargo.lock`。unsafe 只允许存在于 FFI 必需位置，每个块必须写明：指针来源、长度验证、所有权、线程限制和释放责任。

`NativeBridge` 当前要求库名 `sico_android_host`。Rust 输出必须最终命名为 `libsico_android_host.so`，导出的方法签名与 Kotlin 完全一致。桥接建议保持一个窄入口：

```kotlin
external fun dispatch(envelope: ByteArray, payload: ByteArray?): ByteArray
```

JNI 实现必须：

- 在读取/分配前拒绝超过 64 KiB 的 envelope；
- 复制 Java byte array，不在调用返回后保存 JNI 指针或 local reference；
- 对可选 payload 单独做长度、空值和用途验证；
- 用 `catch_unwind` 阻止 Rust panic 穿越 JNI；
- 把错误编码为共享 schema 的稳定响应，或抛出一个明确 Java exception，不能两者同时；
- 不缓存 `JNIEnv`，跨线程只保存合法 global reference，并尽量不保存；
- 清理 pending exception 后停止本次调用，绝不继续执行 Runtime；
- 所有请求携带 schema version 和 request id，未知版本 fail closed。

参考 Android 官方 [JNI tips](https://developer.android.com/ndk/guides/jni-tips)。

## 7. 构建 Rust 与打包 `.so`

构建脚本应从已安装 NDK 推导 LLVM prebuilt 目录，不硬编码某位开发者的路径。两个 target 分别使用 NDK clang linker，再将产物复制到 ABI 对应目录：

```powershell
cargo build -p sico-android-jni --target aarch64-linux-android --release
cargo build -p sico-android-jni --target x86_64-linux-android --release
```

在 Windows 上，具体 linker 名与 host prebuilt 目录由 NDK 实际内容确定。脚本应先检查文件存在，再设置 `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` 与 `CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER`。不得在找不到 linker 时退回 host linker。

复制后校验两个 `.so` 的 SHA-256，并用 NDK `llvm-readelf`/`llvm-objdump` 检查架构与依赖。Android 原生库打包方式参考 [官方 jniLibs/ABI 文档](https://developer.android.com/studio/projects/gradle-external-native-builds)。

```powershell
Push-Location android
.\gradlew.bat clean :host:assembleDebug
.\gradlew.bat :host:bundleRelease
Pop-Location
```

首次实现优先让 `tools/build-android-host.ps1` 串联 target 检查、Rust 构建、复制、摘要、Gradle 构建和 APK 内容检查，避免手工产物漂移。

## 8. Manifest 与入口

现有 manifest 只登记以下外部形态：

- `ACTION_VIEW` + `application/vnd.sico.sapp`；
- `ACTION_SEND` + 单个同 MIME 内容；
- `sico://open` 深链。

`HostActivity` 应在 `onCreate` 和 `onNewIntent` 共用同一解析入口。深链 v0 只允许唤起系统文件选择器，不从 query/path 下载或直接执行包。`ACTION_SEND_MULTIPLE`、`file://`、任意 HTTP URL、未知 MIME 和 clipData 多项必须稳定拒绝。

用户主动选文件时使用 Activity Result API 和 Storage Access Framework。优先 `ACTION_OPEN_DOCUMENT`/适当 contract 获取 `content://`，只使用本次所需的读授权；除非产品明确需要长期重新打开且经过 ADR，不持久化 URI grant。官方 [共享存储文档](https://developer.android.com/training/data-storage/shared/documents-files) 说明用户选定文档不要求申请广泛存储权限。

入口处理伪代码：

```text
Intent -> classify action/type/scheme -> require one URI
       -> open ContentResolver stream -> copy to private staging with 64 MiB + 1 guard
       -> close stream -> strict load/verify copied bytes
       -> create immutable descriptor -> permission intersection -> start or refuse
```

任何 `displayName`、MIME、provider authority、reported size 都是不可信元数据；最终决定只基于复制后的字节与严格 loader。

## 9. 权限与存储

manifest 当前的 `INTERNET` 是平台能力上界，不代表任意 `.sapp` 获得网络。每个受保护操作都执行：

```text
effective = package declared capability
          ∩ Sico user grant
          ∩ Android manifest/runtime grant
          ∩ current lifecycle availability
```

缺少任一项就返回结构化拒绝。运行时权限按具体操作即时检查，不依据启动时缓存；参考 Android [runtime permission 指南](https://developer.android.com/training/permissions/requesting)。v0 不申请 `MANAGE_EXTERNAL_STORAGE`，不扫描共享目录，不把 `.sapp` 解压到外部存储。

安装包、revision、私有数据与临时文件必须放在应用私有目录，并继续复用共享 Host 的身份与路径净化规则。崩溃恢复时清理无引用 staging；卸载后由平台清理应用私有数据。备份策略必须显式决定，默认不要备份签名包、grant、cache 或 Runtime 临时状态。

## 10. 生命周期状态机

Android Activity 生命周期不能等同进程生命周期。实现与测试需参考官方 [Activity lifecycle](https://developer.android.com/guide/components/activities/activity-lifecycle) 和 [process lifecycle](https://developer.android.com/guide/components/activities/process-lifecycle)。

规则：

- `onCreate` 恢复的只是不变 `appIdentity/revisionDigest`，不是运行中的 Runtime；
- `onStart/onResume` 只有在描述符仍能从严格安装记录重建时才创建/恢复；
- `onPause/onStop` 停止接收新 UI 事件，按策略暂停或销毁 Runtime；
- `onDestroy` 必须幂等清理；配置变化不能产生两个活动 Runtime；
- `onNewIntent` 先完成旧实例的明确切换/拒绝，再处理新包；
- `savedInstanceState` 不保存包字节、密钥、JNI handle、Rust 指针或用户 grant 快照；
- 进程被杀后必须重新验证安装记录和 digest，不能把 UI 恢复误作 Runtime 已恢复。

状态迁移错误必须带 request id、旧状态与事件类型，但日志不得包含包内容或私密输入。

## 11. 原生 UI、输入与无障碍

v0 只映射共享最小 UI tree：纵/横布局、Text、Button、EditText/Input。禁止通过 WebView、任意 class name、反射或包内原生代码扩展 UI。

渲染与事件约束：

- node id 在单棵树内唯一，深度/节点数/文本长度继续服从共享 limits；
- 所有 View 只在主线程创建/修改；
- Button click 和文本提交转换为有界 schema event；
- 文本按 UTF-8 编码后检查 4 KiB，不按 UTF-16 code unit 猜长度；
- IME composing state 与最终提交分开，不能每个 composition update 都无限入队；
- lifecycle 非前台时拒绝或合并事件，不积累无界队列；
- 可操作控件具有稳定 label/role/focus order，动态更新触发适当 accessibility event。

用手动 TalkBack 加自动检查验证，参考官方 [accessibility testing](https://developer.android.com/guide/topics/ui/accessibility/testing)。无障碍测试结果应包含触摸探索、键盘/DPAD（模拟器）、编辑框读出、错误提示和返回导航。

## 12. Runtime backend 探针

Wasmtime 对 Android aarch64 仍属于低保证支持层级，不能从桌面成功推断 Android 可用。按以下顺序在每个 ABI 独立探测：

1. Rust `cdylib` 可被 APK 装载并返回 build/schema version；
2. 共享 bridge 畸形/边界请求不崩溃；
3. 解析并验证固定 Component；
4. Pulley backend 执行固定输入得到 `42`；
5. 验证 memory limit、fuel/epoch timeout、trap、cancel 与 Host 生存；
6. 只有在 executable-memory、signal/unwind 与设备矩阵均通过后，才试原生 Cranelift；
7. backend 选择必须写入日志和证据，失败不能静默换 backend 后仍报告成功。

参考 [Wasmtime platform support](https://docs.wasmtime.dev/stability-platform-support.html) 与 [stability tiers](https://docs.wasmtime.dev/stability-tiers.html)。如果 Pulley 或 native 均不满足安全/性能门槛，Android 保持 NO-GO。

## 13. 测试分层与命令

先运行平台无关回归：

```powershell
cargo test --workspace --all-targets
pwsh -NoProfile -File tools\validate-step-0060.ps1
```

再运行 JVM 与设备测试：

```powershell
Push-Location android
.\gradlew.bat :host:testDebugUnitTest
.\gradlew.bat :host:connectedDebugAndroidTest
Pop-Location
```

官方建议用 instrumented tests 验证依赖真实平台的行为，见 [instrumented tests](https://developer.android.com/training/testing/instrumented-tests) 和 [Android testing overview](https://developer.android.com/training/testing)。最低设备矩阵：

| 设备 | 用途 | 不可替代项 |
|---|---|---|
| API 36 x86_64 emulator | 冷启动、Intent、进程死亡、重复自动化 | 不能代替 arm64/native 行为 |
| API 28 emulator/device | 最低版本兼容 | 不能代替 target API 行为 |
| API 36 arm64 physical device | JNI/Runtime、触摸、IME、TalkBack、后台 | M6 GO 的必需证据 |

安装与启动命令中的 application id/activity 必须来自构建产物：

```powershell
adb install -r <debug-apk>
adb shell am start -W -n org.sico.host.android/.HostActivity
adb logcat -c
adb shell am force-stop org.sico.host.android
```

执行 `adb` 前记录目标序列号；多设备时使用 `-s <serial>`，证据中对序列号脱敏。每个负向 case 都应断言稳定错误类，而不是只确认“没有崩溃”。

## 14. 签名、发布与升级

- Debug keystore 仅用于本地开发，不能成为 Sico publisher root；
- Release keystore 的创建、托管、轮换、备份与商店登记属于所有者控制范围；
- APK/AAB 构建成功不代表通过 Play 或其他商店政策；
- 若使用 Play App Signing，要单独记录 upload key 与 app signing key 的角色；
- 升级测试必须覆盖相同 app id 的签名连续性、数据库/schema 迁移、安装包缓存与 grant 失效；
- 禁止在 Git、Gradle properties、日志或 evidence 中保存私钥、口令或完整证书申请资料。

## 15. 验收产物

按 [共用证据格式](./MOBILE-SHARED-CHECKLIST.md#5-证据目录与清单) 归档，并额外保存：

- `gradle-versions.json`：AGP、Gradle、JDK、compile/target/min SDK；
- `apk-content.txt`：两个 ABI 的 `.so` 路径、大小和摘要；
- `intent-cases.json`：VIEW/SEND/deep link/unknown/multiple 的结果；
- `lifecycle-cases.json`：旋转、后台、进程回收、new intent 的状态轨迹；
- `runtime-backend.json`：backend、Wasmtime version、ABI、limits 和 `42` 结果；
- TalkBack/IME 手工步骤与经过脱敏的屏幕/日志证据。

只有 clean build、x86_64 emulator 和 arm64 device 全部通过，才可更新 M6 exit；否则保持 NO-GO，并在交接文档中记录最近失败点。

## 16. 常见故障

| 症状 | 首查 | 禁止的“修复” |
|---|---|---|
| `UnsatisfiedLinkError` | APK 内 ABI 路径、库名、设备 ABI、依赖库 | 删除 ABI 过滤或吞掉异常 |
| JNI 调用后崩溃 | panic、数组长度、pending exception、线程/引用 | 保存 `JNIEnv` 或裸指针重试 |
| 选择文件后无权限 | Intent flags、provider、复制时机、stream 异常 | 申请全盘存储权限 |
| 旋转后双实例 | saved state 与 lifecycle transition 日志 | 用静态全局 Runtime 掩盖 |
| 模拟器通过真机失败 | ABI、JIT/executable memory、厂商后台策略 | 把模拟器结果标成真机证据 |
| TalkBack 无法操作 | label、role、focus order、动态事件 | 仅关闭 lint/accessibility 检查 |
| native backend 失败 | backend probe、signal/unwind、Wasmtime tier | 静默切 Pulley 且不记录 |

## 17. 官方参考

- [Rust Android targets](https://doc.rust-lang.org/rustc/platform-support/android.html)
- [Android NDK downloads](https://developer.android.com/ndk/downloads/)
- [AGP release notes](https://developer.android.com/build/releases/agp-9-0-0-release-notes)
- [Storage Access Framework](https://developer.android.com/training/data-storage/shared/documents-files)
- [Activity lifecycle](https://developer.android.com/guide/components/activities/activity-lifecycle)
- [Process lifecycle](https://developer.android.com/guide/components/activities/process-lifecycle)
- [Runtime permissions](https://developer.android.com/training/permissions/requesting)
- [JNI tips](https://developer.android.com/ndk/guides/jni-tips)
- [Instrumented tests](https://developer.android.com/training/testing/instrumented-tests)
- [Accessibility testing](https://developer.android.com/guide/topics/ui/accessibility/testing)

官方页面和商店要求会变化。每次恢复平台工作都应记录复核日期；若官方基线与本手册冲突，先暂停、更新 ADR/RFC 与本手册，再改代码。
