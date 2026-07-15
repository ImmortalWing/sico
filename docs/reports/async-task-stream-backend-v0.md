# Report: async/task/stream backend v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0035
> - environment: Windows, Rust 1.97.0, Wasmtime 46.0.1, WAT 1.251.0

## 1. Question

RFC-0004 的 future/stream/close/cancel/backpressure 是否已有真实 Component Runtime 证据；若有，compiler 能否在 IR 尚不完整时保持诚实而不模拟成功？

## 2. Method

从锁定 Wasmtime 46.0.1 上游测试范式提取三个最小 Component：future/stream identity、pending future cancel、bounded stream read。每个 WAT 解析为 Component binary并由 Wasmtime instantiate/call。compiler 侧构造 verified Task/Future/Stream IR identity signatures，验证 Core/Component backend 均返回专用 contract refusal。

## 3. Results

```text
FUTURE_STREAM_OK future_roundtrip=closed stream_roundtrip=closed future_cancel=acknowledged stream_capacity=1/5 artifacts=3 sha256=8f6266f22d190eee9f9139e06ed579b67da29d947d3d8efd65ce11bed5b28311
STEP_0035_OK runtime=wasmtime-46.0.1 artifacts=3 deterministic=sha256 future=roundtrip-close-cancel stream=roundtrip-close-capacity-1-of-5 wit=async-flow-v0 backend_refusals=Task,Future,Stream resource_async_tests=10 compile_fail=2 rfc_0004=accepted
```

- future/stream handle 经 canonical lift identity 往返后显式 close；
- pending future read 返回 blocked，cancel 返回 acknowledged；
- 五项 stream 在 capacity 1 时只交付一项，capacity 100 时交付五项并完成；
- 三个 artifacts 合并 hash 两次一致；
- Rust resource/task/stream 10 tests、2 compile-fail 与 WIT parser 通过；
- compiler 三类 async boundaries 均有 feature-specific contract，而非 generic success。

## 4. Boundaries

Runtime capability 不等于 compiler implementation。Sico async source 尚未 lower 到包含 task scope、cancel edge 和 stream bound 的完整 verified IR，因此 STEP-0036 不开放 async build/run。WASI 0.2 adapter、detached task、公平性和跨线程共享仍不属于已接受子集。

## 5. Links

- [`STEP-0035`](../steps/STEP-0035-async-task-stream-backend.md)
- [`RFC-0004`](../rfc/RFC-0004-resource-async-mapping-v0.md)
- [`RFC-0013`](../rfc/RFC-0013-async-backend-support-v0.md)
- [`async-flow WIT`](../../wit/async-flow-v0/world.wit)
