# Report: M2 exit audit

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0029
> - conclusion: GO to M3 Sico IR and Component

## 1. Decision

M2 static semantics 的全部退出门槛已满足，允许进入 M3。该 `GO` 只证明 B frontend、HIR/name/type/control/effect/resource/revision semantics、semantic CLI 与 index/query；它不是 Sico IR、Wasm、Component codegen、Runtime、`run` 或 REPL 已实现的声明。

## 2. Evidence standard

- `proven`：当前 compiler producer、测试、golden、validator 与真实命令直接证明；
- `measured, non-SLA`：当前 Windows/release corpus timing，可复现但不承诺跨机器阈值；
- `deferred by phase`：明确属于 M3 及以后，不是 M2 缺口；
- `missing/contradicted`：任一必需项落入此类则不得退出。

## 3. Requirement-by-requirement audit

| ID | M2 requirement | Strong evidence | Result |
|---|---|---|---|
| M2-01 | full B HIR、stable IDs/ranges、error tree blocked | STEP-0022；54/54 lowering；name/prelude contract | proven |
| M2-02 | core/nominal types、fields、local inference、invariants | STEP-0023；7 valid + 9 exact invalid | proven |
| M2-03 | match/control flow、Option/Result/error mapping | STEP-0024；6 valid + 8 exact invalid | proven |
| M2-04 | effects/capabilities 与 Component boundary semantics | STEP-0025；4 valid + 4 exact invalid | proven |
| M2-05 | affine resources 与 Future/Task/Stream static checks | STEP-0026；6 valid + 6 exact invalid | proven |
| M2-06 | revision/contract dataflow | STEP-0027；2 valid + 2 exact invalid | proven |
| M2-07 | 全 P0 oracle 在 backend 前分类，主要诊断精确且 cascade 有界 | `sico-semantics` + CLI integration；25/25 valid，29/29 invalid | proven |
| M2-08 | name/type/control/effect/resource facts 共享 stable ID/range | STEP-0022–0028 analyzer facts/index tests | proven |
| M2-09 | compiler-produced Semantic Index/query v0 与诚实 completeness/blocking | STEP-0028；10 complete B + 10 partial A + E2001 blocked；5 queries | proven |
| M2-10 | CLI text/JSON 真实执行 semantics，不再伪装 syntax-only success | STEP-0029 real-binary tests；`ok/error/not-run` status | proven |
| M2-11 | deterministic property/limit/corpus performance | 2,048 inputs；cap 100；2,000 locals；depth 200；3 release runs | proven / measured, non-SLA |
| M2-12 | 未实现后端不伪成功 | 无 IR/codegen/runtime crate 或 CLI command；计划明确移交 M3 | proven boundary |
| M2-13 | workspace、STEP-0022–0029、M1、M0 regression | strict fmt/Clippy/test 与全部 validators | proven |

没有 M2 必需项为 `missing` 或 `contradicted`。

## 4. Oracle and diagnostic evidence

完整 B oracle 为 25 valid + 29 invalid。所有 invalid 都在 backend 前由真实 analyzer 拒绝，主要 code/key/message/arguments 与 catalog map 一致；diagnostic source range 落在原 source 内。unknown/error facts 抑制依赖噪声，独立根因仍受 100 条全局上限约束。

## 5. CLI and index evidence

`sico check` 在 syntax success 后一定运行 semantics；text 与 RFC-0001-like JSON 都报告 compiler-produced diagnostic。syntax error 不进入 HIR/analyzer，JSON 报 `type_checker=not-run`。Semantic Index 由 analyzer facts 生成 production SHA-256 snapshot；invalid module 携带 blocking E-code，候选 A examples 诚实标为 partial。

## 6. Property, limit and performance evidence

```text
STEP_0029_OK cli_valid=25 cli_invalid=29 exact_primary=29 text_json=pass semantic_inputs=2048 diagnostic_cap=100 locals=2000 expression_depth=200 perf_runs=3 perf_iterations=200 median_ms=1242.186 median_mib_s=2.808 sla=not-established
```

三轮 release corpus median 为 1242.186 ms / 2.808 MiB/s。原始数据见 [`performance JSON`](../../tests/performance/m2-semantics-windows-release.json)；它只建立当前环境回归基线，不建立 SLA。

## 7. Deferred register

| Item | Current truth | Owner/gate |
|---|---|---|
| typed Sico IR 与 verifier | 未实现 | M3 STEP-0030 起 |
| semantics→IR lowering、Core Wasm、Component/WIT codegen | 未实现 | M3，逐步验证 |
| Runtime 执行、`run`/`build` 与源码直跑 | 未实现 | M3/M4 |
| `.sapp`、cache、signature、sandbox | 未实现 | M4 |
| `-c`、REPL、Desktop/Android host | 未实现 | M4–M6 |
| real AI、跨平台 benchmark、coverage-guided fuzz | 无新增外部证据 | 后续质量/授权 gate |

## 8. M3 authorization

进入 M3 只授权按 [`M3 plan`](../plans/M3-sico-ir-component.md) 建立 IR contract、lowering、codegen 与最小真实链路。不授权为实现方便改变 M2 semantic oracle；任何无法由现有事实唯一决定的 ABI、evaluation order、representation 或 runtime behavior 必须先补 RFC/case。

## 9. Final conclusion

`GO: M2 complete; M3 entry gate satisfied; next STEP-0030.`
