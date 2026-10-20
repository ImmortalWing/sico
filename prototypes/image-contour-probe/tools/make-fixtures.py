#!/usr/bin/env python3
"""M24 gate-3 roster `image-contour@1` independent fixture oracle.

FIXED, INDEPENDENT oracle for the `sico:user/image-contour@1` `find` core
(RFC-0043 §Summary, input frozen by Amendment A2, list result by Amendment A1).
It shares no code with the Rust guest core: it re-implements the A2 input
decode, the frozen grey8 4-connected component labeling (foreground `>= 128`,
closed external region -> bounding box, row-major seed order, area `>= min-area`
filter) and the A1 `list<region>` encoding from the contract text alone, then
emits a frozen `manifest.json` corpus (schema
`sico.m24.roster-contour-fixtures.v0`) that the Rust runner test reads
(`tests/corpus/image-contour/manifest.json`). Core and oracle agreeing byte-for-
byte is the content proof.

numpy is absent offline; this is stdlib-only Python (RFC-0043 §Corpora names
"Python/numpy" as the independent-oracle pattern; a pure-Python fixed oracle
satisfies it). Usage: python make-fixtures.py <output-manifest-path>
"""
import json
import struct
import sys

MAX_PIXEL_BYTES = 1 << 28
U64_MAX = (1 << 64) - 1
BPP = {0: 4, 1: 1}
FOREGROUND_MIN = 128


def pack_bitmap(width, height, fmt, pixels):
    return struct.pack("<II", width, height) + bytes([fmt, 0, 0, 0]) + pixels


def decode_bitmap(b):
    """Mirror the Rust core's frozen A2 check order; ('ok', tup) or ('err', c)."""
    if len(b) < 12:
        return ("err", "stride-mismatch")
    width, height = struct.unpack("<II", b[0:8])
    fmt = b[8]
    if b[9] != 0 or b[10] != 0 or b[11] != 0:
        return ("err", "stride-mismatch")
    bpp = BPP.get(fmt)
    if bpp is None:
        return ("err", "unsupported-format")
    if width == 0 or height == 0:
        return ("err", "stride-mismatch")
    wh = width * height
    if wh > U64_MAX:
        return ("err", "dimension-overflow")
    pb = wh * bpp
    if pb > U64_MAX:
        return ("err", "dimension-overflow")
    if pb > MAX_PIXEL_BYTES:
        return ("err", "allocation-limit")
    if len(b) != 12 + pb:
        return ("err", "stride-mismatch")
    return ("ok", (width, height, fmt, b[12:]))


def list_blob(regions):
    out = struct.pack("<I", len(regions))
    for r in regions:
        out += struct.pack("<IIII", *r)
    return out


def find(b, min_area):
    """Independent find; ('ok', list_bytes) or ('err', case).

    4-connected flood fill (mark-on-push, explicit stack). Bounding box and area
    are set properties of the component, so stack pop order cannot change the
    emitted region; the seed scan is row-major, so components emit in
    lowest-(y,x) seed order — matching the Rust core byte-for-byte.
    """
    d = decode_bitmap(b)
    if d[0] == "err":
        return d
    width, height, fmt, px = d[1]
    if fmt != 1:                      # grey8-only op
        return ("err", "unsupported-format")
    total = width * height
    visited = [False] * total
    regions = []
    for seed in range(total):
        if px[seed] < FOREGROUND_MIN or visited[seed]:
            continue
        sx, sy = seed % width, seed // width
        minx = maxx = sx
        miny = maxy = sy
        area = 0
        stack = [seed]
        visited[seed] = True
        while stack:
            idx = stack.pop()
            area += 1
            x, y = idx % width, idx // width
            if x < minx:
                minx = x
            if x > maxx:
                maxx = x
            if y < miny:
                miny = y
            if y > maxy:
                maxy = y
            if x > 0:
                n = idx - 1
                if not visited[n] and px[n] >= FOREGROUND_MIN:
                    visited[n] = True
                    stack.append(n)
            if x + 1 < width:
                n = idx + 1
                if not visited[n] and px[n] >= FOREGROUND_MIN:
                    visited[n] = True
                    stack.append(n)
            if y > 0:
                n = idx - width
                if not visited[n] and px[n] >= FOREGROUND_MIN:
                    visited[n] = True
                    stack.append(n)
            if y + 1 < height:
                n = idx + width
                if not visited[n] and px[n] >= FOREGROUND_MIN:
                    visited[n] = True
                    stack.append(n)
        if area >= min_area:
            regions.append((minx, miny, maxx - minx + 1, maxy - miny + 1))
    return ("ok", list_blob(regions))


ON, OFF = 255, 0


def grey(width, height, vals):
    assert len(vals) == width * height
    return pack_bitmap(width, height, 1, bytes(vals))


def bgra(width, height):
    return pack_bitmap(width, height, 0, bytes(width * height * 4))


