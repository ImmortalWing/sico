# Report: minimal build/run CLI v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0036
> - environment: Windows; Rust 1.97.0 target; Wasmtime 46.0.1

## 1. Question

M3 已验证的同步 scalar source 能否通过一个稳定 CLI 真实经过 semantics、typed IR、Component codegen 和选定 Wasmtime Runtime，同时保持诊断、错误通道、确定性和失败无产物边界？

## 2. Method

- real binary 对 file/stdin 执行 `build`，两次 artifact 以完整 SHA-256 比较并由 wasmparser 验证；
- real binary 对 Int/Bool/Unit 三个 source 执行 `run --runtime <locked wasmtime>`；
- semantic invalid、unsupported arbitrary Int、缺失 main、已有输出与缺失 Runtime 分别检查 exit 1/2、stderr 和产物状态；
- Runtime 使用进程唯一临时 Component，launch failure 单测证明返回前清理；
- strict Clippy、workspace/M0–M2 与 STEP-0030–0035 regression 作为完成门槛。

## 3. Contract decision

[`RFC-0014`](../rfc/RFC-0014-minimal-build-run-cli-v0.md) 接受 raw `.component.wasm` 的 M3 `build/run`，入口为同步无参无 effect `main()`；build 默认不覆盖，run Runtime 查找顺序为参数、环境变量、PATH。`.sapp`、capability host、sandbox、cache、args、trap taxonomy 和 REPL 明确递延到 M4+。

## 4. Results

```text
STEP_0036_OK cli=build,run inputs=file,stdin artifact=component-wasm deterministic=sha256 runtime=wasmtime-46.0.1 corpus=int-bool-unit semantic_invalid=exit-1-no-artifact unsupported_runtime=exit-2 overwrite=refused entry=sync-scalar-main
```

- file/stdin 三份 raw Component 完整字节一致并通过 wasmparser；
- Wasmtime 46.0.1 对 Int/Bool/Unit 分别输出 `42`、`true`、`()`；
- semantic invalid 保持精确 `E2001`/exit 1，未创建目标；arbitrary Int、missing entry 和 missing Runtime 为 exit 2；
- 已有 artifact 拒绝覆盖且内容不变；Runtime launch failure 后无临时 Component 遗留；
- Wasmtime module cache 在 M3 source-run 中显式关闭，避免在 M4 cache key/permission contract 前产生隐式持久状态。

## 5. Links

- [`STEP-0036`](../steps/STEP-0036-minimal-end-to-end-cli.md)
- [`RFC-0014`](../rfc/RFC-0014-minimal-build-run-cli-v0.md)
- [`M3 end-to-end corpus`](../../tests/end-to-end/)
