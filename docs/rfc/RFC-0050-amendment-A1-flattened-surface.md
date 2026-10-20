# RFC-0050 Amendment A1: flattened codec surface over the v0 user-WIT value set (PROPOSED)

> - status: **accepted by owner re-adjudication 2026-09-30 (STEP-0330)** — binding form of RFC-0050 §1/§3; RFC-0050 body updated in place with this amendment noted
> - date: 2026-09-30
> - amends: [`RFC-0050`](./RFC-0050-image-codec-jpeg-png-v0.md) §1 (surface), §3 (error set)
> - evidence: `prototypes/image-codec-probe` (measurement-only feasibility probe, Windows x64 `1.98.0-x86_64-pc-windows-gnu`, wasmtime `=47.0.2`, `png =0.18.1`)

## 1. Why this amendment exists (measured defect, not preference)

RFC-0050 §1 as accepted declares:

```text
decode-png(data: Bytes) -> Result[Image, CodecError]
record Image { width: U64, height: U64, pixels: Bytes }
enum CodecError { bad-signature | truncated | trailing-data | oversized | unsupported-profile | corrupt-stream }
```

A `record` and an `enum` are **outside the frozen v0 user-WIT value set**.
`sico-semantics::interface_value_offender` (RFC-0039 §2.4 amendment A7)
accepts only scalars (`Bool/I64/U64/Text/Bytes`), one level of
`List[T]`/`Option[T]` over a scalar, and returns of `Result[ok, error]`
with a **scalar** error; records and enums are refused with **E8017**. So
the accepted §1 surface cannot be lowered through the existing pure-package
path as written — the contract and the executable gate disagree.

Per AGENTS.md §1 ("if documentation disagrees with executable evidence,
keep the lower support claim, record the discrepancy, and repair the
authoritative documents"), this amendment repairs §1 to the flattened form
the probe demonstrates, and records the discrepancy for owner
re-adjudication. It does **not** widen any capability or change §2 profiles,
§3 limits, or the determinism guarantee.

## 2. A1 — flattened surface (scalars only)

```text
decode-jpeg(data: Bytes)            -> Result[Bytes, Text]
decode-png(data: Bytes)             -> Result[Bytes, Text]
encode-png(pixels: Bytes, width: U64, height: U64) -> Result[Bytes, Text]
encode-jpeg(pixels: Bytes, width: U64, height: U64) -> Result[Bytes, Text]
```

- **Decode ok payload (`Bytes`)**: a 16-byte little-endian header
  (`width` u64 LE, then `height` u64 LE) followed by the RFC-0041 BGRA8
  pixel bytes (packed rows, top-left origin, stride = width·4). The former
  `record Image` is thus carried in-band; the header replaces the two scalar
  fields and `pixels` is the remainder.
- **Encode input**: `pixels` is the BGRA8 byte string (no header); `width`
  and `height` ride separate `U64` parameters, so the encoder never parses
  the header and the dimension/buffer check stays explicit.
- **Error (`Text`)**: the case name from the §3 closed set, delivered as a
  case-name string — the same precedent the HTTP provider uses for its typed
  error cases. The discriminant is the `Result` arm; the `Text` payload is
  the case name, so no `enum` crosses the boundary.

This is exactly the surface the probe's `image-codec-probe@1` package
implements and verifies (9/9 decode cases against an independent Python/zlib
oracle, byte-stable encode, frozen `{IHDR,IDAT,IEND}` chunk set, round-trip
identity, 3/3 typed encode refusals).

## 3. A1 — error-set addition (measured gap)

RFC-0050 §3's closed `CodecError` set has **no case for an encode-side
buffer/dimension mismatch** (`pixels.len() != width·height·4`). The probe hit
this and provisionally mapped it to `corrupt-stream`, flagged in source. A1
adds one case so the refusal is named rather than borrowed:

- `encode-buffer-mismatch` — the supplied pixel buffer length does not equal
  `width·height·4` for the declared dimensions. Typed refusal; nothing is
  written. (Decode-side stream corruption keeps `corrupt-stream`.)

The §3 v0 closed set becomes: `bad-signature`, `truncated`, `trailing-data`,
`oversized`, `unsupported-profile`, `corrupt-stream`, `encode-buffer-mismatch`.

## 4. What A1 does NOT change

- §2 accepted/refused profiles (JPEG baseline/progressive rules, PNG bit
  depth 8 / color types / no interlace / no `tRNS`, frozen encoder settings:
  PNG RGBA-8 Paeth zlib-9 IHDR/IDAT/IEND only; JPEG SOF0 quality 90 4:4:4).
- §3 limits (`width·height ≤ 2^28`, header-checked before allocation,
  limit+1 typed) and the determinism / byte-stability guarantee.
- §4 implementation home: a **versioned pure package** on the M7 path. The
  probe confirms the pure-package route is feasible, so RFC-0050 §1's
  "zero compiler/Runtime changes" claim survives and the codec does **not**
  move to a host provider.

## 5. Re-adjudication record

Owner accepted A1 (flattened scalar surface + `encode-buffer-mismatch`) as the
binding §1/§3 form on 2026-09-30 (STEP-0330 re-adjudication). RFC-0050's body
is updated in place to this form with a pointer to this amendment; the codec
slices (#2/#3) are unblocked. The probe remains measurement-only and moves no
gate by itself.
