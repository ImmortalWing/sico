# STEP-0089: scoped HTTP Component provider

> - status: complete
> - phase: M9
> - started: 2026-07-18
> - completed: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Add outbound HTTP as a versioned, default-deny Component/Host provider with exact endpoint grants, bounded request/response transport, no redirects or ambient sockets, and cancellation integrated with the runner's typed control outcomes.

## 2. Delivered

- `sico:script/http@0.1.0`, `sico.http.request` source typing/lowering and record/result Canonical ABI are complete; record fields remain available after Result payload projection.
- `sico run --allow-net HOST:PORT` passes repeatable exact grants to `sico-runner`; package imports map to `network.connect`.
- The raw HTTP/1.x Host provider validates scheme/host/port/method/body before DNS, never follows redirects, uses fixed request headers, and bounds URL, response headers and bodies.
- Blocking connect/read/write runs on a worker; total timeout and CancelToken polling abandon it without blocking the runner outcome. After abandonment the Store's HTTP channel fails closed to prevent worker accumulation.

## 3. Exit evidence

`tools/validate-step-0089.ps1` is green: real `sico run` POST roundtrip (`202:pong`), direct GET roundtrip (`200:get`), exact body and request targets, default/wrong-port denial, HTTPS and unsupported-method refusal, surfaced 302 without following, 64 KiB header and 8 MiB body bounds, 5 s total timeout, prompt cancellation with control exit 123, and exact `network.connect` package mapping. Report: [`scoped-http-provider-v0`](../reports/scoped-http-provider-v0.md).

## 4. Honest limits

TLS, IPv6/IDNA, proxies, credentials, upload streaming, chunked responses and DNS pinning are not implemented. A system-resolved DNS grant authorizes that exact hostname/port; literal local endpoints require an explicit literal grant. An abandoned OS-blocked worker dies with the one-shot runner process, and no further HTTP worker is admitted in that Store.

## 5. Links

- [`RFC-0031`](../rfc/RFC-0031-scoped-http-v0.md)
- [`M9 plan`](../plans/M9-streaming-async-interactive.md) (§7)
- [`STEP-0088`](./STEP-0088-async-runner-streaming-stdio.md)
