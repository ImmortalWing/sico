#!/usr/bin/env python3
"""M22 component-layer emitter vocabulary census (STEP-0537, T beat).

The `match` site ledger (STEP-0536) records the **component axis** of every
real `match` site, but every component-refused site carries the same generic
refusal text, so the ledger cannot say *what* the guest emitter is missing.
This census answers exactly that question, per site, from the frozen sources:

  1. It re-derives each site's **same-file dependency closure** with the frozen
     rule of the Rust firmware (`match_sico_functions` / `match_sico_callees` /
     `match_sico_closure`), and self-checks that rule against the ledger's own
     `closure-too-large` verdicts -- reason for reason, count for count.
  2. It asks the **Rust oracle** for the canonical IR of that closure through
     the committed diagnostic `crates/sico-ir/tests/dump_oracle_ir.rs` and
     tallies the operation names, the intrinsic names and the terminator kinds.
  3. It compares that vocabulary against the vocabulary the **guest emitter**
     actually supports, read out of `selfhost/codegen.sico` itself, so the
     work list can never drift away from the code.
  4. (STEP-0538) It also reports the **core helper** each site needs.  The
     backend emits a helper function for some intrinsics besides the call
     site itself, so a site can be blocked by a missing helper body even when
     the guest already knows the call site.  The helper rule is read out of
     the backend's own source (`helper_dependencies` and `INTRINSIC_HELPERS`),
     never re-implemented here, and the source digest is frozen into the
     artifact.

The result is a named gap list, not a capability claim: the guest still refuses
every item the census reports as missing.

Exit codes: 0 = the frozen artifact still describes the frozen S,
1 = the artifact drifted, the closure rule disagrees with the ledger, or the
oracle could not be reached.

Usage:
  python tools/census-m22-emitter-vocabulary.py --update
  python tools/census-m22-emitter-vocabulary.py --check
  python tools/census-m22-emitter-vocabulary.py --json
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[1]
LEDGER = ROOT / "selfhost" / "match-component-coverage-v1.json"
ARTIFACT = ROOT / "selfhost" / "emitter-vocabulary-v1.json"
CODEGEN = ROOT / "selfhost" / "codegen.sico"
BACKEND_SOURCE = ROOT / "crates" / "sico-codegen-wasm" / "src" / "lib.rs"
SCHEMA = "sico.m22.emitter-vocabulary.v1"

# The core helpers the guest emitter owns, by the guest function that emits
# the helper body.  The census fails loudly when a helper-body function shows
# up in the guest source without an entry here, so the guest's helper coverage
# and the census can never drift apart silently.
GUEST_HELPER_BODIES = {
    "sico.bytes.utf8_decode": "cg_utf8_validate_helper_body_hex",
    "sico.text.concat": "cg_concat_helper_body_hex",
    "sico.text.length": "cg_text_length_helper_body_hex",
}

# A monomorphized collection or record-list intrinsic: the backend leaks the
# canonical name as its own helper.  The census treats every bracketed
# `sico.<family>.<operation>[...]` name that way; the backend's own grammar is
# narrower, so the census can only over-report a helper here, never miss one.
MONOMORPH_PATTERN = re.compile(r"^sico\.[a-z]+\.[a-z_]+\[[^\]]+\]$")

# The frozen closure bound of the Rust firmware.
CLOSURE_FUNCTION_LIMIT = 64
CLOSURE_BYTE_LIMIT = 262_144

# The frozen sources the site ledger is derived from.
FROZEN_SOURCES = [
    "codegen.sico",
    "compiler.sico",
    "compiler_lexer.sico",
    "compiler_parser.sico",
    "compiler_semantics.sico",
    "formatter.sico",
    "parser.sico",
]

# The guest wrapper the firmware wraps every closure in.
PRELUDE = (
    "record ScriptInput:\n  field arguments: List[Text]\n  field stdin: Bytes\nend record\n\n"
    "record ScriptOutput:\n  field stdout: Bytes\n  field stderr: Bytes\n  field exit_code: I64\nend record\n\n"
    "enum ScriptErrorCode:\n  case InvalidInput\nend enum\n\n"
    "record ScriptError:\n  field code: ScriptErrorCode\n  field message: Text\nend record\n\n"
)
RUN = (
    "function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:\n"
    "  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))\n"
    "end function\n"
)

# The guest emitter's vocabulary, read out of the guest source itself.
OP_PATTERN = re.compile(r'cg_is_word\(coph_name, "([a-z0-9_]+)"\)')
INTRINSIC_PATTERN = re.compile(r'cg_is_word\(cih_name, "([^"]+)"\)')


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def functions(text: str) -> list[tuple[str, str]]:
    """The (name, body) pairs of the frozen S, in file order."""
    lines = text.split("\n")
    out: list[tuple[str, str]] = []
    index = 0
    while index < len(lines):
        trimmed = lines[index].lstrip()
        if trimmed.startswith("function "):
            name = ""
            for character in trimmed[len("function ") :]:
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


def callees(text: str, known: list[str]) -> list[str]:
    """The same-file functions a body calls.

    A name is a call only when it is not a member access (`U64.checked_add(`)
    and not part of a longer identifier, so `sico.text.length(` never becomes a
    local callee.  This is the frozen rule of the Rust firmware.
    """
    found: list[str] = []
    for index, character in enumerate(text):
        if character != "(":
            continue
        start = index
        while start > 0 and text[start - 1].isascii() and (text[start - 1].isalnum() or text[start - 1] == "_"):
            start -= 1
        if start == index or start == 0:
            continue
        before = text[start - 1]
        if before == "." or (before.isascii() and (before.isalnum() or before == "_")):
            continue
        name = text[start:index]
        if name in known and name not in found:
            found.append(name)
    return found


def closure(text: str, root: str) -> tuple[str | None, str | None]:
    """The transitive same-file dependency closure of `root`, in file order."""
    table = functions(text)
    names = [name for name, _ in table]
    selected = [False] * len(table)
    stack = [root]
    count = 0
    size = 0
    while stack:
        name = stack.pop()
        if name not in names:
            continue
        position = names.index(name)
        if selected[position]:
            continue
        selected[position] = True
        count += 1
        size += len(table[position][1])
        if count > CLOSURE_FUNCTION_LIMIT or size > CLOSURE_BYTE_LIMIT:
            return None, f"closure-too-large:{count}fn/{size}B"
        for callee in callees(table[position][1], names):
            stack.append(callee)
    body = "\n".join(body for (name, body), keep in zip(table, selected) if keep) + "\n"
    return body, None


def oracle_binary(work: pathlib.Path) -> pathlib.Path | None:
    """The built `dump_oracle_ir` diagnostic binary, if there is one."""
    deps = ROOT / "target" / "debug" / "deps"
    if not deps.is_dir():
        return None
    candidates = [
        path
        for path in deps.glob("dump_oracle_ir-*")
        if path.is_file() and path.suffix != ".d" and os.access(path, os.X_OK)
    ]
    if not candidates:
        return None
    candidates.sort(key=lambda path: path.stat().st_mtime, reverse=True)
    return candidates[0]


def build_oracle(work: pathlib.Path) -> pathlib.Path | None:
    """Build the committed oracle diagnostic and return its binary path."""
    cargo = pathlib.Path(os.environ.get("USERPROFILE", str(pathlib.Path.home()))) / ".cargo" / "bin" / "cargo.exe"
    if not cargo.exists():
        cargo = pathlib.Path(shutil.which("cargo") or "cargo")
    result = subprocess.run(
        [str(cargo), "test", "--locked", "--offline", "-p", "sico-ir", "--test", "dump_oracle_ir", "--no-run",
         "--message-format=json"],
        cwd=ROOT, capture_output=True, text=True,
    )
    for line in result.stdout.splitlines():
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if payload.get("reason") != "compiler-artifact":
            continue
        if payload.get("target", {}).get("name") != "dump_oracle_ir":
            continue
        executable = payload.get("executable")
        if executable and pathlib.Path(executable).exists():
            return pathlib.Path(executable)
    return oracle_binary(work)


def oracle_ir(binary: pathlib.Path, source: pathlib.Path, out: pathlib.Path, work: pathlib.Path) -> dict:
    """The canonical IR of `source`, as the Rust oracle produces it."""
    environment = dict(os.environ)
    environment["ORACLE_SOURCE"] = str(source)
    environment["ORACLE_OUT"] = str(out)
    result = subprocess.run(
        [str(binary), "--nocapture", "--test-threads=1"],
        cwd=ROOT, env=environment, capture_output=True, text=True,
    )
    if result.returncode != 0 or not out.exists():
        raise RuntimeError(f"the oracle diagnostic failed for {source.name}: {result.stdout[-400:]}")
    return json.loads(out.read_text(encoding="utf-8"))


def vocabulary(module: dict) -> tuple[set[str], set[str], set[str]]:
    """The (ops, intrinsics, terminators) a module uses."""
    ops: set[str] = set()
    intrinsics: set[str] = set()
    terminators: set[str] = set()
    for function in module.get("functions", []):
        for block in function.get("blocks", []):
            for instruction in block.get("instructions", []):
                operation = instruction.get("operation", {})
                name = operation.get("op", "")
                ops.add(name)
                if name == "intrinsic":
                    intrinsics.add(operation.get("data", {}).get("name", ""))
            terminators.add(block.get("terminator", {}).get("kind", ""))
    return ops, intrinsics, terminators


def guest_vocabulary() -> dict:
    """The vocabulary the guest emitter supports, read from the guest source."""
    text = CODEGEN.read_text(encoding="utf-8")
    return {
        "ops": sorted(set(OP_PATTERN.findall(text))),
        "intrinsics": sorted(set(INTRINSIC_PATTERN.findall(text))),
        "helpers": guest_helpers(text),
        "source": "selfhost/codegen.sico",
        "source_sha256": sha256_text(text),
    }


def guest_helpers(text: str) -> list[str]:
    """The core helpers the guest emitter can emit.

    The guest emits one helper body per helper it owns; the census checks
    that the guest source and the table agree, so a new helper body in the
    guest stops the census (red) until the table names it.
    """
    found = set(re.findall(r"function (cg_[A-Za-z0-9_]*helper_body_hex)\(", text))
    expected = set(GUEST_HELPER_BODIES.values())
    if found != expected:
        raise RuntimeError(
            "the guest helper bodies and the census table disagree: "
            f"unlisted={sorted(found - expected)} absent={sorted(expected - found)}"
        )
    return sorted(GUEST_HELPER_BODIES)


def backend_helper_rule() -> dict:
    """The backend's helper rule, read out of the oracle's own source.

    The Rust backend decides which core helper a module needs in
    `helper_dependencies`; the census reads that rule out of the backend's
    source instead of re-implementing it, so the two cannot drift apart
    silently -- the artifact records the source digest, and any change to the
    backend invalidates the artifact.
    """
    text = BACKEND_SOURCE.read_text(encoding="utf-8")
    block = re.search(r"const INTRINSIC_HELPERS[^=]*=\s*&\[(.*?)\n\];", text, re.S)
    if block is None:
        raise RuntimeError("the backend's INTRINSIC_HELPERS list was not found")
    intrinsic_helpers = sorted(set(re.findall(r'"([^"]+)"', block.group(1))))
    function = re.search(r"fn helper_dependencies\(.*?\n\}\n", text, re.S)
    if function is None:
        raise RuntimeError("the backend's helper_dependencies function was not found")
    arms: dict[str, list[str]] = {}
    for names_text, helpers_text in re.findall(
        r'(?m)^\s*((?:"[^"]+"\s*\|\s*)*"[^"]+")\s*=>\s*&\[([^\]]*)\],', function.group(0)
    ):
        helpers = re.findall(r'"([^"]+)"', helpers_text)
        for name in re.findall(r'"([^"]+)"', names_text):
            arms[name] = helpers
    if not arms:
        raise RuntimeError("the backend's helper_dependencies arms were not found")
    return {
        "source": "crates/sico-codegen-wasm/src/lib.rs",
        "source_sha256": sha256_text(text),
        "intrinsic_helpers": intrinsic_helpers,
        "explicit_arms": {name: arms[name] for name in sorted(arms)},
    }


def required_helpers(name: str, rule: dict) -> list[str]:
    """The core helpers the backend emits for the intrinsic `name`."""
    arms = rule["explicit_arms"]
    if name in arms:
        return list(arms[name])
    if name in rule["intrinsic_helpers"]:
        return [name]
    if MONOMORPH_PATTERN.match(name):
        return [name]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--update", action="store_true", help="rewrite the frozen artifact")
    group.add_argument("--check", action="store_true", help="fail when the artifact drifted")
    group.add_argument("--json", action="store_true", help="print the artifact as JSON")
    parser.add_argument("--dump-binary", help="path to the built dump_oracle_ir diagnostic")
    arguments = parser.parse_args()

    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    sources = {name: (ROOT / "selfhost" / name).read_text(encoding="utf-8") for name in FROZEN_SOURCES}
    guest = guest_vocabulary()
    rule = backend_helper_rule()
    supported_ops = set(guest["ops"])
    supported_intrinsics = set(guest["intrinsics"])
    guest_helpers = set(guest["helpers"])

    work = ROOT / "target" / "tmp" / "m22-emitter-vocabulary"
    work.mkdir(parents=True, exist_ok=True)

    # The closure rule must reproduce the ledger's own closure verdicts.
    mismatches = 0
    for site in ledger["sites"]:
        if not site["verdict"].startswith("closure-too-large"):
            continue
        _, reason = closure(sources[site["path"]], site["function"])
        if reason != site["verdict"]:
            mismatches += 1
            print(f"  CLOSURE MISMATCH {site['id']}: census={reason} ledger={site['verdict']}", file=sys.stderr)
    if mismatches:
        print(f"EMITTER_VOCABULARY_RED closure-rule mismatches={mismatches}", file=sys.stderr)
        return 1

    binary = pathlib.Path(arguments.dump_binary) if arguments.dump_binary else None
    if binary is None:
        binary = build_oracle(work)
    if binary is None or not binary.exists():
        print("EMITTER_VOCABULARY_RED the oracle diagnostic is not available", file=sys.stderr)
        return 1

    sites = []
    op_frequency: Counter[str] = Counter()
    intrinsic_frequency: Counter[str] = Counter()
    terminator_frequency: Counter[str] = Counter()
    required_helper_frequency: Counter[str] = Counter()
    missing_helper_frequency: Counter[str] = Counter()
    for site in ledger["sites"]:
        if not site["verdict"].startswith("ir-equal+component-refused"):
            continue
        body, reason = closure(sources[site["path"]], site["function"])
        if body is None:
            print(f"  {site['id']} closure {reason}", file=sys.stderr)
            return 1
        source = work / f"{site['id']}.sico"
        source.write_text(PRELUDE + body + "\n" + RUN, encoding="utf-8", newline="\n")
        module = oracle_ir(binary, source, work / f"{site['id']}.json", work)
        ops, intrinsics, terminators = vocabulary(module)
        missing_ops = sorted(ops - supported_ops)
        missing_intrinsics = sorted(intrinsics - supported_intrinsics)
        helpers: set[str] = set()
        for intrinsic in intrinsics:
            helpers.update(required_helpers(intrinsic, rule))
        required = sorted(helpers)
        missing_helpers = sorted(helpers - guest_helpers)
        op_frequency.update(missing_ops)
        intrinsic_frequency.update(missing_intrinsics)
        terminator_frequency.update(terminators)
        required_helper_frequency.update(required)
        missing_helper_frequency.update(missing_helpers)
        sites.append({
            "id": site["id"],
            "path": site["path"],
            "function": site["function"],
            "parameter_count": site["parameter_count"],
            "ops": sorted(ops),
            "intrinsics": sorted(intrinsics),
            "terminators": sorted(terminators),
            "missing_ops": missing_ops,
            "missing_intrinsics": missing_intrinsics,
            "required_helpers": required,
            "missing_helpers": missing_helpers,
            "additions_needed": len(missing_ops) + len(missing_intrinsics) + len(missing_helpers),
        })

    histogram = Counter(site["additions_needed"] for site in sites)
    artifact = {
        "schema": SCHEMA,
        "guest_emitter": guest,
        "backend_helper_rule": rule,
        "sites": sites,
        "aggregate": {
            "sites": len(sites),
            "missing_op_frequency": dict(sorted(op_frequency.items(), key=lambda item: (-item[1], item[0]))),
            "missing_intrinsic_frequency": dict(sorted(intrinsic_frequency.items(), key=lambda item: (-item[1], item[0]))),
            "required_helper_frequency": dict(sorted(required_helper_frequency.items(), key=lambda item: (-item[1], item[0]))),
            "missing_helper_frequency": dict(sorted(missing_helper_frequency.items(), key=lambda item: (-item[1], item[0]))),
            "terminator_frequency": dict(sorted(terminator_frequency.items(), key=lambda item: (-item[1], item[0]))),
            "additions_needed_histogram": {str(key): histogram[key] for key in sorted(histogram)},
        },
    }

    if arguments.json:
        print(json.dumps(artifact, indent=2, sort_keys=True))
        return 0

    if arguments.update:
        ARTIFACT.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")
        print(f"EMITTER_VOCABULARY_WRITTEN sites={len(sites)}")
        return 0

    if arguments.check:
        if not ARTIFACT.exists():
            print("EMITTER_VOCABULARY_RED the frozen artifact is missing", file=sys.stderr)
            return 1
        frozen = json.loads(ARTIFACT.read_text(encoding="utf-8"))
        if frozen != artifact:
            print("EMITTER_VOCABULARY_RED the frozen artifact no longer describes the frozen S", file=sys.stderr)
            return 1
        print(f"EMITTER_VOCABULARY_CURRENT sites={len(sites)}")
        return 0

    print(f"sites={len(sites)}")
    print("=== the emitter vocabulary the guest is missing ===")
    for name, count in artifact["aggregate"]["missing_op_frequency"].items():
        print(f"  {count:4} sites  op        {name}")
    for name, count in artifact["aggregate"]["missing_intrinsic_frequency"].items():
        print(f"  {count:4} sites  intrinsic {name}")
    for name, count in artifact["aggregate"]["missing_helper_frequency"].items():
        print(f"  {count:4} sites  helper    {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
