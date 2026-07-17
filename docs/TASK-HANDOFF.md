# Sico 任务交接：M8 Script Profile 与外部阻塞轨

> - 更新时间：2026-07-17
> - 仓库：`E:\github\sico`
> - 分支：`main`
> - 当前开发版本：`0.0.2-dev`；已发布归档：`v0.0.1`
> - 远端：`origin=https://gitcode.com/ImmortalWings/sico.git`；`github=https://github.com/ImmortalWing/sico.git`
> - 交接基线：以包含本文件的当前 `git HEAD` 为准
> - 开发归档：`codex/archive-v0.0.1-development-history` 已同步到两个远端
> - 工作区状态：以包含本文件的当前提交为 clean baseline

当前 M8 的逐文件状态、验证边界与下一条命令见 [`M8 STEP-0076 接手文档`](./handoffs/M8-STEP-0076.md)。

## 1. 一句话状态

M0–M5 已 GO；M6 仍为 `blocked-external-runner` 且不得宣称 Android 完成。M7 STEP-0062–0069 仓库本地轨已完成；STEP-0074 建立了只读 registry origin、operator bundle、Windows release 集成和 `sico-app dev` 单命令流程。公网 production 仍为 `blocked-external-deployment-inputs`。当前主动开发方向是 M8 Script Profile：STEP-0075 bounded batch Script WIT/adapter/runner 契约已冻结，STEP-0076 正在执行 direct-runner 与 Program/Adapter 纵向原型；M9 已规划 streaming/async/HTTP/watch/REPL，但不得误报为已实现。

当前权威结论：[`M7 exit audit`](./reports/m7-exit-audit.md)；Android 子轨仍服从 [`M6 exit audit`](./reports/m6-exit-audit.md)。

## 2. 已完成工作

| Step | Commit | 已有证据 |
|---|---|---|
| STEP-0054 | `210300a` | 30 项 threat matrix、ADR-0005、RFC-0021、Android ABI/runner 证据上限 |
| STEP-0055 | `37603ed` | 共享 Mobile Host core、64 KiB typed bridge、owned bytes、panic/error mapping |
| STEP-0056 | `b1c4c8b` | View/Send/picker/deep-link 契约、content URI、一次有界复制、拒绝语料 |
| STEP-0057 | `9948f64` | 五项 capability 精确映射、临时 URI grant、app-private storage |
| STEP-0058 | `1115989` | native/Pulley 后端计划、Activity/process 生命周期状态机、终态竞争处理 |
| STEP-0059 | `a99e448` | native widget 映射、touch/IME 事件、TalkBack 顺序、输入/速率/队列上限 |
| STEP-0060 | `772e37e` | 同一签名包 Desktop Wasmtime 返回 `42`；Mobile Host metadata 完全一致 |
| STEP-0061 | `a7f8d51` | 8,192 security properties、全 workspace regression、host bridge 性能和 NO-GO audit |

以上提交已推送到 `origin/main`。

## 3. 当前真实阻塞

本机复查结果：无 Android SDK、NDK、ADB、AVD、emulator、Gradle、Java 或已连接设备；仓库也没有可构建 Android Host。当前 stable Rust 为 1.97.0，但 Android Rust targets 未安装。旧快照曾记录以下 targets，不代表当前环境：

- `aarch64-linux-android`
- `x86_64-linux-android`

Android SDK/NDK 许可必须由仓库所有者接受，Agent 不得代替用户接受法律条款。目标环境与 runner matrix 见：

- [`environment-2026-07-16.json`](../tests/android-host/environment-2026-07-16.json)
- [`runner-gate.json`](../tests/android-host/runner-gate.json)

解除外部阻塞所需：

1. 已接受许可的 Android SDK（ADR 当前目标 API 36）；
2. NDK r27d LTS；
3. ADB 与 x86_64 emulator/AVD；
4. 一台 arm64 Android 设备；
5. 可记录设备型号、系统/API、ABI、运行命令和测试日志的稳定 runner。

## 4. 不要误判为已经完成的部分

当前 `android/host` 不是可构建 Android 应用，只包含 Manifest 和 Kotlin 契约文件：

