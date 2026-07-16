# CLI 参考

```text
sico <COMMAND>
```

可用命令：`check`、`format`、`outline`、`build`、`run`、`inspect`。以实际二进制帮助为准：

```powershell
sico --help
sico <COMMAND> --help
```

## `sico check`

```text
sico check [--json] <FILE|->
```

- 同时执行 syntax 和 static semantics；
- `-` 从 stdin 读取源码；
- `--json` 输出 RFC-0001 机器诊断；
- 成功退出 `0`，源码诊断退出 `1`，I/O、非法 UTF-8 等工具错误退出 `2`。

## `sico format`

```text
sico format [--check|--write] <FILE|->
```

- 无选项：规范格式输出到 stdout；
- `--check`：不修改文件，非规范格式退出 `1`；
- `--write`：原地重写文件；
- `--check` 与 `--write` 互斥；
- 有阻塞 syntax error 时拒绝产生格式结果。

## `sico outline`

```text
sico outline [--json] <FILE|->
```

按源码顺序列出顶层 parser declarations。`--json` 输出 `sico.outline.v0`。它是结构视图，不替代完整 Semantic Index。

## `sico build`

```text
sico build [OPTIONS] <FILE|->
```

| 选项 | 含义 |
|---|---|
| `-o, --output <ARTIFACT>` | 输出路径；stdin 输入时必需 |
| `--app-id <ID>` | 应用 ID，默认 `dev.sico.app` |
| `--app-version <VERSION>` | 应用版本，默认 `0.0.0`；建议显式指定 |
| `--sign-key <SEED_FILE>` | 64 个 lowercase hex 字符表示的 32-byte development seed |
| `--raw-component` | 内部编译器回归边界，不是用户应用分发格式 |

默认生成与源文件同名的 `.sapp`。输出以 temporary + atomic rename 写入，并拒绝覆盖已有文件。

## `sico inspect`

```text
sico inspect [--json] [--trusted-key <PUBLIC_KEY_FILE>] <PACKAGE>
```

严格验证 `.sapp`，但从不执行。`--trusted-key` 只改变本地 development trust 判定；签名有效但未提供对应 key 时显示 `development-valid-untrusted`。当前 package stdin inspection 未实现，应传文件路径。

## `sico run`

```text
sico run [OPTIONS] <FILE|-> [-- [ARG]...]
```

| 选项 | 含义 |
|---|---|
| `--app-id`、`--app-version`、`--sign-key` | 源码运行时的构建参数 |
| `--trusted-key <FILE>` | 信任 development-signed `.sapp` 的公钥 |
| `--allow-unsigned-dev` | 显式允许运行 unsigned development `.sapp` |
| `--grant <CAPABILITY>` | 授予 guest 已请求的一项能力，可重复 |
| `--storage-root <DIR>` | storage capability 的 Host-owned 根目录 |
| `--cache-dir <DIR>` | 指定源码运行缓存 |
| `--no-cache` | 禁用源码运行缓存 |
| `--runtime <WASMTIME>` | 指定 Wasmtime 路径 |

Runtime 查找顺序：`--runtime`、`SICO_WASMTIME`、`PATH`。源码输入是显式本地开发流程；`.sapp` 输入必须通过 trust policy。

`-- ARG...` 当前只是保留语法，任何非空应用参数都会退出 `2`，不会被静默丢弃。

## 退出码

| Code | 含义 |
|---:|---|
| 0 | 成功 |
| 1 | 源码诊断 |
| 2 | CLI、I/O、包、信任、能力或 Host 配置错误 |
| 3 | typed domain error（预留） |
| 4 | lifecycle cancellation（预留） |
| 5 | timeout |
| 6 | resource limit 或 guest trap |

脚本不要解析自然语言错误文本来代替退出码和 JSON schema。
