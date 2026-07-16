# 安装与构建

当前开发线 `0.0.2-dev` 尚未提供正式安装器或包管理器发布，受验证方式是从源码构建。发布标签 `v0.0.1` 保留旧的一体化 CLI；本页描述 `main` 的模块化命令。

## 当前受验证环境

- Windows x86_64；
- Rust `1.97.0-x86_64-pc-windows-gnu`；
- PowerShell；
- Wasmtime `46.0.1`。

macOS/Linux 只有契约证据；Android 与 HarmonyOS/OpenHarmony 尚不能作为可用安装目标。详见[限制与平台状态](./LIMITATIONS.md)。

## 获取源码

```powershell
git clone https://github.com/ImmortalWing/sico.git
cd sico
```

GitCode 镜像：

```powershell
git clone https://gitcode.com/ImmortalWings/sico.git
cd sico
```

复现已发布的旧流程：

```powershell
git checkout v0.0.1
```

## 安装 Rust 工具链

```powershell
rustup toolchain install 1.97.0-x86_64-pc-windows-gnu --profile minimal --component clippy,rustfmt
```

## 构建用户工具

```powershell
cargo build --locked --release `
  -p sico-cli `
  -p sico-app-cli `
  -p sico-language-server `
  -p sico-ai-tools
```

主要产物：

```text
target/release/sico.exe
target/release/sico-app.exe
target/release/sico-lsp.exe
target/release/sico-ai-tool.exe
```

验证：

```powershell
.\target\release\sico.exe --version
.\target\release\sico-app.exe --version
```

## 准备 Runtime

```powershell
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
& $env:SICO_WASMTIME --version
```

也可以把兼容的 `wasmtime` 放入 `PATH`，或向 `sico-app run` 传入 `--runtime <PATH>`。自动化运行证据当前只覆盖 Wasmtime `46.0.1`。

可选地把工具加入当前会话：

```powershell
$env:PATH = "$PWD\target\release;$env:PATH"
sico --help
sico-app --help
```

不要把 `target/` 中的开发构建误认为正式签名安装器。
