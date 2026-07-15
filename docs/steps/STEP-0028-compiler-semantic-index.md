# STEP-0028: 生成 compiler-produced Semantic Index/query v0

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

新增 `sico-index`，从 `sico-semantics` 的真实 facts 生成 RFC-0002 `sico.semantic-index.v0` snapshot，并执行 outline/describe/slice/impact/flow 五类 query。

## 2. Boundaries

- production snapshot 使用真实 SHA-256 descriptor，禁止 `fixture:`；
- semantic ID 不含 offset，source range 保持 UTF-8 byte + 1-based scalar line/column；
- facts 明确 declared/verified provenance，diagnostic 决定 complete/partial/blocked_by；
- 代表性 examples 仍是候选 A，不能伪称可由 B compiler 完整检查；应产生 honest partial module；
- 不实现持久缓存、网络服务、跨包依赖发现或 M3 IR facts。

## 3. Acceptance

- 10 个成功 B module 生成 complete compiler snapshot 与稳定 symbol/fact IDs/ranges；
- 10 个代表性 A example 生成 honest partial modules；semantic invalid B module 引用 blocking E-code；
- 五类 operations 在同 snapshot 上执行，budget/truncation/completeness 一致；
- serialized index/query response 满足 RFC-0002 v0 schema required contract；
- workspace/M1/M0 regression。

## 4. Commit

`feat(index): [STEP-0028] produce semantic index and queries`

## 5. Changes and validation

- 新增 `sico-index` workspace crate 与 production SHA-256 snapshot descriptor；
- compiler facts → modules/symbols/facets/relations/source ranges；
- complete/partial/blocking quality 和 stale snapshot/budget enforcement；
- 10 complete B modules、10 honest partial A examples、E2001 blocking propagation；
- 五类 operations 与 RFC-0002 schema/fixture regression 通过。

```text
STEP_0028_OK complete_modules=10 representative_partial=10 invalid_blocked=E2001 operations=5 snapshot=sha256 schema=index-query-response-v0 stable_ids=pass ranges=pass budgets=pass
```

## 6. Next

STEP-0029 集成 semantic CLI text/JSON，补 fuzz/limits/performance，并执行 M2 exit audit。
