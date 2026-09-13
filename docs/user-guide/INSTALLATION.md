# 安装与构建

当前开发线 `0.1.0` 提供 Windows SDK 的终端安装脚本，但尚未提供图形安装器或包管理器发布。发布标签 `v0.0.1` 保留旧的一体化 CLI；本页描述 `main` 的模块化命令。

## 当前受验证环境

- Windows x86_64；
- Rust `1.98.0-x86_64-pc-windows-gnu`；
- PowerShell；
- Wasmtime `46.0.1`。

macOS/Linux 只有契约证据；Android 与 HarmonyOS/OpenHarmony 尚不能作为可用安装目标。详见[限制与平台状态](./LIMITATIONS.md)。

## 使用 EXE 安装（推荐）

从 Release 下载与系统架构匹配的 `sico-init-v<VERSION>-x86_64-pc-windows-gnu.exe`，双击运行即可。它是离线、自包含的用户级安装器，会：

- 安装到 `%LOCALAPPDATA%\Programs\Sico`；
- 持久加入当前用户 PATH；
- 验证编译器和内置 Wasmtime；
- 完成后显示结果，不需要打开终端；
- 不需要管理员权限，也不需要预先安装 Rust/Cargo。

安装完成后，新开 PowerShell 或 CMD 验证：

```powershell
sico --version
sico-app --version
```

当前 EXE 尚未做 Windows Authenticode 签名，因此 Windows 可能显示“未知发布者”或 SmartScreen 提示。只应从项目的 GitHub/GitCode Release 下载，并按 `SHA256SUMS` 核对摘要。

## 更新 Sico

当前没有后台自动更新服务。下载更高版本的 `sico-init-*.exe` 并再次双击即可更新；安装器会验证新版本、替换已有 Sico 文件，并保持 PATH 中只有一个安装目录。

使用 SDK ZIP 时，解压新版本并重新运行：

```powershell
.\install-windows.cmd
```

更新不会删除用户的 `.sico` 源码或项目目录。发布版本回退目前没有单独入口；需要回退时先卸载，再安装目标版本。

## 从 Windows SDK 安装（高级方式）

下载并解压 `sico-sdk-v<VERSION>-x86_64-pc-windows-gnu.zip`，在解压目录打开 PowerShell，然后执行：

```powershell
.\install-windows.cmd
```

默认安装到 `%LOCALAPPDATA%\Programs\Sico`，并把它的 `bin` 目录持久加入**当前用户 PATH**。关闭并重新打开终端后验证：

```powershell
sico --version
sico-app --version
sico-dev .\hello.sico
```

脚本重复运行会安全更新已有的 Sico 安装，不会重复添加 PATH 项。

如果确实要让本机所有用户使用，请先以管理员身份打开 PowerShell，再执行：

```powershell
.\install-windows.cmd -Scope Machine
```

机器级安装默认位于 `%ProgramFiles%\Sico`。普通个人电脑建议使用默认的用户级安装，避免不必要的管理员权限。

卸载时必须使用与安装时相同的作用域：

```powershell
# 用户级卸载
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\Sico\install-windows.ps1" -Action Uninstall

# 机器级卸载（管理员终端）
powershell -ExecutionPolicy Bypass -File "$env:ProgramFiles\Sico\install-windows.ps1" -Action Uninstall -Scope Machine
```

卸载器只会删除带有 Sico 安装标记的目录，并同步移除对应 PATH 项。

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
rustup toolchain install 1.98.0-x86_64-pc-windows-gnu --profile minimal --component clippy,rustfmt
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
