# RFC-0017: Capability closure and permission intersection v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target phase: M4
> - supersedes: -
> - superseded-by: -

## Summary

M4 Runtime 在 host link 前证明 source effect evidence、manifest capability requests 与 top-level Component imports 三方闭合，再要求每项 request 出现在 host/user grant set。额外 host grant 不向应用暴露；unknown import/grant 默认拒绝。

## Stable v0 namespace

| Component import prefix | Capability |
|---|---|
| `wasi:filesystem/`, `sico:storage/` | `storage.read-write` |
| `wasi:clocks/` | `clock.read` |
| `wasi:random/` | `random.read` |
| `wasi:sockets/`, `wasi:http/` | `network.connect` |
| `sico:log/` | `log.write` |

v0 故意使用少量 coarse capability。无法映射的 import 不按字符串猜测，也不使用 Wasmtime unknown-import default/trap；直接拒绝。后续细分 storage read/write 或 network host/port scope 时必须新 RFC 和迁移规则。

## Closure algorithm

1. structural/signature trust 得到 `TrustedPackage`；
2. source effect evidence 必须与 manifest capabilities 完全相等；
3. loader 独立解析 Component top-level imports 并映射，结果必须与 manifest 完全相等；
4. required set 必须是 host grants 的子集，否则 load fails；
5. `AuthorizedPackage.granted_capabilities` 只包含 required set，因此 host 的额外权限不会形成 ambient authority。

M4 compiler only supports effect-free scalar Components, so normal source builds close over the empty set. Imported-component fixtures prove the non-empty gate independently; supporting effectful Sico lowering still requires a compiler adapter and is not implied by package support.

## Failure contract

Unknown capability/import、source-manifest mismatch、import-manifest mismatch 和 host denial 是不同分类。默认消息只包含 stable capability/import identity，不包含宿主路径、账户或 policy internals。

## Links

- [`STEP-0041`](../steps/STEP-0041-capability-closure-permission-intersection.md)
- [`SEM-083`](../../SEMANTICS.md#sem-083权限取交集确认)
- [`RFC-0016`](./RFC-0016-development-signing-trust-v0.md)
