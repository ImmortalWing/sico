# RFC-0016: Development signing and trust policy v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - target phase: M4
> - supersedes: -
> - superseded-by: -

## Summary

`.sapp` v0 development signing uses Ed25519 over RFC-0015's domain-separated canonical unsigned archive. Structural verification produces `VerifiedPackage`; execution requires a distinct `TrustedPackage`, preventing callers from treating valid bytes as trusted identity.

## Contract

- message is `SICO-SAPP-DEV-SIGNATURE-V0\0 || canonical unsigned archive`;
- `signature.json` is strict canonical JSON with schema, `ed25519-dev-v0`, 32-byte public key and 64-byte signature as lowercase hex;
- verifier rejects weak keys and uses Ed25519 strict verification;
- default execution policy requires an exact locally trusted public key;
- unsigned packages are accepted only by explicit `AllowUnsignedDevelopment` policy and remain labelled `UnsignedDevelopment`;
- private 32-byte signing seed is caller-owned, never serialized, logged or accepted from manifest/resource content;
- app id, version, Component/resource hashes, capabilities and limits are inside the signed archive, preventing cross-package replay.

## Trust boundary

A development signature proves possession of local key material only. It is not a production publisher identity, certificate, transparency record, revocation decision, update authorization or platform trust. M4 does not persist private keys or infer trust from display name. Production identity and update policy remain M7.

## Rejected alternatives

- shared repository key: rejects, because public source would make every package appear equally signed;
- HMAC: rejects, because verification would require distributing the signing secret;
- signature embedded in manifest: rejects circular canonicalization;
- accepting any valid Ed25519 key by default: rejects cryptographic validity without local authorization.

## Evidence

Tests cover deterministic signatures, valid trusted key, wrong trusted key, explicit unsigned development, stripping, and cross-version replay. Ed25519 implementation is pinned and `verify_strict` is used; dependency documentation: <https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/struct.VerifyingKey.html>.

## Links

- [`STEP-0040`](../steps/STEP-0040-development-signing-trust-policy.md)
- [`RFC-0015`](./RFC-0015-sapp-package-format-v0.md)
