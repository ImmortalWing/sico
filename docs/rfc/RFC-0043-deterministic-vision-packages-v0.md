# RFC-0043: Deterministic vision packages v0

> - status: accepted-by-session-directive; roster completed by owner re-adjudication 2026-09-30 (STEP-0330, A4 adds `image-contour@1`); completion evidence still pending per package
> - date: 2026-09-11
> - phase: M17 plan §3.2
> - depends: RFC-0041 (image data contract, accepted), RFC-0039 §2.3/§2.4 (package resolution + user WIT), M7 binary carrying (measured, STEP-0161)
> - Amendment A1 (region): [`Amendment A1 (ACCEPTED)`](./RFC-0043-amendment-A1-region-packed-bytes.md) — the gate-3 roster (`image-match`/`image-grid`/`image-contour`, 3/3 return `region`/`list<region>`) is **rejected by the accepted E8017 package value set** (no `record`, measured STEP-0330 §8). A1 carries `region` in-band as packed `Bytes` (RFC-0041 `region` = 4×u32 = 16 B) + error as case-name `Text`, mirroring the accepted `RFC-0050-A1`. **Accepted by owner re-adjudication 2026-10-01** — this unblocks the three roster packages as pure `sico:user/` packages. Implementation has **not** started; each package still needs its own ADR-0019 core + independent numpy-oracle content corpus + runner content test, one STEP each, so **gate 3 stays 未达 until all three land with byte-exact corpora**. The §Summary/§A4 signatures below are unchanged (A1 fixes only the wire encoding).
> - Amendment A2 (bitmap input): [`Amendment A2 (ACCEPTED)`](./RFC-0043-amendment-A2-bitmap-input-serialization.md) — A1 collapsed the roster **inputs** to bare `Bytes` but left how a `record bitmap` packs into that `Bytes` undefined (RFC-0041 §3 defers canonical serialization). **Owner re-adjudication 2026-10-01 chose A2**: a self-describing packed input header `width u32 LE · height u32 LE · format u8 (0=bgra8/1=grey8) · reserved u8[3]=0 · pixels`, mirroring the accepted image-codec payload-header precedent. With A1 (result) + A2 (input) both frozen, all three roster packages are now buildable as pure `sico:user/` packages; **implementation proceeding now, gate 3 stays 未达 until each package lands its own byte-exact corpus + STEP.**

## Summary

Deterministic CV operations ship as versioned pure packages over the
accepted `sico:image@0.1.0` data contract. The v0 roster:

- `image-threshold@1`: `binarize(bitmap: bitmap, threshold: u8, invert: bool) -> bitmap` (grey8) and `otsu(bitmap: bitmap) -> u8`
- `image-color@1`: `to-grey8(bitmap: bitmap) -> bitmap` (BT.601 luma, exact-integer rounding: floor((299r+587g+114b)/1000))
- `image-match@1`: `template(bmp: bitmap, tpl: bitmap) -> result<region, error>` (SAD, deterministic scan order, ties broken by lowest (y,x))
- `image-grid@1`: `detect-lines(bitmap: bitmap, min-run: u32) -> list<region>`
- `image-contour@1` (A4, owner re-adjudication 2026-09-30 / STEP-0330): `find(bmp: bitmap, min-area: u32) -> list<region>` — external border following on the grey8/threshold raster, deterministic scan order (row-major, ties by lowest (y,x)), integer-only, closed-region output; degenerate (empty / single-pixel / min-area limit+1) inputs are typed refusals or empty lists per the content-asserting corpus. Closes the M24 §3 gate 3 / §8.3 "template match / grid / contour" roster naming against this RFC.

## Contracts

- **Determinism**: same input bytes + same package version → identical
  output bytes on every supported platform. No floating point in v0
  (fixed-point/integer only); any platform-divergent fast path is a
  typed refusal, not a tolerance label.
- **Authority**: packages are pure (RFC-0039 §2.4); no ambient
  filesystem/screen/network access. Bitmaps arrive via the caller.
- **Corpora**: every package ships a fixed input corpus with a
  Python/numpy fixed oracle (M14 pattern) and **content-asserting**
  fixtures (STEP-0131 lesson: element contents, not counts).
- **Tolerance policy**: exact-integer only in v0; "approximate" is not a
  v0 concept.
- **Scaling limits**: dimension caps from RFC-0041's allocation ceiling;
  every limit+1 is a typed error.

## Accepted amendments (implementation, 2026-09-13)

- A1: `image-vision@1` ships four ops — `to-grey8`, `threshold`
  (`threshold(pixels, threshold, pixel_count)`: param 2 = threshold,
  param 3 = pixel count = loop bound), `occupancy` (per-column 255/0),
  and `occupancy-mask` (per-column ASCII '0'/'1' so text-based consumers
  can decode; v0 has no byte-at intrinsic). Mask threshold fixed at 128
  in the mask contract.
- A2: `to-grey8` luma = `(299r + 587g + 114b) >> 10` (denominator 1024,
  exact-integer truncation — i32-safe since 1000·255 < 2^31).
- A3: package cores reserve the low 64 KiB of guest memory for lowered
  arguments (bump allocations otherwise clobber caller-lowered inputs).
- A4 (owner re-adjudication 2026-09-30, STEP-0330): the v0 roster gains
  `image-contour@1` (`find(bmp, min-area) -> list<region>`, external border
  following, integer-only, deterministic row-major scan, ties by lowest
  (y,x)), aligning this RFC with the M24 §3 gate 3 / §8.3 "template match /
  grid / contour" roster. The status line's "pending completion evidence"
  caveat is unchanged: each package still lands its own content-asserting
  corpus and STEP before gate 3 closes.

## Non-goals (v0)

Floating-point CV, ML inference (separate §3.4 contract), GPU paths
(§3.3 ADR), color management (RFC-0041 non-goal).
