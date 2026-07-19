# Scoped HTTP Component provider evidence

> - date: 2026-07-19
> - step: STEP-0089
> - status: complete

## Implemented boundary

`sico.http.request(method, url, body)` lowers to the versioned `sico:script/http@0.1.0` Component import and returns `Result[HttpResponse, Text]`. The compiler preserves `HttpResponse.status/body` field layouts through Result match payload projection. `sico run --allow-net HOST:PORT` passes repeatable grants to `sico-runner`; the package closure maps the import to `network.connect`.

The Host checks GET/POST, URL syntax, `http` scheme, canonical ASCII DNS/IPv4 host, exact port and request size before DNS. It emits fixed headers, does not follow redirects, accepts HTTP/1.0/1.1 with one Content-Length or connection-close framing, and caps URL/header/body at 8 KiB/64 KiB/8 MiB. Connect and total deadlines are 2 s/5 s. Blocking socket work runs on an abandonable worker; cancellation returns the control outcome 123.

## Measured evidence

`tools/validate-step-0089.ps1` proves, without external network access:

- real `sico run` compiler/cache/runner/Component loopback POST: request target `/echo?q=1`, exact eight-byte body, response `202:pong`, exit 0;
- direct runner GET loopback: request target `/read?q=2`, response `200:get`, exit 0;
- no grant and wrong port → typed `denied` 122 before socket; wildcard grant rejected by runner CLI 121;
- HTTPS and unsupported method fail closed; 302 + Location is returned as `302:redirect` and never followed;
- response headers over 64 KiB and declared body over 8 MiB → typed `resource-limit` without truncation;
- silent server → typed `timeout` in 5,175 ms for the final run (5 s provider deadline);
- cancellation during blocked response read → runner `Cancelled` exit 123 in 128 ms for the final run;
- workspace codegen/package/CLI tests and runner release tests green; package import maps exactly to `network.connect`.

## Honest limits

v0 has no TLS, IPv6/IDNA, proxies, credentials, custom headers, streaming uploads, chunked response decoding or resolved-IP pinning. DNS uses the system resolver after an exact hostname/port grant. A socket worker abandoned after timeout/cancellation can remain OS-blocked until the one-shot runner process exits; after abandonment that Store's HTTP channel fails closed, so guest recovery logic cannot accumulate more workers.
