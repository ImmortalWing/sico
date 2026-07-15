# Report: full B HIR and name/prelude contract v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0022
> - environment: Windows, Rust 1.97.0

## 1. Question

全部 54 个成功 B source 能否在不提前决定类型/控制流语义的前提下，稳定 lowering 为覆盖所有 semantic token、跨行 delimiter、line/block、ID 和 source range 的 HIR，并彻底阻止 error tree？

## 2. Method

lowering 只接受 `Parse::is_success()`；顶层 declaration 复用 parser 的权威范围，每个 declaration/body line 分配 preorder ID，保留非 trivia token 与结构 kind，并在 declaration 级建立 balanced token tree，以覆盖匿名函数跨行关闭外层调用的 B 形态。对 54 files 比较 structural snapshot、二次 lowering equality、连续 IDs、range scalar boundary 和 token text/source slice；对 12 mutation 验证拒绝。

## 3. Results

```text
STEP_0022_OK b_lowered=54 snapshots=54 stable_ids=pass source_maps=pass semantic_tokens=complete mutations_blocked=12 prelude_types=16 fixture_members=3 namespace_collision=undecided-rfc-gated
```

- deterministic lowering：54/54；
- structural snapshots：54/54；
- source maps：全部 HIR ID 有合法 UTF-8 half-open range；
- semantic token：text 与 source slice 逐 token 相等；
- declaration-level delimiter tree：支持跨行匿名 function/call；
- error tree：12/12 mutation 返回 typed refusal；
- prelude：16 type constructors、3 个 P0 fixture members；不承诺公开 SDK 名称；
- namespace collision：无判别案例，因此保持 RFC-gated 未决定。

## 4. Limits

HIR 当前是 untyped structural representation；expression type、pattern coverage、effect/resource state 由 STEP-0023–0027 逐项产生事实。RFC-0007 不接受任何新的运算符优先级或跨 namespace collision 行为。

## 5. Links

- [`STEP-0022`](../steps/STEP-0022-full-hir-name-prelude-contract.md)
- [`RFC-0007`](../rfc/RFC-0007-m2-hir-name-prelude-contract-v0.md)
- [`prelude manifest`](../../semantics/prelude-v0.json)
- [`HIR snapshots`](../../tests/hir/b-shapes.txt)
- [`sico-hir`](../../crates/sico-hir/src/lib.rs)
