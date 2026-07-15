# STEP-0047: Shared Host core and install/open pipeline

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现 signed-only、copy-by-digest、atomic install 和 reverify-on-open 的 shared Rust Host core。

## 2. Context and evidence

ADR-0004 要求 Desktop adapter 不能复制或绕过 M4 loader。`sico-host-core` 直接依赖 `sico-package` 的 `TrustedPackage`/`AuthorizedPackage` gate。

## 3. Scope

包含 immutable store、metadata、signer/app/revision/capability identity、downgrade refusal、tamper refusal、atomic staging。不包含权限持久记录、process lifecycle 或 shell association。

## 4. Decision

store 使用 hash-only direct children；revision directory 由 staging directory 原子 rename 安装。打开时不信任 metadata 或已安装路径，重新验证 digest、signature trust、identity 与 capability closure。

## 5. Changes

- 新 workspace crate `sico-host-core`；
- `HostStore::install_path/install_bytes/open_installed`；
- canonical `InstalledRevision` 与 `OpenedPackage`；
- app+signer identity、package revision、capability fingerprint；
- Windows reparse/Unix symlink refusal、signed-only persistent install；
- signer separation、downgrade、tamper、unsigned/path integration tests。

## 6. Validation

```text
cargo clippy -p sico-host-core --all-targets -- -D warnings
cargo test -p sico-host-core
STEP_0047_OK crate=sico-host-core install=signed-only,atomic,copy-by-digest open=digest,trust,identity,closure-reverified signer_isolation=pass downgrade=refused tamper=refused host_tests=3 next=STEP-0048
```

## 7. Metrics

3 integration tests；3 stable hashes；2 canonical revision files；0 parser copies；1 shared package authorization implementation。

## 8. Risks and follow-ups

Atomic directory rename is same-volume only, guaranteed by staging beneath revision parent. Numeric dotted versions receive ordering; other legal version strings require explicit caller policy. STEP-0048 must key durable permissions with the emitted app identity and capability fingerprint。

## 9. Audit links

- [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
- [`review report`](../reports/desktop-host-install-open-v0.md)
- [`STEP-0046`](./STEP-0046-desktop-host-threat-lifecycle-contract.md)
