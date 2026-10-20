#!/usr/bin/env python3
"""M24 gate-3 roster `image-match@1` independent fixture oracle.

This is the FIXED, INDEPENDENT oracle for the `sico:user/image-match@1` package
core (RFC-0043 §Summary as frozen by Amendment A1 for the result and A2 for the
input). It shares no code with the Rust guest core: it re-implements the A2
input decode, the deterministic integer SAD template scan (row-major, ties by
lowest (y,x)), and the A1 region encoding from the contract text alone, then
emits a frozen `manifest.json` content corpus that the Rust runner test reads
(`tests/corpus/image-match/manifest.json`). If core and oracle disagree on any
fixture the test fails -- that agreement IS the content proof.

numpy is not installed offline (M24 host), so this is stdlib-only Python; the
corpus is tiny integer windows that plain loops handle exactly. RFC-0043
§Corpora names "Python/numpy" as the independent-oracle pattern; a pure-Python
fixed oracle satisfies it (a different language, independent of the Rust core).

Usage: python make-fixtures.py <output-manifest-path>
"""
import json
import struct
import sys

MAX_PIXEL_BYTES = 1 << 28
U64_MAX = (1 << 64) - 1
BPP = {0: 4, 1: 1}  # 0 = bgra8, 1 = grey8


def pack_bitmap(width, height, fmt, pixels):
    """A2 self-describing packed header + tightly-packed pixels."""
    return struct.pack("<II", width, height) + bytes([fmt, 0, 0, 0]) + pixels


def decode_bitmap(b):
    """Mirror the Rust core's frozen A2 check order; return ('ok', tup) or
    ('err', case). Order is load-bearing: it selects the single returned case."""
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


def sad_template(b, t):
    """Independent SAD match; ('ok', region_bytes) or ('err', case)."""
    db = decode_bitmap(b)
    if db[0] == "err":
        return db
    dt = decode_bitmap(t)
    if dt[0] == "err":
        return dt
    bw, bh, bf, bpx = db[1]
    tw, th, tf, tpx = dt[1]
    if bf != tf:
        return ("err", "unsupported-format")
    if tw > bw or th > bh:
        return ("err", "region-out-of-bounds")
    bpp = BPP[bf]
    best_x = best_y = 0
    best_sad = None
    for y in range(bh - th + 1):          # row-major: y outer
        for x in range(bw - tw + 1):      # x inner; strict < keeps first min
            sad = 0
            for dy in range(th):
                for dx in range(tw):
                    bi = ((y + dy) * bw + x) * bpp + dx * bpp
                    ti = (dy * tw + dx) * bpp
                    for c in range(bpp):
                        sad += abs(bpx[bi + c] - tpx[ti + c])
            if best_sad is None or sad < best_sad:
                best_sad = sad
                best_x, best_y = x, y
    region = struct.pack("<IIII", best_x, best_y, tw, th)
    return ("ok", region)


def window_bitmap(width, height, fmt, origin_x, origin_y, tw, th, pixels):
    """Extract a (tw x th) sub-window at (origin_x, origin_y) as a standalone
    A2 bitmap."""
    bpp = BPP[fmt]
    out = bytearray()
    for dy in range(th):
        for dx in range(tw):
            sx = origin_x + dx
            sy = origin_y + dy
            base = (sy * width + sx) * bpp
            out += pixels[base:base + bpp]
    return pack_bitmap(tw, th, fmt, bytes(out))


def grey(width, height, fn):
    px = bytearray()
    for y in range(height):
        for x in range(width):
            px.append(fn(x, y) & 0xFF)
    return pack_bitmap(width, height, 1, bytes(px))


def bgra(width, height, fn):
    px = bytearray()
    for y in range(height):
        for x in range(width):
            r, g, b, a = fn(x, y)
            px += bytes([b & 0xFF, g & 0xFF, r & 0xFF, a & 0xFF])
    return pack_bitmap(width, height, 0, bytes(px))


def raw(bmp):
    return bmp


def ok_case(name, bmp, tpl):
    res = sad_template(bmp, tpl)
    assert res[0] == "ok", f"{name}: expected ok, got {res}"
    return {"name": name, "bmp_hex": bmp.hex(), "tpl_hex": tpl.hex(),
            "expect": {"kind": "ok", "region_hex": res[1].hex()}}


def err_case(name, bmp, tpl, case):
    res = sad_template(bmp, tpl)
    assert res == ("err", case), f"{name}: oracle produced {res}, wanted {case}"
    return {"name": name, "bmp_hex": bmp.hex(), "tpl_hex": tpl.hex(),
            "expect": {"kind": "err", "case": case}}


