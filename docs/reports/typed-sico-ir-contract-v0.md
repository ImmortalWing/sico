# Report: typed Sico IR contract and verifier v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0030
> - environment: Windows, Rust 1.97.0

## 1. Question

能否在不提前决定 Wasm ABI、数值 runtime representation 或 async scheduling 的前提下，建立足以独立阻断 malformed compiler state 的 typed IR contract？

## 2. Method

新增 `sico-ir` closed data model。IR 用顺序 FunctionId/BlockId/ValueId、原 source byte ranges、typed instruction 与显式 terminator。独立 verifier 不信任 builder，重建 signature/value/CFG tables 并检查规范性。serialization 只在 verifier 无错误后执行。

## 3. Results

```text
STEP_0030_OK schema=sico.ir.v0 types=13 operations=16 terminators=4 valid_shapes=2 mutations=9 semantic_gate=E2001 frontend_gate=pass stable_ids=pass ranges=pass diagnostic_cap=100
```

- identity 与 branch/declared-effect shapes 通过；
- canonical JSON 重复 byte-identical，并 roundtrip 回相同 Module；
- 9 类 schema/structural/type mutation 被拒绝；
- 101 个错误输入只保留 100 个 verifier root errors；
- semantic E2001 与 syntax failure 均在 IR builder 前被阻止；
- 13 类 type identity 可稳定序列化，但不冒充对应 backend 已实现。

## 4. Boundaries

v0 block 只能使用函数参数或同 block earlier value，尚无 block parameters/phi。operation contract 允许后续阶段表达 resource/async，但 ownership flow、canonical ABI 和 Runtime mapping 未在本步实现。RFC-0003/0004 继续保持 proposed。

## 5. Links

- [`STEP-0030`](../steps/STEP-0030-typed-sico-ir-contract-verifier.md)
- [`RFC-0008`](../rfc/RFC-0008-typed-sico-ir-contract-v0.md)
- [`sico-ir`](../../crates/sico-ir/src/lib.rs)
