# 编译、打包与运行

当前模块化管线明确分为三步：

```text
source.sico
    │ sico build
    ▼
WebAssembly Component
    │ sico-app pack
    ▼
.sapp
    │ sico-app run / sico-desktop-host open
    ▼
Wasmtime / platform Host
```

## 开发期快捷运行

安装 SDK 后执行一个命令即可临时编译、打包并运行源码：

```powershell
sico-app dev .\demo.sico
```

该命令只适合运行自己创建且来源可信的本地源码。它创建唯一的临时工作目录，以 unsigned development trust 运行，并在退出时删除 Component 和 `.sapp`。编译器仍作为独立进程调用，`sico-app` 没有获得编译器 crate 依赖。

从独立 PowerShell 窗口或快捷方式启动、需要在结果后等待确认时执行：

```powershell
sico-app dev .\demo.sico
```

从文件管理器启动并需要暂停窗口时，仍可使用兼容脚本 `tools/sico-dev.ps1 -Pause`。

保留中间产物用于排查时执行：

```powershell
sico-app dev .\demo.sico --keep-artifacts
```

需要能力授权时可以继续传递开发参数：

```powershell
sico-app dev .\demo.sico `
  --grant storage.read-write `
  --storage-root .\app-storage
```

快捷命令属于 `sico-app` 应用工具，不是 `sico` 编译器子命令，也不会让编译器依赖 package、Runtime 或 Host。

## 编译 Component

```powershell
sico build -o demo.component.wasm app.sico
```

同一源码和编译器版本产生确定性 Component。语言 CLI 不读取签名密钥、不选择 Runtime，也不创建应用缓存。

## 构建应用包

```powershell
sico-app pack `
  --app-id dev.example.demo `
  --app-version 0.0.2 `
  -o demo.sapp `
  demo.component.wasm
```

`.sapp` 包含 canonical manifest、Component、资源、hash 和可选 development signature。同一输入与配置产生确定性 package bytes。写入使用临时文件并拒绝覆盖现有产物。

## 运行应用包

unsigned development package：

```powershell
sico-app run --allow-unsigned-dev demo.sapp
```

development-signed package：

```powershell
sico-app run --trusted-key trusted-public-key.hex demo.sapp
```

Runtime 查找顺序为 `--runtime`、`SICO_WASMTIME`、已安装 SDK 的 bundled Runtime、`PATH`。

## 为什么不再直接运行源码

`main` 不提供 `sico run app.sico`。显式的编译、打包、授权和运行步骤使以下边界可审计：

- 编译器不获得签名密钥或 Host 权限；
- Runtime 不隐式编译源码；
- `.sapp` 是唯一应用信任输入；
- 平台 Host 只消费已经验证和授权的包。

旧的一体化 source-run/cache 流程保存在 `v0.0.1` 与归档分支中。

## stdout、stderr 与退出码

- guest stdout/result 转发到 stdout；
- guest stderr 与 Host/tool error 使用 stderr；
- Runtime stdin 当前关闭；
- 非空应用参数当前拒绝；
- timeout、资源超限和 trap 使用稳定的非零退出码。

## 能力授权

```powershell
sico-app run `
  --allow-unsigned-dev `
  --grant storage.read-write `
  --storage-root .\app-storage `
  demo.sapp
```

Host grant 不能创造 package 没有请求的能力。先用 `sico-app inspect --json` 查看已验证请求；storage grant 缺少 `--storage-root` 会被拒绝。
