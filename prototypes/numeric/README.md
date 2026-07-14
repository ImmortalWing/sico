# Sico numeric prototype

STEP-0008 的独立 Rust 原型，验证：

- 任意精度 `Int` 普通运算不发生固定宽度溢出；
- sign/magnitude 大端最短编码具有唯一形式；
- Decimal 使用任意精度 coefficient 与非负 scale 表示精确有限十进制；
- Decimal 加法、乘法和显式 half-even 舍入不经过二进制浮点；
- 输入、结果、scale 和跨界载荷受可注入限额保护；
- WIT 没有任意精度数值时，使用规范 record 或显式受检窄化。

原型错误是实验 API，不等于最终 Sico 表层错误模型。达到 Runtime 硬上限时究竟返回错误还是终止隔离实例，由发生阶段和 STEP-0008 报告区分。

## Reproduce

```powershell
cargo test --manifest-path prototypes/numeric/Cargo.toml
cargo run --release --manifest-path prototypes/numeric/Cargo.toml --bin numeric-probe -- --output prototypes/numeric/results/windows-x86_64-gnu.json
```

Windows GNU 目标通过仓库 `.cargo/config.toml` 使用 Rust 自带的 `rust-lld`，不依赖 JavaScript、Node.js 或系统 GCC。
