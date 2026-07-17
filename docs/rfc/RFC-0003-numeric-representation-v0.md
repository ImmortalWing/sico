# RFC-0003: Numeric representation v0

> - status: proposed
> - date: 2026-07-14
> - authors: autonomous-agent
> - target language/platform version: draft
> - supersedes: -
> - superseded-by: -

## Summary

保留 Sico `Int` 的任意精度值语义，提出唯一的 sign/magnitude Component 编码；把 Decimal 定义为标准库精确有限十进制候选，使用规范 coefficient/scale 编码，并要求所有窄化、舍入和资源边界显式。

## Problem

WIT 只提供固定宽度整数和 IEEE 浮点数，不能直接表达 Sico 任意精度 `Int` 或精确 Decimal。若直接暴露 Rust `BigInt` 字节、宿主补码、浮点数或某个固定精度库，会产生负零、前导符号字节、溢出、舍入差异和版本耦合。AI 也难以从局部源码判断转换会不会丢失数据。

## Goals and non-goals

目标：

- 普通 `Int` 算术不因固定机器宽度溢出；
- 一个数值只有一个规范跨界编码；
- 固定宽度转换和 Decimal 舍入显式且可失败；
- 在分配与运算边界应用可测试资源限额；
- WIT 记录不依赖 Rust 内存布局。

非目标：

- 在本 RFC 中决定 Decimal 表层字面量；
- 承诺无限 CPU 或内存；
- 把 Money、币种、显示 scale 或会计规则合并进 Decimal；
- 定义高性能大整数算法、完整 Decimal 除法上下文或正式 ABI 稳定期。

## Semantics

### Int

`Int` 是任意精度有符号整数。加、减、乘等普通运算先产生数学整数，不发生固定宽度回绕、饱和或未定义行为。实现必须对输入和中间/结果资源应用明确限额；达到隔离硬上限不是业务错误。

转换到 `S8`—`S64`、`U8`—`U64` 等固定宽度类型必须显式、受检并在越界时失败。固定宽度类型之间也不默认窄化。

### Decimal candidate

Decimal 是标准库值类型候选，不是核心内建标量。其有限值表示为：

```text
value = coefficient * 10^(-scale)
coefficient: Int
scale: U32
```

数值编码移除 coefficient 的十进制尾零；零固定为 coefficient `0`、scale `0`。因此 `1.2300` 与 `1.23` 是同一数值和同一编码。展示小数位、计价精度和货币规则属于 Money/格式化层元数据。

加法和乘法精确执行，不经过 Float。降低 scale 必须指定舍入模式；v0 原型验证 `toward-zero` 和 `half-even`。除法必须提供最大 scale/精度和舍入上下文，本提案暂不定义其 API。

### Limits

至少区分：整数 magnitude 字节、Decimal 输入/结果 digits、Decimal scale、WIT payload 字节。编译期超大字面量应产生短诊断；处理不可信边界数据应返回边界验证错误；运行中威胁实例隔离的硬限额按 SEM-074/093 产生受控 trap。普通数值操作不因此普遍改写为业务 `Result`。

## Syntax candidates

本 RFC 不决定 Decimal 字面量。现实候选为：

```text
Decimal.parse_exact("12.30")
Decimal.from_parts(coefficient: 123, scale: 1)
```

未来若增加专用字面量，必须仍映射到相同值语义、规范编码和显式上下文规则，且参与 STEP-0012/0013 的候选语法评测。

## Positive and negative cases

正例：NUM-001 的 `999999999999999999999999 + 1` 精确通过；`0.1 * 0.2` 的 Decimal 结果精确为 `0.02`；`2.355` 在显式 half-even、scale 2 下为 `2.36`。

负例：隐式 Int→Float64 继续由 NUM-101 拒绝；负零 sign/magnitude、非零前导字节、Decimal coefficient 尾零、超过 digits/scale/payload 上限以及越界的 Int→S64 均拒绝。

建议短诊断：

```text
integer does not fit S64
integer exceeds configured byte limit
decimal scale exceeds configured limit
decimal encoding is not canonical
rounding mode required
```

稳定 code/key 在本 RFC 接受前另行登记，不复用业务类型错误编号。

## AST and IR

- AST 保存整数源码跨度、原始词素和已解析数学值；非法或超限词素在 lowering 前拒绝；
- Sico IR 使用规范任意精度常量，不使用宿主 `i64` 承载所有 `Int`；
- `Int` 算术降低到经验证的 runtime/library 操作，固定宽度边界使用显式 checked conversion；
- Decimal 在 v0 保持名义标准库类型和普通调用，不新增核心 IR 标量；
- 优化前后必须保持规范值和限额观察点，不得把精确运算替换为 Float。

