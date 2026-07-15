# STEP-0024: 实现 match/control flow 与 Option/Result/error mapping

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 exhaustive-match 与 result-mapping 的 14 个 B case 建立真实静态语义：sealed enum/product coverage、重复 arm、Result ok/error、同错误 `try`、显式 `map_error` coverage 与裸 Result 丢弃检查。

## 2. Boundaries

- 只激活 E3001、E3002、E3003、E3101、E3102、E3103、E3104；
- 当前 enum 均按 sealed 处理，wildcard 不可代替显式 coverage；
- `try` 不做隐式 error conversion；`map_error` 必须显式覆盖源 error enum；
- unknown/error type 抑制依赖诊断，重复/coverage 根因只报一次；
- 不提前实现 effect/capability、resource ownership、async/stream 或 revision。

## 3. Acceptance

- match + result 6/6 valid 零诊断；
- 8/8 invalid 各恰好一个 primary diagnostic，code/key/message/arguments 与 catalog map 相同；
- enum variant、match coverage 与 Result flow facts 有 stable composite ID/range；
- 其余 B group 不产生未拥有的 E3xxx 诊断；
- workspace/M1/M0 regression。

## 4. Commit

`feat(semantics): [STEP-0024] check match and result flow`

## 5. Changes and validation

- enum variant/payload model、sealed enum/product coverage 与重复 arm 检查；
- Result ok/error、同 error `try`、total `map_error` 与未处理 Result 检查；
- 6/6 valid 零诊断；8/8 invalid 各一个精确 primary diagnostic；
- 其余 B group 不产生未拥有 E3xxx；workspace/M1/M0 regression 通过。

```text
STEP_0024_OK valid=6 invalid=8 exact_primary=8 codes=E3001,E3002,E3003,E3101,E3102,E3103,E3104 sealed_coverage=pass result_flow=pass stable_facts=pass cascades=bounded
```

## 6. Next

STEP-0025 只实现 capability + component 的 4 valid/4 invalid oracle。
