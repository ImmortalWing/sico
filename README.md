# Sico 用户手册

Sico（Simple Coding）是一门面向 AI 理解、生成、检查和修复代码的编程语言。`main` 采用类似 OpenJDK 的模块化单仓库：语言编译器、应用 Runtime 和平台 Host 同仓开发，但具有独立依赖边界、命令和发布物。

> - 当前开发版本：`0.1.0`
> - 已归档版本：`v0.0.1`
> - 当前 runtime-verified 平台：Windows x86_64 + Wasmtime 46.0.1
> - Android、HarmonyOS/OpenHarmony 与 Linux 原生 Host 尚未完成运行时验证

需要复现 `v0.0.1` 的原始开发流程时，请检出标签 `v0.0.1` 或分支 `codex/archive-v0.0.1-development-history`。以下文档描述当前 `main`。

## 快速开始

普通用户从 Release 下载 `sico-init-v<VERSION>-x86_64-pc-windows-gnu.exe`，双击即可完成安装和用户 PATH 配置，不需要管理员权限或 Rust/Cargo。安装完成后重新打开终端：

```powershell
sico --version
```

SDK ZIP 内仍保留 `install-windows.cmd/.ps1`，供自动化、机器级安装和卸载使用。详见[安装与构建](./docs/user-guide/INSTALLATION.md)。

从源码开发时，安装 Git、Rust `1.98.0` 与 PowerShell，然后在仓库根目录执行：

```powershell
cargo build --locked --release -p sico-cli -p sico-app-cli
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
$env:PATH = "$PWD\target\release;$env:PATH"
```

创建 `hello.sico`：

```sico
function main() returns Int:
  return 40 + 2
end function
```

开发时可以用一个命令完成临时编译、打包和运行：

```powershell
sico-app dev .\hello.sico
```

命令使用显式的本地 unsigned development trust，运行结束后删除临时 Component 和 `.sapp`；它直接调用独立 `sico` 编译器，不改变 `sico`、`sico-app` 和 Runtime 的模块边界。

在独立窗口中运行且希望查看结果后再关闭时使用：

```powershell
.\tools\sico-dev.ps1 .\hello.sico -Pause
```

显式完成编译、打包、检查和运行：

```powershell
sico check hello.sico
sico format --check hello.sico
sico build -o hello.component.wasm hello.sico
sico-app pack --app-id dev.example.hello --app-version 0.0.2 -o hello.sapp hello.component.wasm
sico-app inspect --json hello.sapp
sico-app run --allow-unsigned-dev hello.sapp
```

预期运行结果：

```text
42
```

`--allow-unsigned-dev` 只适用于自己构建且来源可信的本地开发包。

## 命令所有权

| 命令 | 职责 |
|---|---|
| `sico check` | 检查语法与静态语义 |
| `sico format` | 输出、检查或写入规范格式 |
| `sico outline` | 输出顶层声明 |
| `sico build` | 把源码编译为 WebAssembly Component |
| `sico-app pack` | 把 Component 打包并可选签名为 `.sapp` |
| `sico-app inspect` | 验证并检查 `.sapp`，不执行 |
| `sico-app run` | 通过 trust/capability gate 运行 `.sapp` |
| `sico-app dev` | 通过独立编译器和相同 trust/Runtime gate 一命令运行本地源码 |
| `sico-desktop-host` | 安装、打开和管理桌面应用生命周期 |
| `sico-registry` | 只读提供已经签名的 registry transport tree（operator 工具） |

`sico` 不依赖 Runtime 或 Host；`sico-app` 不隐式编译源码。这个显式边界可避免把语言检查、应用信任和平台权限混成一个操作。

## 详细手册

- [用户手册索引](./docs/user-guide/README.md)
- [安装与构建](./docs/user-guide/INSTALLATION.md)
- [五分钟入门](./docs/user-guide/GETTING-STARTED.md)
- [语言基础与可运行子集](./docs/user-guide/LANGUAGE-BASICS.md)
- [CLI 参数](./docs/user-guide/CLI.md)
- [编译、打包与运行](./docs/user-guide/BUILD-RUN.md)
- [包、签名与信任](./docs/user-guide/PACKAGES-AND-TRUST.md)
- [编辑器与 LSP](./docs/user-guide/EDITOR.md)
- [AI 工具](./docs/user-guide/AI-TOOLS.md)
- [故障排查](./docs/user-guide/TROUBLESHOOTING.md)
- [限制与平台状态](./docs/user-guide/LIMITATIONS.md)

## 开发 Sico

- [开发手册](./docs/development/README.md)
- [模块边界](./docs/development/MODULE-BOUNDARIES.md)
- [Windows 手动发布](./docs/development/RELEASING.md)
- [Registry origin 部署](./deploy/registry-origin/README.md)
- [完整架构与设计](./DEVELOPMENT.md)
- [项目状态与审计记录](./docs/README.md)

根 README 与用户手册只描述当前 `main` 的可执行界面；历史设计与旧命令请从归档分支读取。
