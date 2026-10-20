# RFC-0050: M24 image codec package — JPEG↔PNG v0 (deterministic, fail-closed)

> - status: **accepted by owner directive 2026-09-30 「完整完成M22-M26」(STEP-0323)**
> - date: 2026-09-25
> - phase: M24 §8.4 conversion-engine contract (first increment JPEG↔PNG)
> - depends: RFC-0041 (image data contract: BGRA8, packed rows, top-left origin, stride = w·4), RFC-0015/0024/0026 (M7 package digest and registry discipline), M24 plan §8.4/§8.5, STEP-0287 inventory

## 1. Summary

Freeze the conversion engine of the GUI format-converter pilot as a
versioned package, **`sico:user/image-codec@1`**, with four pure
functions over the RFC-0041 pixel contract.

> **Amendment A1 (accepted by owner re-adjudication 2026-09-30, STEP-0330).**
> The original §1 declared `record Image` and `enum CodecError`, which fall
> outside the frozen v0 user-WIT value set — `interface_value_offender`
> (RFC-0039 §2.4 A7) refuses records/enums with **E8017**, so the accepted
> surface could not be lowered through the pure-package path. A1 flattens the
> surface to scalars only (below) and adds the `encode-buffer-mismatch` error
> case to §3. See
> [`RFC-0050-amendment-A1-flattened-surface`](./RFC-0050-amendment-A1-flattened-surface.md).
> §2 profiles, §3 limits, determinism and the §4 pure-package implementation
> home are unchanged.

```text
decode-jpeg(data: Bytes)                             -> Result[Bytes, Text]
decode-png(data: Bytes)                              -> Result[Bytes, Text]
encode-png(pixels: Bytes, width: U64, height: U64)   -> Result[Bytes, Text]
encode-jpeg(pixels: Bytes, width: U64, height: U64)  -> Result[Bytes, Text]

// decode ok payload (Bytes): 16-byte little-endian header
//   width u64 LE | height u64 LE
// followed by RFC-0041 BGRA8 pixels (packed rows, top-left origin,
// stride = width*4). The former `record Image` rides in-band.
// error (Text): the §3 case name (HTTP case-name precedent); the
// discriminant is the Result arm, so no enum crosses the boundary.
```

Determinism is the contract, not an implementation detail: for a fixed
input, `encode-png` and `encode-jpeg` outputs are **byte-stable**. The
deterministic reference is the only path in v0 — no acceleration, no
parallel variants. Consumers install through the M7 registry path with
zero compiler/Runtime changes (the image-vision@1 pattern).

## 2. Accepted decode profiles (v0; everything else typed-refused)

**JPEG (decode):** baseline sequential DCT (SOF0), Huffman coding, 8-bit
precision; YCbCr with 4:4:4 / 4:2:2 / 4:2:0 sampling and 8-bit grayscale;
restart markers accepted. **Refused:** progressive (SOF2), arithmetic
coding, 12-bit, lossless (SOF3) and hierarchical (SOF5–SOF7) processes,
CMYK/YCCK color (v0), EXIF embedding is ignored — the decoded raster is
the raw coded raster, **EXIF orientation is NOT applied** (recorded so
the corpus pins orientation-neutral behavior; applying it would be a v1
decision with its own corpus). ICC profiles and thumbnails ignored.

**PNG (decode):** bit depth 8 only; color types
greyscale-8 / greyscale+alpha-8 / truecolor-RGB-8 / truecolor+alpha-RGB-8;
all five standard row filters on decode (they are part of the format);
`tRNS` transparent-color chunks are **refused** in v0 (avoid silent
keying). **Refused:** interlace (Adam7), bit depths 1/2/4/16, 16-bit,
palette color type (v0).

**Encode (both):** `encode-png` emits color-type RGBA-8, non-interlaced,
every scanline filter type 4 (Paeth), zlib level 9 fixed, and **only**
IHDR/IDAT/IEND chunks (no ancillary chunks of any kind). `encode-jpeg`
emits baseline sequential (SOF0) Huffman, quality **90** on the standard
IS tables, **4:4:4** (no chroma subsampling), no EXIF/ICC/JFIF thumbnail
emission beyond the required JFIF header. These settings are frozen by
name here and pinned byte-exactly by the corpus; changing any of them is
a new package version.

## 3. Limits and failure surface (all typed; no trap, hang, partial output)

- Declared limits, checked from headers **before** pixel allocation:
  dimension bound and pixel-count bound (`width·height ≤ 2^28`) — the
  +1 cases are typed refusals, never OOM.
- `CodecError` cases (v0 closed set, delivered as case-name `Text` per A1):
  `bad-signature`, `truncated`, `trailing-data`, `oversized`,
  `unsupported-profile`, `corrupt-stream`, `encode-buffer-mismatch`
  (A1: the supplied pixel buffer length != `width·height·4` for the declared
  dimensions; decode-side stream corruption keeps `corrupt-stream`).
  Each decode failure consumes only its input; nothing is written.
- Cancellation mid-decode/encode yields the runner's typed cancellation
  outcome; the pure functions never emit partial files. Writing the
  result to disk is the existing file capability with its typed save
  failures (M24 §8.4 persistence row).
- Repeat conversion of the same input is byte-identical (§1 determinism);
  the §8.5 corpus pins this per fixture.

## 4. Compatibility and honesty gates

- Additive: no existing surface changes; image-vision@1 is untouched and
  remains pixel-processing-only.
- The implementation STEP must land the §8.5 refusal corpus line by line
  (bad magic, truncated, trailing data, oversized limit+1, unsupported
  color/JPEG profile, cancellation, save failure, byte-stable repeat,
  batch sampling) — every case a typed outcome, verified through the real
  runner, plus the machine support matrix row and validator.
- Implementation home: versioned package built on the M7 path; the
  implementation STEP selects vetted codec crates or in-house code inside
  the package implementation — the **frozen corpus is the oracle** for
  every profile decision above, and `sico check`/static conformance never
  substitutes for decode/encode execution.
- Non-goals (v0): progressive JPEG, CMYK, palette/16-bit/interlaced PNG,
  metadata preservation, EXIF orientation, resizing/scaling, formats
  beyond JPEG/PNG, batch APIs (the GUI pilot loops in Sico), any
  acceleration.
