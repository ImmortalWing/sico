# STEP-0035: 审计并冻结 async/task/stream backend

> - status: complete
> - phase: M3
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

满足 RFC-0004 的真实 Runtime gate，明确 Runtime 已支持的 future/stream contract 与 compiler 当前可证明范围；unsupported source/IR 必须专用拒绝，禁止模拟成功。

## 2. Acceptance

- Wasmtime 46.0.1 真实往返并关闭 future/stream readers；
- pending future cancel 被 producer 确认；
- stream capacity 1/5 与 5/5 completion 实测；
- 三个 Component artifacts 重复解析 SHA-256 一致；
- async-flow WIT 由 parser 验证；
- Task/Future/Stream codegen 返回专用 contract refusal；
- resource/task/stream dynamic、compile-fail、workspace/M2/M1/M0 regression。

## 3. Semantic boundary

RFC-0004 的 mapping/close/cancel/backpressure 方向现已接受。compiler 仍未生成 async Component：当前 IR 没有完整 structured task scope、stream bound 与 cancellation CFG，提前 codegen 会丢语义。因此本步实现的是可审计 Runtime contract 和专用 backend refusal，而不是假装完整 async compiler 已存在。

## 4. Commit

`feat(async): [STEP-0035] verify future stream runtime contracts`

## 5. Validation

```text
STEP_0035_OK runtime=wasmtime-46.0.1 artifacts=3 deterministic=sha256 future=roundtrip-close-cancel stream=roundtrip-close-capacity-1-of-5 wit=async-flow-v0 backend_refusals=Task,Future,Stream resource_async_tests=10 compile_fail=2 rfc_0004=accepted
```

## 6. Next

STEP-0036 只把已经真实 codegen/Runtime 通过的同步 scalar Component subset 接入 `sico build/run`；async source 仍返回明确 unavailable，不能注册成功路径。
