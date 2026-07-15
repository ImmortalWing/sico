# RFC-0014: Minimal build/run CLI v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

冻结 M3 同步 scalar 子集的 `sico build` 与 `sico run` 契约。两条命令都必须经过 source、syntax、完整 M2 semantics、typed IR verifier 和 Component codegen；`run` 只能再把内存中的 Component 交给选定的 Wasmtime 46.0.1 Runtime。该契约不是 `.sapp`、capability host、sandbox 或通用应用入口协议。

## Command surface

```text
sico build <FILE|-> [-o|--output <COMPONENT>]
sico run <FILE|-> [--runtime <WASMTIME>]
```

- file build 默认产物为把输入扩展名替换成 `.component.wasm`；stdin build 必须显式提供 `--output`；
- build 拒绝覆盖已存在产物。成功只向 stdout 写一行 `built <path>`；失败不得创建或修改目标；
- run 不创建持久产物。Runtime 选择顺序是 `--runtime`、`SICO_WASMTIME`、PATH 中的 `wasmtime`；
- M3 source-run 显式关闭 Wasmtime compiled-module cache；cache key、失效、权限和清理规则在 M4 package contract 中统一冻结；
- run 将 Wasmtime stdout/stderr 原样转发，成功不附加编译器文本；临时 Component 在 Runtime 返回前清理；
- M3 入口必须是唯一同步 `main()`、无参数、无 effects，结果仅为 `Unit`、`Bool` 或 compile-time proven fitting `Int`；调用形式固定为 Component export `main()`。

## Exit and channel contract

| Exit | Meaning | Channel/artifact rule |
|---:|---|---|
| 0 | build 或 Runtime 成功 | build 状态写 stdout；run 原样转发 Runtime channels |
| 1 | 已注册 syntax/semantic source diagnostic | text diagnostic 写 stderr；不产生持久产物，也不启动 Runtime |
| 2 | I/O、unsupported lowering/codegen、入口、产物、Runtime 启动/执行错误 | tool error 写 stderr；失败 build 不创建或覆盖目标 |

后端 unsupported 不是源码已经违反稳定语义，因此不能伪装成 exit 1。Runtime 非零退出在 M3 统一为 exit 2；trap/domain error/capability denial 的稳定分类属于 M4 Runtime contract。

## Decisions and rejected alternatives

- 接受 raw `.component.wasm` 作为 M3 build 产物，以便证明确定性链路；拒绝在 `.sapp` manifest、签名和权限模型前生成伪包；
- 接受显式“不覆盖”策略，避免 Windows 上无跨平台原子替换保证时破坏已有产物；M4 可在 package build contract 中重新审查 `--force`；
- 接受外部 Wasmtime 进程边界，使 CLI 测试能锁定真实 Runtime 版本且不在 M3 引入宿主 SDK；
- 拒绝把 effect metadata 静默丢弃后 codegen；任何 effectful function 在没有 host adapter 时 typed-refuse；
- 拒绝 `-c`、REPL、参数透传、通用 exports、WASI imports、缓存与 `.sapp` inspect，这些能力依赖 M4 设计。

## Acceptance evidence

- file/stdin build 生成 byte-identical、通过 wasmparser 的 Component；
- Wasmtime 46.0.1 从 source 直跑 `main()` 并返回预期 scalar；
- syntax/semantic invalid 在 IR 前 exit 1，unsupported backend/entry/Runtime 在 exit 2；
- existing output 在所有失败路径保持不变，run 临时 Component 被清理；
- M0–M2 与 STEP-0030–0035 regression 保持通过。

## Links

- [`STEP-0036`](../steps/STEP-0036-minimal-end-to-end-cli.md)
- [`RFC-0011`](./RFC-0011-deterministic-core-wasm-backend-v0.md)
- [`RFC-0012`](./RFC-0012-component-wit-boundary-v0.md)
- [`ADR-0002`](../adr/ADR-0002-runtime-platform-baseline.md)
