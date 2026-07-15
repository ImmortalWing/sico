# STEP-0048: Permission UI model and durable decision records

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现只消费 verified `OpenedPackage` 的 permission prompt、allow-once/deny/persistent decisions 和 fail-closed durable store。

## 2. Context and evidence

RFC-0020 要求 prompt 显示实际 capability closure，并把持久授权绑定 app+signer identity 与 capability fingerprint；manifest raw text 不可直接变成权限。

## 3. Scope

包含 typed prompt、exact decision set、session-only grants、canonical persistent record、corrupt/unknown/identity drift refusal。不包含平台 dialog renderer。

## 4. Decision

permission record 每个 app identity/capability fingerprint 一个 immutable canonical JSON。缺失返回 `PromptRequired`；损坏或身份漂移返回 error，caller 必须重新提示，不能当作授权。

## 5. Changes

- `PermissionStore`、`PermissionSession`、`PermissionPrompt`；
- `Deny/AllowOnce/AllowPersistent` exact decisions；
- session terminal clear；
- persistent record app ID、signer、creation revision、capability fingerprint closure；
- permission directory link refusal与 atomic create-new；
- persistent/once/deny/corrupt/incomplete decision tests。

## 6. Validation

```text
cargo clippy -p sico-host-core --all-targets -- -D warnings
cargo test -p sico-host-core
STEP_0048_OK prompt=authorized-closure decisions=deny,allow-once,allow-persistent record=canonical,app-signer-capability session=terminal-expiry corrupt=fail-closed permission_tests=3 host_tests=6 next=STEP-0049
```

## 7. Metrics

3 permission tests；3 decision variants；7 prompt identity/capability fields；1 canonical schema；0 raw manifest permission paths。

## 8. Risks and follow-ups

Persistent partial grants are safe but will continue prompting for nonpersistent capabilities. Record replacement/revocation UI is deferred to platform adapter; v0 immutable record mismatch fails closed rather than overwriting silently。

## 9. Audit links

- [`RFC-0020`](../rfc/RFC-0020-desktop-ui-permission-contract-v0.md)
- [`STEP-0047`](./STEP-0047-shared-host-install-open.md)