- 没有 `settings.gradle(.kts)`、根/模块 `build.gradle(.kts)` 或 Gradle wrapper；
- Manifest 声明了 `.HostActivity`，但仓库中没有 `HostActivity.kt`；
- `NativeBridge.kt` 会加载 `sico_android_host`，但没有对应 Rust `cdylib`、JNI exports 或 ABI `.so`；
- 没有 APK/AAB 构建、安装或 instrumentation test；
- Wasmtime/Pulley 尚未在 Android 上链接或执行；
- touch、IME、TalkBack 与 lifecycle 只有共享 Rust 测试和 Kotlin contract review，没有设备证据。

因此“Rust Android target `cargo check` 通过”只证明 Android-neutral core 可交叉检查，不能证明 APK、JNI 或 Runtime 可用。

## 5. 恢复 M6 的推荐顺序

### 5.1 固定并记录 runner

由用户完成 SDK/NDK 许可与设备准备后，先记录而不是直接修改 GO 结论：

```powershell
adb --version
sdkmanager --list_installed
emulator -list-avds
adb devices -l
rustup target list --installed
```

新增带日期的环境快照，不覆盖历史 `environment-2026-07-16.json`。在证据完成前，`runner-gate.json` 继续保持 blocked。

### 5.2 补齐可构建 Android Host

1. 建立最小 Kotlin/Android Gradle 工程，保持 `minSdk 28`、`targetSdk 36` 和两个 ABI；
2. 实现 `HostActivity`，严格复用现有 Intent、permission、lifecycle 和 UI adapter；
3. 新建隔离的 JNI/FFI crate，例如 `sico-android-jni`；不要移除 `sico-mobile-host-core` 的 `#![forbid(unsafe_code)]`；
4. JNI crate 只负责拥有/复制 JNI bytes、调用 `MobileHostCore`、捕获 panic 和返回稳定错误；trust/package/identity 决策仍由共享 core 负责；
5. 为 arm64-v8a 与 x86_64 生成并打包 `.so`，确认 `System.loadLibrary("sico_android_host")` 实际成功；
6. 接入 Wasmtime native 或 Pulley 后端，以 ADR-0005 的 executable-memory probe 决定，不得静默回退到未审计执行器；
7. 增加 JVM/unit、instrumentation 和设备端证据采集脚本。

若 JNI crate 必须使用 `unsafe`，只允许在最小 FFI crate 内使用，并为每个边界记录 pointer/length/lifetime/null/exception invariant；共享 Host core 继续禁止 unsafe。

### 5.3 完成 STEP-0060 设备 parity

必须使用 Desktop 测试所生成的同一份 `.sapp` bytes，不得为 Android 重编译：

- Desktop 与 Android 记录相同 SHA-256 revision digest；
- Android Runtime 返回 `42`；
- app identity、signer、capability fingerprint 与 Desktop 一致；
- mutation、伪造 identity、signer/capability drift 全部拒绝；
- View、Send、picker 与 deep-link-to-picker 四个入口完成 copy-then-reverify；
- background、process death、timeout、cancel、crash 后 Host UI 仍存活且无 orphan work；
- touch、IME 和 TalkBack 在设备上通过；
- x86_64 emulator 与 arm64 device 都有日志。

完成后更新 STEP-0060 状态、parity report 和验证器，使其从 `STEP_0060_PARTIAL` 转为真实 `STEP_0060_OK`。

### 5.4 重跑 STEP-0061 退出审计

补充 Android cold/warm startup 非 SLA 基线、设备矩阵和安全用例，然后运行：

```powershell
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
& "$HOME/.cargo/bin/cargo.exe" check --offline --locked -p sico-mobile-host-core --target aarch64-linux-android
& "$HOME/.cargo/bin/cargo.exe" check --offline --locked -p sico-mobile-host-core --target x86_64-linux-android
```

注意：当前以下验证器故意冻结了 blocked/partial 结论，设备证据完成后必须连同文档一起更新，不能只改输出文本：

- `tools/validate-step-0054.ps1` 预期 SDK/runner unavailable；
- `tools/validate-step-0060.ps1` 预期 `partial-runtime-evidence`；
- `tools/validate-step-0061.ps1` 预期 NO-GO 和 blocked runner。

只有所有 M6 exit gate 有真实证据时，才把 M6 plan、ROADMAP、STATUS、README 与 audit 同步改为 GO/complete。

