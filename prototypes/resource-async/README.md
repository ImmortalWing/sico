# Sico resource and async prototype

STEP-0009 的独立 Rust 原型，验证 affine resource、结构化 task、one-shot future、有界 stream 和 [`async-flow-v0`](../../wit/async-flow-v0/world.wit) WASI 0.3 WIT 映射。

```powershell
cargo test --manifest-path prototypes/resource-async/Cargo.toml
cargo run --release --manifest-path prototypes/resource-async/Cargo.toml --bin resource-async-probe -- --output prototypes/resource-async/results/windows-x86_64-gnu.json
```

Rust 所有权测试只证明这些规则可以实现，不代表 Sico 编译器已经具备所有权检查。
