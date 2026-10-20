# 限制与平台状态

`main` 当前版本为 `0.1.0`，不是跨平台 production release。已发布的 `v0.0.1` 保持不变。

## 当前平台状态

| 平台 | 状态 | 可以声明的内容 |
|---|---|---|
| Windows x86_64 | runtime-verified | compiler、Wasmtime、development `.sapp`、Desktop Host 测试路径 |
| macOS | contract-verified | association contract；无原生 runner |
| Linux | runner runtime-verified（无 Desktop Host） | runner 级已在 WSL2 Ubuntu-24.04（Linux x64 native）重跑全部 provider/runner/http2 语料全绿（STEP-0126），M14 Linux 14/14 套件全绿（STEP-0141）；仍无 Linux Desktop Host 与安装器 |
| Android | blocked-not-implemented | shared core/Kotlin contract；无完整 Gradle/JNI/APK/device 路径 |
| HarmonyOS/OpenHarmony | proposed-not-implemented | 开发手册；平台选择、工程、HAP/Node-API/runner 未完成 |

## 语言与 Runtime 限制

- 语言前端覆盖范围大于 Component codegen；精确到每个特性的 check/build/run 结果以 [application-profile-v0](../../tests/language-matrix/application-profile-v0.json) 矩阵为唯一权威来源；
- Script Profile 已验证 aggregate `ScriptInput/ScriptOutput/Error`、bounded args/stdin 和 streaming resources；
- task scopes 为顺序执行（parallelism-1）；M11 的有界 scheduler、downward 取消树与 race/select 已在 runner 落地，但一等 Task/Future/Stream 值仍为 typed 拒绝；
- `sico watch` 只接受单文件与 buffered empty stdin，streaming stdin 不可重放；
- source-level debugging 已实现 minimal DAP v0 子集（12 个支持的请求、20 个 typed 拒绝、6 个事件，Windows runtime-verified）；没有已发布的编辑器扩展，需要手工配置，见[编辑器与 LSP](./EDITOR.md)；
- `sico-app` 只接收 Component 或 `.sapp`，不会隐式编译源码；
- 本地源码执行使用 `sico run`；persistent 单文件重编译使用 `sico watch`。

## 工具限制

- 没有官方编辑器扩展，只有通用 stdio LSP binary；
- LSP 没有 rename、semantic tokens、workspace watcher 或 persistent index；
- `sico-ai-tool` 不调用模型，只检查和验证结构化输入；
- `sico test` 已可用（STEP-0140）：发现 `NAME.sico` + `NAME.test.json` 测试对并经真实 `sico-runner` 在默认有界限制下执行；manifest 字段 `stdin`/`args`/`expect_exit`/`expect_stdout` 全可选且拒绝未知字段，详见 [CLI 参考](./CLI.md)；
- Component 编译、应用打包和 Runtime 运行必须使用不同命令。

## 发布与信任限制

- 没有 production installer；
- Windows zip 只是 smoke distribution；
- development signature 不等于 production identity；
- 本地 publisher policy/registry/update 是验证 fixture，不是公共服务；
- 没有 production key custody、透明度服务或独立第三方 pilot 证据。

## AI 证据边界

已完成一次权威 live-model 评测（STEP-0128，报告 [ai-eval-live-model-v1](../reports/ai-eval-live-model-v1.md)）：DeepSeek `deepseek-chat`，2880 次真实调用，实际成本 USD 0.78，总体 0.909544（2245/2880 全对），达到 ADR-0012 修订后的 ≥ 0.900 目标。已登记缺口：B-repair 0.833。该成绩只绑定上述模型、prompt、语料与预算，不是通用模型成绩；离线 compiler oracle、synthetic harness 和固定任务集也不能解释为其他模型的准确率、延迟、token 或成本成绩。

权威状态见[项目状态](../STATUS.md)、[M7 退出审计](../reports/m7-exit-audit.md)和[平台开发手册](../platforms/README.md)。
