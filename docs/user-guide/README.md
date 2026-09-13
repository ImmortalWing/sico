# Sico 用户手册

本目录描述 Sico `main`（`0.1.0`）当前真实可用的模块化用户流程。已发布的 `v0.0.1` 及其一体化 CLI 保存在标签和归档分支中。

## 推荐阅读顺序

1. [安装与构建](./INSTALLATION.md)：安装 Windows SDK，或从源码构建 `sico`、`sico-app`、`sico-lsp` 和 `sico-ai-tool`。
2. [五分钟入门](./GETTING-STARTED.md)：检查、格式化、构建、检查包并运行第一个程序。
3. [语言基础与可运行子集](./LANGUAGE-BASICS.md)：当前语法和 compiler/Runtime 支持边界。
4. [CLI 参考](./CLI.md)：所有命令、参数、输入输出和退出码。
5. [编译、打包与运行](./BUILD-RUN.md)：源码、Component、`.sapp`、Wasmtime 和能力授权。
6. [包、签名与信任](./PACKAGES-AND-TRUST.md)：unsigned development、开发签名与本地信任。
7. [编辑器与 LSP](./EDITOR.md)：`sico-lsp` 的 stdio 接入和功能限制。
8. [AI 工具](./AI-TOOLS.md)：compiler-backed inspect/fix JSON 协议。
9. [故障排查](./TROUBLESHOOTING.md)：常见错误、退出码和修复方法。
10. [限制与平台状态](./LIMITATIONS.md)：尚未实现或没有证据的能力。

## 文档约定

- SDK 用户的命令示例可在任意目录执行；源码开发命令以仓库根目录为当前目录。
- Windows 是当前唯一 runtime-verified 的 Desktop Host 平台。
- 终端安装器会持久配置 PATH；源码构建的 `target/release/sico.exe` 与 `sico-app.exe` 也可在临时加入 PATH 后简写。
- 用户应用分发格式是 `.sapp`；语言 CLI 只生成 WebAssembly Component。
- “签名有效”不等于“本机信任”，development key 也不等于 production publisher。

如果要参与 Sico 自身开发，请转到[开发手册](../development/README.md)。
