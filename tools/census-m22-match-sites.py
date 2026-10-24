#!/usr/bin/env python3
"""M22 R0.4: the frozen `match` site ledger for the self-hosting source set.

STEP-0535 proved four **synthetic** `match` fixtures byte for byte at the
component layer and then wrote "42 / 519 sites (shape面)".  The owner audit
(AGENTS.md §10.1 / §10.4 R0.4) rejects that extrapolation: the 42 candidates
are real S sites, and each of them needs its own verdict against the Rust
oracle.

This script owns the ledger `selfhost/match-component-coverage-v1.json`:

* it re-derives the candidate sites from the **frozen** S (every `match`
  statement whose head is `U64.checked_add`, at any parameter count);
* it keeps the recorded verdict and digests of the sites that did not move;
* with `--update` it merges the actual outcomes measured by the firmware test
  `sico_compiler_reproduces_the_frozen_match_site_ledger` (which writes
  `sico-m22-match-coverage/actual.json` into the temporary directory).

The ledger is a **frozen expectation table**: the firmware test fails when a
site's verdict moves, exactly like the frontend frontier probe.

Usage:
    python tools/census-m22-match-sites.py            # rewrite, keeping verdicts
    python tools/census-m22-match-sites.py --update   # merge measured verdicts
    python tools/census-m22-match-sites.py --check    # verify only, no write
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[1]
SOURCE_DIRECTORY = REPO / "selfhost"
LEDGER = SOURCE_DIRECTORY / "match-component-coverage-v1.json"
ACTUAL = pathlib.Path(tempfile.gettempdir()) / "sico-m22-match-coverage" / "actual.json"

FN_START = re.compile(r"^function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
MATCH = re.compile(r"^(\s*)match\s+(.*?):\s*$")
CASE = re.compile(r"^\s*case\s+([A-Za-z_][A-Za-z0-9_]*)\s*(\([^)]*\))?\s*:\s*$")

SCHEMA = "sico.m22.match-coverage.v1"

# The number of `U64.checked_add` match statements in the frozen S, at any
# parameter count.  STEP-0534 measured 354 of them behind the parameter-count
# gate and 42 below it; STEP-0536 removed the gate, so the candidate set is
# the whole 396.
EXPECTED_SITES = 396

# The files whose whole-file canonical IR differential is pinned by the
# firmware; a site whose closure is too large leans on that pin.
WHOLE_FILE_PINS = {
    "parser.sico": "sico_guest_arena_accommodates_full_parser_selfhost_input",
}


def enumerate_sites() -> list[dict[str, object]]:
    """Every `checked_add` match head, at any parameter count."""
    sites: list[dict[str, object]] = []
    for path in sorted(SOURCE_DIRECTORY.glob("*.sico")):
        text = path.read_text(encoding="utf-8")
        lines = text.split("\n")
        current_function = ""
        current_function_line = 0
        current_params = 0
        index = 0
        while index < len(lines):
            line = lines[index]
            matched = FN_START.match(line)
            if matched:
                current_function = matched.group(1)
                current_function_line = index + 1
                raw = line[line.index("(") + 1 : line.rindex(")")]
                current_params = len([part for part in raw.split(",") if part.strip()])
            matched = MATCH.match(line)
            if not matched:
                index += 1
                continue
            indent = len(matched.group(1))
            match_line = index + 1
            head = matched.group(2).strip()
            operator = "?"
            open_paren = head.find("(")
            if open_paren >= 0:
                prefix = head[:open_paren].strip()
                operator = prefix.split(".")[-1] if prefix else "?"
            labels: list[str] = []
            shapes: list[str] = []
            case_indent: int | None = None
            index += 1
            while index < len(lines):
                body_line = lines[index]
                body_indent = len(body_line) - len(body_line.lstrip())
                if body_line.strip() == "end match" and body_indent == indent:
                    break
                case = CASE.match(body_line)
                if case and (case_indent is None or body_indent == case_indent):
                    case_indent = body_indent
                    labels.append(case.group(1))
                    statements = 0
                    kinds: list[str] = []
                    index += 1
                    while index < len(lines):
                        inner_line = lines[index]
                        inner_indent = len(inner_line) - len(inner_line.lstrip())
                        if inner_line.strip() == "" or inner_indent <= indent:
                            break
                        if CASE.match(inner_line):
                            break
                        kinds.append(inner_line.strip().split(" ", 1)[0])
                        statements += 1
                        index += 1
                    shapes.append(f"{statements}:{'/'.join(kinds)}")
                    continue
                index += 1
            # The case bodies are consumed above, so a `match` nested inside a
            # case body is not counted twice -- the same top-level-only rule
            # the STEP-0534 census used (519 statements, 478 of them top level).
            if operator != "checked_add":
                continue
            sites.append(
                {
                    "path": path.name,
                    "line": match_line,
                    "function": current_function,
                    "function_line": current_function_line,
                    "parameter_count": current_params,
                    "head": head,
                    "labels": "+".join(labels),
                    "case_shape": " | ".join(shapes),
                }
            )
    return sites


def load_ledger() -> dict[str, dict[str, object]]:
    if not LEDGER.exists():
        return {}
    data = json.loads(LEDGER.read_text(encoding="utf-8"))
    return {
        f"{site['path']}:{site['line']}:{site['function']}": site
        for site in data.get("sites", [])
    }


def load_actuals() -> dict[tuple[str, str], dict[str, str]]:
    if not ACTUAL.exists():
        return {}
    data = json.loads(ACTUAL.read_text(encoding="utf-8"))
    return {(entry["path"], entry["function"]): entry for entry in data}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help="merge measured verdicts")
    parser.add_argument("--check", action="store_true", help="verify only, never write")
    arguments = parser.parse_args()

    sites = enumerate_sites()
    if len(sites) != EXPECTED_SITES:
        print(f"CENSUS_MISMATCH expected={EXPECTED_SITES} actual={len(sites)}")
        return 1
    previous = load_ledger()
    actuals = load_actuals()

    records = []
    for number, site in enumerate(sites, start=1):
        key = f"{site['path']}:{site['line']}:{site['function']}"
        old = previous.get(key, {})
        actual = actuals.get((str(site["path"]), str(site["function"])))
        verdict = str(old.get("verdict", "pending"))
        source_digest = str(old.get("source_sha256", ""))
        ir_digest = str(old.get("ir_sha256", ""))
        component_digest = str(old.get("component_sha256", ""))
        note = str(old.get("note", ""))
        if actual is not None and (arguments.update or verdict == "pending"):
            verdict = str(actual["verdict"])
            source_digest = str(actual["source_sha256"])
            ir_digest = str(actual["ir_sha256"])
            component_digest = str(actual["component_sha256"])
            if verdict.startswith("closure-too-large"):
                pin = WHOLE_FILE_PINS.get(str(site["path"]))
                if pin is None:
                    print(
                        f"CENSUS_MISMATCH {site['path']} has no whole-file IR pin for a "
                        "closure-too-large site"
                    )
                    return 1
                note = f"IR witness: the whole-file differential pin {pin}"
            elif not note:
                note = "measured by the frozen ledger firmware test"
        records.append(
            {
                "id": f"MS-{number:03d}",
                "path": site["path"],
                "line": site["line"],
                "function": site["function"],
                "function_line": site["function_line"],
                "parameter_count": site["parameter_count"],
                "head": site["head"],
                "labels": site["labels"],
                "case_shape": site["case_shape"],
                "verdict": verdict,
                "source_sha256": source_digest,
                "ir_sha256": ir_digest,
                "component_sha256": component_digest,
                "note": note,
            }
        )

    ledger = {
        "schema": SCHEMA,
        "generated_by": "tools/census-m22-match-sites.py",
        "source_set": "selfhost/*.sico (frozen S)",
        "rule": (
            "a `match` statement whose head is U64.checked_add, at any parameter "
            "count -- the 396 candidate sites of the frozen S (42 below the "
            "removed parameter-count gate and 354 behind it)"
        ),
        "sites": records,
    }
    text = json.dumps(ledger, indent=2, ensure_ascii=False, sort_keys=False) + "\n"

    if arguments.check:
        if not LEDGER.exists():
            print("LEDGER_MISSING")
            return 1
        current = LEDGER.read_text(encoding="utf-8")
        if current != text:
            print("LEDGER_STALE")
            return 1
        print(f"LEDGER_CURRENT sites={len(records)}")
        return 0

    LEDGER.write_text(text, encoding="utf-8", newline="\n")
    pending = sum(1 for record in records if record["verdict"] == "pending")
    print(f"LEDGER_WRITTEN sites={len(records)} pending={pending}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
