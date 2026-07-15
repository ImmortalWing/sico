# STEP-0038: `.sapp` threat model 与 package contract

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

在实现 loader 前固定 `.sapp` v0 的攻击面、canonical bytes、manifest、hash/signature domain、版本拒绝和硬 parser limits，并建立可由后续实现逐项执行的恶意 package matrix。

## 2. Context and evidence

M3 只交付 raw deterministic Component；`DIRECTION.md`、`DEVELOPMENT.md` 与 SEM-083 要求 `.sapp` 将 Component、manifest、权限、资源和身份分层，默认拒绝未知/额外能力。ZIP/tar 的额外 canonicalization 面对 v0 没有收益。

## 3. Scope

包含 archive framing、路径规则、canonical JSON、hash/signature input、版本/unknown rejection、大小/数量/深度上限和 18 项攻击 fixture。生产发布者身份、registry、撤销和更新不在 M4。

## 4. Options and decision

比较 ZIP、tar、canonical CBOR 和固定 framing；接受无压缩 fixed framing + canonical JSON。它不复制 Component ABI，只解决应用级封装，并使每个 byte 都有唯一来源。签名选择 Ed25519 development domain，生产 trust 延后 M7。

## 5. Changes

- 接受 [`RFC-0015`](../rfc/RFC-0015-sapp-package-format-v0.md)；
- 新增 [`threat matrix`](../../tests/packages/threat-matrix.json)；
- 新增 STEP-0038 validator，冻结文档和 corpus 数量/唯一性；
- STATUS 进入 M4 in-progress，下一步 STEP-0039。

## 6. Validation

```text
STEP_0038_OK format=sapp-v0 archive=fixed-uncompressed manifest=canonical-json threat_cases=18 limits=6 hash=sha256 signature_domain=ed25519-dev-v0 next=STEP-0039
```

本步骤不执行任意不可信包；所有 matrix 必须在 STEP-0039/0040/0041 的真实 parser/verification tests 中获得拒绝证据。

## 7. Metrics

18 个 threat cases；6 类 hard parser ceiling；性能无数据，不建立 SLA。

## 8. Risks and follow-ups

fixed framing 不是通用 archive，第三方工具支持有限；`sico inspect` 是正式检查入口。生产签名、更新和撤销仍属于 M7，不能从 development key 推导发布者信任。

## 9. Audit links

- [`M4 plan`](../plans/M4-sapp-runtime.md)
- [`RFC-0015`](../rfc/RFC-0015-sapp-package-format-v0.md)
