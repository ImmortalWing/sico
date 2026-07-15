# RFC-0019: Package CLI, source cache, args and stdio v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target phase: M4
> - supersedes: RFC-0014 public build/run surface
> - superseded-by: -

## Summary

M4 的正式应用 surface 是 deterministic `.sapp`：`sico build` 默认产包，`sico inspect` 独立验证且不执行，`sico run` 在 trust/capability/cache gate 后执行。M3 raw Component 仅以显式 `build --raw-component` 保留为 compiler regression boundary。

## Commands

### `sico build FILE|-`

- 默认输出 `FILE.sapp`；stdin 必须显式 `--output`；
- `--app-id` 默认 `dev.sico.app`，`--app-version` 默认 `0.0.0`；
- `--sign-key FILE` 读取恰好 64 个 lowercase hex 字符的 32-byte development seed；seed 不写入 package、stdout、stderr 或 cache filename；
- output 使用 create-new temporary + atomic rename，拒绝覆盖；
- `--raw-component` 是 M3 内部证据开关，不是应用分发格式。

### `sico inspect PACKAGE`

先 strict structural/hash/Component validation，再验证签名（若存在），从不执行 package。text/`--json` 输出 app identity、package/component digest、resources、source effects、capabilities/imports、limits 和 trust status。没有 local key 时，有效签名显示为 `development-valid-untrusted`，不能被误读为执行授权；`--trusted-key` 才显示 `development-trusted`。

### `sico run FILE.sapp|SOURCE`

- package input 必须显式提供 `--trusted-key` 或 `--allow-unsigned-dev`；source input 是用户主动发起的 local development build，进入 unsigned-development policy；
- package 在执行前依次通过 strict verify、signature policy、source/manifest/import closure、host grants；
- `--grant CAPABILITY` 不会暴露未请求能力；storage grant 还要求 `--storage-root`；
- source-run 默认使用 domain-keyed cache，可用 `--cache-dir`/`SICO_CACHE_DIR` 定位或 `--no-cache` 关闭；
- `.sapp` input 不进入 source cache。

## Cache contract

key 是 `SHA-256("SICO-SOURCE-CACHE-V0\\0" || app-id || NUL || version || NUL || exact-source-bytes || optional-signing-seed)`；文件名只暴露 digest。cache miss 构建 canonical package 并以 create-new/atomic rename 安装。cache hit 必须重新 strict verify 并比较 app identity/version；corrupt 或 stale entry 直接拒绝，绝不执行，也不静默覆盖审计证据。

cache 是性能工件而不是 trust root。即便本地 cache 可写，verify/trust/capability gate 仍与直接 package run 相同。

## Args, stdin, stdout and stderr

v0 entry 是同步 scalar `main()`，因此 `-- ARG...` 语法被保留但任何非空参数稳定退出 2；不得静默丢弃。source `-` 使用 stdin 读取 source；package inspect stdin 与 guest stdin 尚不支持。Runtime process stdin 关闭。

`run` stdout 只转发 guest stdout/result，stderr 先转发 guest stderr，再用于 host/tool error；cache hit/miss 不污染任一 channel。`build`/`inspect` 的结构化输出不属于 guest channel。

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | source diagnostic |
| 2 | CLI/package/trust/capability/host setup error |
| 3 | typed domain error |
| 4 | lifecycle cancellation |
| 5 | timeout |
| 6 | resource limit or guest trap |

v0 scalar compiler 目前只实际产生 0/1/2/5/6；3/4 为稳定预留映射，不能伪造为已实现 lifecycle/domain transport。

## Security and compatibility

- key file parsing strict lowercase，避免宽松编码歧义；
- inspection 的 cryptographic validity 不等于 local trust；
- raw Component 不能通过 `.sapp` run 路径绕过 manifest/signature/capability；
- args、guest stdin、production publisher identity、registry/update 留待后续 RFC。

## Validation

deterministic file/stdin package build、raw Component internal build、signed inspect、trusted run、missing trust refusal、non-empty args refusal、cache corruption refusal、tampered inspect refusal和真实 Wasmtime result corpus 全部自动化。

## Links

- [`STEP-0044`](../steps/STEP-0044-package-cli-source-cache.md)
- [`RFC-0014`](./RFC-0014-minimal-build-run-cli-v0.md)
- [`RFC-0018`](./RFC-0018-runtime-limits-fault-taxonomy-v0.md)
