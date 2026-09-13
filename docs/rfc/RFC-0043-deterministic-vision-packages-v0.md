# RFC-0043: Deterministic vision packages v0

> - status: draft — accepted-by-session-directive pending completion evidence
> - date: 2026-09-11
> - phase: M17 plan §3.2
> - depends: RFC-0041 (image data contract, accepted), RFC-0039 §2.3/§2.4 (package resolution + user WIT), M7 binary carrying (measured, STEP-0161)

## Summary

Deterministic CV operations ship as versioned pure packages over the
accepted `sico:image@0.1.0` data contract. The v0 roster:

- `image-threshold@1`: `binarize(bitmap: bitmap, threshold: u8, invert: bool) -> bitmap` (grey8) and `otsu(bitmap: bitmap) -> u8`
- `image-color@1`: `to-grey8(bitmap: bitmap) -> bitmap` (BT.601 luma, exact-integer rounding: floor((299r+587g+114b)/1000))
- `image-match@1`: `template(bmp: bitmap, tpl: bitmap) -> result<region, error>` (SAD, deterministic scan order, ties broken by lowest (y,x))
- `image-grid@1`: `detect-lines(bitmap: bitmap, min-run: u32) -> list<region>`

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

## Non-goals (v0)

Floating-point CV, ML inference (separate §3.4 contract), GPU paths
(§3.3 ADR), color management (RFC-0041 non-goal).