## 6. M7 分轨交接边界

平台无关轨可继续，但不直接进入公开 registry、真实 production signing 或移动支持声明：

1. STEP-0062：生态/发布 threat model 与兼容契约（已完成）；
2. STEP-0063：production publisher identity、key custody/rotation/revocation（本地 policy/fixtures 已完成；真实身份与 custody 待所有者）；
3. STEP-0064：signed registry publish/discovery/download（本地 signed metadata、不可变 transport 与严格复验已完成；公开服务待授权）；
4. STEP-0065：安全更新与回滚（本地 monotonic/staging/advisory/recovery 闭环已完成）；
5. STEP-0066：标准库与依赖稳定边界（本地 resolver/lock/compatibility/capability 闭环已完成）；
6. STEP-0067：LSP、编辑器与调试流程（bounded stdio LSP、compiler/index/format、shell-free run 与 explicit debug refusal 已完成）；
7. STEP-0068：AI tooling protocol 与评测（compiler-backed offline 已完成；真实模型待凭据和成本授权）；
8. STEP-0069：仓库内洁净室 Component/真实应用试点与 M7 exit audit（本地已完成；真实第三方证据待外部参与者）。

以下操作仍需用户明确授权或提供材料：

- production publisher 法律身份、生产密钥托管与恢复策略；
- 公共 registry 域名、namespace、服务账户、法律条款与公开发布；
- 真实 AI API 凭据和成本预算；
- 应用商店签名、账户和发布。

开发签名密钥不得提升或复用为 production publisher key。

## 7. 关键文件入口

- 当前状态：[`STATUS.md`](./STATUS.md)
- 路线图：[`ROADMAP.md`](./ROADMAP.md)
- M6 计划：[`M6-android-host.md`](./plans/M6-android-host.md)
- M6 审计：[`m6-exit-audit.md`](./reports/m6-exit-audit.md)
- M7 计划：[`M7-ecosystem-release.md`](./plans/M7-ecosystem-release.md)
- Android ADR：[`ADR-0005`](./adr/ADR-0005-android-host-runtime-lifecycle-boundary-v0.md)
- JNI/Intent RFC：[`RFC-0021`](./rfc/RFC-0021-android-intent-jni-contract-v0.md)
- Mobile Host core：[`sico-mobile-host-core`](../crates/sico-mobile-host-core/src/lib.rs)
- Kotlin adapter：[`android/host`](../android/host/src/main/kotlin/dev/sico/host)
- M6 property test：[`android_security_properties.rs`](../crates/sico-mobile-host-core/tests/android_security_properties.rs)
- Desktop/Mobile parity test：[`desktop_android_parity.rs`](../crates/sico-mobile-host-core/tests/desktop_android_parity.rs)
- 移动平台开发文档索引：[`platforms/README.md`](./platforms/README.md)
- Android 详细开发手册：[`ANDROID-DEVELOPMENT.md`](./platforms/ANDROID-DEVELOPMENT.md)
- 鸿蒙详细开发手册：[`HARMONY-DEVELOPMENT.md`](./platforms/HARMONY-DEVELOPMENT.md)
- 移动平台共用验收清单：[`MOBILE-SHARED-CHECKLIST.md`](./platforms/MOBILE-SHARED-CHECKLIST.md)
- Linux 详细开发手册：[`LINUX-DEVELOPMENT.md`](./platforms/LINUX-DEVELOPMENT.md)
- Production origin：[`STEP-0074`](./steps/STEP-0074-production-deployment-origin-and-ux.md)、[`ADR-0008`](./adr/ADR-0008-production-registry-origin.md)、[`operator guide`](../deploy/registry-origin/README.md)
- Ecosystem trust core：[`sico-ecosystem`](../crates/sico-ecosystem/src/lib.rs)
- Publisher policy RFC：[`RFC-0023`](./rfc/RFC-0023-production-publisher-policy-v0.md)
- Signed registry RFC：[`RFC-0024`](./rfc/RFC-0024-signed-registry-metadata-v0.md)
- Secure update RFC：[`RFC-0025`](./rfc/RFC-0025-secure-update-recovery-v0.md)
- Dependency lock RFC：[`RFC-0026`](./rfc/RFC-0026-dependency-lock-compatibility-v0.md)
- Language server RFC：[`RFC-0027`](./rfc/RFC-0027-language-server-editor-protocol-v0.md)
- Language server core：[`sico-language-server`](../crates/sico-language-server/src/lib.rs)
- AI tooling RFC：[`RFC-0028`](./rfc/RFC-0028-ai-tooling-inspect-fix-v0.md)
- AI tooling core：[`sico-ai-tools`](../crates/sico-ai-tools/src/lib.rs)
- 用户手册：[`README.md`](../README.md)、[`docs/user-guide`](./user-guide/README.md)
- 开发手册：[`docs/development`](./development/README.md)
- 洁净室 pilot：[`pilots/third-party-component`](../pilots/third-party-component/README.md)
- M7 退出审计：[`m7-exit-audit.md`](./reports/m7-exit-audit.md)

