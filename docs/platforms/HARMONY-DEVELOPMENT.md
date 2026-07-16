# Sico 鸿蒙 Host 详细开发手册

> 状态：`proposed / not-implemented`
> 当前用途：可恢复的预研与实施手册，不是已接受的平台契约
> 最后复核：2026-07-16

本手册覆盖 HarmonyOS 商业生态与 OpenHarmony 社区平台的开发准备。两者共享概念和部分工具链，但 SDK、API、签名、分发与设备兼容范围不能默认等价。开始编码前必须选定目标变体并用 ADR 冻结；本手册中的鸿蒙架构在此之前均为 proposed。

## 1. 当前真实状态

仓库当前没有：

- HarmonyOS/OpenHarmony 目标 ADR、桥接 RFC 或执行计划；
- DevEco Studio、目标 SDK、模拟器/真机环境证据；
- ArkTS/ArkUI、Stage model、Node-API/C++ 或 Rust OHOS 工程；
- HAP/APP、签名、安装、UI、生命周期或 Runtime 运行证据。

可复用的只有 `.sapp`/Component、安全边界、`sico-mobile-host-core` 消息契约和桌面/Android 的设计经验。因此不得把 Android adapter 复制后改名，也不得把 Rust 的 OHOS target tier 当作 Wasmtime 已支持鸿蒙的证明。

## 2. 先选择平台变体

在 STEP/ADR 中填写并评审以下表格后才能建工程：

| 决策 | HarmonyOS | OpenHarmony |
|---|---|---|
| SDK 来源 | Huawei Developer / DevEco Studio | OpenHarmony SDK/Public SDK 与对应 IDE/构建工具 |
| 目标设备与系统版本 | 实际 HarmonyOS NEXT 设备/模拟器 | 指定 OpenHarmony 发行版与设备 |
| 应用模型 | Stage model | Stage model |
| 分发 | AppGallery Connect/企业渠道 | 目标发行版规定的安装/分发 |
| 签名身份 | Huawei 开发者/组织账号管理 | OpenHarmony 签名工具及发行方策略 |
| API level | 选择已安装且设备支持的一组 | 选择与源码/SDK release 匹配的一组 |
| 支持承诺 | 只承诺已测型号与版本 | 只承诺已测发行版与设备 |

若产品需要同时支持两者，应建立两个平台 profile、两套证据和明确的兼容层；不能用一个 HAP 的成功泛化到另一平台。

