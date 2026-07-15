# Report: resources, tasks, and streams v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0026
> - environment: Windows, Rust 1.97.0

## 1. Question

affine-resources、future-task 与 stream 的 12 个 B source 能否由局部静态流状态检查，并使 move/close、await/task scope 与 stream bound 的 6 个非法案例各产生唯一根因？

## 2. Method

从 resource signature 区分 consuming `self` 与 `borrow self`，为 resource binding 跟踪 available/moved/closed。async function call 建立 Future，await 消费一次；spawn 建立 task，task group 内直接返回 spawn 被判为 escape。对 Stream 参数识别 next/collect operation，检查 await 与显式 `limit`。每次状态转移/operation 绑定 HIR ID、slot 和 source range；测试读取 12 个 B source 与权威 map，并检查其他组无 E5xxx。

## 3. Results

```text
STEP_0026_OK valid=6 invalid=6 exact_primary=6 codes=E5001,E5002,E5101,E5102,E5201,E5202 affine_flow=pass structured_tasks=pass stream_bounds=pass stable_facts=pass
```

- 6/6 valid：零诊断；
- 6/6 invalid：各一个 primary diagnostic，完整匹配 catalog map；
- resource：move 转移 binding，consuming method 关闭状态，borrow method 保持可用；
- Future/Task：Future 只 await 一次，spawn 不得直接离开 task group；
- Stream：next 要求 await，collect 要求显式 limit；
- facts：resource/async state 与 stream operation 使用稳定复合 ID 和合法 range；
- 其余 B group 无 E5xxx 假诊断。

## 4. Limits

这是当前 P0 的单函数、结构化局部流分析，不承诺跨函数 alias/borrow 推理、path-sensitive join、runtime destructor、scheduler 顺序或 backpressure。上述能力需要新增判别 case/RFC 后才能扩展。

## 5. Links

- [`STEP-0026`](../steps/STEP-0026-resources-async-streams.md)
- [`sico-semantics`](../../crates/sico-semantics/src/lib.rs)
- [`diagnostic case map`](../../diagnostics/semantic-case-map.json)
