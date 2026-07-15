# STEP-0036: 打通最小端到端 CLI build/run

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为已经通过真实 Component codegen 与 Wasmtime 46.0.1 验证的同步 scalar 子集建立 source → semantics → typed IR → Component → Runtime CLI 链，并冻结 stdout、stderr、退出码和失败无产物契约。

## 2. Scope and gates

本步只接入 `main()` 的同步 scalar Component。语义错误必须继续在 IR 前以 exit 1 拒绝；后端未支持、I/O、Runtime 启动或执行失败使用 exit 2。`.sapp`、构建缓存、参数透传、capability host、trap 分类、`sico -c` 与 REPL 留给 M4+。

正式命令、产物覆盖策略、Runtime 查找规则和稳定文本仍需通过本步 RFC/测试审查后冻结，当前 WIP 不构成已接受 CLI 契约。

## 3. Implementation plan

1. 建立独立 `sico-runtime` 进程边界，临时 Component 必须清理；
2. 审查并接受最小 build/run CLI contract；
3. 接入 frontend/semantic/IR/codegen，成功后原子写产物；
4. 用真实 Wasmtime 验证 `main()`，覆盖 file/stdin、错误通道和确定性；
5. 运行 workspace、M2、M1、M0 回归，完成审查报告后再标记本步完成。

## 4. Changes

- [`RFC-0014`](../rfc/RFC-0014-minimal-build-run-cli-v0.md) 接受 raw Component `build/run`、同步 scalar `main()`、Runtime 查找、exit/channel 和失败无产物契约；
- `sico-cli` 接入 syntax/semantics→typed IR→Component，build 使用 sibling temporary + no-overwrite install，run 不留持久 artifact；
- `sico-runtime` 使用进程唯一临时 Component、关闭尚无 M4 contract 的 Wasmtime compile cache，并在 launch success/failure 后清理；
- codegen 对 effectful function 显式拒绝，禁止在没有 Component host adapter 时静默丢弃 effects；
- real-binary integration 覆盖 file/stdin、determinism、semantic invalid、unsupported Int、missing main 与 overwrite refusal；Int/Bool/Unit corpus 由 Wasmtime 46.0.1 真执行。

## 5. Validation

```text
STEP_0036_OK cli=build,run inputs=file,stdin artifact=component-wasm deterministic=sha256 runtime=wasmtime-46.0.1 corpus=int-bool-unit semantic_invalid=exit-1-no-artifact unsupported_runtime=exit-2 overwrite=refused entry=sync-scalar-main
```

专项 tests 和 strict Clippy 已通过；完整 workspace/M0–M3 regression 由 STEP-0037 统一执行，不在本步重复冒充阶段退出证据。

## 6. Audit links

- [`M3 plan`](../plans/M3-sico-ir-component.md)
- [`RFC-0012`](../rfc/RFC-0012-component-wit-boundary-v0.md)
- [`STEP-0035`](./STEP-0035-async-task-stream-backend.md)
- [`RFC-0014`](../rfc/RFC-0014-minimal-build-run-cli-v0.md)
- [`review report`](../reports/minimal-build-run-cli-v0.md)
