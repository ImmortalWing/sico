# STEP-0041: capability closure 与 permission intersection

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现 SEM-083 四方权限模型的 package/runtime gate：source、manifest、Component imports 必须闭合，host grant 必须覆盖全部 required capability，额外 grant 不可见。

## 2. Context and evidence

M3 compiler scalar Component imports 为空；M4 仍需在 loader 处理任意不可信 Component 时证明 import closure，不能依赖编译器自证或 Wasmtime unknown-import fallback。

## 3. Scope

定义五个 capability v0、WASI/Sico import prefix mapping、`authorize` 与 `AuthorizedPackage`。不实现具体 WASI adapter；storage/root 和 Wasmtime flags 在 STEP-0042。

## 4. Decision

接受 [`RFC-0017`](../rfc/RFC-0017-capability-closure-permission-v0.md)。三方集合要求完全相等，host grant 要求 superset，最终实际集合为 required set。

## 5. Changes

- loader independently extracts Component imports；
- unknown import/grant、source mismatch、import mismatch、host denial 分类；
- imported Component fixture 证明 non-empty clock capability；
- host 同时 grant clock/random 时，应用只得到 clock。

## 6. Validation

```text
cargo clippy -p sico-package --all-targets --all-features -- -D warnings
cargo test -p sico-package
STEP_0041_OK closure=source-manifest-import host=required-subset actual=intersection capability_names=5 package_tests=8 imported_fixture=clock unknown=default-deny next=STEP-0042
```

## 7. Metrics

5 个 stable capability names、7 个 import prefixes、2 个专项 integration tests；无 ambient grant。

## 8. Risks and follow-ups

coarse storage/network scope 只适合 M4；M5+ 若暴露真实 UI/network/storage API，必须细化 grant data，而不是复用字符串扩大权限。

## 9. Audit links

- [`RFC-0017`](../rfc/RFC-0017-capability-closure-permission-v0.md)
- [`STEP-0040`](./STEP-0040-development-signing-trust-policy.md)
