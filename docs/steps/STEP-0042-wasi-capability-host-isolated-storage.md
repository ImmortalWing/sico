# STEP-0042: WASI capability host 与 isolated storage

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

把 `AuthorizedPackage` 的实际 capability set 转成默认关闭的 Wasmtime WASI flags，并建立 stable per-app storage identity/root、path/link/cross-app/quota audit。

## 2. Context and evidence

SEM-083 要求 host grant 不能成为 ambient authority；Windows junction/reparse 与 raw app ID path 都会破坏 app isolation。外部 Wasmtime 的 preopened directory 是 guest filesystem boundary。

## 3. Scope

包含 hashed identity、canonical direct-child root、symlink/reparse/special-file refusal、host-management path resolver、tree/quota audit 与 file/clock/random/network flags。不实现 UI、生产 storage migration 或后台长期 write quota adapter。

## 4. Decision

接受 [`ADR-0003`](../adr/ADR-0003-isolated-storage-wasi-host-v0.md)。trust identity 参与 storage identity；unsigned package 与任一 signed package 分离。

## 5. Changes

- `AppStorage`、`StorageUsage`、`StorageError` 与 `prepare_storage`；
- Windows reparse point/Unix symlink 均拒绝；
- `wasi_arguments` 默认 `-S cli=n`，无 storage/network ambient flags；
- only authorized storage gets one `/data` preopen；network never inherits host and v0 disables UDP/DNS lookup；
- quota/cross-app fixture leaves second root untouched after first root exceeds quota。

## 6. Validation

```text
cargo clippy -p sico-runtime --all-targets --all-features -- -D warnings
cargo test -p sico-runtime
STEP_0042_OK storage_identity=app-plus-trust-hash roots=direct-child path=normalized links=symlink,reparse cross_app=denied quota=audit wasi=explicit-default-off runtime_tests=4 next=STEP-0043
```

Windows test creates a real directory junction inside app storage and verifies audit rejection.

## 7. Metrics

4 Runtime tests；2 app roots；1 real junction fixture；storage quota test 5 bytes used vs 4-byte policy。

## 8. Risks and follow-ups

当前 CLI process adapter 的 persistent quota 在 execution 前后审计，瞬时写入依赖 STEP-0043 time/fuel bound；未来长期 writable Host 必须使用 in-process quota filesystem adapter。该边界不阻塞 M4 scalar package，但禁止据此宣称通用长期 storage host 已生产化。

## 9. Audit links

- [`ADR-0003`](../adr/ADR-0003-isolated-storage-wasi-host-v0.md)
- [`STEP-0041`](./STEP-0041-capability-closure-permission-intersection.md)
