# STEP-0033: 实现 deterministic Core Wasm backend

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

建立只接受 verified IR 的正式 Core Wasm backend，对已证明的 numeric/control scalar subset 生成 byte-identical artifact，并由真实 validator 和独立 Wasm engine 执行。

## 2. Boundaries

- 本步只接受 RFC-0011 的 bounded scalar probe ABI，不把 arbitrary Sico `Int` 固化为 `i64`；
- aggregate、call、effect/resource/revision、Component/canonical ABI 与 async 尚不生成；
- Node 原生 WebAssembly 仅作独立测试预言机，不进入 Sico Runtime 或 CLI；
- selected Wasmtime Runtime 的 Component host call 留给 STEP-0034–0036。

## 3. Acceptance

- 后端在 codegen 前独立运行 IR verifier，malformed IR 无产物；
- numeric source 与 runtime Bool branch 生成两个 deterministic artifacts；
- repeated build byte-identical，完整字节由 hex snapshot 固定；
- artifacts 通过 `wasmparser 0.253` 与 Node 24 原生 WebAssembly validator；
- numeric 执行为 42，control true/false 分别执行为 7/9；
- arbitrary-precision sample typed-refuse，不发生截断；
- workspace/M2/M1/M0 regression。

## 4. Commit

`feat(wasm): [STEP-0033] generate deterministic Core Wasm`

## 5. Changes and validation

- workspace 新增 `sico-codegen-wasm` crate；
- 使用 `wasm-encoder 0.253` 固定 section/function/export/local 顺序；
- 支持 constant-proven integer return、Bool parameter/result 与 two-way return branch；
- `wasmparser` 在 Rust integration tests 中验证每个 artifact；
- 独立 runtime validator 从 committed hex snapshot 加载模块，避免测试执行绕过被审查产物。

```text
STEP_0033_OK artifacts=2 deterministic=byte-identical validator=wasmparser-0.253 engine=node-webassembly numeric=42 control_true=7 control_false=9 invalid_ir=blocked arbitrary_int=refused wasm_abi=bounded-scalar-probe
```

## 6. Next

STEP-0034 先审查并接受 WIT/canonical ABI mapping，再实现 Component boundary codegen、Component validator、selected Wasmtime host call，以及 Result/resource/value record 往返。
