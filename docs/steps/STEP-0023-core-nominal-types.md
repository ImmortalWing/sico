# STEP-0023: 实现 core/nominal types、fields 与 invariants

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 numbers-units 与 nominal-invariants 的 16 个 B case 建立真实静态语义：module symbol/type table、function signatures/parameters、newtype nominal identity、record fields/constructors、局部 expression inference、return/call compatibility 与可常量判定 invariant。

## 2. Boundaries

- 只激活 E2001、E2002、E2010、E2011、E2020；
- 无 implicit conversion；newtype/record 即使底层/字段相同也不兼容；
- invariant v0 只对当前 P0 可常量求值的整数 `<=` construction 判定，不扩展通用 theorem proving；
- unresolved/unknown type 抑制依赖诊断；不提前实现 match/Result/effect/resource/async/revision。

## 3. Acceptance

- numbers + nominal 7/7 valid zero diagnostics；
- 9/9 invalid 各恰好一个 primary diagnostic，code/key/message/arguments 与 catalog map 相同；
- diagnostic range 在 source 内，facts 有 stable composite ID/type/range；
- 其他 B group 不 panic，未拥有的规则不产生假诊断；
- workspace/M1/M0 regression。

## 4. Commit

`feat(semantics): [STEP-0023] check core and nominal types`

## 5. Changes and validation

- 新增 `sico-semantics` workspace crate，建立 deterministic module model 与 semantic facts；
- 实现 function signature/parameter/local inference、nominal newtype/record relation、field constructor 与常量 invariant；
- 7/7 valid 零诊断；9/9 invalid 各一个精确 primary diagnostic；
- 其余 B group 不产生未拥有的 E2xxx 诊断；workspace/M1/M0 regression 通过。

```text
STEP_0023_OK valid=7 invalid=9 exact_primary=9 codes=E2001,E2002,E2010,E2011,E2020 nominal_identity=pass fields=pass invariant=constant-v0 stable_facts=pass cascades=bounded
```

## 6. Next

STEP-0024 只扩展 match/control flow 与 Option/Result/error mapping 的 6 valid/8 invalid oracle。
