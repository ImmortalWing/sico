# M11 plan: Secure HTTP Provider and Automation SDK

> - status: planned after M10 GO
> - created: 2026-07-19
> - phase: M11
> - reserved steps: STEP-0103–0110
> - entry requirement: M10 GO with structured Runtime events, redaction and typed cancellation
> - implementation boundary: HTTP/TLS/DNS/secrets are Host-provider responsibilities, not compiler responsibilities

## 1. Outcome

Make Sico useful for real production API automation without granting ambient network or secret access. A Script must be able to perform bounded HTTPS requests with certificate and hostname verification, safe DNS handling, streaming request/response bodies, explicit redirect rules and Host-injected credentials. The same cancellation, event, redaction and per-run isolation model must cover every network phase.

The target is a secure client-side automation SDK, not a general socket API or web browser. M11 should make common API workflows possible while keeping every endpoint, credential and byte budget auditable.

Representative target flows:

```powershell
sico run fetch.sico --allow-endpoint https://api.example.com:443
sico run upload.sico --allow-endpoint https://storage.example.com:443 --secret github-token
sico watch sync.sico --allow-endpoint https://api.example.com:443
```

The command spelling is provisional until STEP-0103 accepts the contract. No M11 syntax or support claim exists merely because this plan reserves it.

## 2. M9/M10 baseline and bottleneck

M9 HTTP v0 already proves:

- a versioned `sico:script/http@0.1.0` Component/Host boundary;
- no-network-by-default and exact ASCII host/IPv4 plus port grants;
- GET/POST, bounded URL/headers/body, no redirect following, timeout and typed cancellation;
- real loopback HTTP execution and package capability mapping;
- per-Store abandonment rules that prevent worker accumulation.

Its honest limits are no TLS, IPv6/IDNA, proxy, credentials, custom headers, streaming upload, chunked response decoding or DNS pinning. Those limits prevent safe use with most production APIs.

M10 is an entry requirement because certificate, redirect, DNS and credential failures must use structured faults/events and mandatory redaction, while client cancellation must reach connect/TLS/read/write phases through one typed path.

## 3. Scope and non-goals

### 3.1 In scope

- HTTPS with mature TLS implementation, SNI, certificate-chain and hostname validation;
- explicit endpoint grants including scheme, canonical host and effective port;
- IPv4, IPv6 and IDNA canonicalization with DNS rebinding defenses;
- bounded request headers and streaming request/response bodies;
- HTTP/1.1 Content-Length, connection-close and chunked transfer handling;
- explicit redirect policy and cross-origin reauthorization;
- Host-owned secret references and credential/header injection with redaction;
- per-Store connection pooling, cancellation, deadlines and persistent-runner isolation;
- standard-library automation helpers that compile to the same provider boundary;
- CLI/package/tooling policy integration and actual-platform audit.

### 3.2 Non-goals

- custom TLS, certificate, cryptographic primitive or random-number implementation;
- generic TCP/UDP sockets, listening servers, peer-to-peer or raw packet access;
- ambient proxy/environment discovery, unrestricted system proxy use or PAC execution;
- browser cookies, JavaScript, HTML, CORS emulation or a browser security model;
- automatic credential discovery from environment, home directories or cloud metadata;
- wildcard internet access as the default grant;
- WebSocket, HTTP/2, HTTP/3 or QUIC in v1 unless STEP-0103 separately proves necessity and bounded support;
- transparent decompression without an explicit expanded-byte limit;
- parallel Task scheduler, races/select or M12 concurrency work;
- public registry deployment, mobile Runtime completion or unsupported-platform claims.

## 4. Architecture and ownership

```text
Sico source and standard-library helper
  → semantics/IR: type, effect and capability verification
  → codegen: versioned Component import only
  → WIT + manifest: request/resource/capability contract
  → secure HTTP Host provider: TLS, DNS, redirect, streaming, secrets
  → runner policy: endpoint grants, budgets, cancellation, events
```