## 8. Public rollout inputs (future unassigned STEP)

STEP-0074 的 loopback 与 operator ZIP 不得改标为公网 production。恢复实际部署时，仓库所有者需要一次性提供：

1. production hostname 与 DNS 控制方式；
2. hosting provider/account、目标 region 和可用的部署访问；
3. 法律 publisher identity 与 namespace；
4. root/release/recovery/rotation/revocation 五类 custody 的实际负责人/设备策略；
5. TLS/edge、监控告警、备份保留和恢复目标。

输入到位后分配新的未使用 STEP，先部署空只读 origin 和外部健康检查，再通过离线/admin 流程同步真实签名 metadata/blob。STEP-0075 已分配给 M8 Script Profile，不得复用。签名私钥不得进入 origin 主机或仓库。

## 9. 接手者完成检查表

- [ ] 工作区 clean，`main` 与远端基线一致；
- [ ] 阅读 ADR-0005、RFC-0021、M6 audit 和 runner gate；
- [ ] 用户已接受 Android SDK/NDK 许可；
- [ ] Gradle Android Host、HostActivity 和隔离 JNI crate 可构建；
- [ ] arm64-v8a 与 x86_64 `.so` 均能加载；
- [ ] 同一 `.sapp` digest 在 Desktop/Android 返回相同结果和 authority metadata；
- [ ] Intent/lifecycle/native UI/device performance matrix 有原始日志；
- [ ] STEP-0060 从 partial 转为 complete；
- [ ] STEP-0061 从 NO-GO 转为 GO；
- [ ] STATUS、ROADMAP、README、M6/M7 plan 与验证器同步；
- [ ] 每个非平凡步骤单独提交并推送；
- [x] STEP-0062 平台无关契约完成，且 M6 仍保持 blocked；
- [x] STEP-0063 publisher policy/key lifecycle 本地闭环完成，且无真实 production credential；
- [x] STEP-0064 signed registry publish/discover/download 本地闭环完成，且无公开 namespace、账户或外部发布；
- [x] STEP-0065 secure update/recovery 本地闭环完成，且无公共服务或移动安装器操作；
- [x] STEP-0066 dependency/standard-library stability 本地闭环完成，且无外部包源操作；
- [x] STEP-0067 bounded LSP/editor workflow 已完成，且无 shell/process/network side effect；
- [x] STEP-0068 compiler-backed AI tooling/offline evaluation 已完成，且未把 fixture 冒充模型成绩；
- [x] STEP-0069 两版洁净室 release drill/Wasmtime/security/performance 与项目审计已完成，且未冒充真实第三方证据；
- [x] STEP-0070 Android/鸿蒙开发手册与共用验收清单已保存；
- [x] STEP-0071 Linux Desktop Host 开发与验收手册已保存；
- [x] STEP-0072 已建立根用户入口、独立开发手册、十项细分用户手册与机器文档契约；
- [x] STEP-0073 已归档 v0.0.1，并完成语言 CLI、应用 CLI、Runtime 与 Host 的机器可验证模块边界；
- [x] STEP-0074 已完成只读 registry origin、operator bundle、release 集成和 `sico-app dev`，且保持 public production evidence 为 false；
- [x] production identity/key custody/public service 已在所有者决策边界停止，未生成或复用生产密钥；
- [x] Android、Harmony 与 Linux 证据等级保持未升级，等待各自工具链、实现和 runner。
