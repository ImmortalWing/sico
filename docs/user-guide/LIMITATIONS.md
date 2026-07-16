# 限制与平台状态

`main` 当前版本为 `0.0.2-dev`，不是跨平台 production release。已发布的 `v0.0.1` 保持不变。

## 当前平台状态

| 平台 | 状态 | 可以声明的内容 |
|---|---|---|
| Windows x86_64 | runtime-verified | compiler、Wasmtime、development `.sapp`、Desktop Host 测试路径 |
| macOS | contract-verified | association contract；无原生 runner |
| Linux | contract-verified-not-runtime-verified | XDG/MIME contract 与开发手册；无原生 Host/Runtime runner |
| Android | blocked-not-implemented | shared core/Kotlin contract；无完整 Gradle/JNI/APK/device 路径 |
| HarmonyOS/OpenHarmony | proposed-not-implemented | 开发手册；平台选择、工程、HAP/Node-API/runner 未完成 |

## 语言与 Runtime 限制

- 语言前端覆盖范围大于 Component codegen；
- 已验证 Runtime 入口主要是同步 scalar `main()`；
- Task/Future/Stream 有静态协议证据，但 source backend 会明确拒绝；
- application args 与 guest stdin 尚未实现；
- source-level debugging/DAP 尚未实现；
- `sico-app` 只接收 Component 或 `.sapp`，不会隐式编译源码；
- 旧 source-run/cache 只保存在 `v0.0.1` 归档中。

## 工具限制

- 没有官方编辑器扩展，只有通用 stdio LSP binary；
- LSP 没有 rename、semantic tokens、workspace watcher 或 persistent index；
- `sico-ai-tool` 不调用模型，只检查和验证结构化输入；
- `sico test` 不是现有 CLI 子命令；
- Component 编译、应用打包和 Runtime 运行必须使用不同命令。

## 发布与信任限制

- 没有 production installer；
- Windows zip 只是 smoke distribution；
- development signature 不等于 production identity；
- 本地 publisher policy/registry/update 是验证 fixture，不是公共服务；
- 没有 production key custody、透明度服务或独立第三方 pilot 证据。

## AI 证据限制

当前真实模型调用次数为 0。离线 compiler oracle、synthetic harness 和固定任务集不能解释为真实模型准确率、延迟、token 或成本成绩。

权威状态见[项目状态](../STATUS.md)、[M7 退出审计](../reports/m7-exit-audit.md)和[平台开发手册](../platforms/README.md)。
