# 平台开发文档

本目录保存 Android、鸿蒙与 Linux 平台的可恢复开发手册。它们是实施说明，不是平台实现或目标平台验收证明。

| 文档 | 当前状态 | 用途 |
|---|---|---|
| [Android 开发手册](./ANDROID-DEVELOPMENT.md) | 契约已冻结，工程与设备证据缺失 | 从现有 Kotlin 适配器补齐可构建 Host、JNI、APK/AAB 与设备验收 |
| [鸿蒙开发手册](./HARMONY-DEVELOPMENT.md) | 预研方案，平台基线与工程未冻结 | 选择 HarmonyOS/OpenHarmony 后建立 Stage/ArkUI/Node-API/Rust Host |
| [Linux 开发手册](./LINUX-DEVELOPMENT.md) | 关联契约已验证，原生 build/UI/Runtime 未验证 | 补齐 XDG、MIME、GTK、Wayland/X11、进程监管、打包与 Linux runner 验收 |
| [移动平台共用检查表](./MOBILE-SHARED-CHECKLIST.md) | 可执行检查表 | 统一安全边界、证据格式、跨平台一致性和交付门槛 |
| [机器可读状态](../../tests/platform/mobile-documentation-contract.json) | 文档契约 | 防止状态、目标架构和限制在后续修改中漂移 |
| [Linux 机器可读状态](../../tests/platform/linux-documentation-contract.json) | 文档契约 | 冻结当前代码缺口、建议目标与实施顺序 |

状态词只允许按以下含义使用：

- `verified`：仓库内有可重复命令与对应产物/日志；
- `contract-verified`：仅共享核心或适配器契约通过主机测试；
- `blocked`：恢复条件已知，但当前环境不具备；
- `proposed`：尚需 ADR/RFC、工具链或设备试验确认；
- `not-implemented`：没有可构建、可运行的平台实现。

最后复核：2026-07-16。
