# STEP-0117: connection lifecycle, SDK and tooling integration

> - status: complete-core
> - phase: M12
> - started: 2026-09-02
> - completed: -
> - owners: autonomous-agent

## 1. Result

`crates/sico-http-provider/src/engine.rs`: the per-Store HTTP engine composing every policy layer.

- `HttpEngine::execute`: canonicalize → pinned resolution (bounded, IPv4-deterministic on dual-stack) → per-use address gate → exact endpoint-grant check → TLS or plain exchange with strict framing → opt-in redirect chain with cross-origin secret stripping and grant-scoped following. One engine = one Store; `begin_request` enforces the 16 in-flight cap as a typed error; one terminal outcome per request (`Response`/`Failed`).
- Full roundtrip test: granted TLS request to a deterministic local CA server with server-side secret injection (`x-api-key` header filled from the Host store, value never returned to the caller); ungranted endpoint and denied-address requests fail closed with typed reasons; DNS rebinding refusal happens before connect.
- Secret wiring: `SecretStore::injected_values_for` returns wire headers only for bindings authorized on the exact endpoint identity.

Status `complete-core`: the guest-visible WIT surface and standard-library helpers land in the runner integration follow-up; the contract and all policy layers are frozen and proven here.

## 2. Audit links

- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) §7
- [`STEP-0113`](./STEP-0113-endpoint-authority-dns.md), [`STEP-0114`](./STEP-0114-streaming-bodies.md), [`STEP-0115`](./STEP-0115-redirect-policy.md), [`STEP-0116`](./STEP-0116-secret-provider-redaction.md)
