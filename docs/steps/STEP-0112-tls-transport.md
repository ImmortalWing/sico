# STEP-0112: TLS transport and certificate validation

> - status: complete
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Result

Delivered together with STEP-0111 (see its doc §3 for the validator evidence): `crates/sico-http-provider` implements the rustls 0.23 transport foundation — deterministic local CA/leaf fixtures (`deterministic_ca`), a one-shot TLS echo server with a bounded accept loop (`TlsEchoServer`), and a verifying client probe (`tls_probe`) that returns `Connected`/`Refused` with no insecure fallback anywhere in the crate.

Exit evidence (all in `tools/validate-step-0111.ps1`):

- valid chain succeeds through the verifying client (`valid_chain_completes_a_verifying_roundtrip`, echoed byte-exact);
- wrong hostname (IP literal not in SAN) refused; untrusted issuer (wrong CA) refused; malformed CA PEM refused **before any TCP connect** (serverless test, sub-second);
- HTTP v0 loopback corpus stays green (STEP-0089 regression);
- static scan proves no verification-disable path (`danger()` / `with_custom_certificate_verifier` / `danger_accept_invalid_certs` / `InsecureSkipVerify` absent).

## 2. Audit links

- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) §3 TLS trust policy
- [`STEP-0111`](./STEP-0111-secure-http-rfc.md)
