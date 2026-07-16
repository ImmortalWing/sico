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

Runtime 查找顺序为 `--runtime`、`SICO_WASMTIME`、`PATH`。

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