| Concern | Owner | Explicit boundary |
|---|---|---|
| source typing/effects for HTTP operations | semantics/IR | no TLS/socket types or dependencies |
| Component import/lift-lower | codegen | no DNS, trust store or credential implementation |
| request/response/resource schema | versioned WIT/RFC | no platform policy hidden in compiler |
| TLS/DNS/HTTP transport | separate Host-provider module/crate | uses a mature audited library; no custom crypto |
| endpoint/secret grants | runner/Host policy and manifest intersection | default deny; exact identities |
| ergonomic JSON/form/pagination helpers | standard library | lower to provider API; no authority widening |
| execution plans, diagnostics and redaction display | tooling | no secret bytes and no direct network authority |

The preferred implementation is a separable provider crate/module under the runner boundary, with narrow typed configuration and no dependency from compiler crates. If offline dependency availability blocks a mature TLS stack, M11 pauses at the contract/prototype gate rather than implementing TLS manually.

## 5. Versioned contracts to freeze in STEP-0103

### 5.1 Provider and WIT versioning

M11 introduces a new compatible major/minor identity rather than mutating `sico:script/http@0.1.0` silently. The RFC must decide whether this is `http@0.2.0` or a new `secure-http@0.1.0` package based on WIT compatibility evidence.

The contract needs:

- immutable request head: method, URL, bounded headers and options;
- affine request-body writer/output resource;
- response head plus affine response-body input resource;
- explicit finish/close/drop and EOF/error distinctions;
- structured HTTP/TLS/DNS/permission/limit/cancel errors;
- cancellation token ownership through the runner rather than guest-forged control;
- no exposure of sockets, certificates' private keys or raw secret storage.

The existing buffered HTTP v0 remains supported as a bounded compatibility profile. Buffered and streaming consumption of the same body cannot be mixed.

### 5.2 Endpoint authority

Candidate grant identity:

```text
scheme + canonical host + effective port
```

Examples:

- `https://api.example.com:443` does not grant `http://api.example.com:80`;
- a DNS name grant does not grant a literal IP URL;
- an IPv6 literal grant is distinct from a DNS name resolving to that address;
- omitted ports canonicalize to the scheme default before policy comparison;
- user-info, fragments and ambiguous/noncanonical authority forms are rejected.

No wildcard is accepted in v1 unless a later narrowly specified suffix policy proves resistant to public-suffix and confusion attacks. Path-prefix restriction may be added as a narrowing policy, never as a substitute for endpoint identity.

### 5.3 TLS trust policy

- TLS uses a mature library and secure protocol defaults; no SSL or legacy downgrade fallback.
- SNI and certificate hostname checks use the same canonical host identity.
- Trust roots come from an explicitly selected Host trust mode: audited system roots or an explicitly provided pinned/bundled root set.
- invalid chain, expired/not-yet-valid certificate, wrong hostname, unknown issuer and malformed handshake produce structured failures. Revocation/policy status is enforced only when the selected trust mode provides a verifiable mechanism; unsupported revocation checking is reported explicitly rather than claimed silently.
- certificate verification cannot be disabled by guest code. Any development-only insecure mode must be an explicit Host flag, visually noisy, excluded from packages and unavailable in normal execution plans.
- optional public-key/certificate pinning, if accepted, narrows system trust and is bound to an exact endpoint.

### 5.4 DNS and address policy

- IDNA conversion and Unicode display are separated; policy compares canonical ASCII host identity.
- DNS results are validated once per connection attempt and the chosen IP is pinned for that socket.
- redirect and connection reuse re-run the required policy checks.
- loopback, link-local, multicast, unspecified, documentation, carrier-grade NAT and private ranges are denied for DNS hostnames by default unless an explicit private-network grant exists.
- literal private/loopback endpoints require a literal, explicit development/test grant.
- resolution result counts, address bytes and connect attempts are bounded; no unbounded search/fallback loop.

### 5.5 Redirect policy

- default is no automatic redirect, preserving M9 behavior;
- optional automatic redirects are capped at five;
- same-origin redirects require the same exact grant;
- cross-origin redirects require a separate endpoint grant and reauthorization;
- `Authorization`, cookies and all secret-derived headers are stripped whenever origin changes, even if the destination is also granted;
- HTTPS-to-HTTP downgrade is denied;
- method rewriting follows an explicitly frozen status-code matrix; ambiguous behavior is refused;
- redirect loops return a structured error.

