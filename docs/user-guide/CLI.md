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
sico build [-o|--output <COMPONENT>] <FILE|->
```

把源码编译为 WebAssembly Component。文件输入默认输出同名 `.component.wasm`；stdin 必须指定输出路径。不会打包、签名或执行。

## `sico run` 与 `sico watch`

```text
sico run [--fs-read-root PATH]... [--fs-write-root PATH]... [--allow-net HOST:PORT]... <FILE|-> [-- [ARG]...]
sico watch [--poll-ms 25] [--max-runs COUNT] [相同 provider grants] <FILE> [-- [ARG]...]
```

`run` 编译 Script Profile 并通过独立 `sico-runner` 执行。`watch` 保持一个 runner 进程，只在新源码成功编译且稳定 100 ms 后重跑；无效源码不会替换上一健康 generation。每次重跑都有新的 Store 与资源表，provider grants 在进程启动后不可扩大。

Watch v0 只监视一个文件并使用轮询；不接受 stdin source，也拒绝 streaming-stdin Component。`--max-runs` 主要用于自动化验证，默认持续运行。

## `sico repl`

```text
sico repl [--json]
```

逐行计算现有 compile-time `Int` expression。命令为 `:history`、`:reset`、`:export PATH`、`:quit`。每个 cell 最多 4 KiB，会话最多 256 cells/16 KiB source history；失败 cell 不进入历史。`:history` 与 `:export` 会重新编译并校验全部 cell，export 只创建新文件、不覆盖已有路径。

REPL v0 不支持跨 cell 声明。源码 top level 只接受 declarations；请使用显式 `function main(...)`。unrestricted statements 与 `script:` block 会以 E1013 拒绝。

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

## `sico-app inspect`

```text
sico-app inspect [--json] [--trusted-key <PUBLIC_KEY_FILE>] <PACKAGE.sapp>
```

严格验证包但不执行。有效签名在未提供对应 key 时显示为 `development-valid-untrusted`。

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

| Code | 含义 |
|---:|---|
| 0 | 成功 |
| 1 | `sico` 源码诊断 |
| 2 | CLI、I/O、包、信任、能力或 Host 配置错误 |
| 3 | typed domain error |
| 4 | lifecycle cancellation |
| 5 | timeout |
| 6 | resource limit 或 guest trap |