## Component/WIT mapping

v0 候选记录：

```wit
enum integer-sign { negative, zero, positive }
record big-int {
  sign: integer-sign,
  magnitude-be: list<u8>,
}
record decimal-value {
  coefficient: big-int,
  scale: u32,
}
```

整数 magnitude 是无符号最短大端字节；零 magnitude 为空；禁止负零和前导零。Decimal coefficient 复用该记录，并要求尾零规范化。可证明适合固定宽度的接口应优先直接使用 WIT `s8`—`s64`/`u8`—`u64`，避免无意义分配。

记录的字段语义由 WIT 包版本控制，不由 Rust struct 布局控制。STEP-0010 已用真实 Component/Canonical ABI 往返 512-byte magnitude 的 4096-bit 值与 Decimal 记录，字段和值保持不变；规范拒绝与限额仍由 STEP-0008 Rust 原型验证。

## AI evaluation

AI 任务至少覆盖：选择固定宽度或任意精度边界、修复隐式转换、识别 Decimal 舍入缺失、解释规范化和修复非规范 WIT 输入。诊断应指出目标宽度或具体限额，不输出大数完整内容。

本步骤只有 Rust 正反测试，没有真实模型运行，因此不声明 AI 成功率变化。

## Compatibility

这是 M0 `proposed` 接口。接受前允许改变 WIT 包名和字段，但任何改变必须更新版本、fixtures 和 adapter 计划。接受后，改变 sign、字节序、零表示或 Decimal 规范化都属于破坏性接口变更。

## Security and privacy

- 在解析、WIT 解码和运算前后检查限额；
- 乘法对可证明超限结果预检，避免先完成巨额分配；
- 诊断只报告位数/字节数、上限和类型，不回显可能敏感的完整数值；
- Runtime 仍必须限制 Component memory、fuel/epoch 和调用 payload，语言级检查不能代替沙箱；
- 反序列化拒绝非规范等值编码，避免哈希、签名和缓存键歧义。

## Alternatives

- **所有整数固定 64 位**：WIT 简单，但违反 NUM-001，并迫使普通算术选择回绕、trap、饱和或 `Result`；不采用。
- **补码字节串**：紧凑，但最短负数、符号扩展和零有更多规范化规则；v0 不采用。
- **十进制文本跨界**：易调试，但解析成本、Unicode/语法约束和字节体积更大；保留给文本协议，不作为 Component 值 ABI 首选。
- **固定 96/128 位 Decimal**：实现成熟且有硬上限，但精度、scale 和溢出会进入公共语义；保留为后续性能候选。
- **保留 Decimal 尾零**：适合展示 quantum，但会让数值等价与编码身份分裂；展示 scale 转移到领域类型。

## Validation and acceptance criteria

当前原型已满足：

- 10 个 Rust 单元测试通过；
- 4096-bit 操作、规范往返、显式舍入和限额拒绝实测；
- 两个独立进程的 Int/Decimal 规范字节 checksum 一致；
- Windows x86_64 GNU + rust-lld release 探针通过。

STEP-0010 已满足下列第 1、2 项；RFC 保持 `proposed`，直到其余条件完成：

1. ~~STEP-0010 用真实 WebAssembly Component 和 Runtime 往返该 WIT 类型；~~
2. ~~WIT grammar/Canonical ABI 工具验证通过；~~
3. 至少验证一个非 Windows 构建环境，或在 M0 审计中明确保留跨平台风险；
4. 决定资源超限在编译期、边界调用和实例执行中的稳定诊断/trap 分类。

## M8 fixed-width implementation note

STEP-0077 implements the Script v0 `I64/U64` subset without changing this RFC's arbitrary-precision `Int` direction. Explicit in-range literal construction is available; arbitrary runtime `Int` conversion remains unimplemented. Checked add/sub produce typed `NumericError.overflow` or `NumericError.underflow`, and fixed-width comparisons preserve signedness through verified IR and Wasm. This evidence does not complete the remaining arbitrary-precision Runtime or cross-platform acceptance gates of this RFC.

## Links

- [STEP-0008](../steps/STEP-0008-numeric-representation-prototypes.md)
- [numeric prototype](../../prototypes/numeric/README.md)
- [numeric report](../reports/numeric-representation-v0.md)
- [WIT type reference](https://component-model.bytecodealliance.org/design/wit.html)
- SEM-021、SEM-024、SEM-074、SEM-093、SEM-121、NUM-001、NUM-101
