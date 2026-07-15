# STEP-0030: 冻结 typed Sico IR contract 与独立 verifier

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

建立 `sico-ir` workspace crate，冻结 typed IR v0 的 ID、source map、type/operation、control-flow、canonical JSON 与 verifier 边界，并证明 invalid source/IR 不能进入后端。

## 2. Decision gate

- 接受 [`RFC-0008`](../rfc/RFC-0008-typed-sico-ir-contract-v0.md) 的结构契约；
- `Int` 保持规范数学整数文本，不把 RFC-0003 proposed representation 偷换成 i64；
- resource/async operation 仅登记 typed identity，具体 lowering/ABI 仍由 STEP-0032/0035 与 RFC-0004 gate 决定；
- v0 不允许隐式跨 block value capture；后续如需 phi，必须新增显式 block parameter contract。

## 3. Acceptance

- stable function/block/value IDs 与 source ranges；
- 13 类 phase types、16 类 operations、4 类 terminators 可确定性序列化；
- verifier 独立检查 schema、limits、IDs、ranges、def-before-use、types、call/effect/CFG/return；
- 至少 9 类 malformed mutation 被拒绝，invalid IR 不序列化；
- syntax/semantic invalid source 在 IR construction 前阻止；
- verifier diagnostics 精确上限 100；
- workspace 与 M2/M1/M0 regression。

## 4. Commit

`feat(ir): [STEP-0030] define typed IR and verifier`

## 5. Changes and validation

- 新增 `sico-ir`，提供 closed typed data model、`require_semantic_success`、`verify` 与 `canonical_json`；
- canonical IDs 使用声明/结构顺序，range 为原 source UTF-8 byte half-open range；
- valid identity/branch/effect shapes 与全部 phase type serialization 通过；
- schema/ID/range/entry/value/return/constant/target mutation 和 100-error cap 通过；
- RFC-0003/0004 未接受的 ABI/runtime 选择保持未实现。

```text
STEP_0030_OK schema=sico.ir.v0 types=13 operations=16 terminators=4 valid_shapes=2 mutations=9 semantic_gate=E2001 frontend_gate=pass stable_ids=pass ranges=pass diagnostic_cap=100
```

## 6. Next

STEP-0031 在 RFC-0008 边界内实现 core expression/control/data lowering，并单独冻结 evaluation/error order。
