# STEP-0113: canonical endpoints, IPv6/IDNA and DNS pinning

> - status: complete
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Result

`crates/sico-http-provider/src/authority.rs`: the single strict authority parser shared by policy and transport, plus the address-policy gate.

- `canonicalize_url` / `canonicalize_authority`: one canonicalization, refusal-not-mapping for every RFC-0037 §2 confusion class — non-canonical IPv4 (octal/hex/leading zeros/trailing dot), bare decimal hosts, IPv4-mapped IPv6, zone/scope ids, non-ASCII and uppercase hosts, percent-encoded hosts, user-info, fragments, non-canonical/zero/padded ports, unknown schemes, `[`/`]` mismatches.
- IDNA policy: grants and URLs speak canonical ASCII; punycode (`xn--…`) is ordinary LDH; Unicode is refused, never converted — homoglyph/percent tricks cannot bypass a grant because both sides share this parser.
- `classify` + `address_allowed_for_scheme`: per-use address gate. Plain `http`/`https` refuse loopback/link-local/private/CGNAT/unspecified ranges; development `http+private`/`https+private` schemes accept them. The gate runs per connection attempt, per redirect target and per pool reuse (DNS rebinding defense is per-use, never cached).
- 13/13 provider tests green, including the pinned-address policy matrix and punycode/Unicode pairs.

## 2. Audit links

- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) §2/§4
- [`STEP-0111`](./STEP-0111-secure-http-rfc.md)
