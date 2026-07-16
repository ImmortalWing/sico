# Sico 用户手册

本目录描述 Sico `v0.0.1` 当前真实可用的用户流程。该版本是 1.0 前工程里程碑，语言、CLI、`.sapp` 和工具协议仍可能发生不兼容变化。

## 推荐阅读顺序

1. [安装与构建](./INSTALLATION.md)：从源码构建 `sico`、`sico-lsp` 和 `sico-ai-tool`。
2. [五分钟入门](./GETTING-STARTED.md)：检查、格式化、构建、检查包并运行第一个程序。
3. [语言基础与可运行子集](./LANGUAGE-BASICS.md)：当前语法和 compiler/Runtime 支持边界。
4. [CLI 参考](./CLI.md)：所有命令、参数、输入输出和退出码。
5. [构建、运行与缓存](./BUILD-RUN.md)：源码、Component、`.sapp`、Wasmtime 和能力授权。
6. [包、签名与信任](./PACKAGES-AND-TRUST.md)：unsigned development、开发签名与本地信任。
7. [编辑器与 LSP](./EDITOR.md)：`sico-lsp` 的 stdio 接入和功能限制。
8. [AI 工具](./AI-TOOLS.md)：compiler-backed inspect/fix JSON 协议。
9. [故障排查](./TROUBLESHOOTING.md)：常见错误、退出码和修复方法。
10. [限制与平台状态](./LIMITATIONS.md)：尚未实现或没有证据的能力。

## 文档约定

- 命令示例以仓库根目录为当前目录。
- Windows 是当前唯一 runtime-verified 的 Desktop Host 平台。
- `target/release/sico.exe` 可简写为 `sico`，前提是已经加入 `PATH`。
- 用户应用分发格式是 `.sapp`；`--raw-component` 仅用于内部编译器回归。
- “签名有效”不等于“本机信任”，development key 也不等于 production publisher。

如果要参与 Sico 自身开发，请转到[开发手册](../development/README.md)。
