# RFC-0043 Amendment A2: bitmap input serialization for the roster `Bytes` signatures (ACCEPTED)

> - status: **accepted by owner re-adjudication 2026-10-01** (raised 2026-10-01). The owner chose **A2 — the self-describing packed `Bytes` input header** (`width u32 LE · height u32 LE · format u8 · reserved u8[3]=0 · pixels`), over A2-alt (scalar dims). This completes the roster input half that A1 left open (A1 froze only the **result** encoding: `region`/`list<region>` → packed `Bytes`, `image-error` → case-name `Text`). With A1 + A2 accepted, both the input and result encodings of `image-match@1` / `image-grid@1` / `image-contour@1` are frozen, so all three are buildable as pure `sico:user/` packages. **Implementation is proceeding now; M24 §3 gate 3 remains 未达 until all three packages land their own ADR-0019 core + independent numpy-oracle content corpus + runner content test, one STEP each — this acceptance flips nothing by itself.**
> - date: 2026-10-01
> - amends: [`RFC-0043`](./RFC-0043-deterministic-vision-packages-v0.md) §Summary + [`Amendment A1`](./RFC-0043-amendment-A1-region-packed-bytes.md) §2 (the `bmp: Bytes` / `bitmap: Bytes` **input** side of `image-match@1` / `image-grid@1` / `image-contour@1`)
> - depends: [`RFC-0041`](./RFC-0041-image-data-contract-v0.md) §1 `record bitmap { width, height, stride, format: pixel-format{bgra8,grey8}, orientation{top-left}, pixels: list<u8> }` + `image-error`; [`RFC-0050 Amendment A1`](./RFC-0050-amendment-A1-flattened-surface.md) + STEP-0331 (the accepted precedent for a **self-describing packed image `Bytes`**: a `decode-png` ok-payload carries `width u64 LE · height u64 LE · BGRA8 pixels`)

## 1. Why this amendment exists (measured contract gap, not preference)

A1 rewrites the three roster signatures to pure-`Bytes` forms so the **result**
lowers through the accepted E8017 value set. But the pre-A1 signature took a
`bitmap` **record**, and A1 collapsed the *inputs* to `Bytes` as well
(`template(bmp: Bytes, tpl: Bytes)` / `detect-lines(bitmap: Bytes, min-run: U64)`
/ `find(bmp: Bytes, min-area: U64)`) without defining **how a `record bitmap`
is packed into an input `Bytes`**. That is an open contract decision:

- RFC-0041 §1 defines `bitmap` only as a WIT *record* (width/height/stride/
  format/orientation/pixels) and **§3 explicitly defers canonical
  serialization** ("out of scope until a measured consumer needs it").
- A1 §2 froze only the `region` result framing; it is silent on the input.

So a two-opaque-`Bytes` `template` call carries no width/height/format and
cannot be reconstructed into a 2D buffer. Building any roster package would
require the implementer to **pick** an input serialization — precisely the
"self-invent a binding / semantically-arbitrary lowering" that AGENTS.md §5
forbids and 不静默改合同 prohibits. A2 surfaces that choice for the owner.

## 2. A2 — self-describing packed `Bytes` input (recommended), mirroring image-codec

Reuse the **accepted** image-codec payload-header precedent (STEP-0331), with a
format tag because the roster runs over grey8/bgra8 (RFC-0041 `pixel-format`),
and freeze a fixed little-endian framing (no new type, no ABI widening):

```text
bitmap -> Bytes:
  width        u32 LE   (bytes 0..4)
  height       u32 LE   (bytes 4..8)
  format       u8       (byte 8: 0 = bgra8, 1 = grey8)   [RFC-0041 pixel-format]
  reserved     u8[3]    (bytes 9..12: MUST be zero)      [orientation is v0-only top-left]
  pixels       (width * bytes-per-pixel(format)) * height bytes, tightly packed
                        (v0 stride == width * bpp; stride field is implicit/derived)
```

