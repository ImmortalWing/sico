# STEP-0046: Desktop Host threat, lifecycle and platform contract

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

在写入 Desktop Host 前冻结 install/open immutable identity、permission record、single-instance lifecycle、UI input 和 Windows/macOS/Linux evidence boundary。

## 2. Context and evidence

M4 GO 提供 verified/authorized/limited `.sapp`。Desktop 新增 shell association、persistent state、permission phishing、TOCTOU、process/window lifecycle 风险。官方平台资料确认三类 association mechanism 不同，必须通过 adapter 隔离。

## 3. Scope

包含 24-case threat matrix、app/revision/capability identity、persistent install trust、state machine、typed UI ceilings、platform evidence labels。不实现 Host code、registry mutation、GUI 或 production identity。

## 4. Decision

接受 [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md) 与 [`RFC-0020`](../rfc/RFC-0020-desktop-ui-permission-contract-v0.md)。稳定 app identity 使用 app ID + trust identity，revision 使用 exact package digest；persistent permission 额外绑定 capability fingerprint。

## 5. Changes

- 24 类 install/open/permission/lifecycle/UI/platform attack fixtures；
- copy-by-digest、reverify-on-open、signed-only persistent install；
- single supervised guest/bounded open event policy；
- typed no-script UI model与 node/depth/text/event ceilings；
- Windows runtime-verified、macOS/Linux contract/compile/runtime 分级证据规则。

## 6. Validation

```text
STEP_0046_OK threats=24 identity=app-plus-trust revision=package-digest permission=capability-fingerprint install=signed-only lifecycle=single-supervised ui=typed-bounded platform_evidence=explicit next=STEP-0047
```

## 7. Metrics

24 threat cases；6 threat classes；7 lifecycle states；7 UI node variants；6 UI/event ceilings；3 platform adapter families。

## 8. Risks and follow-ups

Windows 是当前唯一可实测平台；macOS/Linux 不会因文档或 cfg code 被宣称 runtime pass。development signer 仍不是 production publisher。STEP-0047 必须复用 M4 loader 类型，不能复制 parser。

## 9. Audit links

- [`ADR-0004`](../adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md)
- [`RFC-0020`](../rfc/RFC-0020-desktop-ui-permission-contract-v0.md)
- [`threat matrix`](../../tests/desktop-host/threat-matrix.json)