def ok_case(name, b, min_area):
    res = find(b, min_area)
    assert res[0] == "ok", f"{name}: expected ok, got {res}"
    return {"name": name, "bmp_hex": b.hex(), "min_area": min_area,
            "expect": {"kind": "ok", "list_hex": res[1].hex()}}


def err_case(name, b, min_area, case):
    res = find(b, min_area)
    assert res == ("err", case), f"{name}: oracle produced {res}, wanted {case}"
    return {"name": name, "bmp_hex": b.hex(), "min_area": min_area,
            "expect": {"kind": "err", "case": case}}


def main():
    cases = []

    # --- ok ---------------------------------------------------------------
    # one solid 3x3 block at rows 1..3, cols 1..3 of a 5x5 raster.
    block3 = [OFF] * 5 + [OFF, ON, ON, ON, OFF] + [OFF, ON, ON, ON, OFF] \
        + [OFF, ON, ON, ON, OFF] + [OFF] * 5
    cases.append(ok_case("single-blob-3x3", grey(5, 5, block3), 1))
    # two horizontal components (area 2 then area 3): min-area boundary inclusive.
    bars = [ON, ON, OFF, ON, ON, ON]
    cases.append(ok_case("two-comps-min-area-boundary", grey(6, 1, bars), 2))
    # same raster, min-area 3 excludes the area-2 component.
    cases.append(ok_case("two-comps-min-area-excludes-small", grey(6, 1, bars), 3))
    # a vertical 1x5 component: bounding box height covers the run.
    cases.append(ok_case("vertical-component", grey(1, 5, [ON] * 5), 1))
    # a closed 5x5 ring (perimeter ON, 3x3 interior OFF): one region, hole does
    # not split it.
    ring = []
    for y in range(5):
        for x in range(5):
            ring.append(ON if (x in (0, 4) or y in (0, 4)) else OFF)
    cases.append(ok_case("ring-hole-single-region", grey(5, 5, ring), 1))
    # diagonal-only touching (2x2 ON at (0,0) and (1,1)): 4-connectivity keeps
    # them two separate regions, emitted in row-major seed order.
    cases.append(ok_case("diagonal-disconnect-4conn", grey(2, 2, [ON, OFF, OFF, ON]), 1))
    # no foreground at all -> count-0 ok.
    cases.append(ok_case("all-off-empty", grey(4, 4, [OFF] * 16), 1))
    # min-area larger than any component -> count-0 ok (filter, not a refusal).
    cases.append(ok_case("min-area-too-large-empty", grey(6, 1, bars), 100))

    # --- err: A2 decode refusals + the grey8-only rule ---------------------
    cases.append(err_case("bad-format-tag",
                          struct.pack("<II", 4, 4) + bytes([7, 0, 0, 0]), 2,
                          "unsupported-format"))
    cases.append(err_case("bgra8-rejected-grey8-only", bgra(2, 2), 1,
                          "unsupported-format"))
    valid = grey(4, 4, [OFF] * 16)
    cases.append(err_case("reserved-nonzero",
                          valid[:9] + bytes([0xAA]) + valid[10:], 2, "stride-mismatch"))
    cases.append(err_case("too-short-header", grey(4, 4, [ON] * 16)[:11], 2,
                          "stride-mismatch"))
    cases.append(err_case("zero-axis",
                          struct.pack("<II", 0, 4) + bytes([1, 0, 0, 0]), 2,
                          "stride-mismatch"))
    cases.append(err_case("length-mismatch", grey(4, 4, [ON] * 16) + b"\x00\x00", 2,
                          "stride-mismatch"))
    big = struct.pack("<II", 1, (1 << 28) + 1) + bytes([1, 0, 0, 0])
    cases.append(err_case("allocation-limit", big, 2, "allocation-limit"))
    over = struct.pack("<II", (1 << 32) - 1, (1 << 32) - 1) + bytes([0, 0, 0, 0])
    cases.append(err_case("dimension-overflow", over, 2, "dimension-overflow"))

    manifest = {
        "schema": "sico.m24.roster-contour-fixtures.v0",
        "package": "image-contour",
        "function": "find",
        "oracle": "prototypes/image-contour-probe/tools/make-fixtures.py",
        "semantics": "grey8 4-connected components, foreground >= 128, closed "
                     "external region -> bounding box (minx,miny,w,h), row-major "
                     "seed order (ties lowest (y,x)), area >= min-area (inclusive, "
                     "filter); list = count u32 LE + regions (x,y,width,height u32 "
                     "LE); no region -> count 0 ok",
        "cases": cases,
    }
    out = sys.argv[1]
    with open(out, "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"wrote {len(cases)} cases "
          f"({sum(c['expect']['kind']=='ok' for c in cases)} ok + "
          f"{sum(c['expect']['kind']=='err' for c in cases)} err) -> {out}")


if __name__ == "__main__":
    main()
