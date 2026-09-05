# STEP-0125: guest-visible http@0.2.0 runner integration

> - status: complete
> - phase: M12 (residual integration deliverable (a) from STEP-0118)
> - started: 2026-09-05
> - completed: 2026-09-05
> - owners: autonomous-agent
> - contract: RFC-0037 (accepted); no new RFC required

## 1. Outcome

`http@0.2.0` is guest-visible: a Program Component importing
`sico:script/http@0.2.0` runs in the runner against the secure provider
engine, with live streaming (chunked / content-length / until-close), a
per-Store connection pool with reuse, idempotent-only retry, streaming
chunked upload, and Host-injected secrets. The compiler frontend does not
yet emit these imports — that is the M14 application-language baseline —
so guest-path evidence uses directly encoded Program Components
(`runner/sico-runner/tests/http2.rs`), which is exactly what
"guest-visible" means here: any Component importing the frozen interface
runs, regardless of which tool encoded it.

Exit-gate impact (M12 plan §10): gate 7 moves from partial to GO
(cancellation/timeout/retry/pooling now have one bounded lifecycle per
Store); gate 4's live-streaming clause is exercised by real streaming
resources; gate 1's versioned coexistence is now exercised by real
Components importing `0.2.0` beside unchanged `0.1.0` clients.

## 2. Provider additions (`crates/sico-http-provider`)

- `streaming.rs`: `BodyReader` (strict incremental decode of
  Content-Length / chunked / until-close framing over one pinned
  connection; chunk-size lines, chunk-terminating CRLF and trailers are
  verified; budget accounting on every chunk), `ConnectionPool`
  (≤ 32 pooled, ≤ 8 per endpoint, 15 s idle bound, stale/dropped-entry
  detection), `is_idempotent` (GET/HEAD only), and the deterministic
  `HttpFixtureServer` (real HTTP/1.1 over the deterministic rcgen CA,
  keep-alive, plan-driven responses incl. close-before-response, stall,
  slow-body, chunked).
- `engine.rs`: `execute_streaming` (policy loop identical to the buffered
  path — canonicalize → address gate → exact grant → secret injection —
  plus pooled-connection reuse with stale fallback, ≤ 1 extra fresh
  connect for idempotent empty-body head-phase failures, total deadline
  across the chain), `ResponseBody` (clean EOF returns the connection to
  the Store pool; error/drop closes the socket and releases the in-flight
  slot), `open_upload`/`UploadPipe` (depth-1 rendezvous chunk channel,
  cancellation probe, chunked request framing, one terminal outcome),
  shared atomic in-flight cap (16), typed IO classification
  (timeout / premature close / io).
- The frozen buffered `execute()` path is byte-for-byte untouched; all
  33 pre-existing provider tests stayed green throughout.

## 3. Runner integration (`runner/sico-runner`)

- `http2.rs` links `sico:script/http@0.2.0` default-deny: `request`
  (buffered, worker-executed, abandoned-Store fail-closed on
  timeout/cancel exactly like `0.1.0`), `request-streaming`,
  `[method]response-body.read` (dedicated worker per body so a blocked
  read stays cancellable; depth-1 rendezvous = backpressure; EOF is the
  empty ok list), `open-upload` / `[method]request-body.write` /
  `finish` (affine writer, one terminal outcome), per-Store
  `Http2State` (engine + pool + abandonment flag + ≤ 16 live bodies).
  Errors cross as typed `http-error` discriminants only; provider
  diagnostics never cross.
- `NetGrants::grant_secure` accepts `https://`, `http://`,
  `https+private://` spellings, canonicalized once by the provider's
  authority parser; `0.1.0` `host:port` grants remain valid and map to
  plain `http` endpoints (RFC-0037 compatibility clause).
- `HttpPolicy` freezes pinned PEM trust roots and the Host-owned secret
  registry into one prepared generation. Empty trust roots = every
  handshake fails closed; there is no insecure fallback (the
  `system-roots` flattening is a Host-caller concern and is not claimed).
- Runner CLI: `--allow-endpoint`, `--http-trust-roots` (PEM files,
  ≤ 1 MiB), `--secret-file` (JSON name/value/endpoint/header,
  ≤ 64 KiB; values enter via file, never argv, never the guest).
- `wit/script-http2-v0/http2.wit` is the draft WIT realization of
  RFC-0037 §1.

## 4. Guest-path evidence (`runner/tests/http2.rs`, 6 tests)

Real guest Components encoded with `wasm-encoder` (component-level
instance type with resource types and borrow/own wrappers, canonical-ABI
retpad result areas, shared transport memory) run through the runner:

1. buffered GET roundtrip; second request to the same endpoint reuses one
   pooled TCP connection inside one Store (server sees 1 connection,
   2 requests);
2. streaming chunked download is byte-exact through `request-streaming` +
   `read` until EOF;
3. ungranted endpoint → typed `permission` (exit 110), zero connections;
4. upload roundtrip: chunked request framing observed by the server and
   the authorized secret injected host-side (guest never sees it);
5. mid-stream cancellation: the run commits the M10 run-level
   `Cancelled` outcome promptly (≤ 300 ms chunk cadence) instead of
   waiting out the server body;
6. close-before-response on a streaming request → typed `protocol`
   (exit 116), no retry without the idempotent flag.

## 5. Validation

- `tools/validate-step-0125.ps1`: fmt, clippy `-D warnings`, provider
  tests (48), runner tests (37 lib + 37 runner + 6 http2, run serially —
  the RSS-bound DAP tests are measurement-noisy under host parallelism;
  the same flakiness reproduces on the STEP-0118 commit), STEP-0089
  oracle rerun, module boundaries (27 root packages unchanged — the
  runner is its own workspace), insecure-API scan.
- Recorded deviations: runner `tests/runner.rs` run serially (see above);
  source-level `http@0.2.0` emission is not claimed anywhere.

## 6. Residual risks / boundaries

- Chunked response decoding now exists in the streaming path; the frozen
  buffered `0.1.0` host path still refuses chunked responses (unchanged,
  by design).
- `system-roots` trust mode is not implemented (no native-certs
  dependency); TLS requires explicit pinned roots — fail-closed.
- Secret values persist in Host memory for the process lifetime; the
  provider's redaction/fingerprint contract covers all emitted paths.
- Linux x64 provider-corpus rerun is deliverable (b) — STEP-0126.
