# STEP-0111: secure HTTP/provider/authority RFC

> - status: complete
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Objective

Freeze the M12 contract surface per the M12 plan §6 STEP-0111 deliverables: RFC-0037 (WIT versioning, endpoint identity, TLS trust, DNS policy, redirect matrix, secret model, resource bounds), the provider module boundary with dependency evidence, the strict fixture corpus plan, and the HTTP v0 compatibility story.

## 2. Deliverables and status

1. [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) — draft→accepted in this step.
2. Dependency evidence: rustls 0.23 + rcgen 0.13 + rustls-pemfile 2 fetched and a deterministic local CA→TLS roundtrip executed successfully on Windows x64 (2026-09-02) — the plan's "mature TLS dependency is available" gate is met with recorded licenses.
3. Fixture corpus plan (each fixture lands in the step validators that implement it):
   - authority: non-canonical IPv4 (octal/hex/leading zeros), IPv4-mapped IPv6, zone ids, user-info, fragments, percent-encoding, wildcard grants, oversize URL/headers;
   - TLS: valid chain accept; wrong-hostname/expired/unknown-issuer/self-signed/malformed PEM refuse;
   - DNS: private-range resolution refusal, pinning across answers, bounded attempts;
   - secrets: manifest∩host∩invocation mismatch, cross-origin non-forwarding, redaction canaries;
   - compatibility: STEP-0089 loopback corpus rerun unchanged.
4. Provider boundary: `sico-http-provider` crate (created in STEP-0112), dependencies only rustls/rustls-pemfile/rcgen; module assignment decided then.

## 3. Validation

`tools/validate-step-0111.ps1`: RFC presence/decisions, dependency lock entries, rustls roundtrip probe as a committed test, fixture-plan completeness against the M12 threat list. Actual results recorded at completion.

## 4. Audit links

- [`M12 plan`](../plans/M12-secure-http-automation-sdk.md) §6 STEP-0111
- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md)
