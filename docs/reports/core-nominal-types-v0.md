# Report: core and nominal types v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0023
> - environment: Windows, Rust 1.97.0

## 1. Question

numbers-units 与 nominal-invariants 的 16 个 B source 能否由真实 module/type/function model 检查，并使 9 个非法案例各自产生唯一、精确且有界的根因诊断？

## 2. Method

新增 `sico-semantics` crate，从成功 HIR 建立 newtype、record、field、invariant 与 function signature 表；按局部变量环境推断 P0 表达式、构造调用与字段访问。类型关系保持 nominal equality，不做隐式数值转换；unknown 抑制依赖诊断。测试直接读取 16 个 B source 与权威 `semantic-case-map.json`，比较 code、key、message、arguments、range，并确认其余 B group 不获得本步骤未拥有的 E2xxx 诊断。

## 3. Results

```text
STEP_0023_OK valid=7 invalid=9 exact_primary=9 codes=E2001,E2002,E2010,E2011,E2020 nominal_identity=pass fields=pass invariant=constant-v0 stable_facts=pass cascades=bounded
```

- 7/7 valid：零诊断；
- 9/9 invalid：每个文件恰好一个 primary diagnostic，完整匹配 catalog map；
- newtype 与 record：底层或字段结构相同仍不兼容；
- record constructor：缺失/未知字段分别命中 E2010/E2011；
- invariant v0：仅判定 P0 整数字面量构造中的 `min <= max`；
- semantic facts：使用 `(HIR ID, slot)` 复合 ID，包含类型、函数、参数、字段、局部变量与合法 source range；
- cascade：unknown type 不生成派生 mismatch，诊断总量上限为 100。

## 4. Limits

本步骤没有决定通用数值提升、泛型约束、符号重载或 theorem proving。match、Option/Result、effect/capability、resource/async、revision 仍未检查，由 STEP-0024–0027 按 oracle 分步实现。

## 5. Links

- [`STEP-0023`](../steps/STEP-0023-core-nominal-types.md)
- [`sico-semantics`](../../crates/sico-semantics/src/lib.rs)
- [`diagnostic case map`](../../diagnostics/semantic-case-map.json)
