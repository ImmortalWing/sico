# Sico 用户手册

Sico（Simple Coding）是一门面向 AI 理解、生成、检查和修复代码的正规编程语言。源码经过静态检查和 Sico IR lowering，编译为 WebAssembly Component，并以 `.sapp` 作为应用包交给 Sico Runtime/Host 执行。

> - 当前版本：`v0.0.1`
> - 发布性质：1.0 前工程里程碑，语言和工具接口可能不兼容变化
> - 当前受验证平台：Windows x86_64 + Wasmtime 46.0.1
> - 产品状态：仓库本地轨已闭环；production 与跨平台交付仍受外部证据阻塞

## 快速开始

Sico 尚未提供正式安装器，当前从源码构建。需要 Git、rustup 和 PowerShell。

```powershell
git clone https://github.com/ImmortalWing/sico.git
cd sico
cargo build --locked --release -p sico-cli
$env:SICO_WASMTIME = & .\tools\ensure-wasmtime.ps1
$env:PATH = "$PWD\target\release;$env:PATH"
sico --version
```

创建 UTF-8（无 BOM）的 `hello.sico`：

```sico
function main() returns Int:
  return 40 + 2
end function
```

检查、格式化、运行、构建和检查 package：

```powershell
sico check hello.sico
sico format --check hello.sico
sico run hello.sico
sico build --app-id dev.example.hello --app-version 0.0.1 hello.sico
sico inspect hello.sapp
sico run --allow-unsigned-dev hello.sapp
```

预期运行结果：

```text
42
```

`--allow-unsigned-dev` 只适用于可信的本地开发包。签名、信任和 capability 配置请阅读详细手册。

## 常用命令

| 命令 | 用途 |
|---|---|
| `sico check [--json] FILE` | 检查 syntax 和 static semantics |
| `sico format [--check\|--write] FILE` | 输出、检查或写入规范格式 |
| `sico outline [--json] FILE` | 查看顶层声明 |
| `sico build [OPTIONS] FILE` | 构建 deterministic `.sapp` |
| `sico inspect [--json] PACKAGE` | 严格验证并检查 package，不执行 |
| `sico run [OPTIONS] FILE` | 运行源码或经过 trust gate 的 `.sapp` |

执行 `sico <COMMAND> --help` 查看当前二进制的完整参数。

## 详细用户手册

- [用户手册索引](./docs/user-guide/README.md)
- [安装与构建](./docs/user-guide/INSTALLATION.md)
- [五分钟入门](./docs/user-guide/GETTING-STARTED.md)
- [语言基础与可运行子集](./docs/user-guide/LANGUAGE-BASICS.md)
- [CLI 完整参考](./docs/user-guide/CLI.md)
- [构建、运行与缓存](./docs/user-guide/BUILD-RUN.md)
- [包、签名与信任](./docs/user-guide/PACKAGES-AND-TRUST.md)
- [编辑器与 LSP](./docs/user-guide/EDITOR.md)
- [AI 工具](./docs/user-guide/AI-TOOLS.md)
- [故障排查](./docs/user-guide/TROUBLESHOOTING.md)
- [限制与平台状态](./docs/user-guide/LIMITATIONS.md)

## 使用前需要知道

- 当前 Component/Runtime 真正端到端验证的是同步 scalar 子集，主要入口返回 `Int`、`Bool` 或 `Unit`。
- 前端能够检查 records、enums、Result、resources 和 async 等更丰富语义，但部分程序仍会在 codegen 阶段得到明确 refusal。
- `examples/` 是语言设计历史与编译器回归资料，不是当前入门教程。
- `.sapp` 的 development signature 不等于 production publisher identity。
- 当前没有官方编辑器扩展、source debugger、production installer 或公共 registry。
- Android、Harmony 和 Linux 手册不代表这些平台已经 runtime-verified。

完整限制见[限制与平台状态](./docs/user-guide/LIMITATIONS.md)。

## 开发 Sico 本身

编译器、Runtime、Host、平台适配和仓库贡献说明已迁移到：

- [Sico 开发手册](./docs/development/README.md)
- [完整架构与开发设计](./DEVELOPMENT.md)
- [项目状态与审计记录](./docs/README.md)

用户文档只描述当前可执行功能；设计历史、实验边界和未完成平台工作以开发手册及审计记录为准。
