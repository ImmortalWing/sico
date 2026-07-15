# STEP-0034: 实现 Component/WIT boundary codegen

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

让正式 compiler 把受支持 Core Wasm 确定性 lift 为 WebAssembly Component，冻结经真实证据支持的 WIT Result/value record/resource 结构映射，并在 selected Wasmtime Runtime 完成 host-call 往返。

## 2. Boundaries

- compiler-generated Component 当前只支持 flat Bool/Unit 与 constant-proven fitting integer probe；
- WIT aggregate/resource 往返由真实 binding/host probe证明结构映射，但 compiler 尚不生成其 memory/resource adapters；
- RFC-0003 arbitrary `Int`/Decimal 与 RFC-0004 async/future/stream 均不因本步自动接受；
- `sico:boundary-probe@0.1.0` 是 versioned audit package，不是最终产品 ABI。

## 3. Acceptance

- 两个 compiler Component 重复生成 byte-identical并通过 `wasmparser 0.253`；
- 官方 Wasmtime 46.0.1 调用 compiler-generated `main()` 返回 42；
- WIT 由 `wit-parser 0.253`、wit-bindgen 与 Wasmtime bindgen 真实消费；
- Result Ok/Err、big-int/Decimal records、resource new/borrow/drop 和 host call 往返；
- prototype build/run 失败立即阻止 PASS，不再允许 stale artifact；
- workspace/M2/M1/M0 regression。

## 4. Commit

`feat(component): [STEP-0034] freeze WIT boundary and runtime lift`

## 5. Validation

```text
STEP_0034_OK compiler_components=2 deterministic=byte-identical validator=wasmparser-0.253 runtime=wasmtime-46.0.1 compiler_result=42 wit_parser=0.253 result_ok=pass result_error=pass records=big-int,decimal resource=owned-borrowed-drop host_output=50
```

## 6. Next

STEP-0035 必须先按 RFC-0004 gate 审计 Wasmtime/WASI 0.3 async support；只有真实 `future<T>` 与 `stream<T>` close/cancel/backpressure 往返后才能接受对应 backend subset，其他路径继续明确诊断。
