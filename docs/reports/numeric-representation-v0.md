# Report: Numeric representation prototype v0

> - status: complete
> - date: 2026-07-14
> - related-step: STEP-0008
> - environment: Windows 11 10.0.26100 x86_64; Intel i5-6300HQ 4C/4T; rustc 1.97.0; cargo 1.97.0; GNU target + rust-lld

## 1. Question

Sico 能否保持任意精度 `Int` 和精确 Decimal 语义，同时得到唯一、受限、WIT 可表示的跨组件记录，而不依赖 JavaScript、浮点近似或 Rust 内存布局？

## 2. Method

在独立 Cargo crate 中使用 `num-bigint 0.5.1` 表示数学整数，自行实现规范 sign/magnitude 边界、coefficient/scale Decimal、显式舍入和 `NumericLimits`。测试覆盖合法往返、非法编码、超限、固定宽度窄化、精确运算和文本幂等性。

release 探针对 4096-bit Int 与 29-digit Decimal 各重复 2,000 次加法/乘法，并生成规范字节 FNV-1a checksum。探针运行两次；耗时是单机指示值，不作为稳定 benchmark。两个输出保留在 `prototypes/numeric/results/`。

## 3. Reproduction

```powershell
cargo fmt --manifest-path prototypes/numeric/Cargo.toml -- --check
cargo test --manifest-path prototypes/numeric/Cargo.toml
cargo run --release --manifest-path prototypes/numeric/Cargo.toml --bin numeric-probe -- --output prototypes/numeric/results/windows-x86_64-gnu.json
cargo run --release --manifest-path prototypes/numeric/Cargo.toml --bin numeric-probe -- --output prototypes/numeric/results/windows-x86_64-gnu-repeat.json
```

工具版本：

```text
rustc 1.97.0 (2d8144b78 2026-07-07)
cargo 1.97.0 (c980f4866 2026-06-30)
rustfmt 1.9.0-stable (2d8144b788 2026-07-07)
target x86_64-pc-windows-gnu
LLVM 22.1.6
```

## 4. Raw evidence

| File | SHA-256 |
|---|---|
| `windows-x86_64-gnu.json` | `4ee34ea667149dfbff2d57d7ac86cfe526995b65196154721f05cab21ff16ed6` |
| `windows-x86_64-gnu-repeat.json` | `ff88f1afde40881a2bb299c232bca7baed1c8d6de11640d5177f7020b279da76` |

文件整体哈希因计时字段不同而不同；规范数值 checksum 和 canonical text 应相同。

## 5. Results

### Correctness

| Evidence | Result | Status |
|---|---:|---|
| Rust unit tests | 10 passed, 0 failed | verified |
| 4096-bit operand + 1 | 4097-bit exact result | measured |
| Int WIT-candidate payload | 514 bytes | measured |
| Int over-limit case | rejected | verified |
| Decimal exact sum | `12345678901234567890.1234568` | measured |
| Decimal result | 27 coefficient digits, scale 7 | measured |
| Decimal WIT-candidate payload | 16 bytes | measured |
| non-canonical Decimal wire | rejected | verified |
| repeated Int checksum | `55d8491b11ad948c` both runs | measured |
| repeated Decimal checksum | `c69072d00c90b010` both runs | measured |

`514`/`16` 是原型规范字段的逻辑 payload 大小，不包含 Canonical ABI 对齐、线性内存 allocator 或 Component framing；真实 ABI 大小留给 STEP-0010。

### Indicative timing, first retained run

| Operation | Iterations | Total ns | Approx. ns/op |
|---|---:|---:|---:|
| 4096-bit Int add | 2,000 | 5,827,300 | 2,914 |
| 4096-bit Int multiply | 2,000 | 16,653,900 | 8,327 |
| 29-digit Decimal add | 2,000 | 2,869,600 | 1,435 |
| 29-digit Decimal multiply | 2,000 | 7,589,000 | 3,795 |

这些数字包含限额检查、分配和 `black_box`，没有预热、统计采样或 CPU 固频，只能证明本规模可执行，不能用于 Runtime 选型或跨语言性能宣称。

## 6. Interpretation

- **verified**：任意精度 Int 能满足 NUM-001，且不必把所有普通算术改成固定宽度溢出错误；
- **verified**：内部 `BigInt` 零字节形式与公开规范不同，转换层必须显式规范化，不能直接序列化库对象；
- **verified**：sign + 最短大端 magnitude 能局部拒绝负零、前导零和空非零；
- **verified**：coefficient/scale 能精确表示有限 Decimal，加乘不经过 Float；
- **inferred**：规范去尾零适合作为数值身份和签名输入，但 Money 展示 scale 必须独立；
- **proposed**：该记录作为 Sico Component v0 的公共表示，仍需真实 Canonical ABI 往返。

## 7. Limitations

- 只在 Windows x86_64 GNU + rust-lld 实测；
- WIT 文件尚未由 Component 工具解析，也未进入真实 Component；
- 没有除法、超越函数、Decimal 上下文传播或固定 96/128 位性能对照；
- 单次计时不是统计 benchmark；
- FNV-1a 只用于快速确定性比较，不用于签名或安全哈希；
- `list<u8>` 的宿主分配在进入 Rust 校验前已经发生，Runtime 仍需线性内存和调用 payload 上限；
- 没有真实 AI 生成、理解或修复运行。

## 8. Decision impact

- 支持继续保留 SEM-021 的任意精度 `Int` 草案；
- 支持 RFC-0003 的 sign/magnitude 与 coefficient/scale 提案；
- 不支持把 `num-bigint` 私有序列化或 Rust struct 布局定为 ABI；
- 不足以接受 RFC-0003；STEP-0010 必须完成真实 WIT/Component 往返；
- STEP-0009 可复用“语言值、WIT 记录、Runtime policy 分层”的验证方法。

## 9. Links

- [STEP-0008](../steps/STEP-0008-numeric-representation-prototypes.md)
- [RFC-0003](../rfc/RFC-0003-numeric-representation-v0.md)
- [prototype](../../prototypes/numeric/README.md)
- [WIT type reference](https://component-model.bytecodealliance.org/design/wit.html)