优先采用 Stage model。Huawei 官方将其作为 HarmonyOS NEXT 首选与长期演进模型，核心是 UIAbility/ExtensionAbility 和前后台生命周期，见 [Stage model](https://developer.huawei.com/consumer/cn/arkui/arkui-stage/)；OpenHarmony 的 [UIAbility lifecycle](https://gitee.com/openharmony/docs/blob/master/en/application-dev/application-models/uiability-lifecycle.md) 也定义 Create、Foreground、Background、Destroy 与 `onNewWant`。

## 3. 拟定 v0 架构

推荐先实现窄桥接：

```text
Want / Document Picker / permission / UIAbility / ArkUI
                         |
                       ArkTS
                         |
             Node-API C++ shim (copy + validate)
                         |
                  stable C ABI
                         |
             Rust sico-harmony-ffi cdylib
                         |
               sico-mobile-host-core
                         |
       strict .sapp loader / Component Runtime / UI schema
```

首版不建议 Rust 直接持有 `napi_env` 或跨线程操作 ArkTS 对象。C++ shim 按官方 Node-API 线程规则完成类型检查、字节复制与结果创建；Rust 只暴露稳定 C ABI，便于独立测试 unsafe、ownership 和 panic boundary。若以后改为 Rust Node-API binding，必须由新 RFC 证明线程、GC、异步回调和版本兼容。

## 4. 目标仓库布局

用 DevEco/目标 SDK 生成的 Native C++ Empty Ability 模板为准，建议落到：

```text
harmony/
  README.md
  host/
    AppScope/app.json5
    build-profile.json5
    hvigorfile.ts
    oh-package.json5
    entry/
      build-profile.json5
      oh-package.json5
      src/main/module.json5
      src/main/ets/entryability/EntryAbility.ets
      src/main/ets/pages/Index.ets
      src/main/ets/platform/WantAdapter.ets
      src/main/ets/platform/PermissionAdapter.ets
      src/main/ets/platform/LifecycleAdapter.ets
      src/main/ets/platform/UiAdapter.ets
      src/main/cpp/CMakeLists.txt
      src/main/cpp/napi_init.cpp
      src/main/cpp/sico_harmony_ffi.h
      src/main/resources/...
crates/sico-harmony-ffi/
tools/build-harmony-host.ps1
tools/test-harmony-host.ps1
tests/platform/evidence/harmony/<date>/<run-id>/
```

SDK 生成模板决定实际 module、native library 与 ABI 目录名；模板生成后要提交一个结构 ADR，记录哪些文件由 IDE 管理。不要手工拼出一个看似正确但无法被当前 hvigor/SDK 识别的工程。

## 5. 环境准备与版本冻结

至少需要：

- 与选定平台匹配的 DevEco Studio/IDE、SDK/API、Native SDK/NDK；
- SDK 自带或兼容的 Node.js、hvigor、CMake/Ninja/clang；
- `hdc` 与 arm64 真机；如果目标 SDK 支持，再配置 x86_64 模拟器；
- Rust stable 和 `aarch64-unknown-linux-ohos`；模拟器需要时再装 `x86_64-unknown-linux-ohos`；
- 对应开发者账号/调试证书，但账号条款和证书由所有者控制。

Rust 官方把 aarch64、armv7、x86_64 OHOS targets 列为 Tier 2 with host tools，并明确需要 SDK，且 OpenHarmony SDK 不能直接替 Rust 完成链接配置；见 [Rust OpenHarmony support](https://doc.rust-lang.org/stable/rustc/platform-support/openharmony.html)。Sico v0 只计划 arm64 device 与 x86_64 emulator，不因 target 存在就扩展 armv7 产品范围。

环境审计示例：

```powershell
rustc --version --verbose
cargo --version
rustup target list --installed
hdc version
hdc list targets
Get-ChildItem Env:DEVECO_SDK_HOME,Env:OHOS_SDK_HOME -ErrorAction SilentlyContinue
```

安装 Rust targets：

```powershell
rustup target add aarch64-unknown-linux-ohos x86_64-unknown-linux-ohos
```

然后创建 `tests/platform/evidence/harmony/<date>/<run-id>/environment.json`，至少冻结：platform variant、IDE build、SDK/API、compatible/target API、native toolchain、hvigor、Node、Rust、targets、设备型号/架构/系统 build。不要只记录“最新版”。

## 6. 工程与模块配置

用官方模板创建一个 Stage/Empty Ability + Native C++ 工程。应用包由一个或多个 HAP 模块组成，模块配置、abilities、skills 与 requestPermissions 位于 `module.json5`；结构参考 OpenHarmony [Stage package structure](https://gitee.com/openharmony/docs/blob/master/en/application-dev/quick-start/application-package-structure-stage.md) 与 [module configuration](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/quick-start/module-configuration-file.md)。

首个提交只做：

1. 空 `EntryAbility` 启动一个 `Index` 页面；
2. 页面显示 app/schema/native build version；
3. C++ Node-API module 返回固定 UTF-8 字符串；
4. Debug HAP 可构建、签名、安装与启动；
5. 不接入文件、网络、Runtime 或广泛 permission。

冻结字段至少包括 bundle name、module name、ability name、device types、compile/compatible/target SDK 和 signing profile。Node-API module 注册名、ArkTS import 名与生成的 `lib<module>.so` 必须一致；官方 [Node-API 使用流程](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/napi/use-napi-process.md) 给出了 `libentry.so` 类似导入关系和真机验证要求。

## 7. Rust OHOS 交叉编译

新建 `sico-harmony-ffi`：

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
sico-mobile-host-core = { path = "../sico-mobile-host-core" }
```

对外只导出 C ABI，不导出 Rust ABI：

```c
typedef struct {
    uint8_t *ptr;
    size_t len;
    uint32_t status;
} SicoOwnedBuffer;

uint32_t sico_harmony_dispatch(
    const uint8_t *envelope, size_t envelope_len,
    const uint8_t *payload, size_t payload_len,
    SicoOwnedBuffer *out);

void sico_harmony_buffer_free(SicoOwnedBuffer buffer);
```

正式 RFC 必须补齐 ABI version、null/zero-length 语义、最大长度、错误码、allocation owner、free 幂等要求与 panic 行为。Rust 入口先验证长度和空指针组合，用 `catch_unwind` 捕获 panic；C++ 必须且只能调用对应 free。C++/ArkTS 不得猜测 Rust allocator，也不得保存 Rust slice 指针。

Rust 官方说明需要为 OHOS SDK clang 建 linker wrapper，并在 Cargo target 配置中指定 linker。实现脚本应从已审计的 native SDK 根目录生成本机临时 wrapper/config，不能提交绝对路径，也不能在 OHOS linker 缺失时退回 host clang。示意命令：

```powershell
cargo build -p sico-harmony-ffi --target aarch64-unknown-linux-ohos --release
cargo build -p sico-harmony-ffi --target x86_64-unknown-linux-ohos --release
```

只有实际 SDK 版本确定后，才把 target triple、sysroot、clang target 与 API suffix 写入 `tools/build-harmony-host.ps1`。脚本需在构建前打印非敏感版本，在构建后用 SDK readelf 工具验证 ELF machine、SONAME、依赖和未解析符号。

## 8. Node-API C++ shim

Node-API 层只负责平台对象与拥有字节的 C ABI 之间转换：

- 检查 argument count 与 exact type；
- 接受 `ArrayBuffer`/`Uint8Array` 时立即复制或在同步调用期间严格限定生命周期；
- envelope 超过 64 KiB、文本事件超过 4 KiB 时不调用 Rust；
- 不缓存或跨线程使用 `napi_env`；
- Rust worker 线程不得直接调用 Node-API；异步结果通过官方线程安全调度机制回到 ArkTS 线程；
- 每条错误只生成一次 ArkTS exception 或结构化 response；
- module unload/Ability destroy 后不再交付回调；
- native resource 的 finalize 必须幂等，并能处理半初始化状态。

官方流程强调 Node-API 接口应在 ArkTS 线程使用，`napi_env` 与线程绑定，并建议使用设备而非 Previewer 验证 native module。Sico 的设备测试因此是硬门槛。

## 9. Want、文件选择与外部打开

入口处理分两类：

1. 用户在 Sico Host 内主动选择 `.sapp`；
2. 系统通过 Want/skills 把明确类型的单个 URI 交给 Sico Host。

首版优先完成用户选择器。外部 skills 必须等 action/entity/type/URI 行为在目标 SDK 与设备上验证后再开放。Want 的字段和显式/隐式匹配参考 [Want overview](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/application-models/want-overview.md)。文件 picker 参考 [保存/选择用户文件](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/file-management/save-user-file.md)，跨应用 URI/FD 授权参考 [应用文件分享](https://gitee.com/openharmony/docs/blob/master/en/application-dev/file-management/share-app-file.md)。

处理规则与 Android 一致但 API 独立实现：

```text
Want -> exact action/type/URI count validation
     -> open URI/FD under current temporary authorization
     -> stream copy to app sandbox with 64 MiB + 1 guard
     -> close FD/release temporary object
     -> strict verify copied bytes
     -> immutable descriptor -> grant intersection -> Runtime
```

不要把 URI/FD、Want parameters、display name、reported size 或 MIME 当作可信。临时授权可能随目标应用退出而撤销，所以必须在当前入口内完成复制，不能把 URI 保存后稍后执行。`onCreate` 与 `onNewWant` 调用同一分类器；重复 Want 必须显式拒绝、排队或替换，不能启动第二个 Runtime。

深链首版只打开 Sico 内部选择页面，不允许 URL 参数指定远程包，不实现自动下载，不接受任意 scheme。

## 10. 权限与应用沙箱

鸿蒙权限名和授权行为必须从所选 SDK 的官方元数据生成/核对，不能从 Android permission 一对一翻译。正式实现前新增 ADR/RFC，列出每个 Sico capability 对应：

| Sico capability | manifest/requestPermissions | user authorization | sandbox/系统限制 | Host 拒绝码 |
|---|---|---|---|---|
| network | 待目标 SDK 冻结 | 按权限级别 | endpoint/生命周期限制 | 待 RFC |
| user-selected file read | 优先 picker/临时 URI grant | 用户选择动作 | copy-only | 待 RFC |
| isolated storage | 通常无需 user grant | 无 | app sandbox + identity path | 待 RFC |

有效授权仍是 package declaration、Sico 用户 grant、平台授权与当前 lifecycle 的交集。受保护操作每次调用都重新检查，权限撤销后 fail closed。OpenHarmony 用户授权流程见 [request user authorization](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/security/AccessToken/request-user-authorization.md)。

`.sapp`、安装记录、grant 与 Runtime 数据只放应用沙箱。是否备份、跨设备迁移或共享必须单独设计；默认关闭。不得为了简化 picker 而申请广泛媒体/文件权限。

## 11. UIAbility 生命周期

Stage model 的 v0 状态映射：

| UIAbility callback | Host 动作 |
|---|---|
| Create | 初始化平台适配器；解析首次 Want；只恢复 immutable descriptor |
| Foreground | 重新验证 descriptor/grant；创建或恢复唯一 Runtime；开始 UI 事件 |
| Background | 停止新事件；按策略暂停/销毁；刷新非敏感状态 |
| `onNewWant` | 通过统一入口分类；串行切换/拒绝，不创建并行 Runtime |
| Destroy | 幂等取消任务、释放 Node-API/Rust 资源、清理 staging |

后台管理严格，不能依赖应用在后台长期执行。需要后台能力时必须采用匹配场景的 ExtensionAbility，并新增 capability/security review，不能让 UIAbility 暗中常驻。

恢复状态不保存包字节、FD、`napi_env`、C++/Rust 指针、Runtime handle、临时 permission token 或签名密钥。进程死亡后从严格安装记录和 digest 重建；若 revision 不存在或授权变化，显示明确拒绝。

## 12. ArkUI 原生 UI

首版使用 ArkUI 声明式原生组件映射共享最小 UI schema：

| 共享节点 | ArkUI 候选 | 约束 |
|---|---|---|
| vertical layout | `Column` | 有界 children/depth |
| horizontal layout | `Row` | 有界 children/depth |
| text | `Text` | 长度限制、可访问文本 |
| button | `Button` | 稳定 id/label、一次事件 |
| input | `TextInput`/`TextArea`（冻结其一） | UTF-8 4 KiB、IME composition 有界 |

参考 [ArkUI 开发框架](https://developer.huawei.com/consumer/cn/arkui/)。禁止 Web 运行包内脚本、动态加载包内 `.so`、反射创建任意组件。ArkUI state 只保存可显示数据；Runtime 和 permission state 由 Host 状态机拥有。

设备验收至少覆盖触摸、返回导航、软键盘中英文输入、composition、焦点顺序、字体放大、深色模式、横竖屏（若支持）、系统屏幕阅读器的 role/label/action。Previewer 截图不能代替设备交互证据。

## 13. Runtime backend 可行性探针

Rust OHOS target 可构建不等于 Wasmtime 能在目标系统安全运行。Wasmtime 官方平台页没有给出 OHOS 生产支持承诺，因此按最小、可停止的探针推进：

1. `sico-harmony-ffi` 在目标 SDK 编译链接；
2. HAP 内 native module 在 arm64 device 装载并返回版本；
3. bridge 对畸形/边界 envelope fail closed；
4. Wasmtime core 能编译/链接且加载固定 Component；
5. 优先使用 Pulley 执行结果 `42`；
6. 验证内存限制、trap、timeout/cancel、线程、unwind 与 Host 生存；
7. 调查系统 executable-memory/signals 后才允许原生 codegen 实验；
8. 每次记录 backend，绝不静默 fallback；
9. 任一步失败即保存日志并保持 NO-GO，不用 mock 结果替代。

平台判断参考 [Wasmtime platform support](https://docs.wasmtime.dev/stability-platform-support.html) 与 [stability tiers](https://docs.wasmtime.dev/stability-tiers.html)。如果只能编译共享 core、不能装载/执行 Component，状态只能是 `cross-compiled`，不是 Runtime 通过。

## 14. 构建、安装与测试

具体 hvigor task 名以生成工程的 wrapper help 为准。不要把其他版本博客里的命令当作当前 SDK 真值。首次恢复时：

```powershell
Push-Location harmony\host
.\hvigorw.bat --help
.\hvigorw.bat clean
# 从 --help/IDE build log 选择并记录当前版本的 HAP assemble task
Pop-Location

hdc list targets
hdc help
```

安装命令也应先以当前 `hdc help` 确认参数，然后把确切命令写进 evidence；商业 HarmonyOS 若要求通过 DevEco Run/签名 profile 安装，就保存 IDE build/install log，不伪造通用命令。

测试分层：

- Rust host tests：FFI 纯函数、所有权、limits、panic conversion；
- OHOS cross-build：两个目标检查链接、ELF 与导出符号；
- C++ unit tests：Node-API 参数、buffer copy、free/finalize；
- ArkTS unit tests：Want classifier、permission/lifecycle reducer、UI event limits；
- arkXtest：Ability、UI、设备交互和进程行为；
- arm64 device：native module、picker、permission、lifecycle、输入/无障碍、Runtime；
- x86_64 emulator：仅在所选 SDK 真正支持时加入，不能替代真机。

OpenHarmony [arkXtest 指南](https://gitee.com/openharmony/docs/blob/master/en/application-dev/application-test/arkxtest-guidelines.md) 覆盖 JsUnit 与 UiTest。每个平台 API 测试必须在 emulator/device 执行，不能只跑 Previewer。

## 15. HAP/APP、签名与分发

HAP 是模块包，APP 是面向分发的应用包；具体组合和签名以选定平台 SDK 为准。OpenHarmony 包结构参考 [HAP package](https://gitee.com/openharmony/docs/blob/OpenHarmony-v5.0.0-Release/zh-cn/application-dev/quick-start/hap-package.md)，签名工具参考 [hapsigner](https://gitee.com/openharmony/developtools_hapsigner/blob/master/README.md)。

规则：

- IDE 自动生成的 debug 证书只用于开发；
- bundle name、组织身份、release certificate/profile、密钥托管与轮换由所有者批准；
- HarmonyOS 对外提交按 [官方提交应用流程](https://developer.huawei.com/consumer/cn/app/submit) 准备 DevEco、SDK、测试和 AppGallery Connect 资料；
- 构建出 HAP/APP 不等于已满足商店、隐私、安全或企业发布要求；
- 不在仓库、日志、截图、evidence 保存私钥、密码、完整账号标识或设备序列号；
- 不把开发证书声明为 Sico publisher identity。

## 16. 最低设备验收矩阵

| 类别 | 必测内容 |
|---|---|
| 工程 | clean build、重复 build、HAP/APP 内容与 SHA-256 |
| Native | arm64 `.so` 装载、C ABI 边界、panic/exception、释放 |
| Want | 首次、`onNewWant`、unknown type、multiple URI、撤销授权 |
| 文件 | picker、64 MiB/64 MiB+1、复制中断、provider/FD 异常 |
| 权限 | grant/deny/revoke、后台调用、capability intersection |
| 生命周期 | 前后台、系统回收、旋转/窗口变化、升级与重复入口 |
| UI | 触摸、IME、4 KiB+1、焦点、字体、屏幕阅读器 |
| Runtime | Pulley `42`、trap、timeout、memory、cancel、Host 生存 |
| 安全 | 错签名、包篡改、revision/capability mismatch、畸形 bridge |

证据按 [共用格式](./MOBILE-SHARED-CHECKLIST.md#5-证据目录与清单) 保存，并增加 `platform-variant.json`、SDK component list、HAP/APP content list、native export list、Want/permission/lifecycle case files。只有 arm64 真机完整通过后，才可将状态升为 `runtime-verified`。

## 17. 故障处理

| 症状 | 首查 | 不可采用 |
|---|---|---|
| ArkTS import native module 失败 | module 注册名、生成 `.so` 名、ABI、HAP libs 内容 | 改名碰运气或吞异常 |
| Rust OHOS 链接失败 | SDK release、sysroot、clang target/API、linker wrapper | 退回 host clang 后声称成功 |
| Node-API 随机崩溃 | `napi_env` 线程、GC 生命周期、buffer ownership、finalize | 跨线程保存 env/裸指针 |
| Want 有 URI 但打不开 | 临时授权、FD 生命周期、provider 返回值 | 保存 URI 到下次启动再读 |
| 后台任务被终止 | UIAbility 生命周期、后台策略、是否需要 ExtensionAbility | 忽略系统限制或无限重启 |
| Previewer 正常真机失败 | native module、权限、SDK/设备 API、ABI | 用 Previewer 结果验收 |
| Wasmtime 编译或运行失败 | 未支持 OS API、signals、threads、executable memory | 把 Rust Tier 2 当 Wasmtime 支持证明 |
| 商业/开源设备行为不同 | platform variant、SDK、签名、系统 build | 合并两套结果为“鸿蒙已通过” |

## 18. 开工与完成门槛

开工门槛：平台变体、SDK/API、arm64 设备、签名/许可证责任人、Stage model ADR、FFI RFC、权限矩阵和证据目录均已确定。

完成门槛：干净构建、可安装签名包、arm64 真机入口/权限/生命周期/UI/Runtime/负向安全全部通过；同一 `.sapp` 与 Desktop/Android（如可用）得到相同 digest 和结果；全 workspace 回归通过；复核者能依文档重现。缺一项就保持 NO-GO。

## 19. 官方参考

- [HarmonyOS 文档中心](https://developer.huawei.com/consumer/cn/doc/)
- [HarmonyOS 开发入门](https://developer.huawei.com/consumer/cn/develop-novice-guide/)
- [Stage model](https://developer.huawei.com/consumer/cn/arkui/arkui-stage/)
- [ArkUI](https://developer.huawei.com/consumer/cn/arkui/)
- [应用规划与模板](https://developer.huawei.com/consumer/cn/app/planning/)
- [OpenHarmony UIAbility lifecycle](https://gitee.com/openharmony/docs/blob/master/en/application-dev/application-models/uiability-lifecycle.md)
- [OpenHarmony Node-API process](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/napi/use-napi-process.md)
- [OpenHarmony module configuration](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/quick-start/module-configuration-file.md)
- [OpenHarmony user authorization](https://gitee.com/openharmony/docs/blob/master/zh-cn/application-dev/security/AccessToken/request-user-authorization.md)
- [OpenHarmony app file sharing](https://gitee.com/openharmony/docs/blob/master/en/application-dev/file-management/share-app-file.md)
- [Rust OpenHarmony platform support](https://doc.rust-lang.org/stable/rustc/platform-support/openharmony.html)

API、工具链与商店要求会变化。每次恢复都要重查所选变体的官方文档并记录复核日期；若与手册冲突，先修改 ADR/RFC/手册，再修改代码。
