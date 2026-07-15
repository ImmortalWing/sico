# Report: Component/WIT boundary v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0034
> - environment: Windows, Rust 1.97.0, wasm-encoder/wasmparser/wit-parser 0.253.0, Wasmtime 46.0.1

## 1. Question

正式 compiler 能否生成可由 selected Runtime 执行的 deterministic Component，同时在不提前接受 numeric/async 语义的前提下证明 Result、value record 与 resource 的真实 Canonical ABI 边界？

## 2. Method

compiler 直接嵌入 STEP-0033 Core module，实例化、alias export、定义 Component function type并执行 canon lift。Rust tests 比较完整字节并用 `wasmparser` 验证；官方 Wasmtime CLI 调用生成 artifact。共享 WIT probe 同时由 parser、guest bindgen 和 host bindgen消费，Rust guest/Wasmtime host 执行两次确定性往返。

旧 validator 审计发现：root workspace 建立后 nested Cargo build 会失败，但脚本未检查退出码，仍读取陈旧 artifact 打印 PASS。本步增加 workspace exclude 与每条 build/run exit gate，并在修复后从修改过的 WIT/guest/host 重新构建，排除了 stale evidence。

## 3. Results

```text
STEP_0034_OK compiler_components=2 deterministic=byte-identical validator=wasmparser-0.253 runtime=wasmtime-46.0.1 compiler_result=42 wit_parser=0.253 result_ok=pass result_error=pass records=big-int,decimal resource=owned-borrowed-drop host_output=50
```

- compiler numeric/control Components：完整字节重复一致且 Component validation 通过；
- selected Wasmtime 调用 compiler `main()`：42；
- host call：35 → counter add 7 → host add 8 → 50；
- resource events：new → borrowed add/value → log → owned drop，顺序稳定；
- Result：Ok(42) 与 Err(conflict) 都保持原 channel；
- 4096-bit magnitude 512 bytes 与 Decimal record 字段往返不变；
- 两次 sync Component SHA-256 相同。

## 4. Boundaries

compiler 尚未生成 aggregate memory adapters、WIT imports 或 resource table glue，因此不宣称一般 Component application 已完成。BigInt/Decimal record roundtrip 是 ABI 保真证据，RFC-0003 仍为 proposed。原生 async function 回归虽通过，但 future/stream gate 留给 STEP-0035。

## 5. Links

- [`STEP-0034`](../steps/STEP-0034-component-wit-boundary.md)
- [`RFC-0012`](../rfc/RFC-0012-component-wit-boundary-v0.md)
- [`boundary WIT`](../../wit/boundary-probe-v0/world.wit)
- [`Component artifacts`](../../tests/wasm/artifacts.hex)
