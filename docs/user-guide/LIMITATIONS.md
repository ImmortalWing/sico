# 限制与平台状态

Sico `v0.0.1` 是工程里程碑，不是跨平台 production release。

## 当前平台状态

| 平台 | 状态 | 可以宣称的内容 |
|---|---|---|
| Windows x86_64 | runtime-verified | CLI、Wasmtime、development `.sapp`、Desktop Host 测试路径 |
| macOS | contract-verified | 生成的 association contract；无原生 runner |
| Linux | contract-verified-not-runtime-verified | XDG/MIME contract 与开发手册；无原生 Host/Runtime runner |
| Android | blocked-not-implemented | shared core/Kotlin contract；无完整 Gradle/JNI/APK/device 路径 |
| HarmonyOS/OpenHarmony | proposed-not-implemented | 开发手册；平台选择、工程、HAP/Node-API/runner 均未完成 |

## 语言与 Runtime 限制

- 语言前端覆盖范围大于 Component codegen；
- 已验证 Runtime 入口主要是同步 scalar `main()`；
- 部分函数调用和复杂 operation 尚不能 codegen；
- Task/Future/Stream 有静态/协议证据，但 source backend 明确拒绝；
- application args 与 guest stdin 尚未实现；
- source-level debugging/DAP 尚未实现；
- 一般 cross-package build、public dependency source 和 stable standard library 尚未形成用户级发布服务。

## 工具限制

- 没有官方编辑器扩展，只有通用 stdio LSP binary；
- LSP 没有 rename、semantic tokens、workspace watcher 或 persistent index；
- `sico-ai-tool` 不生成修复、不调用模型，只检查和验证结构化输入；
- `sico test` 不是现有 CLI 子命令；
- `--raw-component` 不属于应用分发流程。

## 发布与信任限制

- 没有 production installer；
- Windows zip 是 smoke distribution；
- development signature 不等于 production identity；
- 本地 publisher policy/registry/update 是验证 fixture，不是公共服务；
- 没有公开 namespace、法律条款、生产密钥托管或透明度服务证据；
- 没有独立第三方完成并回传 pilot 证据。

## AI 证据限制

当前模型调用次数为 0。离线 compiler oracle、synthetic harness 和固定任务集不能被解释为真实模型准确率、延迟、token 或成本成绩。

## 权威状态

- [项目状态](../STATUS.md)
- [M7 与项目退出审计](../reports/m7-exit-audit.md)
- [平台开发手册](../platforms/README.md)
- [项目完成度审计](../reports/project-completion-audit.md)

只有新增的原始 runner/production/third-party evidence 通过审计后，才能提高上述证据等级。
