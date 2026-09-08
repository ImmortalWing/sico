# STEP-0136: source-level http@0.2.0 emission (buffered one-shot)

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05) profile item 8; RFC-0037
>   (http@0.2.0 interface, guest-visible since STEP-0125)

## 1. What was implemented

RFC-0038 profile item 8: Sico source can now emit `sico:script/http@0.2.0`
imports beside the frozen `0.1.0` compatibility surface.

- **Source surface**: `sico.http2.request(method: Text, url: Text, body:
  Bytes) -> Result[Http2Response, Text]`. The spelling follows the repo's
  established internal name for the @0.2.0 interface generation
  (`runner/http2.rs`, STEP-0125). `Http2Response` is a user-declared
  record (`status: I64`, `body: Bytes`) — the same pattern as the 0.1.0
  `HttpResponse` fixture.
- **Semantics/IR**: signature registered in both intrinsic tables;
  `Http2Response` layout (status s64, body (ptr,len)) added to the
  canonical flat/record tables.
- **Codegen**: the Component imports `sico:script/http@0.2.0` when the
  program calls the intrinsic, lowering `request` through the shared
  transport memory exactly like `0.1.0`. The guest passes Host-default
  options (empty headers, no redirect follow, no retry, `timeout-ms: 0`)
  — least-privilege defaults; policy stays Host-owned. The response
  `headers` list is lifted and deliberately narrowed off the v0 source
  record. The typed `http-error` enum surfaces as its **case-name Text**
  through a static data-segment table (eight absolute pointer+length
  pairs in WIT declaration order, appended after the literal pool) — no
  Host text crosses the boundary, honoring RFC-0037's no-leak rule.
- **Trap fix during bring-up**: the discriminant bounds check ran
  unconditionally; on the ok side offset 8 holds the low status half
  (status ≥ 8 trapped). The discriminant is now forced to 0 when the tag
  says ok before the table lookup.

## 2. Deliberately out of scope (declared, not silent)

`request-streaming`, `open-upload`/`finish` and the
`response-body`/`request-body` resources remain Component-level only
(runner-evidenced since STEP-0125). From source they are unknown callees
at check (the declared §2.2 gap class) and typed `unsupported call target`
refusals at build. Matrix rows updated:
`http-0-2-0-guest-visible` (now check=build=run) and new refused row
`http2-streaming-upload-from-source`.

## 3. End-to-end evidence

`tests/end-to-end/script-http2-request.sico` compiled by the real CLI and
run through the real runner against the deterministic TLS fixture server
(`runner/sico-runner/tests/http2_source.rs`):

- granted endpoint: stdout `200:source-http2`, exactly one request hit;
- ungranted endpoint: stdout `error:permission`, exit 1, **zero
  connections** — the typed taxonomy reaches source level.

## 4. Validation

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test http2_source -- --test-threads=1`: 2/2 green.
- Root workspace full test suite green; `cargo fmt` / `clippy -D warnings` clean; `tools/validate-step-0131.ps1` green; `git diff --check` clean.
- The frozen `fixed-width-*`/script component snapshots are unaffected
  (http2 emission is gated on use of the new intrinsic).
