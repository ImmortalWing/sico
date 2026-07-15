# RFC-0013: async backend support v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

记录 Wasmtime 46.0.1 对 RFC-0004 future/stream contract 的真实支持，同时冻结当前 compiler 的诚实边界：Task/Future/Stream 具有专用 typed backend refusal，在 cancellation、bound 和 structured scope 尚未进入可验证 IR/codegen 前不生成伪异步 Component。

## Runtime-supported subset

- `future<u32>` 和 `stream<u32>` 可作为 Component typed value 跨 canonical lift 往返；
- Future/Stream reader 必须显式 close 或由 guard 关闭，禁止只 drop Rust wrapper 而泄漏 store table；
- pending future read 可以被取消，producer 收到 finish 并确认 cancel；
- stream producer 只按 consumer capacity 交付：capacity 1 从 5 项只交付 1 项，capacity 100 交付 5 项并报告完成；
- `async func`、future、stream 使用 Component Model async builtins，不降级到 WASI 0.2 pollable。

## Compiler boundary

- Sico `Task<T>` 是结构化语言内 handle，不映射成 WIT resource 或普通可复制值；
- Sico `Future<T>` 的 codegen 必须有显式 async Component function、single-consume 和 cancellation edge；
- Sico `Stream<T>` 的 codegen 必须有显式 bound、close/cancel edge 和终止 result；
- 当前 source lowering 尚未产生足以证明上述边界的 async IR，因此 Core/Component backend 返回 `AsyncUnsupported { feature, contract }`；
- generic success、mock scheduler 或把 Future/Stream 当 `u32` handle 均禁止。

## Determinism and validation

三个 locked Component WAT probes 解析为确定字节，连接后的 SHA-256 固定；重复运行 output byte-identical。所有 probes 由 Wasmtime Component validator/Runtime 实际加载执行。

## Acceptance evidence

- future/stream identity + close；
- pending future cancellation acknowledgement；
- stream capacity 1/5 bounded delivery；
- Task/Future/Stream 三类 compiler typed refusal；
- resource/task/stream Rust tests、WIT parser、compile-fail 与前序 regression。

## Links

- [`RFC-0004`](./RFC-0004-resource-async-mapping-v0.md)
- [`async-flow WIT`](../../wit/async-flow-v0/world.wit)
- [`STEP-0035`](../steps/STEP-0035-async-task-stream-backend.md)