- bytes-per-pixel: `bgra8 = 4`, `grey8 = 1`. Total length MUST equal
  `12 + width * bpp * height`; anything else is a typed refusal, never a
  partial read.
- The **same** `image-error` case names as A1's result arm (dimension-overflow,
  allocation-limit, stride-mismatch, region-out-of-bounds, unsupported-format)
  are returned as the `Result`'s `Text` arm on a malformed input — so input
  refusals and result refusals share one closed vocabulary, no new error type.
- **Refusal mapping (fail closed, limit+1 tested)**:
  - `format ∉ {0,1}` → `unsupported-format`;
  - `width == 0 || height == 0` → `stride-mismatch` (a zero-axis bitmap is
    degenerate for the v0 roster ops; the empty/`min-area` limit+1 degenerate
    cases stay per §Summary);
  - `width * bpp * height` overflows u64 → `dimension-overflow`;
  - pixel payload length `≠ width*bpp*height` → `stride-mismatch`;
  - payload bytes `> 2^28` (RFC-0041 D4 ceiling) → `allocation-limit`.
- **No semantic change** to any algorithm: A2 fixes only the input *wire
  framing*; SAD template match, deterministic row-major scan, ties by lowest
  `(y,x)`, integer-only, and region containment (RFC-0041/§A4) are unchanged.

## 3. Alternatives (the owner may choose one instead)

- **A2-alt — drop the packed input entirely and follow the shipped `image-vision@1`
  precedent**: pass dimensions as explicit scalar params, pixels as raw `Bytes`
  (image-vision's four ops are `func(pixels: Bytes, width: U64, height: U64)`).
  Roster signatures become, e.g.,
  `template(pixels: Bytes, pw: U64, ph: U64, tpl: Bytes, tw: U64, th: U64) -> Result[Bytes, Text]`.
  Zero header invention, reuses an already-accepted input shape, and every
  param is already in the E8017 set. Downside: more params, and the format
  (grey8 vs bgra8) must be fixed per interface-version or carried as a `U64`
  format tag rather than a 1-byte in-band flag.
- **A2-b — widen the v0 value set to accept `bitmap`/`region` records directly**
  (= fold this into the heavier RFC-0043-A2 "widen E8017" route, new ADR +
  `interface_value_offender` change). Same blast radius rejected for A1 §3;
  listed only for completeness.

**A2 (recommended)** because it keeps the accepted A1 signature shape
(`bmp: Bytes`), needs no signature reshaping, and copies a header convention the
codec already proved byte-exact. **A2-alt** is the fallback if the owner prefers
to avoid any in-band header and mirror image-vision's scalar-dims surface exactly.

## 4. What A2 does NOT change

- RFC-0041 `record bitmap`/`region` definitions and `from-rows`/`crop` semantics.
- RFC-0043 determinism, exact-integer tolerance policy, `limit+1` typed
  refusals, pure-package authority, and the content-asserting corpus
  requirement (independent numpy oracle, element contents not counts).
- A1's frozen **result** encoding (`region` → 16 B LE, `list<region>` →
  count-prefixed, error → case-name `Text`) — A2 only completes the input half.
- M24 §3 gate 3 stays **未达** until A2 (or the chosen alternative) is accepted
  **and** all three packages land their own corpus + STEP. This draft flips
  nothing.

## 5. Re-adjudication requested (owner) — **RESOLVED 2026-10-01: owner accepted A2**

Freeze the roster **input** `Bytes` framing: **A2** self-describing
`width/height/format/zero-pad/pixels` header (recommended, mirrors the accepted
image-codec payload) or **A2-alt** scalar-dims + raw-pixels signature (mirrors
the shipped `image-vision@1`). Until one is accepted the three roster packages
cannot be built as pure packages with a *frozen* input contract, gate 3 stays
未达, and I will not implement against or silently invent a bitmap wire format.
If accepted, the three slices mirror #2 PNG exactly: an ADR-0019 Rust core + an
independent numpy-oracle corpus + runner content tests, one STEP each.