### 5.6 Secret and credential model

Scripts reference opaque secret names/handles, never ambient environment variables or raw Host secret stores. Candidate capability identity:

```text
secret.use:<name> + exact endpoint + injection policy
```

The Host may inject an authorized secret into a frozen header/query/signing policy without returning the secret bytes to the guest. Initial v1 should prefer header injection (`Authorization`, API-key header) over arbitrary template expansion.

Rules:

- package manifest, Host grants and invocation request intersect exactly;
- a secret grant is endpoint- and injection-policy-specific;
- secret names are safe metadata; secret values never enter debug maps, events, cache keys, command lines or normal errors;
- stdout/stderr, Runtime events, DAP variables and AI summaries pass through mandatory value/fingerprint redaction;
- redirects never forward secret-derived material across origin;
- secret buffers are bounded, owned, zeroized where the chosen implementation can guarantee it and never persisted by Sico caches.

### 5.7 Streaming and resource bounds

Candidate hard limits for STEP-0103 acceptance:

- URL at most 8 KiB;
- at most 256 headers; serialized request/response headers at most 64 KiB;
- header name at most 256 bytes and value at most 8 KiB;
- stream chunk at most 64 KiB;
- queue depth 1 per body direction to preserve backpressure;
- default request and response body budget 64 MiB each;
- Host ceiling at most 1 GiB per direction per request;
- at most 16 in-flight requests and 32 pooled connections per Store;
- at most 8 connections per endpoint;
- connect/TLS/first-byte/total/idle deadlines all explicit and bounded;
- at most five redirects and a bounded DNS/address attempt count.

The total body budget is independent from memory usage: bytes move in bounded chunks, but cumulative accounting still prevents unlimited transfer by default. Limit increases require Host policy and cannot be selected solely by guest code.

### 5.8 Transfer and content coding

- Content-Length and chunked framing are parsed strictly; conflicting or duplicate ambiguous lengths fail closed;
- malformed chunk sizes, trailers and premature EOF are structured errors;
- connection-close framing remains bounded by the response budget;
- automatic decompression is off in initial v1 unless expanded-byte accounting is implemented;
- if gzip/br is later enabled, compressed and expanded limits are separate and decompression bombs fail before unbounded allocation;
- streaming upload supports known-length and chunked modes only when the provider can cancel blocked writes and preserve backpressure.

## 6. Execution sequence

### STEP-0103: secure HTTP/provider/authority RFC

Deliver:

- RFC selecting the WIT version, endpoint identity, TLS trust mode, DNS/private-address policy, redirect matrix, secret model and resource bounds;
- provider module boundary and dependency selection criteria;
- strict schema/fixture corpus for grants, URLs, headers, TLS/DNS errors and secret references;
- compatibility story for HTTP v0 buffered clients;
- threat model and platform evidence plan.

Exit evidence:

- all ambiguous/noncanonical authorities, wildcard grants, oversize inputs and cross-origin secret cases fail in fixtures;
- mature TLS dependency is available and license/supply-chain constraints are recorded;
- no implementation/support claim before acceptance.

### STEP-0104: TLS transport and certificate validation

Deliver:

- HTTPS connection path in the Host provider using the accepted mature TLS library;
- SNI, chain, hostname and validity checks;
- explicit system-root and test/pinned-root modes;
- structured handshake/certificate failures and M10 redacted events.

Exit evidence:

- local deterministic CA server plus valid leaf succeeds through real `sico run`;
- wrong hostname, expired/not-yet-valid, unknown issuer, malformed chain, invalid signature and protocol downgrade fail closed;
- HTTP remains available only under an explicit `http://` grant;
- guest cannot disable verification or inject trust roots.

### STEP-0105: canonical endpoints, IPv6/IDNA and DNS pinning

Deliver:

