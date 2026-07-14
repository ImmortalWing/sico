# STEP-0008: 验证 Int 与 Decimal 表示原型

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

用独立 Rust 原型验证 Sico `Int` 任意精度整数与标准库 `Decimal` 的精确运算、确定性序列化、资源限额和 WIT 可表示边界，并形成可撤销的 M0 技术结论。

## 2. Context and evidence

- SEM-021 规定普通 `Int` 是任意精度有符号整数，固定宽度整数只用于明确边界；
- SEM-024 禁止 Float、Int 与 Decimal 隐式转换；
- SEM-093 规定硬资源上限由 Runtime policy 执行；
- SEM-121 要求 Component 边界只直接暴露 WIT 可表示类型；
- NUM-001 要求超过固定宽度的整数运算保持精确；
- WIT 只有固定宽度整数，因此任意精度数值需要显式记录表示或受检窄化；
- 本机使用 `rustc 1.97.0`、`cargo 1.97.0`、`x86_64-pc-windows-gnu` 与 `rust-lld`。

## 3. Scope

包含：

- `Int` 的任意精度运算与规范化 sign/magnitude 表示；
- Decimal 的任意精度十进制 coefficient/scale 表示和精确加乘；
- 明确舍入、规范字符串和 WIT record 映射；
- 输入、结果、scale 与跨界字节限额；
- 正反单元测试、确定性探针和报告。

不包含：

- 正式 Sico parser、type checker、IR 或代码生成；
- 把原型错误类型直接固定为表层语言 API；
- 无界资源承诺、隐式数值转换、浮点互转或完整金融标准库；
- 正式接受尚无足够证据的 Decimal 精度与舍入上下文 RFC。

## 4. Options and decision

`Int` 比较固定宽度、二进制补码字节与 sign/magnitude 字节。原型采用 `BigInt` 内部表示和规范 sign/magnitude 边界：零无 magnitude；非零 magnitude 为最短大端无符号字节；禁止负零和前导零。它局部可验证且不会把宿主补码宽度写进 ABI。

Decimal 比较固定 96/128 位十进制与任意精度 coefficient/scale。原型采用任意精度有符号 coefficient 加非负 `u32` scale，并去除小数尾零形成唯一数值编码。固定精度方案继续作为未来性能对照，不由本步骤直接淘汰。

WIT 边界同时提供两种策略：可证明落在固定宽度范围时显式受检窄化；否则使用由 `list<u8>`、sign 和 scale 组成的版本化记录。资源超限在原型中返回结构化 `LimitExceeded`，最终映射到编译诊断、业务 Result 或隔离 trap 由后续 RFC 按发生阶段区分。

## 5. Plan

1. 建立独立 Cargo 原型和 Windows GNU `rust-lld` 配置；
2. 实现 Int/Decimal 规范表示、运算和限额；
3. 定义 WIT 边界草案和拒绝非规范输入的测试；
4. 运行单元测试与确定性探针，保存机器可读结果；
5. 写实验报告，更新语义开放项、状态和路线图；
6. 运行全量回归，提交并推送。

## 6. Changes

- 新增 `prototypes/numeric` Cargo crate，以 `num-bigint 0.5.1` 实现受限任意精度原型；
- 新增 `CanonicalInt`，固定 sign、最短大端 magnitude、空零编码并拒绝负零/前导零；
- 新增 `ExactDecimal`，实现 coefficient/scale 规范化、精确加乘、toward-zero/half-even 显式舍入；
- 新增 `NumericLimits` 和输入、结果、scale、wire 限额，乘法对可证明超限结果预检；
- 新增受检 Int→S64、WIT 候选记录与 10 个正反单元测试；
- 新增 release `numeric-probe`、两份 Windows 原始结果和跨进程规范 checksum 比较；
- 新增 [`RFC-0003`](../rfc/RFC-0003-numeric-representation-v0.md)（`proposed`）与 [`numeric-representation-v0`](../reports/numeric-representation-v0.md) 报告；
- 新增 Windows GNU `rust-lld` Cargo 配置，不依赖系统 GCC。

## 7. Validation

已运行：

```powershell
cargo fmt --manifest-path prototypes/numeric/Cargo.toml -- --check
cargo clippy --manifest-path prototypes/numeric/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path prototypes/numeric/Cargo.toml
cargo run --release --manifest-path prototypes/numeric/Cargo.toml --bin numeric-probe -- --output prototypes/numeric/results/windows-x86_64-gnu.json
```

结果：格式化通过；Clippy 零 warning；10 tests passed；release 探针通过。现有 PowerShell 离线校验器在本步骤全量回归中再次运行。

## 8. Metrics

- 4096-bit operand 加一得到精确 4097-bit 结果；候选逻辑 wire payload 为 514 bytes；
- Decimal 精确和为 `12345678901234567890.1234568`，27 coefficient digits、scale 7、逻辑 wire payload 16 bytes；
- 两次独立进程的 Int checksum 均为 `55d8491b11ad948c`，Decimal checksum 均为 `c69072d00c90b010`；
- 首份保留运行的 2,000 次总耗时：Int add 5,827,300 ns、Int multiply 16,653,900 ns、Decimal add 2,869,600 ns、Decimal multiply 7,589,000 ns；这些只是单机指示值。

## 9. Risks and follow-ups

- 任意精度可消除固定宽度溢出，但不能消除内存和 CPU 耗尽；
- Decimal 规范化会丢弃展示 scale，金额显示精度必须由 Money/格式化层表达；
- `list<u8>` WIT 表示可移植但不是零拷贝，需在真实 Component 原型中测量；
- STEP-0009 继续验证资源、异步和 WIT 映射，STEP-0010 才验证真实 Component ABI。

## 10. Audit links

- semantics: SEM-021、SEM-024、SEM-093、SEM-121
- case: NUM-001
- report: [`numeric-representation-v0`](../reports/numeric-representation-v0.md)
- RFC: [`RFC-0003`](../rfc/RFC-0003-numeric-representation-v0.md) (`proposed`)
- commit subject: `feat(numeric): [STEP-0008] validate numeric representations`
- next: STEP-0009 resource/async/WIT mapping prototypes
