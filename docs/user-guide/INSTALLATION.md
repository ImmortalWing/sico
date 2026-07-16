# 安装与构建

Sico `v0.0.1` 尚未提供正式安装器或包管理器发布。当前受验证的使用方式是从源码构建。

## 当前受验证环境

- Windows x86_64；
- Rust `1.97.0-x86_64-pc-windows-gnu`；
- PowerShell；
- Wasmtime `46.0.1`，由仓库脚本下载并校验 SHA-256。

macOS/Linux 只有契约证据；Android 与 HarmonyOS/OpenHarmony 尚不能作为可用安装目标。详见[限制与平台状态](./LIMITATIONS.md)。

## 获取源码

GitHub：

```powershell
git clone https://github.com/ImmortalWing/sico.git
cd sico
```

GitCode/AtomGit 镜像：

```powershell
git clone https://gitcode.com/ImmortalWings/sico.git
cd sico
```

发布版本应检出明确标签：

```powershell
git checkout v0.0.1
```

## 安装 Rust 工具链

仓库根目录的 `rust-toolchain.toml` 会请求固定工具链及 `clippy`、`rustfmt`。已有 rustup 时执行：

```powershell
rustup toolchain install 1.97.0-x86_64-pc-windows-gnu --profile minimal --component clippy,rustfmt
```

## 构建用户工具

```powershell
cargo build --locked --release -p sico-cli -p sico-language-server -p sico-ai-tools
```

生成：

```text
target/release/sico.exe
target/release/sico-lsp.exe
target/release/sico-ai-tool.exe
```

验证版本：

```powershell
.\target\release\sico.exe --version
```

## 准备 Runtime

Windows 可使用仓库固定并校验的 Wasmtime：

```powershell
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
& $env:SICO_WASMTIME --version
```

也可以把兼容的 `wasmtime` 放入 `PATH`，或在每次 `sico run` 时传入 `--runtime <PATH>`。当前自动化证据只覆盖 Wasmtime `46.0.1`。

## 可选：加入当前会话 PATH

```powershell
$env:PATH = "$PWD\target\release;$env:PATH"
sico --help
```

不要把 `target/` 中的开发构建误认为正式签名安装器。Windows Desktop Host 的 zip 也只是 smoke distribution。
