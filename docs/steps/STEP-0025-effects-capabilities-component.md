# STEP-0025: 实现 effects/capabilities 与 Component boundary semantics

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 effects-capabilities 与 component-call 的 8 个 B case 建立真实边界检查：调用所需 effect、显式 capability 声明、Component version/type 分离，以及 transport failure 与 domain Result 分层。

## 2. Boundaries

- 只激活 E4001、E4002、E6001、E6002；
- effect 与 capability 分开验证，不从一个声明推导另一个；
- Component version 属于 package metadata，不进入 `Component[...]` 类型；
- `call` 结果只能通过 `ComponentCall[T, E]` 暴露，不折叠为 domain `Result[T, E]`；
- 不实现 Component runtime/WIT codegen，不提前实现 resource/async/revision。

## 3. Acceptance

- capability + component 4/4 valid 零诊断；
- 4/4 invalid 各恰好一个 primary diagnostic，完整匹配 catalog map；
- effect/capability/component call facts 有 stable composite ID/range；
- 其余 B group 不产生未拥有的 E4xxx/E6xxx；
- workspace/M1/M0 regression。

## 4. Commit

`feat(semantics): [STEP-0025] check capability and component boundaries`

## 5. Changes and validation

- 提取显式 effect/capability boundary 与实际 receiver/call facts；
- `effects: none` 与 effectful call 冲突检查；不猜测未定义的 method→effect 映射；
- Component version/type 和 transport/domain failure 分层检查；
- 4/4 valid 零诊断；4/4 invalid 各一个精确 primary diagnostic；
- 其余 B group 无未拥有 E4xxx/E6xxx；workspace/M1/M0 regression 通过。

```text
STEP_0025_OK valid=4 invalid=4 exact_primary=4 codes=E4001,E4002,E6001,E6002 capability_boundary=pass explicit_purity=pass component_layers=separate stable_facts=pass
```

## 6. Next

STEP-0026 只实现 resource/task/stream 的 6 valid/6 invalid oracle。
