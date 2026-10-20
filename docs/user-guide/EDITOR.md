# 编辑器与 LSP

`sico-lsp` 是本地 stdio Language Server。仓库当前没有发布 VS Code、JetBrains、Zed 或其他编辑器扩展，因此需要使用编辑器的通用 LSP client 手工配置。

## 构建

```powershell
cargo build --locked --release -p sico-language-server
```

程序路径：

```text
target/release/sico-lsp.exe
```

## 通用客户端配置

在编辑器的 generic LSP 配置中填写：

| 字段 | 值 |
|---|---|
| command | `E:\path\to\sico\target\release\sico-lsp.exe` |
| transport | stdio |
| language id | `sico` |
| file extension | `.sico` |
| initialization options | 无必需字段 |

不要通过 shell command string 拼接源码路径；客户端应把 executable 和 argument array 分开传递。

## 已实现能力

- UTF-16 editor positions；
- full-document synchronization；
- push/pull diagnostics；
- document symbols；
- completion；
- hover；
- definition；
- bounded references；
- whole-document formatting；
- `sico.check`、`sico.run`、`sico.watch` 和 `sico.repl` editor commands。

这些命令返回共享 `sico.execution-plan.v0`：固定 `sico` executable、argument array、`shell: false`、`cwd: null`、最多 1 MiB 的 client capture、process-tree cancellation 责任，以及 compile-only source coordinate 边界。LSP server 自身不启动程序。

客户端执行 plan 时必须直接调用 executable/arguments，不得重新拼接成 shell string；捕获超过 plan 上限时应截断并标记。取消由客户端终止 direct child tree，但不能把这种终止伪报为 runner typed exit 123。

## 未实现能力

- rename；
- incremental synchronization；
- workspace diagnostics；
- semantic tokens；
- file watcher/background index；
- 已发布的编辑器扩展（当前需手工配置 generic LSP client）。

## 调试（DAP v0）

runner 已实现 minimal DAP 子集（12 个支持的请求、20 个 typed 拒绝、6 个事件；真实断点、暂停、嵌套 stack 与 scalar locals，Windows runtime-verified，M10 GO）。LSP 不内嵌调试器：`sico.debug` 命令接收一个 launch descriptor（`component`/`debugMap`/`debugIdentity`/`source`/`documentId`），返回 data-only 的 `sico.debug-launch-plan.v0`，由编辑器负责启动 `sico-dap` 适配器进程。普通 execution plan 的 `runtime_locations`/`debug_adapter` 仍为 false；不应把普通运行包装成“调试”。

## 资源限制

- 单消息或单文档最多 1 MiB；
- headers 合计最多 8 KiB；
- 最多 128 个 open documents；
- session source 总量最多 8 MiB；
- file URI 最多 4 KiB。

超过限制、重复 `Content-Length`、非法 encoding 或错误生命周期顺序都会 fail closed。

协议级细节见 [RFC-0027](../rfc/RFC-0027-language-server-editor-protocol-v0.md)。