- strict URL/authority canonicalizer shared by policy and transport;
- IPv6 literal and IDNA handling;
- bounded resolution and chosen-address pinning;
- private/special-address classification and explicit private-network grants.

Exit evidence:

- Unicode/punycode equivalents cannot bypass grants;
- decimal/octal/hex IPv4 tricks, IPv4-mapped IPv6 and zone/scope ambiguity fail closed or canonicalize once;
- DNS answer changes cannot redirect an existing authorized connection;
- DNS hostname resolving to a denied private address is refused before connect;
- exact development loopback grants still support deterministic tests.

### STEP-0106: streaming request and response bodies

Deliver:

- affine request writer and response reader resources;
- known-length, chunked and connection-close transfer paths under one budget model;
- cancellation-aware connect/TLS/write/read workers or async implementation;
- strict trailer/framing parser and exact drop/close behavior.

Exit evidence:

- 1 MiB, 16 MiB and 256 MiB upload/download roundtrips are byte-exact with bounded RSS;
- slow producer/consumer demonstrates backpressure without queue growth;
- cancel during DNS/connect/TLS/upload/header/body/flush returns one typed terminal outcome;
- limit, limit+1, malformed chunk, premature EOF, duplicate length and use-after-close cases fail closed.

### STEP-0107: redirect and origin-transition policy

Deliver:

- opt-in redirect engine implementing the accepted status/method matrix;
- exact same-origin/cross-origin reauthorization;
- secret/header stripping and downgrade refusal;
- redirect chain events without leaking sensitive URLs/query values.

Exit evidence:

- same-origin allowed chain, ungranted origin, separately granted origin, loop, >5 hops and HTTPS downgrade cases;
- authorization never crosses origin;
- POST/body replay happens only when the contract marks it replay-safe and the body is reproducible within bounds;
- cancellation and time budget cover the entire chain, not each hop independently.

### STEP-0108: Host secret provider and redaction

Deliver:

- opaque secret registry/provider interface owned by the Host;
- exact package/Host/invocation intersection for `secret.use`;
- authorized header injection and optional narrowly scoped signing adapter;
- shared redaction integration for Runtime events, DAP, CLI and AI summaries.

Exit evidence:

- scripts can use an authorized token without reading its raw bytes;
- missing/wrong secret, wrong endpoint, wrong injection policy and capability drift fail closed;
- canary secret bytes and the exact encoded forms covered by the accepted redaction contract never appear in logs, faults, maps, cache, process argv or tooling output;
- redirects, retries, cancellation and Host failures do not leak or persist secret material.

### STEP-0109: connection lifecycle, SDK and tooling integration

Deliver:

- per-Store connection pool and explicit idle/connection ceilings;
- standard-library helpers for JSON request/response, form/query encoding, pagination and retry policy;
- retry classification limited to idempotent/replayable operations with one total deadline/budget;
- CLI/package grants and `sico.execution-plan` extensions for endpoints and opaque secret names;
- watch/debug/AI integrations consuming M10 events without receiving secrets.

Exit evidence:

- connection reuse occurs only within the same Store/authority set; no socket or credential survives the run;
- persistent runner generations with different grants cannot reuse authority-bearing pool state;
- retry storms, 429/5xx, disconnect and cancellation stay within attempt/time/body budgets;
- SDK helpers compile to the same provider imports and cannot bypass policy;
- shell metacharacters and secret names remain direct bounded arguments/data.

### STEP-0110: M11 security, performance and platform exit audit

Deliver:

- aggregate validator for STEP-0103–0109 plus M0–M10 regression;
- TLS/DNS/redirect/secret threat audit and mutation corpus;
- throughput, latency, RSS, cancellation and connection-lifecycle report;
- actual platform matrix, GO/NO-GO and residual-risk register.

Exit evidence is defined in sections 8–10.

## 7. Threat model

M11 must explicitly test:

