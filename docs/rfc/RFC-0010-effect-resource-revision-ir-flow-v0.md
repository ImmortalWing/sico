# RFC-0010: effect, resource and revision IR flow v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

冻结 effect/capability、affine resource 与 revision guard 在 typed Sico IR 中的显式 flow。verifier 必须在 codegen 前独立拒绝 undeclared effect、resource copy/use-after-consume/leak、borrow escape 和未支配控制流的 revision check。

## Capability and effect

- capability 参数使用 `Capability(name)` type identity，只能来自已验证 function parameter/call edge，constructor 不得伪造；
- effect set 按稳定名称排序去重；每个 `effect-call` 引用一个已声明 effect，并显式携带 capability token 及 source-order arguments；
- capability token v0 不采用 affine consume，但 operation 不隐式扩大 effect set；
- capability method 名和 effect 名可以不同，例如 `store.commit` 受已声明 `store.write` 约束，IR 记录的是 semantic effect identity。

## Affine resource

- owned resource 使用 `OwnedResource(name)`，调用期 borrow 使用 `BorrowedResource(name)`；
- `resource-borrow` 不消费 owner，只产生 call-scoped borrowed value；
- `resource-call` 只接受 borrowed receiver；
- `resource-move` 消费 source 并产生新的 owned identity；
- `resource-drop`/显式 `close` 消费 owner并返回 Unit；
- `using` return 先完成 result expression，再 drop owner，最后 return result；
- verifier 对每个已支持 linear block 重建 live/consumed set，拒绝 copy、double consume、use-after-consume、borrow return 和未清理 live owner。

携带 owned resource 穿越 branch/match 的 block-parameter flow 暂不支持，不能用隐式 capture 模拟。

## Revision guard

- revision equality lowering 为 `revision-check(value, expected) -> Bool`；operand type 必须相同；
- check result 必须直接成为显式 branch condition，不能计算后丢弃或由 Runtime 隐式处理；
- then/fallback 均为普通 source-order blocks；stale 是语言分支结果，不在本步引入隐藏 retry/conflict resolver。

## Rejected alternatives

- capability/effect 只保留字符串调用而不进入 verifier：无法证明声明边界；
- resource 降为可复制整数 handle：破坏 M2 affine semantics；
- 在 return 后插入 drop：不可执行且会泄漏；
- 自动 clone/drop 修复 malformed IR：隐藏 compiler bug；
- Runtime 自动重试 revision conflict：引入不可见语义。

## Acceptance evidence

- CAP-001、RES-001/002、REV-001/002 生成 5 条 deterministic verified snapshots；
- 累计 17/25 valid lowering，8/25 component/async valid typed-refuse；
- undeclared effect、missing drop、double consume 和 unused revision guard mutation 被拒绝；
- 29 invalid 仍由 M2 gate 先拒绝；
- workspace/M2/M1/M0 regression。

## Links

- [`RFC-0008`](./RFC-0008-typed-sico-ir-contract-v0.md)
- [`RFC-0009`](./RFC-0009-core-lowering-evaluation-order-v0.md)
- [`STEP-0032`](../steps/STEP-0032-effect-resource-revision-flow.md)
