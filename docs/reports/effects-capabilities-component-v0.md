# Report: effects, capabilities, and Component boundaries v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0025
> - environment: Windows, Rust 1.97.0

## 1. Question

effects-capabilities 与 component-call 的 8 个 B source 能否以显式边界契约检查，并保持 capability、effect、Component transport failure 与 domain Result 相互分离？

## 2. Method

从 function HIR 提取 effects/capabilities section 和实际 method/component call；receiver 必须出现在 capability boundary。对明确 `effects: none` 的 effectful call 产生 E4002；非空 effect 名与 capability method 的通用映射因无 contract/case 不猜测。`Component[Interface@version]` 在 type range 拒绝，`call` 返回边界必须声明 `ComponentCall[T, E]`，不得直接折叠为 `Result[T, E]`。测试读取 8 个 B source 与权威 map，并确认其他 B group 无 E4xxx/E6xxx 假诊断。

## 3. Results

```text
STEP_0025_OK valid=4 invalid=4 exact_primary=4 codes=E4001,E4002,E6001,E6002 capability_boundary=pass explicit_purity=pass component_layers=separate stable_facts=pass
```

- 4/4 valid：零诊断；
- 4/4 invalid：各一个 primary diagnostic，完整匹配 catalog map；
- capability：实际 receiver 与边界声明分开核对；
- effect：明确 pure boundary 不允许 effectful call；
- Component：version/type 分离，transport/domain failure 分层；
- facts：effect、capability 与 component call 使用稳定 `(HIR ID, slot)` 和合法 range；
- revision 等其他组未因 method 名与抽象 effect 名不同而收到假 E4002。

## 4. Limits

当前资料没有声明通用 capability method→effect 名映射，因此本步骤不推断 `commit` 是否对应 `write`；新增此类检查必须先补 contract/case。M2 不执行 WIT codegen、component linking、trap handling 或 runtime call。

## 5. Links

- [`STEP-0025`](../steps/STEP-0025-effects-capabilities-component.md)
- [`sico-semantics`](../../crates/sico-semantics/src/lib.rs)
- [`diagnostic case map`](../../diagnostics/semantic-case-map.json)
