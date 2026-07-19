# STEP-0089: scoped HTTP Component provider

> - status: in-progress
> - phase: M9
> - started: 2026-07-18
> - updated: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Add outbound HTTP as a versioned, default-deny Component/Host provider with exact endpoint grants, bounded request/response transport, no redirects or ambient sockets, and cancellation integrated with the runner's typed control outcomes.

## 2. Current boundary

- RFC-0031 is proposed and `sico:script/http@0.1.0` exists as a WIT draft.
- Compiler Canonical ABI/codegen work for `sico.http.request` has started.
- The runner deliberately does not link an HTTP provider. `NetGrants` is reserved but inert, so current artifacts fail closed instead of receiving partial network authority.
- No HTTP completion claim is valid until the runner provider, CLI grants and local positive/negative evidence all pass.

## 3. Required exit evidence

1. Exact `http` scheme/host/port authorization; default denial and no wildcard/listener authority.
2. GET/POST loopback success through the real Component path.
3. Typed refusal for ungranted endpoint, wrong port, HTTPS, unsupported method and redirect authority expansion.
4. Enforced request body, response header and response body limits without truncation.
5. Timeout and cancellation during connect/read with runner control exit 123 where cancellation wins.
6. Workspace, runner and prior STEP-0084/0086/0087/0088 regressions remain green.

## 4. Honest limits

TLS, proxies, credentials, upload streaming and DNS pinning are not implemented. RFC-0031 must state any v0 deferral explicitly; unresolved policy cannot be hidden by a permissive implementation.

## 5. Links

- [`RFC-0031`](../rfc/RFC-0031-scoped-http-v0.md)
- [`M9 plan`](../plans/M9-streaming-async-interactive.md) (§7)
- [`STEP-0088`](./STEP-0088-async-runner-streaming-stdio.md)
