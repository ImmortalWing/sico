"""M24 codec probe fixture generator (independent PNG oracle).

Emits hand-built PNG byte streams plus a `manifest.json` describing what
`decode-png` must return for each. The PNGs are written with Python's
`zlib` and manual chunk/CRC framing, so the expected decode payload is
computed by an oracle that shares no code with the `png` crate the guest
links. That independence is the point: if the guest and the fixture agreed
only because they used the same encoder, the probe would prove nothing.

Decode payload shape (measured from guest/src/lib.rs):
    width u64 LE | height u64 LE | BGRA8 pixels (row-major)
so a source RGBA pixel (r, g, b, a) lands as bytes (b, g, r, a).
"""

import json
import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
FIXTURES = os.path.normpath(os.path.join(HERE, "..", "fixtures"))
SIGNATURE = b"\x89PNG\r\n\x1a\n"
MAX_PIXELS = 1 << 28


def chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def ihdr(width, height, bit_depth=8, color_type=6, interlace=0) -> bytes:
    return struct.pack(
        ">IIBBBBB", width, height, bit_depth, color_type, 0, 0, interlace
    )


def idat_from_rgba(width, height, rgba) -> bytes:
    """rgba is a bytes-like of width*height*4 bytes in R,G,B,A order."""
    raw = bytearray()
    stride = width * 4
    for row in range(height):
        raw.append(0)  # filter type 0 (None) per scanline
        raw += rgba[row * stride : (row + 1) * stride]
    return zlib.compress(bytes(raw), 9)


def png(width, height, rgba, color_type=6, bit_depth=8, interlace=0):
    out = bytearray(SIGNATURE)
    out += chunk(b"IHDR", ihdr(width, height, bit_depth, color_type, interlace))
    out += chunk(b"IDAT", idat_from_rgba(width, height, rgba))
    out += chunk(b"IEND", b"")
    return bytes(out)


def bare_ihdr_png(width, height, color_type=6, bit_depth=8, interlace=0):
    """IHDR + IEND only; the guest refuses these on header inspection before
    it ever reaches the pixel decoder, so no IDAT is required."""
    out = bytearray(SIGNATURE)
    out += chunk(b"IHDR", ihdr(width, height, bit_depth, color_type, interlace))
    out += chunk(b"IEND", b"")
    return bytes(out)


def expected_payload(width, height, rgba) -> bytes:
    header = struct.pack("<Q", width) + struct.pack("<Q", height)
    body = bytearray()
    for i in range(0, len(rgba), 4):
        r, g, b, a = rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]
        body += bytes((b, g, r, a))
    return header + bytes(body)


def write(name, data):
    path = os.path.join(FIXTURES, name)
    with open(path, "wb") as handle:
        handle.write(data)
    return name


def main():
    os.makedirs(FIXTURES, exist_ok=True)
    cases = []

    # 1) A real 2x2 RGBA image: the only case that must decode successfully.
    #    Distinct per-channel values so an R/B swap or stride bug cannot pass.
    width, height = 2, 2
    rgba = bytes(
        [
            10, 20, 30, 255,  # (0,0)
            40, 50, 60, 128,  # (1,0)
            70, 80, 90, 0,    # (0,1)
            100, 110, 120, 64,  # (1,1)
        ]
    )
    ok = png(width, height, rgba)
    cases.append(
        {
            "name": "decode-ok-2x2-rgba",
            "file": write("ok-2x2-rgba.png", ok),
            "expect": {
                "kind": "ok",
                "payload_hex": expected_payload(width, height, rgba).hex(),
            },
        }
    )

    # 2) Bad signature.
    bad = bytearray(ok)
    bad[0] = 0x00
    cases.append(
        {
            "name": "decode-bad-signature",
            "file": write("bad-signature.png", bytes(bad)),
            "expect": {"kind": "err", "case": "bad-signature"},
        }
    )

    # 3) Trailing data after IEND.
    cases.append(
        {
            "name": "decode-trailing-data",
            "file": write("trailing-data.png", ok + b"\x00\x00\x00\x00"),
            "expect": {"kind": "err", "case": "trailing-data"},
        }
    )

    # 4) Truncated: drop the IEND chunk entirely (stream ends mid-chunk).
    truncated = ok[: len(SIGNATURE) + len(chunk(b"IHDR", ihdr(width, height)))]
    cases.append(
        {
            "name": "decode-truncated",
            "file": write("truncated.png", truncated),
            "expect": {"kind": "err", "case": "truncated"},
        }
    )

    # 5) Corrupt CRC: flip a byte inside the IDAT data, leaving its CRC stale.
    idat_start = ok.index(b"IDAT") + 4
    corrupt = bytearray(ok)
    corrupt[idat_start] ^= 0xFF
    cases.append(
        {
            "name": "decode-corrupt-crc",
            "file": write("corrupt-crc.png", bytes(corrupt)),
            "expect": {"kind": "err", "case": "corrupt-stream"},
        }
    )

    # 6) limit+1 pixel count: 16384 * 16385 = MAX_PIXELS + 16384 > 2^28.
    #    Refused on header inspection before any allocation, so IDAT-less.
    assert 16384 * 16385 > MAX_PIXELS
    cases.append(
        {
            "name": "decode-oversized-limit-plus-one",
            "file": write("oversized.png", bare_ihdr_png(16384, 16385)),
            "expect": {"kind": "err", "case": "oversized"},
        }
    )

    # 7) Palette (color type 3): outside the v0 accepted profile.
    cases.append(
        {
            "name": "decode-unsupported-palette",
            "file": write("palette.png", bare_ihdr_png(2, 2, color_type=3)),
            "expect": {"kind": "err", "case": "unsupported-profile"},
        }
    )

    # 8) 16-bit depth: outside the v0 accepted profile.
    cases.append(
        {
            "name": "decode-unsupported-depth-16",
            "file": write("depth-16.png", bare_ihdr_png(2, 2, bit_depth=16)),
            "expect": {"kind": "err", "case": "unsupported-profile"},
        }
    )

    # 9) Interlaced: outside the v0 accepted profile.
    cases.append(
        {
            "name": "decode-unsupported-interlaced",
            "file": write("interlaced.png", bare_ihdr_png(2, 2, interlace=1)),
            "expect": {"kind": "err", "case": "unsupported-profile"},
        }
    )

    manifest = {"schema": "sico.m24.codec-fixtures.v0", "decode_cases": cases}
    with open(os.path.join(FIXTURES, "manifest.json"), "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2)
        handle.write("\n")
    print("wrote %d fixtures + manifest.json to %s" % (len(cases), FIXTURES))


if __name__ == "__main__":
    main()
