# Report: match and Result control flow v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0024
> - environment: Windows, Rust 1.97.0

## 1. Question

exhaustive-match 与 result-mapping 的 14 个 B source 能否由 sealed coverage 和 Result flow 规则真实检查，并使 8 个非法案例各自产生唯一、精确且有界的根因诊断？

## 2. Method

从 HIR enum declaration 建立有序 variant/payload model；对 match arm 构造显式 coverage set，检查单 enum、二元 product、重复 arm 和禁止 wildcard。Result 以 `Result[T, E]` 类型参数跟踪；`try` 比较 source/target error，`map_error` 对源 error enum 做显式 coverage，expression statement 拒绝丢弃 Result。测试读取 14 个 B source 和权威 catalog map，并对所有其他 B group 检查无未拥有 E3xxx。

## 3. Results

```text
STEP_0024_OK valid=6 invalid=8 exact_primary=8 codes=E3001,E3002,E3003,E3101,E3102,E3103,E3104 sealed_coverage=pass result_flow=pass stable_facts=pass cascades=bounded
```

- 6/6 valid：零诊断；
- 8/8 invalid：各一个 primary diagnostic，code/key/message/arguments 完整匹配 map；
- enum/product：显式穷尽通过，sealed wildcard 与重复 arm 分别定位根因；
- Result：同 error `try` 通过，不同 error 拒绝；total `map_error` 通过，缺失/wildcard 拒绝；
- 裸 Result expression：命中 E3104；
- facts：variant、match、Result-derived local 均使用稳定 `(HIR ID, slot)` 与合法 range；
- cascade：每个 invalid 只有一个 primary diagnostic，unknown 继续抑制派生噪音。

## 4. Limits

本步骤只接受当前 sealed P0 enum/pattern 子集，不决定开放枚举、guard、通用 destructuring、隐式 error conversion 或 runtime exception。effect/capability、resource/async、revision 仍由后续步骤实现。

## 5. Links

- [`STEP-0024`](../steps/STEP-0024-match-result-control-flow.md)
- [`sico-semantics`](../../crates/sico-semantics/src/lib.rs)
- [`diagnostic case map`](../../diagnostics/semantic-case-map.json)
