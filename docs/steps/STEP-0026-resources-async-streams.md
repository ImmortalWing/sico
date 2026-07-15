# STEP-0026: 实现 affine resources 与 Future/Task/Stream checks

> - status: complete
> - phase: M2
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

为 affine-resources、future-task 与 stream 的 12 个 B case 建立局部静态流状态：resource move/close、Future 单次 await、task group escape、Stream await 与 bounded collect。

## 2. Boundaries

- 只激活 E5001、E5002、E5101、E5102、E5201、E5202；
- resource `self` method 消耗，`borrow self` method 不消耗；move 转移唯一可用绑定；
- Future/Task await 只消费一次；spawn task 不得离开其 task group；
- `stream.next` 必须 await，`stream.collect` 必须显式提供 limit；
- 不实现 runtime cleanup、scheduler/backpressure，不提前实现 revision。

## 3. Acceptance

- resource/task/stream 6/6 valid 零诊断；
- 6/6 invalid 各恰好一个 primary diagnostic，完整匹配 catalog map；
- flow state/operation facts 有 stable composite ID/range；
- 其余 B group 不产生未拥有 E5xxx；
- workspace/M1/M0 regression。

## 4. Commit

`feat(semantics): [STEP-0026] check resource and async flow`

## 5. Changes and validation

- resource signature model 与 available/moved/closed binding flow；
- async function/Future/Task 单次消费与 task group escape 检查；
- Stream next await 与 collect limit 检查；
- 6/6 valid 零诊断；6/6 invalid 各一个精确 primary diagnostic；
- 其余 B group 无未拥有 E5xxx；workspace/M1/M0 regression 通过。

```text
STEP_0026_OK valid=6 invalid=6 exact_primary=6 codes=E5001,E5002,E5101,E5102,E5201,E5202 affine_flow=pass structured_tasks=pass stream_bounds=pass stable_facts=pass
```

## 6. Next

STEP-0027 只实现 revision 的 2 valid/2 invalid oracle。
