# STEP-0037: M3 确定性、质量基线与退出审计

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

用独立 property、limit、Runtime corpus、非 SLA 性能与全阶段 regression 证据逐项审计 M3 exit gate；只有所有门槛通过才给出 GO，并建立 M4 执行计划。

## 2. Scope and boundaries

- 覆盖 source→semantics→IR→Component 的 2,048 个确定性 scalar 输入、1,000-function large Component、IR verifier mutation 与 diagnostic cap；
- 用锁定 Wasmtime 46.0.1 执行 Int/Bool/Unit 三个 compiler-produced Components；
- 留存 Windows release 三轮 compile pipeline 基线，但不建立跨机器 SLA；
- 重跑 M0–M2 与 STEP-0030–0036 validators；
- 不以本步 fuzz 证据扩张 aggregate、arbitrary Int、effect/resource compiler adapter 或 async compiler 支持边界；
- 不实现 `.sapp`、签名、sandbox、storage 或 capability host，这些在 M3 GO 后进入 M4。

## 3. Validation plan

1. strict format/Clippy/workspace tests；
2. deterministic property、large Component 和 malformed IR refusal；
3. file/stdin build、failure-no-artifact、三类 scalar Runtime corpus 与临时文件清理；
4. release measurement 3 runs，提交原始 JSON；
5. 执行 STEP-0030–0037、M2、M1、M0 validators；
6. 逐项写 M3 exit audit，更新 STATUS/ROADMAP/索引并建立 M4 plan。

## 4. Exit condition

[`M3 exit audit`](../reports/m3-exit-audit.md) 已给出精确 GO。专项 workspace format/strict Clippy/tests、2,048-source property、1,000-function limit、三类 Runtime corpus、三轮性能与前序阶段 validators 全部通过。

```text
STEP_0037_OK scalar_inputs=2048 large_functions=1000 verifier_mutations=13 diagnostic_cap=100 runtime_corpus=3 perf_runs=3 perf_builds=3000 median_ms=51.055 median_mib_s=3.213 sla=not-established audit=GO next=STEP-0038
```

## 5. Links

- [`M3 plan`](../plans/M3-sico-ir-component.md)
- [`STEP-0036`](./STEP-0036-minimal-end-to-end-cli.md)
- [`RFC-0014`](../rfc/RFC-0014-minimal-build-run-cli-v0.md)
