# RFC-0031: Scoped HTTP provider v0

> - status: proposed
> - date: 2026-07-18
> - authors: autonomous-agent
> - target language/platform version: M9 draft
> - supersedes: -
> - superseded-by: -

## Summary

One versioned HTTP provider interface, `sico:script/http@0.1.0`, implemented by the Host (runner), callable from Script through the closed intrinsic registry. Authorization is an exact, per-invocation endpoint allowlist: scheme, host and port must all match a granted `host:port` pair. Default remains no network. HTTPS is explicitly out of v0 because the offline toolchain has no TLS stack; `https://` fails closed.

## Problem

Automation needs outbound HTTP, but ambient sockets are unacceptable: a script must not reach arbitrary hosts, local services, redirected targets or listener sockets, and the permission decision must be auditable from the invocation alone.

## Interface

```wit
interface http {
  record response { status: s64, body: list<u8> }
  request: func(method: string, url: string, body: list<u8>) -> result<response, string>;
}
```

`method` is `GET` or `POST` (anything else is a typed refusal). The error text is a bounded diagnostic from a closed set (`denied`, `dns`, `connect`, `protocol`, `timeout`, `cancelled`, `resource-limit`), prefixed with the class; it never embeds the URL, headers or body.

## Semantics

- **Authorization.** `--allow-net host:port` (repeatable) grants exactly that host and port over `http`. The URL's scheme must be `http`, its host must equal the grant byte-for-byte (case-insensitive ASCII), and its port must equal the grant (explicit or the default 80). No wildcards, no CIDR, no listener.
- **DNS.** The granted host is resolved by the system resolver at request time; the `Host` header names the granted host. IP-literal grants (e.g. `127.0.0.1:8080`) connect literally. v0 does not pin resolved addresses to the grant beyond this; a stricter resolved-IP check is later work and is recorded as such.
- **Redirects.** Never followed: a 3xx response is returned to the guest as-is. Redirect targets cannot expand authority.
- **Limits.** Request body ≤ 8 MiB; request headers are the fixed minimal set (`Host`, `Content-Length`, `Connection: close`, `User-Agent: sico-runner/x`); response status line + headers ≤ 64 KiB; response body ≤ 8 MiB; connect timeout 5 s, total timeout 30 s. Exceeding any bound is `resource-limit` or `timeout`, never silent truncation.
- **Cancellation.** The request runs on a worker thread behind the STEP-0088 rendezvous pattern; a fired CancelToken returns `cancelled` promptly without waiting for the socket.
- **Capability.** The interface maps to `network.connect` in the package closure (already a supported grant name). Without `--allow-net` every call returns `denied`.
- **Logging.** v0 logs nothing about requests; there is no credential support, so nothing sensitive exists to redact. Both are later work with their own RFC updates.

## Positive and negative cases

Positive: granted `http://127.0.0.1:PORT` GET/POST round-trips status and body; redirect surfaces the 3xx status to the guest.
Negative (all typed, no socket opened unless stated): ungranted host; ungranted port; `https://`; other methods; oversized request/response body; slow server → timeout; cancel during connect/read.
