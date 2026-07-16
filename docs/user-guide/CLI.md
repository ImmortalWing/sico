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
