#!/usr/bin/env python3
"""M24 gate-3 roster `image-grid@1` independent fixture oracle.

FIXED, INDEPENDENT oracle for the `sico:user/image-grid@1` `detect-lines` core
(RFC-0043 §Summary, input frozen by Amendment A2, list result by Amendment A1).
It shares no code with the Rust guest core: it re-implements the A2 input
decode, the frozen grey8 horizontal-maximal-run detection (foreground `>= 128`,
row-major), and the A1 `list<region>` encoding from the contract text alone,
then emits a frozen `manifest.json` corpus (schema
`sico.m24.roster-grid-fixtures.v0`) that the Rust runner test reads
(`tests/corpus/image-grid/manifest.json`). Core and oracle agreeing byte-for-
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


def detect(b, min_run):
    """Independent detect-lines; ('ok', list_bytes) or ('err', case)."""
    d = decode_bitmap(b)
    if d[0] == "err":
        return d
    width, height, fmt, px = d[1]
    if fmt != 1:                      # grey8-only op
        return ("err", "unsupported-format")
    regions = []
    for y in range(height):           # row-major: y outer, x inner
        x = 0
        while x < width:
            if px[(y * width + x)] >= FOREGROUND_MIN:
                start = x
                while x < width and px[(y * width + x)] >= FOREGROUND_MIN:
                    x += 1
                run_len = x - start
                if run_len >= min_run:
                    regions.append((start, y, run_len, 1))
            else:
                x += 1
    return ("ok", list_blob(regions))


ON, OFF = 255, 0


def grey(width, height, vals):
    assert len(vals) == width * height
    return pack_bitmap(width, height, 1, bytes(vals))


def bgra(width, height):
    return pack_bitmap(width, height, 0, bytes(width * height * 4))


def ok_case(name, b, min_run):
    res = detect(b, min_run)
    assert res[0] == "ok", f"{name}: expected ok, got {res}"
    return {"name": name, "bmp_hex": b.hex(), "min_run": min_run,
            "expect": {"kind": "ok", "list_hex": res[1].hex()}}


def err_case(name, b, min_run, case):
    res = detect(b, min_run)
    assert res == ("err", case), f"{name}: oracle produced {res}, wanted {case}"
    return {"name": name, "bmp_hex": b.hex(), "min_run": min_run,
            "expect": {"kind": "err", "case": case}}


def main():
    cases = []

    # --- ok ---------------------------------------------------------------
    cases.append(ok_case("two-full-hlines", grey(5, 3, [ON]*5 + [OFF]*5 + [ON]*5), 5))
    cases.append(ok_case("single-row-min3", grey(6, 1, [ON, ON, ON, OFF, ON, ON]), 3))
    cases.append(ok_case("single-row-min2", grey(6, 1, [ON, ON, ON, OFF, ON, ON]), 2))
    cases.append(ok_case("min-run-too-large-empty", grey(6, 1, [ON, ON, ON, OFF, ON, ON]), 7))
    cases.append(ok_case("mixed-rows-x-offset",
                         grey(4, 2, [ON, ON, OFF, OFF, OFF, ON, ON, ON]), 2))

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
        "schema": "sico.m24.roster-grid-fixtures.v0",
        "package": "image-grid",
        "function": "detect-lines",
        "oracle": "prototypes/image-grid-probe/tools/make-fixtures.py",
        "semantics": "grey8 maximal horizontal runs, foreground >= 128, run_len "
                     ">= min_run, row-major; list = count u32 LE + regions "
                     "(x,y,width,height u32 LE); no run -> count 0 ok",
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
