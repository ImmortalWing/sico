# STEP-0031: 实现 core expression/control/data lowering

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

将已通过 M2 的 core 子集真实 lowering 到 RFC-0008 typed IR，并以 RFC-0009 固定 binary/call/constructor/try/match 的 source-order、single-evaluation 规则。

## 2. Boundaries

- 支持 parameter/local/constants/add/direct call/intrinsic/newtype/record/variant/Result ok+try/payload-free exhaustive match；
- payload binding、closure、if/revision、effect/resource/async/component 暂不 lowering，返回带 source range 的 typed `Unsupported`；
- 所有 29 个 semantic invalid case 必须在 capability dispatch 前由原 M2 code 拒绝；
- unsupported source 不生成 empty/partial/unreachable-success module。

## 3. Acceptance

- 12 个现有 valid B core cases 真实 lowering、独立 verify、重复 canonical JSON 一致；
- 12 条 compact golden snapshots 固定 function/block/value/operation/terminator shape；
- 剩余 13 个 valid cases诚实 typed-refuse；
- 29/29 invalid 保持精确主要 E-code，均未到 IR support dispatch；
- record field 与 binary operand instruction order 证明 left-to-right once；
- intrinsic/try/match mutation 被 verifier 拒绝；
- workspace/M2/M1/M0 regression。

## 4. Commit

`feat(ir): [STEP-0031] lower core semantics deterministically`

## 5. Changes and validation

- `lower_core` 先执行完整 semantic gate，再从 HIR declaration/token ranges 构建 typed IR；
- signatures、constructors、variants、record fields 静态解析，function IDs 按 declaration order；
- `Float64.from_int`、same-error `try` 与 ordered match 获得明确 IR/verifier rule；
- 12 supported + 13 deferred valid 的 capability split 固定，避免空 interface module 伪成功。

```text
STEP_0031_OK core_valid=12 deferred_valid=13 invalid_blocked=29 snapshots=12 evaluation=left-to-right-single-evaluation operations=18 terminators=5 verifier_mutations=3
```

## 6. Next

STEP-0032 在不改变 core order 的前提下实现 effect/capability、affine resource 与 revision IR flow，并补 ownership verifier。
