# RFC-0037: secure HTTP provider and authority contract v0

> - status: accepted
> - date: 2026-09-02
> - phase: M12 STEP-0111
> - supersedes: nothing (extends `sico:script/http@0.1.0` compatibly)

## Summary

Freeze the M12 secure-HTTP contract: a new versioned provider identity `sico:script/http@0.2.0` that keeps the buffered `0.1.0` surface as a bounded compatibility profile and adds TLS, canonical endpoint authority (`scheme + canonical host + effective port`), IPv6/IDNA/DNS-pinning policy, an explicit redirect matrix, Host-owned opaque secrets, and affine streaming body resources. All transport work lives in a Host-provider module behind rustls; the compiler only sees versioned WIT imports. Failure of any policy layer is a structured typed error through the M10 event/fault model.

## 1. WIT and provider versioning

- New identity: `sico:script/http@0.2.0`. Compatibility evidence: every `0.1.0` buffered request can be expressed as a `0.2.0` one-shot request; `0.1.0` grants remain valid; buffered and streaming consumption of one body never mix.
- One affine `request-body` writer output resource and one affine `response-body` input resource; explicit `finish`/`close`/drop with EOF-vs-error distinctions.
- Structured error taxonomy: `permission` (endpoint/secret grants), `authority` (URL canonicalization), `tls` (chain/hostname/validity), `dns` (resolution/pinning/private-address), `limit` (bytes/headers/time/in-flight), `cancel`, `protocol` (framing), `io`.
- Cancellation remains runner-owned (M10 token → provider phases); guests never forge tokens.
- Provider module: new crate `sico-http-provider` under the runner boundary; compiler crates gain no TLS/DNS/credential dependency.

## 2. Endpoint authority

Grant identity: `scheme + canonical host + effective port`, canonicalized once by a shared strict parser (used identically by policy and transport):

- omitted ports canonicalize to scheme defaults (443/80) before comparison;
- DNS-name grants do not cover literal-IP URLs and vice versa; an IPv6 literal is distinct from a DNS name resolving to it;
- user-info, fragments, non-canonical IPv4 forms (decimal/octal/hex, leading zeros), IPv4-mapped IPv6, zone/scope ids, upper-case or percent-encoded hosts are refused or canonicalized exactly once — never both;
- no wildcard grants in v1; path-prefix restriction may narrow but never replace endpoint identity;
- URL ≤ 8 KiB; headers ≤ 256 count, name ≤ 256 B, value ≤ 8 KiB, serialized ≤ 64 KiB.

## 3. TLS trust policy

- Transport: **rustls 0.23** (ring provider, TLS 1.2+ only, no legacy downgrade), chosen over native-tls for: pure-Rust auditability, no OS-API variance across the M12 platform matrix, MIT/Apache-2.0 licensing, and the probe result in §8. `rustls-pemfile` parses PEM fixtures; **rcgen 0.13** builds deterministic local CA/leaf fixtures. No custom cryptography.
- Trust modes: `system-roots` (default) and `pinned-roots` (explicit Host-provided root set, bound to exact endpoints). Certificate verification cannot be disabled by guest code or packages; no insecure mode exists in release builds.
- SNI and hostname verification use the same canonical host identity as the grant.
- Failures (invalid chain, expired/not-yet-valid, wrong hostname, unknown issuer, malformed handshake) are structured `tls` errors; revocation checking is reported explicitly as unsupported in v1 rather than silently claimed.

## 4. DNS and address policy

- Bounded resolution (≤ 8 addresses, ≤ 4 connect attempts); the chosen IP is pinned for the socket's lifetime; redirects and pool reuse re-run policy.
- Private/special ranges (loopback, link-local, multicast, unspecified, documentation, CGNAT, RFC1918) are denied for DNS hostnames by default; literal private/loopback endpoints need an explicit development grant (`http+private://` scheme spelling in grants only).
- DNS answers resolving into denied ranges are refused before connect (rebinding defense).

## 5. Redirect policy

- Default: no automatic redirect (M9 behavior preserved).
- Opt-in automatic: ≤ 5 hops; same-origin requires the same grant; cross-origin requires its own grant and reauthorization.
- `Authorization` and all secret-derived headers are stripped on any origin change; HTTPS→HTTP downgrade refused; method rewriting follows the frozen 301/302/303→GET (unless 307/308 preserve), 307/308→replay-with-body matrix; loops are structured errors. The chain shares one total deadline and one cancellation token.

## 6. Secret model

- Scripts reference opaque names (`secret.use:<name>` + exact endpoint + injection policy); values live only in the Host; v1 injection is header-only (`authorization` / `x-api-key` templates with exact-name substitution).
- Package manifest ∩ Host grants ∩ invocation must agree exactly; mismatch is `permission`.
- Values never enter debug maps, events, cache keys, argv, faults or tooling output; the accepted M10 redaction contract (fingerprinting) covers all paths; redirects never forward secret-derived material across origin.

## 7. Streaming and bounds

- Chunk ≤ 64 KiB; queue depth 1 per direction (backpressure by blocking); default body budget 64 MiB per direction; Host ceiling 1 GiB; ≤ 16 in-flight requests, ≤ 32 pooled connections, ≤ 8 per endpoint per Store.
- Content-Length/chunked parsed strictly; duplicate/conflicting lengths, malformed chunk sizes, trailers abuse and premature EOF are structured `protocol` errors; connection-close framing is bounded by the response budget; automatic decompression is off in v1.
- Deadlines: connect/TLS/first-byte/total/idle all explicit and bounded; pooling never crosses a Store or generation.

## 8. Dependency evidence (STEP-0111 gate)

Executed and kept as committed tests (2026-09-02, Windows x64, Rust 1.98.0, `sico-http-provider` crate): a `rcgen`-generated deterministic CA issued a `localhost` leaf; a rustls server and a verifying client complete a TLS echo roundtrip (`valid_chain_completes_a_verifying_roundtrip`), and wrong-hostname, untrusted-issuer and malformed-PEM requests are refused (`Refused`), the last before any TCP connect. First probe output: `TLS_ROUNDTRIP_OK reply=ping`. Licenses: rustls MIT/Apache-2.0/ISC, ring ISC/OpenSSL-derived (recorded), rcgen MIT/Apache-2.0, rustls-pemfile MIT/Apache-2.0. Supply-chain: crates pinned by `Cargo.lock`; no `unsafe`-custom crypto introduced.

## 9. Compatibility and platform plan

- `http@0.1.0` buffered clients keep working unchanged; STEP-0112 fixtures rerun the STEP-0089 loopback corpus.
- Platform evidence: Windows x64 first; the same local deterministic TLS fixture corpus reruns on Linux x64 (WSL2 environment available since STEP-0109) before any Linux claim.
- Threat model follows M12 plan §7 verbatim; every listed attack gets a fixture in the step validators.

## 10. Non-goals

As the M12 plan §3.2: no sockets/servers, no ambient credentials, no cookies/JS/browser semantics, no HTTP/2+/QUIC, no automatic decompression, no second scheduler.
