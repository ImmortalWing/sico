# STEP-0032: 实现 effect、resource 与 revision IR flow

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

把 M2 已验证的 capability/effect calls、affine resource close/borrow/using cleanup 与 revision equality guard lowering 到 typed IR，并让 verifier 独立证明关键 flow invariants。

## 2. Boundaries

- capability token 和 effect identity 显式记录，不扩大声明 effect set；
- owned/borrowed resource 具有不同 type identity，所有 consume/drop edge 显式；
- v0 verifier 证明当前 linear resource blocks；跨 branch owned flow 仍须显式 block parameter 后才支持；
- revision check 只产生 branch guard，不增加 hidden retry/runtime conflict handling；
- Component boundary 与 async/task/stream 仍 typed-refuse。

## 3. Acceptance

- CAP-001、RES-001/002、REV-001/002 共 5 个 flow cases deterministic lowering；
- effect call 使用 declared semantic effect；resource borrow/call/drop 顺序明确；revision check 支配 branch；
- 累计 17/25 valid lower，剩余 8 个合法 component/async cases typed-refuse；
- undeclared effect、resource leak/double consume、unused revision guard mutation 被拒绝；
- 29/29 invalid 仍在 IR 前拒绝；
- workspace/M2/M1/M0 regression。

## 4. Commit

`feat(ir): [STEP-0032] verify effect resource and revision flow`

## 5. Changes and validation

- IR 新增 `Capability` type 与 `resource-call` operation；
- lowering 建立 capability/resource method tables，区分 declared effect identity 与 method spelling；
- `using` 固定为 result evaluation → resource drop → return；
- revision equality 生成 `revision-check` 与显式 then/fallback blocks；
- verifier 重建 owned live/consumed state，并检查 effect declaration 与 revision branch use。

```text
STEP_0032_OK cumulative_valid=17 deferred_valid=8 flow_valid=5 snapshots=5 effects=2 resources=2 revisions=2 ownership=affine borrow=call-scoped verifier_mutations=4 invalid_blocked=29
```

## 6. Next

STEP-0033 只接受 verified IR，生成 deterministic Core Wasm，并用真实 validator/engine 执行受支持 numeric/control programs。
