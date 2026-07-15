# Report: effect, resource and revision IR flow v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0032
> - environment: Windows, Rust 1.97.0

## 1. Question

typed IR 能否在不依赖 builder 善意或 Runtime 隐式修复的情况下，独立证明 effect declaration、resource affine cleanup 和 revision guard flow？

## 2. Method

lowering 从 HIR signatures 构建 capability/resource method table，保留 M2 type/effect identity。verifier 对每个支持的 linear block重建 value types、owned live set、consumed set与 revision-check uses。任何错误均在 codegen 前返回 verifier root error。

## 3. Results

```text
STEP_0032_OK cumulative_valid=17 deferred_valid=8 flow_valid=5 snapshots=5 effects=2 resources=2 revisions=2 ownership=affine borrow=call-scoped verifier_mutations=4 invalid_blocked=29
```

- CAP-001：`const-string → effect-call → return`；
- RES-001：显式 close 直接成为唯一 `resource-drop`；
- RES-002：`resource-borrow → resource-call → resource-drop → return`；
- REV-001：method call 记录 declared `store.write` effect；
- REV-002：两个 revision projections 后 `revision-check → branch`，stale fallback 显式；
- mutation：清空 effects、删除 drop、重复 drop、绕开 revision condition 均被拒绝。

## 4. Boundaries

当前 affine verifier 面向本阶段已支持的 linear resource blocks。owned resource 跨 CFG edge、payload binding、Component resource table 与 async cancellation 尚未实现；不会据此宣称 RFC-0004 已接受。Runtime 也没有自动 retry revision conflict。

## 5. Links

- [`STEP-0032`](../steps/STEP-0032-effect-resource-revision-flow.md)
- [`RFC-0010`](../rfc/RFC-0010-effect-resource-revision-ir-flow-v0.md)
- [`flow snapshots`](../../tests/ir/flow-lowering.snap)
