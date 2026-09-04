# STEP-0118: M12 security, performance and platform exit audit

> - status: complete / GO-core
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Gate-by-gate verdict (M12 plan §10)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | versioned WIT/provider contract coexists with HTTP v0 without silent mutation | GO-core | RFC-0037 accepted (`sico:script/http@0.2.0` identity, 0.1.0 buffered profile preserved); STEP-0089 oracle rerun green after every provider change |
| 2 | real HTTPS with valid trust; rejects certificate/hostname failures | GO-core | `sico-http-provider`: valid-chain roundtrip green; wrong-hostname/untrusted-issuer/malformed-PEM refused; no `danger()`/custom-verifier/insecure path (static scan in validator) |
| 3 | endpoint canonicalization, DNS pinning, private-address policy resist the confusion/rebinding corpus | GO-core | `authority.rs` 13-test corpus: IPv4 textual tricks (octal/hex/bare-decimal/leading zeros/trailing dot), IPv4-mapped IPv6, zone ids, percent/Unicode/uppercase hosts, user-info, fragments, bad ports all refused; per-use address gate refuses rebinding before connect |
| 4 | streaming bodies preserve bytes, backpressure, budgets, bounded RSS | GO-core (parser layer) | `framing.rs`: strict framing decisions, chunked reader with bounds-before-allocation, smuggling corpus green; live streaming resources land with runner WIT integration |
| 5 | redirects never widen authority or forward secrets cross-origin | GO-core | `redirect.rs`: frozen status/method matrix, ≤5 hops, loop/downgrade refusals, structural cross-origin header stripping (value-independent) |
| 6 | Host secret use exact, opaque, redacted across CLI/events/DAP/AI/cache | GO-core | `secrets.rs`: exact triple intersection with typed failures; deterministic fingerprint redaction with canary proof (raw/Bearer/Basic forms) |
| 7 | cancellation/timeout/retry/pooling one bounded lifecycle per Store | partial | engine caps (16 in-flight, one terminal outcome per request, total deadline) in place; connection pooling and retry classification are runner-integration work |
| 8 | stdlib/tooling helpers cannot bypass policy | GO (by construction) | helpers lower to the same provider boundary; nothing outside the provider crate performs IO |
| 9 | complete M0–M11 regression green | verified in validator | `validate-step-0118.ps1` reruns the 0110 aggregate chain |
| 10 | platform support limited to actual native execution | Windows x64 verified | provider tests executed on Windows x64 GNU; Linux rerun pending (WSL2 available — parity script covers runner corpus; provider corpus extension is follow-up) |

## 2. Verdict

**M12: GO-core** — the contract (RFC-0037) and all five policy/transport layers (authority, TLS, framing, redirect, secrets+engine) are frozen, implemented and tested (33/33 provider tests, clippy `-D warnings` clean, zero insecure-API surface). Two integration deliverables remain before the M12 plan's full GO: (a) guest-visible WIT `http@0.2.0` resources wired through the runner with live streaming, pooling and retry; (b) the provider corpus rerun on Linux x64. Both are bounded engineering work with the contracts already frozen; the automatic-NO-GO list (custom TLS, ambient credentials, unbounded queues, cross-origin leaks, Store-crossing reuse) shows zero violations.

## 3. External-input recheck (2026-09-02)

| Input | State |
|---|---|
| Public deployment / production identity | still absent |
| Third-party pilots | still absent |
| Live-model credentials (M13) | owner-promised, not yet delivered |
| Mobile runners | still absent |
| Linux x64 | WSL2 environment available (STEP-0109); provider corpus rerun is follow-up work |

## 4. Residual risks

- The engine's chunked-response decoding and connection pooling are the two parser/transport areas where the runner integration must not regress the frozen parser contracts.
- Secret zeroization is bounded by rustls/String semantics; values never persist in caches and never cross process boundaries.
- Revocation checking is explicitly reported unsupported in v1 (RFC-0037 §3).

## 5. Validation

`tools/validate-step-0118.ps1`: fmt/clippy, provider tests, module boundaries (27 packages), RFC decision presence, insecure-API scan, and the full M0–M11 regression via the STEP-0110 aggregate. Results recorded in the validator output at completion.
