# Report: Resource and async WIT prototype v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0009
> - environment: Windows 11 x86_64; rustc/cargo 1.97.0; wit-parser 0.253.0; GNU target + rust-lld

## 1. Question

Sico 的 affine resource、结构化 Task、Future 和有界 Stream 草案能否得到可执行状态机与当前 WASI 0.3 WIT 的一致映射？

## 2. Method

Rust 原型以 Drop/所有权实现资源，以 scope 绑定的消费型 task handle、one-shot producer/reader 和容量固定的队列实现动态规则。两个非法 Rust 程序由独立 PowerShell 执行器调用 `rustc --emit=metadata`，要求稳定错误码。WIT 使用 owned/borrowed resource、`async func`、`future` 和 `stream`，由 `wit-parser 0.253.0` 解析。

## 3. Reproduction

```powershell
cargo fmt --manifest-path prototypes/resource-async/Cargo.toml -- --check
cargo clippy --locked --manifest-path prototypes/resource-async/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path prototypes/resource-async/Cargo.toml
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-resource-async.ps1
cargo run --release --locked --manifest-path prototypes/resource-async/Cargo.toml --bin resource-async-probe -- --output prototypes/resource-async/results/windows-x86_64-gnu.json
```

## 4. Raw evidence

`prototypes/resource-async/results/windows-x86_64-gnu.json`

SHA-256：`8a72efb954832c74c555d6b31e99b500317fe7da2691e708b4937ed06c744f3c`。

## 5. Results

| Evidence | Result | Status |
|---|---:|---|
| Rust dynamic tests | 10 passed | verified |
| WIT parser tests | 1 passed | verified |
| compile-fail | E0382 + E0515 rejected | verified |
| explicit resource close | 1 | measured |
| implicit resource drop | 1 | measured |
| completed tasks | 1 | measured |
| cancelled tasks | 1 | measured |
| stream max in-flight | 2 | measured |
| backpressure events | 1 | measured |
| Clippy warnings | 0 | verified |

探针事件顺序固定为 open→close、open→drop、scope open、task complete、task cancel、scope drop。取消重复调用是幂等的。

## 6. Interpretation

- verified：owned move + consuming close + Drop 可以做到恰好一次释放；
- verified：Rust lifetime/ownership 能拒绝借用逃逸和 move 后使用，说明 Sico 规则可实现，但不是 Sico 编译器证据；
- verified：结构化 scope、reader drop 和 open stream drop 都能传播取消；
- verified：容量边界能局部产生背压而不是继续分配；
- verified：当前 WIT parser 接受 WASI 0.3 原生 async 类型；
- proposed：这些 WIT 类型与 Sico Runtime 的真实调用映射，仍待 STEP-0010。

## 7. Limitations

- 状态机是确定性原型，不是线程安全调度器；
- 没有真实 Component resource table、Canonical ABI 或 Wasmtime Store；
- 未测公平性、并发竞态、跨线程 Send/Sync、fuel/epoch 或大流性能；
- compile-fail 是 Rust 证据，Sico 仍需自身静态检查与短诊断；
- WASI 0.3 发布较新，guest bindings 与 Runtime feature 版本可能不同步。

## 8. Decision impact

支持直接采用 WASI 0.3 async/future/stream，拒绝新造 pollable ABI；支持 WIT owned/borrowed resource 与 Sico affine 语义对齐。RFC-0004 保持 proposed，STEP-0010 必须用真实 Runtime 验证。

## 9. Links

- [STEP-0009](../steps/STEP-0009-resource-async-wit-prototypes.md)
- [RFC-0004](../rfc/RFC-0004-resource-async-mapping-v0.md)
- [prototype](../../prototypes/resource-async/README.md)
