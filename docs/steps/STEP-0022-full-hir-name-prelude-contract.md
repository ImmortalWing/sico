# STEP-0022: 建立 full B HIR 与 name/prelude contract

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

把 54 个成功 B syntax tree 全部 lowering 为 deterministic untyped HIR：顶层 declaration、body line/block、balanced semantic token tree、stable ID 与 UTF-8 source map；冻结 P0 name/prelude 环境，同时继续阻止 error tree 进入 M2。

## 2. Decision

接受 [`RFC-0007`](../rfc/RFC-0007-m2-hir-name-prelude-contract-v0.md)。STEP-0022 表示完整当前 B token/结构表面，但不提前决定后续 type/control/effect/resource 规则。名称空间碰撞因无判别 case 保持未决定。

## 3. Acceptance

- 54/54 deterministic lowering 与完整 token/range equality；
- HIR IDs 从 0 连续，全部 range 是 source scalar boundary；
- 54 structural snapshots；
- 12/12 mutation 返回 typed lowering refusal；
- prelude 类型 arity、fixture members、lookup order 与 unresolved decisions 离线校验；
- workspace/M1/M0 regression。

## 4. Non-goals

不产生 type facts/semantic diagnostics/index，不选择跨 namespace collision，不实现 IR/Wasm，不把 fixture prelude 名称承诺为公开 SDK。

## 5. Commit

`feat(hir): [STEP-0022] lower B syntax into deterministic HIR`

## 6. Changes and validation

- 新增 `sico-hir` workspace crate、preorder IDs、source map、typed line/block 与 declaration-level balanced token trees；
- 54/54 deterministic lowering、54 snapshots、连续 IDs、token/source equality；
- 12/12 mutation typed refusal；
- RFC-0007 接受 P0-only prelude/name boundary，跨 namespace collision 保持未决定；
- workspace fmt/Clippy/tests、M1/M0 regression 全部通过。

```text
STEP_0022_OK b_lowered=54 snapshots=54 stable_ids=pass source_maps=pass semantic_tokens=complete mutations_blocked=12 prelude_types=16 fixture_members=3 namespace_collision=undecided-rfc-gated
```

## 7. Next

STEP-0023 只实现 numbers + nominal 的 7 valid/9 invalid oracle；不得借 HIR token tree 提前实现 match/effect/resource 规则。
