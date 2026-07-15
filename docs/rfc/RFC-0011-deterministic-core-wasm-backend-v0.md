# RFC-0011: deterministic Core Wasm backend v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

冻结第一条 verified Sico IR → deterministic Core WebAssembly 路径。后端必须先独立运行 IR verifier，只为已有证据支持的 scalar/control probe 生成产物；未确定的 arbitrary `Int`、aggregate、effect/resource、Component 与 async 表示必须 typed-refuse。

## Input gate

- `sico-codegen-wasm::compile` 只接受 typed `sico-ir::Module`；
- codegen 首先执行独立 verifier，任何 malformed IR 均返回 `InvalidIr` 且不产生字节；
- source semantic invalid 仍由 M2/M3 lowering gate 在后端前拒绝；后端不修复或猜测 IR。

## Deterministic module layout

- section 顺序固定为 type → function → export → code；
- function、type 与 export 顺序继承 canonical IR function 顺序；
- local 顺序继承稳定 `ValueId`，不使用 hash iteration 或时间戳；
- 同一 verified IR 重复调用 `compile` 必须 byte-identical；已接受 numeric/control probes 以完整 hex snapshot 固定。

## Bounded scalar mapping

- `Bool` parameter/result 暂映射到 Wasm `i32`，仅作为 Core Wasm 内部 probe ABI；
- `Unit` 映射为空 result；
- Sico `Int` 仍是 arbitrary precision，不能一般性映射到 `i64`；
- 仅当 constant folding 在编译期证明最终数学整数适合 `i64` 时，probe 可发出 `i64`；
- `Int` parameter、非 constant `Int`、溢出或超范围 literal 必须返回 typed codegen refusal；
- 此映射不是 WIT/canonical ABI，也不接受 RFC-0003 的 proposed 部分。

## Control subset

- 支持 single-entry return 与 `Bool` condition 的 two-way branch；
- branch target v0 必须只包含受支持 scalar instructions 并以 return 结束；
- jump、match、aggregate、call、effect/resource/revision operation 保持 unsupported；
- 不通过隐式 trap、默认值或 host helper 模拟未实现语义。

## Validation boundary

- `wasmparser 0.253` 对每个生成 artifact 执行真实 binary validation；
- Node 24 的原生 `WebAssembly` engine 作为独立测试预言机加载已提交字节，执行 numeric 与 control 两个 probe；
- Node 不是 Sico 产品依赖、Runtime 选择或 CLI implementation。ADR-0002 选择的 Wasmtime Runtime 不变；selected Runtime/Component host call 属于 STEP-0034–0036。

## Rejected alternatives

- 将所有 `Int` 直接截断为 `i64`：违反 arbitrary-precision contract；
- 在 malformed IR 上继续生成：隐藏 compiler bug；
- 以 mock interpreter 代替真实 Wasm validator/engine：无法证明 artifact 可加载执行；
- 因测试宿主方便而把 JavaScript 加入 Sico Runtime：越过 ADR 与 M3 边界；
- 在本步提前编码 canonical ABI、resource handle 或 async：相关契约尚未由本步接受。

## Acceptance evidence

- numeric source `40 + 2` 生成确定字节并执行为 `42`；
- runtime `Bool` branch 两条路径分别执行为 `7` 与 `9`；
- 两个 artifacts 同输入重复生成 byte-identical，并通过 `wasmparser` 与独立 engine validation；
- malformed IR 与 arbitrary-precision sample 均无产物；
- workspace/M2/M1/M0 regression。

## Links

- [`RFC-0003`](./RFC-0003-numeric-representation-v0.md)
- [`RFC-0008`](./RFC-0008-typed-sico-ir-contract-v0.md)
- [`RFC-0009`](./RFC-0009-core-lowering-evaluation-order-v0.md)
- [`STEP-0033`](../steps/STEP-0033-deterministic-core-wasm-backend.md)
