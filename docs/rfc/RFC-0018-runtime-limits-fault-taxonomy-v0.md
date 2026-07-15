# RFC-0018: Runtime limits and fault taxonomy v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target phase: M4
> - supersedes: -
> - superseded-by: -

## Summary

每次 `.sapp` 执行使用 manifest request 与 host policy 的逐项最小值，并在真实 Wasmtime process 上施加 CPU、时间、memory/table/instance 与 WASI resource/body 限额。guest failure 与 host-fatal setup failure 分通道返回，恶意 guest 结束后 host 必须仍可执行健康包。

## Effective limits

`EffectiveLimits = min(manifest.limits, HostLimits)`，manifest 只能主动收紧 host ceiling，不能扩大权限或资源。v0 固定以下维度：

- `fuel`、`timeout_ms`；
- `memory_bytes`、`table_elements`、`instances`、`tables`、`memories`；
- `wasi_resources`、`hostcall_fuel`、`random_bytes`、`body_bytes`；
- `storage_bytes` 始终来自 host policy，并由 RFC-0017/ADR-0003 的 storage adapter 审计。

Wasmtime invocation 必须关闭 codegen cache，显式传递上述支持的 ceiling，并使用 `trap-on-grow-failure=y`。不接受 package 携带的 precompiled artifact，也不复用 guest process。

## Fault taxonomy

| Class | Source | Channel |
|---|---|---|
| `DomainError` | Sico/WIT 可预期业务错误 | typed result / CLI mapping |
| `CapabilityDenied` | trust/capability closure 或 host grant 拒绝 | load refusal before execution |
| `Cancelled` | caller lifecycle cancellation | host lifecycle result |
| `Timeout` | Wasmtime time/interrupt/epoch exhaustion | guest execution output |
| `ResourceLimit` | fuel、memory/table/resource/grow ceiling | guest execution output |
| `Trap` | 其他 guest trap | guest execution output |
| `HostFatal` | temporary artifact、Runtime launch、storage adapter failure | host error |

v0 scalar process adapter 能直接观察 `Timeout`、`ResourceLimit`、`Trap` 与 `HostFatal`。前三个 lifecycle/domain class 由 package authorization、typed result 和后续 host lifecycle 层产生；不得把它们折叠成 guest trap。stderr 只用于兼容所选 Wasmtime 46.0.1 CLI 的 fault classification，不成为稳定用户消息。

## Host survival and cleanup

每次执行使用 process-unique temporary Component，返回前删除。无限循环 fixture 必须在 fuel/time ceiling 下非零结束；紧随其后的健康 Component 必须成功。launch failure 也必须清理其精确 temporary path，测试不得通过扫描共享临时目录产生并发竞态。

## Non-goals

v0 不承诺跨 Wasmtime 大版本的 stderr 文本稳定性，不实现 in-process epoch callback、长期 writable quota filesystem、task scheduler 或 Desktop lifecycle。升级 Runtime CLI 必须重跑 fault corpus，并在分类漂移时修订本 RFC。

## Validation

- strict Clippy 和全部 `sico-runtime` tests；
- 真实 Wasmtime 46.0.1 infinite-loop fuel/time termination；
- malicious execution 后 healthy execution；
- manifest limit 小于/大于 host ceiling 的逐项最小值断言；
- launch failure 精确 cleanup 与 storage pre/post audit。

## Links

- [`STEP-0043`](../steps/STEP-0043-runtime-limits-fault-taxonomy.md)
- [`RFC-0017`](./RFC-0017-capability-closure-permission-v0.md)
- [`ADR-0003`](../adr/ADR-0003-isolated-storage-wasi-host-v0.md)
