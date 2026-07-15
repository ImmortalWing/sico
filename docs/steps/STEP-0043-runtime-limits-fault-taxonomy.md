# STEP-0043: Runtime limits and fault taxonomy

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

在真实 Wasmtime execution 施加 manifest/host 双重 ceiling，冻结七类 fault，并证明恶意 guest 不能拖垮后续健康执行。

## 2. Context and evidence

STEP-0042 已把授权能力映射为默认关闭的 WASI flags；本步骤补齐 M4 execution resource boundary。选定 Runtime 是仓库固定的 Wasmtime 46.0.1 CLI。

## 3. Scope

包含 fuel/time/memory/table/instance/WASI resource/hostcall/random/body/storage effective limits、fault classification、temporary cleanup、无限循环 fixture 与 host survival。不包含 Desktop lifecycle、in-process scheduler 或生产级长期 writable quota adapter。

## 4. Decision

接受 [`RFC-0018`](../rfc/RFC-0018-runtime-limits-fault-taxonomy-v0.md)。每项 package request 只能收紧 host ceiling；guest fault 作为输出，Runtime setup/storage/launch failure 作为 `HostFatal` error。

## 5. Changes

- 新增 `HostLimits`、`EffectiveLimits`、`FaultClass`、`PackageRuntimeOutput/Error`；
- `run_authorized_package` 传入真实 Wasmtime fuel/timeout/memory/table/instance/WASI limits；
- Component temporary artifact 在 success、trap 与 launch failure 后清理；
- 无限循环 Component 在真实 Runtime 被限制，之后健康 Component 仍成功；
- 修复 temporary cleanup 测试的共享目录并发误判，改为验证精确路径。

## 6. Validation

```text
cargo clippy -p sico-runtime --all-targets --all-features -- -D warnings
SICO_TEST_WASMTIME=<wasmtime-46.0.1> cargo test -p sico-runtime
STEP_0043_OK runtime=wasmtime-46.0.1 limits=fuel,time,memory,table,instance,wasi-resource,hostcall,random,body fault_classes=7 malicious=loop-fuel host_survival=pass runtime_tests=6 next=STEP-0044
```

## 7. Metrics

6 Runtime tests；11 effective limit dimensions；7 fault classes；1 malicious infinite-loop Component；1 post-fault healthy Component。

## 8. Risks and follow-ups

fault mapping 依赖固定 Wasmtime CLI 的 stderr vocabulary，Runtime upgrade 必须重跑 corpus。v0 scalar process adapter 没有 lifecycle cancellation source；`Cancelled` 保留给 M5 host lifecycle，不能伪报为已实现异步取消。

## 9. Audit links

- [`RFC-0018`](../rfc/RFC-0018-runtime-limits-fault-taxonomy-v0.md)
- [`STEP-0042`](./STEP-0042-wasi-capability-host-isolated-storage.md)
