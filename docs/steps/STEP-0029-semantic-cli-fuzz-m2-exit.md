# STEP-0029: 集成 semantic CLI、质量基线并完成 M2 exit audit

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

让 `sico check` 在 text/JSON 两种输出中真实执行静态语义检查，为 semantic analyzer 建立确定性 property/limit/performance 证据，并逐项审计 M2 exit gate。

## 2. Boundaries

- syntax error 阻止 HIR/semantics，JSON 明示 `type_checker=not-run`；
- syntax success 必须运行 analyzer，不能保留 M1 的 `unavailable` 占位；
- semantic diagnostic 的 code/key/message/arguments/range 必须来自 compiler producer；
- 性能数据是当前 Windows/release 可复现基线，不建立跨机器 SLA；
- 不注册或伪实现 Sico IR、Wasm、Component codegen、Runtime、`run`、`build`、`-c` 或 REPL。

## 3. Acceptance

- real-binary CLI 对 25/25 valid 返回成功，对 29/29 invalid 返回精确主要诊断与失败退出码；
- text/JSON status 明确区分 syntax error、semantic success、semantic error；
- 2,048 个确定性输入可重复，100 diagnostic cap、2,000 locals、200 expression depth 有界；
- 54-file corpus × 200 iterations × 3 release runs 留存原始非 SLA 证据；
- STEP-0022–0029、workspace、M1 与 M0 regression 全部通过；
- M2 audit 给出明确 GO/NO-GO，并建立下一阶段计划。

## 4. Commit

`test(semantics): [STEP-0029] integrate CLI and prove M2 exit`

## 5. Changes and validation

- `sico check` 接入 `sico-semantics::analyze`，text/JSON envelope 报告真实 semantic 状态；
- real-binary integration 覆盖完整 54-case B oracle 与 29 个 catalog mapping；
- 新增固定输入 property、诊断上限、大 scope/deep expression tests；
- 新增 release corpus measurement harness 与三轮原始 JSON；
- 新增 M2 逐项退出审计、M3 执行计划和双层 validator。

```text
STEP_0029_OK cli_valid=25 cli_invalid=29 exact_primary=29 text_json=pass semantic_inputs=2048 diagnostic_cap=100 locals=2000 expression_depth=200 perf_runs=3 perf_iterations=200 median_ms=1242.186 median_mib_s=2.808 sla=not-established
M2_EXIT_OK steps=8 hir=54 semantic_valid=25 semantic_invalid=29 exact_primary=29 cli=text-json index_modules=10 queries=5 property_inputs=2048 diagnostic_cap=100 perf_median_ms=1242.186 next=STEP-0030
```

## 6. Next

M2 已完成。按 [`M3 plan`](../plans/M3-sico-ir-component.md) 从 STEP-0030 的 typed Sico IR contract/validator 开始；任何新增 lowering 语义必须先有 RFC/case gate。
