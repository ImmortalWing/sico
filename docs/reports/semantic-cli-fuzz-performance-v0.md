# Report: semantic CLI, deterministic quality and performance v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0029
> - environment: Windows, Rust 1.97.0

## 1. Question

真实 semantic analyzer 能否安全接入 CLI，在完整 P0 oracle 上保持精确诊断，并具备可重复、有限额且不夸大 SLA 的质量证据？

## 2. Method

real-binary integration 逐个执行 54 个 B case，并以 `semantic-case-map.json` 对 29 个 invalid case 的 code/key/message/arguments 做精确比对。property suite 使用固定生成规则构造 2,048 个输入并重复分析，同时单独施加 150 个独立根因、2,000 个 locals 与 200 层表达式。release measurement 对同一 54-file corpus 连续执行三轮，每轮 200 次。

## 3. Results

```text
STEP_0029_OK cli_valid=25 cli_invalid=29 exact_primary=29 text_json=pass semantic_inputs=2048 diagnostic_cap=100 locals=2000 expression_depth=200 perf_runs=3 perf_iterations=200 median_ms=1242.186 median_mib_s=2.808 sla=not-established
```

- CLI：25/25 success，29/29 failure，主要诊断和退出码精确；
- status：syntax error 为 `not-run/false`，semantic success 为 `ok/true`，semantic error 为 `error/true`；
- properties：1,024 accept + 1,024 E2001 reject，重复运行 facts/diagnostics 相等且 range 在 source 内；
- limits：diagnostic 恰好截断到 100；2,000 locals 与 200 层表达式成功、稳定；
- corpus：每轮 10,800 analyses，即 5,000 valid + 5,800 invalid；三轮分类完全一致。

## 4. Performance evidence

| Run | Iterations | Analyses | Elapsed | Throughput |
|---:|---:|---:|---:|---:|
| 1 | 200 | 10,800 | 1227.133 ms | 2.843 MiB/s |
| 2 | 200 | 10,800 | 1316.174 ms | 2.650 MiB/s |
| 3 | 200 | 10,800 | 1242.186 ms | 2.808 MiB/s |
| median | 200 | 10,800 | 1242.186 ms | 2.808 MiB/s |

原始资产见 [`m2-semantics-windows-release.json`](../../tests/performance/m2-semantics-windows-release.json)。该测量只用于同环境数量级回归，没有延迟或吞吐 SLA。

## 5. Limits

这是固定生成器和单机 corpus timing，不等同于 coverage-guided 长期 fuzz、跨平台 benchmark 或形式化证明。CLI 只做 frontend + static semantics；后端和 Runtime 能力仍不存在。

## 6. Links

- [`STEP-0029`](../steps/STEP-0029-semantic-cli-fuzz-m2-exit.md)
- [`M2 exit audit`](./m2-exit-audit.md)
- [`semantic properties`](../../crates/sico-semantics/tests/semantic_properties.rs)
