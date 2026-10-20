# CLI 参数

当前命令分为语言工具 `sico` 和应用工具 `sico-app`。

## `sico check`

```text
sico check [--json] <FILE|->
```

执行 syntax 与 static semantics。成功退出 `0`，源码诊断退出 `1`，I/O 或输入契约错误退出 `2`。

## `sico format`

```text
sico format [--check|--write] <FILE|->
```

无选项时输出到 stdout；`--check` 不修改文件；`--write` 原地写入，两者互斥。

## `sico outline`

```text
sico outline [--json] <FILE|->
```

按源码顺序输出顶层声明。JSON schema 为 `sico.outline.v0`。

## `sico build`

```text
sico build [-o|--output <COMPONENT>] [--profile component-v0|script-v0] [--debug-info] <FILE|->
```

把源码编译为 WebAssembly Component。`--profile` 默认 `component-v0`；`script-v0` 产出 Script Profile Program Component（`sico run` 使用的形态）。`--debug-info` 在 Component 旁生成有界 debug map 与 build identity，供 DAP 使用。文件输入默认输出同名 `.component.wasm`；stdin 必须指定输出路径。不会打包、签名或执行。

## `sico run` 与 `sico watch`

```text
sico run [--json] [--fs-read-root PATH]... [--fs-write-root PATH]... [--allow-net HOST:PORT]... <FILE|-> [-- [ARG]...]
sico watch [--json] [--poll-ms 25] [--max-runs COUNT] [相同 provider grants] <FILE> [-- [ARG]...]
```

`run` 编译 Script Profile 并通过独立 `sico-runner` 执行。`watch` 保持一个 runner 进程，只在新源码成功编译且稳定 100 ms 后重跑；无效源码不会替换上一健康 generation。每次重跑都有新的 Store 与资源表，provider grants 在进程启动后不可扩大。

Watch v0 只监视一个文件并使用轮询；不接受 stdin source，也拒绝 streaming-stdin Component。`--max-runs` 主要用于自动化验证，默认持续运行。

## `sico repl`

```text
sico repl [--json]
```

逐行计算现有 compile-time `Int` expression。命令为 `:history`、`:reset`、`:export PATH`、`:quit`。每个 cell 最多 4 KiB，会话最多 256 cells/16 KiB source history；失败 cell 不进入历史。`:history` 与 `:export` 会重新编译并校验全部 cell，export 只创建新文件、不覆盖已有路径。

REPL v0 不支持跨 cell 声明。源码 top level 只接受 declarations；请使用显式 `function main(...)`。unrestricted statements 与 `script:` block 会以 E1013 拒绝。

## `sico test`

```text
sico test [--json] [PATH]
```

发现 `NAME.sico` + `NAME.test.json` 测试对（目录递归、路径排序固定），经与 `sico run` 相同的缓存身份编译，并由真实 `sico-runner` 在默认有界限制下执行。manifest 每个字段都可选，且拒绝未知字段：

| 字段 | 默认 | 含义 |
|---|---|---|
| `stdin` | `""` | guest stdin |
| `args` | `[]` | `--` 之后的 guest 参数 |
| `expect_exit` | `0` | 期望退出码 |
| `expect_stdout` | `""` | 期望 stdout 字节 |

任何测试失败退出 `1`；路径不存在、manifest 非法或 runner 不可用退出 `2`。v0 覆盖 buffered profile：导入 `sico:script/streams@0.1.0` 的 Component 会以 typed 原因判为失败（runner 会继承 harness stdio）。

## `sico eval`

```text
sico eval [--json] EXPR
```

把 compile-time 常量 `Int` 表达式合成为一个 Script 并经 `sico-runner` 求值；非常量或非法表达式不执行任何代码。

## `sico explain`

```text
sico explain <CODE>
```

打印一个稳定诊断码（如 `E1013`）的 action hint。

## `sico-runner`

独立的 runner 二进制（独立 Cargo workspace）。由 `sico run`/`sico watch`/`sico test` 自动调用，也可直接执行 Program Component：

```text
sico-runner [--json] [--fs-read-root PATH]... [--fs-write-root PATH]... \
  [--allow-net HOST:PORT]... [--allow-endpoint scheme://HOST[:PORT]]... \
  [--http-trust-roots FILE]... [--secret-file FILE] PROGRAM.component.wasm [-- ARGS...]
```

HTTP 授权默认拒绝一切出站请求，显式授予：

| 开关 | 含义 |
|---|---|
| `--allow-net HOST:PORT` | 授予一个精确出站 HTTP 端点（可重复） |
| `--allow-endpoint scheme://HOST[:PORT]` | 授予安全端点（TLS trust roots 生效） |
| `--http-trust-roots FILE` | PEM trust roots bundle，合计 ≤ 1 MiB（可重复） |
| `--secret-file FILE` | secret 注入注册表（JSON，≤ 64 KiB；值只经文件进入，不进入 argv） |