def main():
    # --- ok: a distinct grey8 window that appears exactly once (SAD 0) -------
    bmp = grey(8, 6, lambda x, y: (x * 31 + y * 53) % 251)  # non-repeating
    tpl = window_bitmap(8, 6, 1, 3, 2, 3, 2, bmp[12:])
    cases = [ok_case("grey-exact-offset", bmp, tpl)]

    # --- ok: best-of-several, nonzero min SAD (template = perturbed window) --
    perturbed = bytearray(bmp[12:])
    # template taken from (2,1) 3x2, then shift one pixel by +1 -> min SAD 1
    tpl_src = window_bitmap(8, 6, 1, 2, 1, 3, 2, bmp[12:])
    tpx = bytearray(tpl_src[12:])
    tpx[0] = (tpx[0] + 1) & 0xFF
    cases.append(ok_case("grey-best-nonzero",
                         bmp, pack_bitmap(3, 2, 1, bytes(tpx))))

    # --- ok: uniform image + uniform template -> SAD 0 everywhere -> tie, the
    #         row-major first (lowest (y,x)) must be (0,0) --------------------
    uniform_bmp = grey(5, 4, lambda x, y: 100)
    uniform_tpl = grey(2, 2, lambda x, y: 100)
    cases.append(ok_case("grey-tie-lowest-yx", uniform_bmp, uniform_tpl))

    # --- ok: bgra8 exact window ---------------------------------------------
    cbmp = bgra(6, 5, lambda x, y: (x * 40 % 256, y * 60 % 256, (x + y) * 20 % 256, 255))
    ctpl = window_bitmap(6, 5, 0, 2, 1, 3, 2, cbmp[12:])
    cases.append(ok_case("bgra-exact-offset", cbmp, ctpl))

    # --- ok: bgra8 best with a perturbed channel ----------------------------
    ctpx = bytearray(ctpl[12:])
    ctpx[1] = (ctpx[1] + 3) & 0xFF  # shift green of first template pixel
    cases.append(ok_case("bgra-best-nonzero",
                         cbmp, pack_bitmap(3, 2, 0, bytes(ctpx))))

    # --- err: every RFC-0041 image-error case reachable via `template` -------
    cases.append(err_case("bad-format-tag",
                          grey(4, 4, lambda x, y: 10)[:8] + bytes([7, 0, 0, 0]),
                          tpl, "unsupported-format"))
    reserved = grey(4, 4, lambda x, y: 10)
    cases.append(err_case("reserved-nonzero",
                          reserved[:9] + bytes([0xAA]) + reserved[10:],
                          tpl, "stride-mismatch"))
    cases.append(err_case("too-short-header",
                          grey(4, 4, lambda x, y: 10)[:11],
                          tpl, "stride-mismatch"))
    cases.append(err_case("zero-axis",
                          struct.pack("<II", 0, 4) + bytes([1, 0, 0, 0]),
                          tpl, "stride-mismatch"))
    cases.append(err_case("length-mismatch",
                          grey(4, 4, lambda x, y: 10) + b"\x00\x00",
                          tpl, "stride-mismatch"))
    # allocation-limit: declared pixel payload > 2^28 (height just over the cap)
    big = struct.pack("<II", 1, (1 << 28) + 1) + bytes([1, 0, 0, 0])
    cases.append(err_case("allocation-limit", big, tpl, "allocation-limit"))
    # dimension-overflow: width*height*bpp overflows u64 (both dims 2^32-1, bgra8)
    over = struct.pack("<II", (1 << 32) - 1, (1 << 32) - 1) + bytes([0, 0, 0, 0])
    cases.append(err_case("dimension-overflow", over, tpl, "dimension-overflow"))
    # format mismatch between bmp (bgra8) and tpl (grey8) -> unsupported-format
    cases.append(err_case("format-mismatch",
                          bgra(4, 4, lambda x, y: (1, 2, 3, 4)), uniform_tpl,
                          "unsupported-format"))
    # template larger than image -> region-out-of-bounds
    small_bmp = grey(2, 2, lambda x, y: 5)
    big_tpl = grey(3, 3, lambda x, y: 5)
    cases.append(err_case("template-too-large", small_bmp, big_tpl,
                          "region-out-of-bounds"))

    manifest = {
        "schema": "sico.m24.roster-fixtures.v0",
        "package": "image-match",
        "function": "template",
        "oracle": "prototypes/image-match-probe/tools/make-fixtures.py",
        "note": "region_hex = 16B A1 encoding x,y,width,height u32 LE; inputs "
                "are A2 self-describing packed Bytes; expected outputs computed "
                "independently in pure Python, no code shared with the core.",
        "cases": cases,
    }
    out = sys.argv[1]
    with open(out, "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"wrote {len(cases)} cases ({sum(c['expect']['kind']=='ok' for c in cases)} ok + "
          f"{sum(c['expect']['kind']=='err' for c in cases)} err) -> {out}")


if __name__ == "__main__":
    main()
