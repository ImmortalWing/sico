# Report: deterministic Core Wasm backend v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0033
> - environment: Windows, Rust 1.97.0, wasmparser/wasm-encoder 0.253.0, Node 24.17.0

## 1. Question

verified Sico IR 能否在不猜测 arbitrary numeric、ABI 或 Runtime 语义的情况下，确定性生成可被真实 WebAssembly 工具链加载执行的 Core Wasm？

## 2. Method

后端先调用独立 IR verifier，再按 canonical function/`ValueId` 顺序编码 type、function、export 与 code sections。Rust tests 对相同 IR 生成两次并比较完整字节，随后以 `wasmparser` 验证。独立 Node 原生 WebAssembly engine 从 committed hex artifacts 加载并执行，不复用 compiler evaluator。

## 3. Results

```text
STEP_0033_OK artifacts=2 deterministic=byte-identical validator=wasmparser-0.253 engine=node-webassembly numeric=42 control_true=7 control_false=9 invalid_ir=blocked arbitrary_int=refused wasm_abi=bounded-scalar-probe
```

- source `return 40 + 2` 生成 Core Wasm，独立 engine 返回 `42`；
- `select(Bool) -> Int` 的 runtime true/false 分支分别返回 `7`/`9`；
- 两个 artifacts 通过真实 binary validator，重复 codegen 完整字节一致；
- schema mutation 在 codegen gate 被拒绝；4096-bit sample 返回 representation refusal；
- 全 workspace 以及 M2/M1/M0 validators 保持通过。

## 4. Boundaries

当前 `i64` 只表示 compile-time proven fitting integer probe，不是 Sico arbitrary `Int` 的通用表示。Node 仅为独立 Core Wasm 测试 engine；Sico 选定 Runtime 仍为 ADR-0002 的 Wasmtime。Component、WIT/canonical ABI、Result/value record/resource 与 async 尚未由该后端声明支持。

## 5. Links

- [`STEP-0033`](../steps/STEP-0033-deterministic-core-wasm-backend.md)
- [`RFC-0011`](../rfc/RFC-0011-deterministic-core-wasm-backend-v0.md)
- [`Core Wasm snapshots`](../../tests/wasm/core-wasm.hex)
