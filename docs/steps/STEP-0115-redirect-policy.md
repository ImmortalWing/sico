# STEP-0115: redirect and origin-transition policy

> - status: complete
> - phase: M12
> - started: 2026-09-02
> - completed: 2026-09-02
> - owners: autonomous-agent

## 1. Result

`crates/sico-http-provider/src/redirect.rs`: the RFC-0037 §5 redirect engine as a pure policy layer.

- `decide_hop`: frozen status/method matrix (301/302/303 → GET no replay; 307/308 → method-preserved with replay), opt-in only (default preserves M9 no-redirect), ≤5 hops with structured refusal past the budget, exact-URL loop detection, HTTPS→HTTP downgrade refused, relative `Location` refused (no silent resolution).
- `headers_surviving_origin`: structural stripping — `authorization`, `x-api-key`, `cookie`, `proxy-authorization` and secret-derived headers never cross an origin change, independent of values; nothing is stripped when the origin is unchanged.
- Cross-origin grants: the hop decision is origin-agnostic by design; the caller (runner) must hold a separate grant for the target endpoint before following — that enforcement is exercised in STEP-0117's runner integration where grants live.
- The chain shares one total deadline and one cancellation token (caller contract, RFC-0037 §5).
- 23/23 provider tests green.

## 2. Audit links

- [`RFC-0037`](../rfc/RFC-0037-secure-http-provider-v0.md) §5
- [`STEP-0111`](./STEP-0111-secure-http-rfc.md)
