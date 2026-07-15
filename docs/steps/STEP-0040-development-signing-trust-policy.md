# STEP-0040: development signing 与 trust policy

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现 deterministic Ed25519 development signing、strict verification 和显式 trust policy，使 valid signature、trusted identity 与 unsigned dev 三种状态不被混淆。

## 2. Context and evidence

RFC-0015 已冻结签名 input domain。仅验证 archive/hash 不能证明来源；仅验证任意 public key 也不能代表本机授权。

## 3. Scope

加入 signature record、sign/verify API、weak-key/strict verification、trusted key set、unsigned explicit opt-in 与 replay fixtures。不实现生产 publisher、证书、撤销、registry 或 key persistence。

## 4. Decision

接受 [`RFC-0016`](../rfc/RFC-0016-development-signing-trust-v0.md)。`VerifiedPackage` 与 `TrustedPackage` 是不同类型；Runtime 后续只接受后者。

## 5. Changes

- pinned `ed25519-dalek` 3.0.0；
- `sign_development` 不接触文件系统，仅消费 caller-owned 32-byte seed；
- `verify_trusted` 严格检查 canonical record、scheme、weak key、signature 和 trusted key set；
- signature stripping/wrong-key/replay 返回独立 package errors。

## 6. Validation

```text
cargo clippy -p sico-package --all-targets --all-features -- -D warnings
cargo test -p sico-package
STEP_0040_OK scheme=ed25519-dev-v0 domain=separated strict_verify=pass package_tests=6 fixtures=valid,wrong-key,unsigned,replay trust_type=TrustedPackage next=STEP-0041
```

## 7. Metrics

6 package integration tests，其中 2 个 signing/trust 专项；无生产密钥或外部信任数据。

## 8. Risks and follow-ups

development key 的安全存储由调用方负责；CLI 在 STEP-0044 只接受显式 key file，不生成或提交默认私钥。生产身份仍是 M7 gate。

## 9. Audit links

- [`RFC-0016`](../rfc/RFC-0016-development-signing-trust-v0.md)
- [`STEP-0039`](./STEP-0039-deterministic-sapp-builder-loader-inspect.md)
