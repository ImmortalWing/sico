# RFC-0041: Portable image data contract v0

> - status: accepted
> - accepted: 2026-09-10 (owner session directive "完成M15-17"; D1–D4 accepted as drafted, D4 initial ceiling 2^28 bytes)
> - date: 2026-09-08
> - phase: M17 contract track
> - depends: RFC-0012 (Component/WIT boundary), M16 plan §3 (`screen.capture` commits to owned stride/format buffers), M17 plan §3.1/§4

## Summary

Freeze the portable image/pixel data contract that M17's deterministic CV
packages, M16 capture frames, and any accelerated provider share. One
image definition; explicit decisions on the three open axes (packing,
orientation, color-space conversion); allocation ceilings and typed
failure for every limit+1 case.

## 1. Data contract (candidate, frozen only at acceptance)

```wit
package sico:image@0.1.0;

interface image {
    enum pixel-format { bgra8, grey8 }
    enum orientation { top-left }   // v0: normalized layout, one variant

    /// Owned, immutable pixel buffer. Packed layout by decision D1.
    record bitmap {
        width: u32,
        height: u32,
        stride: u32,          // bytes; >= width * bytes-per-pixel
        format: pixel-format,
        orientation: orientation,
        pixels: list<u8>,
    }

    /// Bounded region in pixel coordinates; construction checks bounds.
    record region { x: u32, y: u32, width: u32, height: u32 }

    // Construction validates: pixels.len() bounds vs stride*height,
    // region containment, and the allocation ceiling below. Every
    // violation is a typed error, never truncation.
    from-rows: func(width: u32, height: u32, stride: u32,
                    format: pixel-format, pixels: list<u8>)
        -> result<bitmap, image-error>;
    crop: func(source: borrow<bitmap>, region: region)
        -> result<bitmap, image-error>;

    enum image-error {
        dimension-overflow,   // width*height/stride arithmetic overflow
        allocation-limit,     // exceeds the ceiling below
        stride-mismatch,      // pixels.len() inconsistent with stride*height
        region-out-of-bounds,
        unsupported-format,
    }
}
```

## 2. Decisions the acceptance freezes

- **D1 — packed (interleaved) layout is the v0 default.** Planar forms
  stay outside v0; conversion, if ever needed, is an explicit versioned
  package operation.
- **D2 — orientation is normalized metadata**: v0 buffers are always
  top-left; the field exists so future variants fail loudly instead of
  silently flipping pixels.
- **D3 — color-space conversion is an explicit operation**, never implicit
  normalization (house rule: no hidden conversion). grey8↔bgra8 conversion
  is one of the §3.2 deterministic packages with its own tolerance policy.
- **D4 — allocation ceiling**: a Host-enforced maximum pixel count and
  byte size per bitmap (initial default: 2^28 bytes ≈ 256 MiB, tuned by
  measurement at implementation); construction beyond it is
  `allocation-limit`, leaving the Host reusable.

## 3. Alignment with neighbors

- M16 capture frames (`RFC-0040 capture.frame`) convert to `bitmap`
  without a second image definition — bgra8 and stride are shared shapes.
- Deterministic CV packages (M17 plan §3.2) consume `bitmap` + `region`
  only; no package may widen authority or read ambient buffers.
- Canonical serialization is out of scope until a measured consumer needs
  it (copying must be explicit; no hidden zero-copy across the Component
  boundary).

## 4. Exit corpus (binding at acceptance)

1. Construction limit+1: every `image-error` case has a fixture.
2. Round-trip: crop-of-crop determinism, byte-exact, across platforms.
3. Tolerance policy: grey8 conversion is exact-integer (documented
   rounding), stated per package, never "approximate".
4. Consumer pilot: the tetris recognition chain consumes `bitmap` from
   M16 capture frames and from synthetic fixtures with identical results
   (the M14 content-assertion lesson applies: element contents, not
   counts).

## 5. Non-goals

- Planar formats, HDR, color management profiles, EXIF/metadata handling.
- GPU or accelerated paths (M17 plan §3.3 has its own ADR).
- Any ambient authority for image packages.
