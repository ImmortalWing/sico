#!/usr/bin/env python3
"""M22 P2 (STEP-0550): the real closure bounds of the frozen `match` sites.

The frozen ledger records `closure-too-large:<n>fn/<b>B` as the site's verdict
when the **same-file dependency closure** of the enclosing function grows past
the firmware's bound.  That verdict is produced *at the moment the bound
trips*, so the recorded `<n>fn/<b>B` is a **lower bound** of the closure, not
its real size: the traversal would have kept growing had the bound not stopped
it.

STEP-0547 §4 established that the bound (64 functions / 262,144 bytes) is a
**fixture self-limit**, not a guest capability boundary: 219 of the 396
candidate sites were never opened on the Component axis at all.  STEP-0550
(P2) lifts the bound, and this script is the **pre-flight measurement** that
says which bound is actually needed -- without paying for a multi-hour
compilation first.

It mirrors the firmware algorithm exactly:

* `runner/sico-runner/tests/selfhost_compiler.rs`
  - `match_sico_functions`  (function extraction)
  - `match_sico_callees`    (same-file call detection)
  - `match_sico_closure`    (transitive closure, file order)

The frozen sources are LF-only (verified: no CRLF in `selfhost/*.sico`), so
Python's universal-newline translation is a no-op and the byte counts are the
same numbers the Rust side measures.

Usage:
    python tools/probe-m22-closure-bounds.py
    python tools/probe-m22-closure-bounds.py --json
    python tools/probe-m22-closure-bounds.py --limits 128 1048576
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
SOURCE_DIRECTORY = REPO / "selfhost"

def _load_census():
    """The census script's name is not a module name, so load it by path."""
    import importlib.util

    path = pathlib.Path(__file__).resolve().parent / "census-m22-match-sites.py"
    spec = importlib.util.spec_from_file_location("census_m22_match_sites", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


enumerate_sites = _load_census().enumerate_sites

LEDGER = SOURCE_DIRECTORY / "match-component-coverage-v1.json"

# The firmware's current bound (selfhost_compiler.rs, STEP-0550).  The old
# bound was 64 functions / 262,144 bytes; pass
# `--limits 64 262144` to reproduce the pre-P2 skip set.
CURRENT_FUNCTION_LIMIT = 256
CURRENT_BYTE_LIMIT = 1_048_576


def read_source(name: str) -> str:
    return (SOURCE_DIRECTORY / name).read_text(encoding="utf-8")


def sico_functions(text: str) -> list[tuple[str, str]]:
    """The firmware's `match_sico_functions`: (name, text) in file order."""
    lines = text.split("\n")
    out: list[tuple[str, str]] = []
    index = 0
    while index < len(lines):
        trimmed = lines[index].lstrip()
        if trimmed.startswith("function "):
            rest = trimmed[len("function ") :]
            name = ""
            for character in rest:
                if character.isascii() and (character.isalnum() or character == "_"):
                    name += character
                else:
                    break
            end = index
            while end < len(lines) and lines[end].strip() != "end function":
                end += 1
            if end < len(lines) and name:
                out.append((name, "\n".join(lines[index : end + 1])))
                index = end + 1
                continue
        index += 1
    return out


def sico_callees(text: str, known: set[str]) -> list[str]:
    """The firmware's `match_sico_callees`: same-file calls, in first-seen order."""
    found: list[str] = []
    for index, character in enumerate(text):
        if character != "(":
            continue
        start = index
        while start > 0:
            byte = text[start - 1]
            if byte.isascii() and (byte.isalnum() or byte == "_"):
                start -= 1
            else:
                break
        if start == index or start == 0:
            continue
        before = text[start - 1]
        if before == "." or (before.isascii() and (before.isalnum() or before == "_")):
            continue
        name = text[start:index]
        if name in known and name not in found:
            found.append(name)
    return found


def sico_closure(text: str, root: str) -> tuple[int, int]:
    """The firmware's `match_sico_closure`, without the bound.

    Returns (function count, byte count) of the **full** transitive closure.
    """
    functions = sico_functions(text)
    names = {name for name, _ in functions}
    selected = [False] * len(functions)
    stack = [root]
    count = 0
    size = 0
    while stack:
        name = stack.pop()
        position = next(
            (i for i, (candidate, _) in enumerate(functions) if candidate == name), None
        )
        if position is None or selected[position]:
            continue
        selected[position] = True
        count += 1
        size += len(functions[position][1])
        for callee in sico_callees(functions[position][1], names):
            stack.append(callee)
    return count, size


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="machine readable output")
    parser.add_argument(
        "--limits",
        nargs=2,
        type=int,
        metavar=("FUNCTIONS", "BYTES"),
        help="simulate the firmware bound and report who still trips it",
    )
    arguments = parser.parse_args()

    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    recorded: dict[tuple[str, str], str] = {}
    for site in ledger["sites"]:
        recorded.setdefault((site["path"], site["function"]), site["verdict"])

    groups: dict[tuple[str, str], int] = {}
    for site in enumerate_sites():
        key = (str(site["path"]), str(site["function"]))
        groups[key] = groups.get(key, 0) + 1

    sources: dict[str, str] = {}
    rows: list[dict[str, object]] = []
    for (path, function), sites in sorted(groups.items()):
        if path not in sources:
            sources[path] = read_source(path)
        count, size = sico_closure(sources[path], function)
        rows.append(
            {
                "path": path,
                "function": function,
                "sites": sites,
                "functions": count,
                "bytes": size,
                "recorded_verdict": recorded.get((path, function), "pending"),
            }
        )

    rows.sort(key=lambda row: (-int(row["bytes"]), str(row["path"])))

    limit_functions, limit_bytes = (
        arguments.limits if arguments.limits else (CURRENT_FUNCTION_LIMIT, CURRENT_BYTE_LIMIT)
    )

    still_too_large = [
        row
        for row in rows
        if int(row["functions"]) > limit_functions or int(row["bytes"]) > limit_bytes
    ]
    sites_too_large = sum(int(row["sites"]) for row in still_too_large)

    measured_now = [
        row
        for row in rows
        if int(row["functions"]) <= limit_functions and int(row["bytes"]) <= limit_bytes
    ]
    sites_measured_now = sum(int(row["sites"]) for row in measured_now)

    if arguments.json:
        print(
            json.dumps(
                {
                    "groups": len(rows),
                    "sites": sum(int(row["sites"]) for row in rows),
                    "limits": {"functions": limit_functions, "bytes": limit_bytes},
                    "measured_groups": len(measured_now),
                    "measured_sites": sites_measured_now,
                    "too_large_groups": len(still_too_large),
                    "too_large_sites": sites_too_large,
                    "max_functions": max(int(row["functions"]) for row in rows),
                    "max_bytes": max(int(row["bytes"]) for row in rows),
                    "rows": rows,
                },
                indent=2,
            )
        )
        return 0

    print(f"groups={len(rows)} sites={sum(int(row['sites']) for row in rows)}")
    print(
        f"limit={limit_functions}fn/{limit_bytes}B  "
        f"measured={len(measured_now)} groups / {sites_measured_now} sites  "
        f"too-large={len(still_too_large)} groups / {sites_too_large} sites"
    )
    print(
        f"max closure = {max(int(row['functions']) for row in rows)}fn / "
        f"{max(int(row['bytes']) for row in rows)}B"
    )
    print()
    print(f"{'sites':>5} {'fn':>4} {'bytes':>9}  {'path':<20} function")
    for row in rows[:40]:
        print(
            f"{int(row['sites']):>5} {int(row['functions']):>4} {int(row['bytes']):>9}  "
            f"{row['path']:<20} {row['function']}"
        )
    if still_too_large:
        print()
        print("still too large under the given bound:")
        for row in still_too_large:
            print(
                f"  {int(row['sites']):>4} sites  {int(row['functions'])}fn/"
                f"{int(row['bytes'])}B  {row['path']}:{row['function']}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
