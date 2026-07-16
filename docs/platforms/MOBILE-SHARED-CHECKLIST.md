# Sico 移动平台共用开发与验收检查表

> 状态：实施检查表；不代表 Android 或鸿蒙已通过平台验收
> 最后复核：2026-07-16

## 1. 不可跨平台稀释的边界

Android 和鸿蒙可以共享 Rust 核心、`.sapp` 解析规则与桥接消息模型，但各自的入口、权限、生命周期、UI、动态库装载和设备证据必须独立完成。桌面结果、JVM 测试、主机 Rust 测试或另一移动平台的结果，均不能替代目标平台的 Runtime 证据。

每个平台都必须保持以下顺序：

1. 接收外部 URI、文件描述符或平台句柄；
2. 只接受已登记 action/type/scheme，并拒绝多文件与未知来源形态；
3. 流式复制到应用私有暂存区，同时执行 64 MiB 上限加一字节探测；
4. 关闭平台句柄，不把平台 URI/FD/指针传进 Rust 核心；
5. 用严格 `.sapp` loader 重新校验签名、清单、能力闭包、revision 与 digest；
6. 形成不可变 `appIdentity + revisionDigest` 运行描述符；
7. 计算声明能力、用户授权与平台权限的交集；
8. 创建单个活动 Runtime；后台/恢复只保存描述符，不序列化活 Runtime；
9. UI 事件经过 4 KiB 限制、有界队列和生命周期检查后再进入 Runtime；
10. 退出时销毁 Runtime、撤销临时授权并清理暂存文件。

桥接 envelope 的编码上限是 64 KiB，单个输入包上限是 64 MiB。任何平台若需要放宽上限，必须先修改共享 RFC/安全属性，不能只改平台常量。

## 2. 适配器对应关系

| 责任 | Android | HarmonyOS/OpenHarmony | 共享输出 |
|---|---|---|---|
| 外部入口 | `Intent`、Activity Result、`content://` | `Want`、DocumentViewPicker/URI/FD | 已复制的私有临时文件 |
| 生命周期 | Activity `onCreate/onStart/onResume/onPause/onStop/onDestroy`、`onNewIntent` | UIAbility Create/Foreground/Background/Destroy、`onNewWant` | 不可变运行描述符与显式状态迁移 |
| 权限 | manifest + runtime permission + SAF grant | `module.json5` requestPermissions + user authorization | capability intersection 或结构化拒绝 |
| UI | 原生 View 层次 | ArkUI 原生组件 | 同一最小 UI tree/event schema |
| Native 边界 | Kotlin/Java → JNI → Rust `cdylib` | ArkTS → Node-API C++ shim → Rust C ABI `cdylib` | `sico-mobile-host-core` 请求/响应 |
| Runtime | Wasmtime 原生或 Pulley，经设备探针选择 | Pulley 优先探针；原生后端须单独证明 | 同一 Component、输入与结果 |
| 调试设备 | x86_64 emulator + arm64 device | x86_64 emulator（若 SDK 支持）+ arm64 device | 分平台日志、摘要与设备清单 |

鸿蒙列目前是 `proposed`，不得据此声称 API 或二进制兼容。HarmonyOS 商业 SDK 与 OpenHarmony Public SDK 必须先选定一种并记录版本。

## 3. 实施顺序

- [ ] 记录 SDK、IDE、构建插件、Rust、目标三元组、设备 OS/API 与许可证来源；
- [ ] 新建平台 ADR，冻结变体、最低/目标 API、ABI、Runtime 后端选择规则；
- [ ] 新建桥接 RFC，冻结字节所有权、错误码、线程、panic/exception 和生命周期；
- [ ] 只建立能启动空原生页面的签名 Debug 工程；
- [ ] 构建并装载返回固定版本信息的 Rust 动态库；
- [ ] 接入共享 `sico-mobile-host-core`，通过 64 KiB 边界与畸形输入测试；
- [ ] 接入文件选择器与外部入口，证明 copy-before-verify；
- [ ] 接入权限交集与私有存储；
- [ ] 接入后台、恢复、进程死亡和重复入口状态机；
- [ ] 接入最小原生 UI、触摸、键盘/IME 与无障碍；
- [ ] 运行最小 Component，固定输出 `42`；
- [ ] 做 Desktop/Android/Harmony 同包、同输入、同 digest、同结果对比；
- [ ] 完成 release 构建，但不使用开发密钥冒充 Sico 生产发布者；
- [ ] 保存机器可读证据并运行仓库全量回归。

## 4. 每个平台最低测试矩阵

| 类别 | 必需用例 | 通过条件 |
|---|---|---|
| 构建 | clean Debug，连续两次构建 | 均成功；锁文件与版本清单可追溯 |
| ABI | arm64 实机；支持时 x86_64 模拟器 | 动态库从包内装载，ABI 与设备匹配 |
| 入口 | 选择器、受支持外部打开、未知 scheme/type、多文件 | 前两项复制后验证；后三项稳定拒绝 |
| 边界 | 0、1、64 KiB、64 KiB+1 envelope；64 MiB、64 MiB+1 包 | 上限内确定性；超限在分配/执行前拒绝 |
| 安全 | 错签名、篡改、capability mismatch、revision mismatch | 全部 fail closed，错误分类稳定 |
| 生命周期 | 冷启动、后台/前台、旋转/配置变化、进程回收、重复入口 | 无双 Runtime；只由描述符重开 |
| UI | 触摸、文本输入、4 KiB+1、焦点、屏幕阅读器 | 事件有界；可操作控件可访问 |
| Runtime | 正常 `42`、trap、timeout、memory limit、cancel | Host 存活，资源回收，故障分类一致 |
| 卸载/升级 | 覆盖安装、数据保留策略、卸载 | 行为与文档一致，无共享目录残留 |

## 5. 证据目录与清单

每次平台验收创建 `tests/platform/evidence/<platform>/<YYYY-MM-DD>/<run-id>/`，至少保存：

```text
environment.json
build.log
install.log
device.json
runtime.json
ui-accessibility.json
security-cases.json
artifacts.sha256
summary.md
```

`environment.json` 至少包含平台变体、SDK/API、IDE/插件、NDK/native SDK、Rust 版本、targets、设备型号/架构/系统版本；`runtime.json` 必须明确 backend 是 native 还是 Pulley；`artifacts.sha256` 必须覆盖 APK/AAB/HAP/APP、Rust 动态库、测试 `.sapp` 和结果文件。日志应删除个人路径、账号、设备序列号和密钥材料。

## 6. GO/NO-GO 规则

只有以下项目全部具备，单个平台才能从 `blocked/proposed` 升级为 `runtime-verified`：

- 可从干净工作区按文档构建、安装和启动；
- arm64 真机完成入口、权限、生命周期、UI 与 Runtime 测试；
- 模拟器结果不能替代真机结果；
- 真实包摘要、动态库摘要和设备信息已归档；
- 所有负向安全用例 fail closed；
- 平台全量测试和 Rust workspace 回归通过；
- 复核者能仅用仓库文档重现；
- 没有把 debug/development 身份描述为生产身份。

任一项缺失时，状态保持 `NO-GO`，但不阻塞与移动 Runtime 无关的工作。

## 7. 所有者操作边界

下列动作必须由项目所有者授权或亲自完成：接受商业 SDK/商店条款、登录厂商账号、创建生产签名身份、连接公开发布服务、支付开发者费用、上传 AppGallery/应用商店、登记公司/隐私/合规资料。开发者可以准备离线工程、命令和占位配置，但不能代替所有者接受条款，也不能把开发证书提升为正式信任根。