`sico run`/`sico watch` 自身只透传 `--fs-read-root`/`--fs-write-root`/`--allow-net`；trust roots 与 secret 注入需直接调用 `sico-runner`。guest stdin 为进程 stdin（≤ 8 MiB）；guest stdout 之外的输出均为 stderr 上的机器可读 JSON 诊断。

## `sico-app pack`

```text
sico-app pack [OPTIONS] <COMPONENT>
```

| 选项 | 含义 |
|---|---|
| `-o, --output <PACKAGE.sapp>` | 输出路径 |
| `--app-id <ID>` | 应用 ID，默认 `dev.sico.app` |
| `--app-version <VERSION>` | 应用版本，默认 `0.0.0` |
| `--sign-key <SEED_FILE>` | 32-byte development seed 的 lowercase hex 文件 |
| `--script` | 打包为 strict manifest v1 Script 包（组合后的 command Component） |

## `sico-app inspect`

```text
sico-app inspect [--json] [--trusted-key <PUBLIC_KEY_FILE>] <PACKAGE.sapp>
```

严格验证包但不执行。有效签名在未提供对应 key 时显示为 `development-valid-untrusted`。

## `sico-app compose`

```text
sico-app compose [-o|--output <COMMAND.component.wasm>] <PROGRAM.component.wasm>
```

把 `sico:script/program@0.1.0` Program Component 与固定版本的 Script Adapter 组合为 WASI command Component；组合是确定性的，输出默认取 program 路径的 `.command.wasm` 扩展名。成功时 stdout 输出 `composed <path>` 与 `sico.script.compose.v0` JSON 报告（含 adapter/program/command 的 SHA-256 与字节数），拒绝覆盖已有产物。

## `sico-app run`

```text
sico-app run [OPTIONS] <PACKAGE.sapp> [-- [ARG]...]
```

| 选项 | 含义 |
|---|---|
| `--trusted-key <FILE>` | 信任 development-signed 包的公钥 |
| `--allow-unsigned-dev` | 显式允许本地 unsigned development 包 |
| `--grant <CAPABILITY>` | 授予包已请求的能力，可重复 |
| `--storage-root <DIR>` | storage capability 的 Host-owned 根目录 |
| `--runtime <WASMTIME>` | 指定 Wasmtime |

任何非空 `ARG` 当前都会退出 `2`，不会被静默丢弃。

## `sico-app dev`

```text
sico-app dev [OPTIONS] <SOURCE.sico>
```

面向本机可信源码的一命令开发入口。它直接启动独立的 `sico` 编译器，生成临时 Component 和 unsigned development `.sapp`，再通过相同的 trust/capability/Runtime gate 执行。

| 选项 | 含义 |
|---|---|
| `--app-id <ID>` | 临时应用 ID |
| `--app-version <VERSION>` | 临时应用版本 |
| `--compiler <SICO>` | 显式编译器路径 |
| `--runtime <WASMTIME>` | 显式 Runtime 路径 |
| `--grant <CAPABILITY>` | 授予已请求能力，可重复 |
| `--storage-root <DIR>` | storage capability 的宿主根目录 |
| `--keep-artifacts` | 保留临时 Component 和 `.sapp` |

编译器查找顺序为 `--compiler`、`SICO_COMPILER`、`sico-app` 同目录、`PATH`。Runtime 查找顺序为 `--runtime`、`SICO_WASMTIME`、已安装 SDK 的 `runtime/`、`PATH`。

## 退出码

`sico` 语言工具（`check`/`format`/`outline`/`build`/`repl` 等）：

| Code | 含义 |
|---:|---|
| 0 | 成功 |
| 1 | 源码诊断 |
| 2 | CLI、I/O 或输入契约错误 |

`sico-app`（`pack`/`inspect`/`run`/`dev`/`compose`）：

| Code | 含义 |
|---:|---|
| 0 | 成功 |
| 2 | CLI、I/O、包、信任、能力或 Host 配置错误 |
| 3 | typed domain error |
| 4 | lifecycle cancellation |
| 5 | timeout |
| 6 | resource limit 或 guest trap |

`sico run`/`sico test`/`sico-runner`（RFC-0029 映射）：

| Code | 含义 |
|---:|---|
| 0–119 | guest 值 |
| 120 | `sico run`/`sico test` 的编译诊断 |
| 121 | CLI/IO 失败 |
| 122 | typed domain error（partial failure） |
| 123 | cancellation |
| 124 | timeout |
| 125 | resource limit 或 guest trap（含 `StackLimit`） |
| 126 | launch failure |
| 127 | runner 不兼容（如找不到 `sico-runner`） |