- certificate/hostname validation bypass and insecure fallback;
- Unicode/IDNA, IPv4 textual and IPv6 canonicalization confusion;
- DNS rebinding and private-network/metadata-service access;
- redirect-based authority expansion and credential forwarding;
- request smuggling through conflicting lengths/chunk framing;
- response splitting and oversized header/trailer fields;
- decompression/body amplification and slowloris-style stalls;
- retry amplification and non-idempotent body replay;
- secret disclosure through logs, errors, debug variables, AI summaries, argv or cache;
- connection/socket/credential leakage across Store or watch generations;
- cancellation races leaving workers or sockets alive.

Hard security gates:

- no verification-disable path available to normal guest/package execution;
- no connection begins before canonical endpoint and address policy pass;
- no cross-origin secret forwarding;
- no ambient env/home/cloud-metadata credential discovery;
- every parser and queue has byte/item/time bounds;
- every network operation belongs to one Store and one terminal outcome;
- mutation, truncation, replay and mix-and-match fixtures fail closed.

## 8. Performance and resource plan

Hard limits come from the accepted RFC. Latency/throughput figures are measured non-SLA goals unless STEP-0103 explicitly promotes them.

Minimum matrix:

- 1/16/256 MiB upload and download, exact bytes and peak RSS;
- small JSON request latency separated into DNS, connect, TLS, first byte and body;
- cold TLS connection versus same-Store pooled connection;
- slow upload/download backpressure;
- cancellation during every phase;
- 1, 8 and 16 concurrent in-flight requests without a parallel language scheduler, driven by Host/provider tests;
- 100 sequential Stores and watch generations with handle/socket/RSS accounting;
- redirect chains, retries and secret injection overhead;
- invalid certificate/DNS/authority cases complete within bounded time.

Candidate goals to freeze in STEP-0103:

- memory growth remains independent of 256 MiB body size apart from fixed chunk/pool buffers;
- fired cancellation token is observed within 100 ms at provider polling/async boundaries;
- same-Store pooled request avoids a second DNS/TCP/TLS handshake;
- no idle pool survives Store teardown;
- full execution remains within the total deadline even across redirects/retries.

Raw samples, host load, TLS library/version, trust mode, server fixture and platform are recorded. Public internet timing is observational only and cannot replace deterministic local TLS fixtures.

## 9. Platform evidence policy

- Windows x64 is the initial execution target inherited from M9/M10 evidence.
- TLS root-store, DNS, IPv6 and signal behavior must be rerun natively before claiming Linux or macOS.
- A local deterministic TLS/DNS test environment is required on every claimed platform.
- Public HTTPS smoke tests may supplement but never replace local certificate/error fixtures.
- Android/Harmony require their separate Runtime/Host tracks and do not inherit desktop provider claims.
- Cross-compilation and WIT/schema parsing count as contract evidence only.

## 10. M11 exit gate

M11 is GO only when:

1. a versioned secure HTTP WIT/provider contract coexists with HTTP v0 without silent semantic mutation;
2. real HTTPS succeeds with valid trust and rejects all required certificate/hostname failures;
3. endpoint canonicalization, DNS pinning and private-address policy resist the documented confusion/rebinding corpus;
4. streaming request/response bodies preserve exact bytes, backpressure, cumulative budgets and bounded RSS;
5. redirects never widen authority or forward secrets across origin;
6. Host secret use is exact, opaque to guest code and redacted across CLI/events/DAP/AI/cache;
7. cancellation, timeout, retry and connection pooling have one bounded lifecycle per Store;
8. standard-library/tooling helpers cannot bypass provider or manifest policy;
9. the complete M0–M10 regression is green;
10. platform support is limited to actual native execution evidence.

Any custom/insecure TLS fallback, ambient credential discovery, unbounded body/queue, cross-origin credential leak or Store-crossing socket reuse is an automatic NO-GO.

## 11. Deferred successor work

M11 intentionally leaves structured parallel execution for a separate milestone. A likely M12 scope is bounded parallel Task scheduling, task groups, `select`/race, concurrency quotas and deterministic cancellation/collection semantics. M11 provider tests may exercise multiple in-flight Host requests internally, but that does not create or claim new language-level concurrency.

If production hosting credentials/domain or Android runners become available, the existing M7/M6 external tracks may resume in parallel as separately numbered work. They must not be folded into M11 or used to weaken its exit gate.
