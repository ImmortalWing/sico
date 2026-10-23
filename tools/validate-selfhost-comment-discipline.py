#!/usr/bin/env python3
"""Selfhost comment discipline (M22 S5, STEP-0508).

The guest frontend's lexer does not skip `//` comments: every word inside a
comment reaches the word list. The guest's structural scans then count those
words as if they were source tokens, so a comment can change how the module
router classifies the function it sits in -- and a comment *inside* a
function body is read as an unclassifiable statement by the statement
scanner, which refuses the whole module with `ERR:E-SH-IR-STATEMENT`.

STEP-0508 hit exactly that: three comment lines were written inside
`construct_lowering`, and the full `selfhost/parser.sico` self-host
regressed with `ERR:E-SH-IR-STATEMENT`. The offending word was `The`.

The rules, per file, follow how the guest actually lowers that file:

  * `selfhost/parser.sico` is lowered as a whole module, so its comments
    are part of the lowered input: they must live between functions and
    must carry none of the words the structural scans count.
  * `selfhost/codegen.sico` is lowered function by function (the extracted
    function text is the input), so only comments *inside* a function body
    reach the scanner; comments between functions never do.

This validator is read-only: it never rewrites the sources.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# The words the guest's structural scans count. A comment carrying one of
# them as a standalone token changes the module router's classification.
FORBIDDEN = (
    "let", "set", "match", "if", "while", "for", "return", "end", "function",
    "record", "enum", "case", "break", "continue", "use", "module",
)

# file -> whether the guest lowers it as a whole module (so the forbidden
# words matter for comments between functions as well).
FILES = {
    "selfhost/parser.sico": True,
    "selfhost/codegen.sico": False,
}


def function_spans(lines: list[str]) -> list[tuple[int, int]]:
    """The (start, end) line spans of every function body, 0-based."""
    spans: list[tuple[int, int]] = []
    index = 0
    while index < len(lines):
        if lines[index].startswith("function "):
            end = index
            while end < len(lines) and lines[end] != "end function":
                end += 1
            spans.append((index, end))
            index = end
        index += 1
    return spans


def owner(spans: list[tuple[int, int]], number: int) -> int | None:
    for start, end in spans:
        if start < number <= end:
            return start
    return None


def check(path: Path, whole_module: bool) -> list[str]:
    text = path.read_text(encoding="utf-8")
    lines = text.split("\n")
    spans = function_spans(lines)
    problems: list[str] = []

    for number, line in enumerate(lines, start=1):
        stripped = line.strip()
        if not stripped.startswith("//"):
            continue
        if owner(spans, number - 1) is not None:
            problems.append(
                "%s:%d: comment inside a function body (the guest's statement "
                "scanner reads it as source): %s" % (path, number, stripped))
            continue
        if not whole_module:
            continue
        for word in re.split(r"[^A-Za-z_][^A-Za-z0-9_]*", stripped):
            if word in FORBIDDEN:
                problems.append(
                    "%s:%d: comment carries the scanned word [%s]: %s"
                    % (path, number, word, stripped))
    return problems


def main() -> int:
    problems: list[str] = []
    for relative, whole_module in FILES.items():
        path = ROOT / relative
        if not path.exists():
            problems.append("%s: missing" % relative)
            continue
        problems.extend(check(path, whole_module))

    if problems:
        print("SELFHOST_COMMENT_DISCIPLINE_RED", file=sys.stderr)
        for problem in problems:
            print("  " + problem, file=sys.stderr)
        return 1
    print("SELFHOST_COMMENT_DISCIPLINE_GREEN files=%d" % len(FILES))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
