# Report: core lowering and evaluation order v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0031
> - environment: Windows, Rust 1.97.0

## 1. Question

能否把当前已证明的 core semantic subset 转成可独立验证的 IR，同时让不支持的合法语言 construct 明确拒绝，而不是生成看似成功的空壳？

## 2. Method

`lower_core` 先调用完整 M2 analyzer。成功后从 deterministic HIR 收集 function/type signatures，再按 RFC-0009 source order 构建 instructions/blocks。每个完成 module 再经过独立 verifier；任何 unsupported construct 或 verifier error 都不返回 Module。

## 3. Results

```text
STEP_0031_OK core_valid=12 deferred_valid=13 invalid_blocked=29 snapshots=12 evaluation=left-to-right-single-evaluation operations=18 terminators=5 verifier_mutations=3
```

- 12/25 valid：NUM 4、NOM 3、payload-free MATCH 3、CAP pure 1、RESULT same-error try 1；
- 13/25 valid：因 effect/resource/async/component/closure/payload binding 等能力未到阶段而 typed-refuse；
- 29/29 invalid：原主要 E-code 在 IR support dispatch 前拒绝；
- snapshots：12 条 function/block/operation/terminator shape 稳定；
- order probe：`1+2` 完成后才执行 `3+4`，最后按源码字段顺序 construct；
- negative：unknown intrinsic、bad try type、bad match target 均由独立 verifier 拒绝。

## 4. Limits

match arm 暂不接收 payload binding，Result `map_error` closure 和 revision `if` 仍未 lowering。`Try` 固定控制语义但尚未 codegen；STEP-0033 前不声明可执行。RFC-0003/0004 仍未因本步升级。

## 5. Links

- [`STEP-0031`](../steps/STEP-0031-core-lowering-evaluation-order.md)
- [`RFC-0009`](../rfc/RFC-0009-core-lowering-evaluation-order-v0.md)
- [`core snapshots`](../../tests/ir/core-lowering.snap)
