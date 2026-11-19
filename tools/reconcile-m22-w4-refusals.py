#!/usr/bin/env python3
"""M22 W4 refusal-axis reconciliation for the self-hosting corpus.

W4 (AGENTS.md section 10.11-C / plan section 3.3.7) requires the guest
self-host frontend's refusal behaviour to be reconciled per corpus entry
against the Rust oracle's frozen expectation, on the (verdict, diagnostic id,
span) triple.  This script is the read-only reconciliation driver.

Two corpora share the same path set (235 entries):

* ``selfhost/script-build-corpus-v0.json``  -- the W4 target set.  Its refused
  entries (``rust_script_build == "refused"``) carry the frozen build-stage
  diagnostic ids.  At the STEP-0220 freeze this was 37 accepted / 178 refused
  (LEXICAL 116 / SCRIPT_ABI 28 / misc 34); the current file has drifted to
  42 accepted / 193 refused (LEXICAL 116 / SCRIPT_ABI 31 / misc 46).  The
  SCRIPT_ABI entries are explicitly NOT guest-frontend-reachable
  (``check = accepted`` / ``build = refused``; STEP-0561 documented the
  exclusion), so they are classified ``not-guest-reachable`` rather than run
  through the guest and misreported as a soundness hole.

* ``selfhost/corpus-v0.json`` -- the check-stage corpus.  Its refused entries
  (``rust_check == "refused"``, currently 162 = LEXICAL 116 + E-code 46) are
  exactly the guest-frontend-reachable subset, and this is the set the frozen
  parity probe ``tools/probe-m22-corpus-parity.ps1`` runs against.

The guest is run exactly like the frozen parity probe: the source is piped on
stdin to ``target/debug/sico.exe run selfhost/compiler.sico`` with
``SICO_RUNNER`` pointing at the runner executable, and the guest's refusal
identity is the ``ERR:`` marker in its outcome JSON.  Classification follows
``selfhost/corpus-parity-map-v1.json`` (the STEP-0564 mapping contract):

    exact       guest refused AND (rust id -> guest code) pair is registered
    structural  guest refused but the pair is NOT registered (verdict-only)
    zero        guest did NOT refuse (lowered = soundness hole, failed = fuel
                wall or other runtime failure)
    not-guest-reachable  SCRIPT_ABI exclusion (build-stage only)

SPAN AXIS: the frozen corpora carry no span and the guest emits a bare
``ERR:E-SH-IR-*`` code with no span.  True (verdict, id, span) triple parity is
structurally unreachable with current evidence -- registered as a residual gap
in ``selfhost/corpus-parity-map-v1.json`` .span_axis -- so this script reports
the span axis as ``not-comparable`` and does not assert span equality.  It
never fabricates an expected span.

The script is read-only by default: it prints a per-entry line plus a summary,
and only writes files when an explicit ``--out`` (CSV) or ``--json-out``
(JSON) path is given.  ``--update`` (freeze the measured expectations) is
deliberately fail-closed: it requires an explicit ``--freeze-out`` path and
refuses to overwrite an existing file or any path under ``selfhost/``, so a
stray invocation cannot clobber a frozen corpus.

Usage:
    python tools/reconcile-m22-w4-refusals.py                 # build corpus, all, stdout
    python tools/reconcile-m22-w4-refusals.py --corpus check --limit 40
    python tools/reconcile-m22-w4-refusals.py --corpus check --out target/tmp/w4.csv
    python tools/reconcile-m22-w4-refusals.py --update --freeze-out target/tmp/w4-freeze.json
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]

CORPUS_CHECK = REPO / "selfhost" / "corpus-v0.json"
CORPUS_BUILD = REPO / "selfhost" / "script-build-corpus-v0.json"
MAP = REPO / "selfhost" / "corpus-parity-map-v1.json"
GUEST = REPO / "selfhost" / "compiler.sico"
CLI = REPO / "target" / "debug" / "sico.exe"
RUNNER = REPO / "runner" / "sico-runner" / "target" / "debug" / "sico-runner.exe"

REPORT_SCHEMA = "sico.m22.w4-refusal-reconcile.v0"
RUNNER_TIMEOUT_SECONDS = 150

# Frozen STEP-0220 plan counts (plan section 3.3.7 / the corpus line).
FROZEN_TOTAL = 178
FROZEN_LEXICAL = 116
FROZEN_SCRIPT_ABI = 28
FROZEN_MISC = 34

# Expected span is structurally unavailable (see module docstring).
SPAN_STATUS = "not-comparable: frozen corpus carries no span; guest emits a bare ERR:E-SH-IR-* code with no span (see selfhost/corpus-parity-map-v1.json .span_axis)"


def die(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    sys.exit(1)


def load_json(path: pathlib.Path):
    if not path.is_file():
        die(f"required input not found: {path}")
    try:
        with open(path, encoding="utf-8") as fh:
            return json.load(fh)
    except json.JSONDecodeError as exc:
        die(f"invalid JSON in {path}: {exc}")


def load_corpus(kind: str) -> dict:
    if kind == "check":
        return load_json(CORPUS_CHECK)
    if kind == "build":
        return load_json(CORPUS_BUILD)
    die(f"unknown corpus '{kind}' (expected 'check' | 'build')")


def load_mapping() -> dict:
    m = load_json(MAP)
    if m.get("schema") != "sico.m22.corpus-parity-map.v1":
        die(f"unexpected mapping schema '{m.get('schema')}' (expected sico.m22.corpus-parity-map.v1)")
    if int(m.get("version", 0)) != 1:
        die(f"unsupported mapping version {m.get('version')} (expected 1)")
    entries = m.get("mappings") or []
    if not entries:
        die("mapping artifact registers no equivalence classes")
    rust_to_guest: dict[str, set[str]] = {}
    pair_seen: set[tuple[str, str]] = set()
    for e in entries:
        rc = e.get("rust_code")
        gc = e.get("guest_code")
        if not rc or not gc:
            die("mapping entry has empty rust_code/guest_code")
        if e.get("grain") not in ("refined", "coarse"):
            die(f"mapping grain '{e.get('grain')}' is not refined|coarse")
        pair = (rc, gc)
        if pair in pair_seen:
            die(f"duplicate mapping pair {pair[0]}|{pair[1]}")
        pair_seen.add(pair)
        rust_to_guest.setdefault(rc, set()).add(gc)
    return {"entries": entries, "rust_to_guest": rust_to_guest, "m": m}


def run_guest(entry: dict) -> dict:
    """Run the guest frontend on one corpus entry; replicate probe-m22-corpus-parity.ps1."""
    src_path = REPO / entry["path"]
    if not src_path.is_file():
        return {"ok": False, "verdict": "MISSING-SOURCE", "code": None}
    try:
        source = src_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return {"ok": False, "verdict": f"READ-FAILED:{exc}", "code": None}
    source = source.replace("\r\n", "\n")

    env = os.environ.copy()
    env["SICO_RUNNER"] = str(RUNNER)
    try:
        proc = subprocess.run(
            [str(CLI), "run", str(GUEST)],
            input=source.encode("utf-8"),
            capture_output=True,
            timeout=RUNNER_TIMEOUT_SECONDS,
            env=env,
        )
    except subprocess.TimeoutExpired:
        return {"ok": False, "verdict": "FAILED:timeout", "code": None}
    except OSError as exc:
        return {"ok": False, "verdict": f"FAILED:os-error:{exc}", "code": None}

    raw = (proc.stdout + proc.stderr).decode("utf-8", "replace")
    flat = raw.replace("\r\n", " ").replace("\n", " ")
    marker = flat.find("ERR:")
    if marker >= 0:
        code = flat[marker:]
        end = code.find('"')
        if end > 0:
            code = code[:end]
        code = code.rstrip("\\\" ")
        return {"ok": True, "verdict": f"REFUSED:{code}", "code": code}
    if '"schema":"sico.ir.v0"' in flat:
        return {"ok": True, "verdict": "LOWERED", "code": None}
    return {"ok": True, "verdict": "FAILED:" + flat[:80], "code": None}


def classify(entry: dict, guest: dict, rust_to_guest: dict) -> tuple[str, str]:
    """Return (parity, diff_class) for one entry.

    build-stage SCRIPT_ABI entries are not guest-frontend-reachable
    (check = accepted / build = refused); they are excluded rather than run.
    """
    rust_ids = [str(d) for d in (entry.get("build_diagnostic_ids") or entry.get("diagnostic_ids") or [])]
    if any(d == "SCRIPT_ABI" for d in rust_ids):
        return "not-guest-reachable", "not-guest-reachable"

    if not guest["ok"]:
        return "zero", "missing-source" if guest["verdict"] == "MISSING-SOURCE" else "run-failed"

    if guest["code"] is None:
        if guest["verdict"] == "LOWERED":
            return "zero", "zero-lowered"
        if "FuelExhausted" in guest["verdict"] or 'resource-limit.fuel' in guest["verdict"]:
            return "zero", "zero-fuel"
        return "zero", "zero-failed-other"

    mapped = "unmapped"
    for rid in rust_ids:
        if rid in rust_to_guest and guest["code"] in rust_to_guest[rid]:
            mapped = guest["code"]
            break
    if mapped == "unmapped":
        return "structural", "structural-unmapped"
    return "exact", "exact"


def entry_expected(entry: dict, kind: str) -> dict:
    if kind == "build":
        ids = entry.get("build_diagnostic_ids") or []
        verdict = "REFUSED" if entry.get("rust_script_build") == "refused" else "accepted"
    else:
        ids = entry.get("diagnostic_ids") or []
        verdict = "REFUSED" if entry.get("rust_check") == "refused" else "accepted"
    return {
        "verdict": verdict,
        "diagnostic_ids": list(ids),
        "span": SPAN_STATUS,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--corpus", choices=["build", "check"], default="build",
                    help="which corpus to traverse (default: build = the W4 target set)")
    ap.add_argument("--limit", type=int, default=0,
                    help="run only the first N refused entries (0 = all)")
    ap.add_argument("--out", help="write per-entry CSV to this path (default: stdout)")
    ap.add_argument("--json-out", help="write the full report JSON to this path")
    ap.add_argument("--update", action="store_true",
                    help="freeze the measured expectations (fail-closed: requires --freeze-out)")
    ap.add_argument("--freeze-out", help="path for --update output; refuses selfhost/ and existing files")
    args = ap.parse_args()

    if args.update:
        if not args.freeze_out:
            die("--update requires --freeze-out <path>; refusing to guess a freeze location")
        fo = pathlib.Path(args.freeze_out).resolve()
        if fo.is_file():
            die(f"--update refuses to overwrite existing file: {fo}")
        if str(fo).replace("\\", "/").startswith(str(REPO / "selfhost").replace("\\", "/") + "/"):
            die("--update refuses to write under selfhost/ (frozen corpus space)")
        # --update is a deliberate freeze action; it is NOT executed by this
        # driver.  The caller must have reviewed the measured results first.
        print(f"--update would freeze expectations to {fo} (not executed in read-only mode)")

    corpus = load_corpus(args.corpus)
    mapping = load_mapping()
    rust_to_guest = mapping["rust_to_guest"]

    if args.corpus == "build":
        refusals = [e for e in corpus["entries"] if e.get("rust_script_build") == "refused"]
    else:
        refusals = [e for e in corpus["entries"] if e.get("rust_check") == "refused"]
    if args.limit > 0:
        refusals = refusals[: args.limit]

    counts = {"exact": 0, "structural": 0, "zero": 0, "not-guest-reachable": 0}
    diff_counts = {}
    rows = []
    for entry in refusals:
        expected = entry_expected(entry, args.corpus)
        guest = run_guest(entry)
        parity, diff = classify(entry, guest, rust_to_guest)
        counts[parity] = counts.get(parity, 0) + 1
        diff_counts[diff] = diff_counts.get(diff, 0) + 1
        rows.append({
            "path": entry["path"],
            "expected_verdict": expected["verdict"],
            "expected_diagnostic_ids": "|".join(expected["diagnostic_ids"]),
            "expected_span": SPAN_STATUS,
            "actual_verdict": guest["verdict"],
            "mapped_id": guest["code"] if guest["code"] else "",
            "parity": parity,
            "diff_class": diff,
        })

    # Per-line output.
    for r in rows:
        flag = {"exact": "EXACT  ", "structural": "STRUCT ", "zero": "ZERO   ",
                "not-guest-reachable": "N/A    "}[r["parity"]]
        print(f"{flag} {r['path'][:56]:<56} exp={r['expected_verdict']}:{r['expected_diagnostic_ids'][:18]:<18} "
              f"guest={r['actual_verdict'][:34]:<34} cls={r['diff_class']}")

    # Composition facts (frozen point vs current).
    lex = sum(1 for e in refusals if "LEXICAL" in (e.get("build_diagnostic_ids") or e.get("diagnostic_ids") or []))
    abi = sum(1 for e in refusals if "SCRIPT_ABI" in (e.get("build_diagnostic_ids") or e.get("diagnostic_ids") or []))
    misc = len(refusals) - lex - abi

    summary = {
        "schema": REPORT_SCHEMA,
        "corpus": args.corpus,
        "checked_count": len(rows),
        "exact_count": counts["exact"],
        "structural_count": counts["structural"],
        "zero_count": counts["zero"],
        "not_guest_reachable_count": counts["not-guest-reachable"],
        "diff_class_counts": diff_counts,
        "composition_current": {"lexical": lex, "script_abi": abi, "misc": misc, "total": len(refusals)},
        "composition_frozen": {"lexical": FROZEN_LEXICAL, "script_abi": FROZEN_SCRIPT_ABI,
                               "misc": FROZEN_MISC, "total": FROZEN_TOTAL},
        "composition_matches_frozen": (lex == FROZEN_LEXICAL and abi == FROZEN_SCRIPT_ABI
                                       and misc == FROZEN_MISC and len(refusals) == FROZEN_TOTAL),
        "span_axis": SPAN_STATUS,
        "mapped_by": {"schema": mapping["m"].get("schema"), "version": mapping["m"].get("version")},
        "entries": rows,
    }

    if args.json_out:
        p = pathlib.Path(args.json_out)
        p.parent.mkdir(parents=True, exist_ok=True)
        with open(p, "w", encoding="utf-8", newline="\n") as fh:
            json.dump(summary, fh, ensure_ascii=False, indent=1)
            fh.write("\n")
        print(f"\nwrote {p}")

    if args.out:
        p = pathlib.Path(args.out)
        p.parent.mkdir(parents=True, exist_ok=True)
        with open(p, "w", encoding="utf-8", newline="\n") as fh:
            writer = csv.DictWriter(fh, fieldnames=list(rows[0].keys()) if rows else
                                    ["path", "expected_verdict", "expected_diagnostic_ids", "expected_span",
                                     "actual_verdict", "mapped_id", "parity", "diff_class"])
            writer.writeheader()
            writer.writerows(rows)
        print(f"wrote {p}")

    print(f"\nW4_RECONCILE corpus={args.corpus} checked={len(rows)} "
          f"exact={counts['exact']} structural={counts['structural']} "
          f"zero={counts['zero']} not-guest-reachable={counts['not-guest-reachable']}")
    print(f"COMPOSITION current={len(refusals)} (lexical={lex} script_abi={abi} misc={misc}) "
          f"frozen={FROZEN_TOTAL} (lexical={FROZEN_LEXICAL} script_abi={FROZEN_SCRIPT_ABI} misc={FROZEN_MISC}) "
          f"matches_frozen={summary['composition_matches_frozen']}")
    print(f"SPAN_AXIS {SPAN_STATUS[:80]}...")

    return 0 if counts["zero"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
