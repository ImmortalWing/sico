# STEP-0116: Host secret provider and redaction

> - status: complete
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Result

`crates/sico-http-provider/src/secrets.rs`: Host-owned opaque secret registry.

- `SecretBinding { name, endpoint, policy }`: the exact `secret.use:<name> + endpoint + injection-policy` triple; `resolve` refuses unknown names, wrong endpoints, mismatched policies and missing values with stable typed errors (`UnknownSecret`/`WrongEndpoint`/`WrongPolicy`/`MissingValue`); `manifest_intersects` enforces the exact package∩Host∩invocation intersection.
- Injection policies (header-only per RFC-0037 §6): `Authorization: Bearer`, `Authorization: Basic` (dependency-free base64), and exact-name custom headers.
- `redact`: deterministic fingerprint redaction covering the raw value, its `Bearer` and `Basic`-encoded forms — the canary test proves secret bytes never survive any redacted output path.
- 33/33 provider tests green including the intersection-mismatch and canary corpora.

## 2. Audit links

- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) §6
