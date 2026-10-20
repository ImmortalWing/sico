#!/usr/bin/env python3
"""DeepSeek adapter for the M23 item 3 bare-literal re-measurement (STEP-0490).

It generates the AI-corpus leg of the item-3 measurement: it asks the frozen
`sico-ai-eval-v1` generation prompts (A0 syntax) for Sico programs, saves every
raw model output verbatim as a corpus candidate, and writes the run file in
the `ai-eval/run-format.md` v1 shape.

Why this file is versioned: the M13 adapter lived under
`target/ai-eval-experiments/adapter/` (gitignored) and was lost with the
target directory. Item 3 must be re-runnable by an independent party, so the
adapter, the cost accounting and the corpus manifest live in the repository.

Standard library only (no third-party HTTP client), so the run needs no
package installation on the validation host.

Usage:
    # live run (needs the provider key in the environment)
    python tools/ai-eval/deepseek_generation_corpus.py \
        --prompt-packets target/ai-eval-experiments/prompt-packets.json \
        --run-output target/ai-eval-experiments/item3/run.json \
        --corpus-output target/ai-eval-experiments/item3/corpus \
        --repetitions 3 --budget-cap-usd 10

    # offline plan / replay (no key, no network)
    python tools/ai-eval/deepseek_generation_corpus.py --prompt-packets <p> --dry-run
    python tools/ai-eval/deepseek_generation_corpus.py --prompt-packets <p> \
        --fixture-dir <dir-with-<task_id>.json>

Identity and parameters are pinned to the M13 run (docs/reports/ai-eval-live-model-v1.md):
provider deepseek, model deepseek-chat, temperature 0, top_p 1, seed unsupported,
4 in-flight requests, cost computed from provider-reported tokens at the
published DeepSeek rates (0.27 / 1.10 USD per 1M input / output tokens).
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

PROVIDER = "deepseek"
BASE_URL = "https://api.deepseek.com/chat/completions"
MODEL = "deepseek-chat"
TEMPERATURE = 0.0
TOP_P = 1.0
SEED_SUPPORTED = False
CONCURRENCY = 4
RATE_IN_USD_PER_MILLION = {"input": 0.27, "output": 1.10}
REQUEST_TIMEOUT_SECONDS = 120
MAX_RETRIES = 4


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cost_of(input_tokens: int, output_tokens: int) -> float:
    return (
        input_tokens * RATE_IN_USD_PER_MILLION["input"]
        + output_tokens * RATE_IN_USD_PER_MILLION["output"]
    ) / 1_000_000


def load_packets(path: Path) -> list[dict]:
    packets = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(packets, dict):
        packets = packets.get("packets", [])
    if not packets:
        raise SystemExit("prompt packet file contains no packets")
    return packets


def call_model(api_key: str, system_prompt: str, user_prompt: str) -> tuple[str, int, int]:
    """One chat-completions call. Returns (raw_text, input_tokens, output_tokens)."""
    payload = {
        "model": MODEL,
        "temperature": TEMPERATURE,
        "top_p": TOP_P,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
    }
    request = urllib.request.Request(
        BASE_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
            "User-Agent": "sico-ai-eval/1",
        },
        method="POST",
    )
    last_error: Exception | None = None
    for attempt in range(MAX_RETRIES):
        try:
            with urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
                body = json.loads(response.read().decode("utf-8"))
            choice = body["choices"][0]
            usage = body.get("usage") or {}
            return (
                choice["message"]["content"],
                int(usage.get("prompt_tokens", 0)),
                int(usage.get("completion_tokens", 0)),
            )
        except urllib.error.HTTPError as error:
            last_error = error
            if error.code == 429 and attempt + 1 < MAX_RETRIES:
                time.sleep(2**attempt)
                continue
            raise
        except urllib.error.URLError as error:
            last_error = error
            if attempt + 1 < MAX_RETRIES:
                time.sleep(2**attempt)
                continue
            raise
    raise SystemExit(f"provider call failed: {last_error}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prompt-packets", required=True)
    parser.add_argument("--run-output", default="")
    parser.add_argument("--corpus-output", default="")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--budget-cap-usd", type=float, default=10.0)
    parser.add_argument("--fixture-dir", default="")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    packets = load_packets(Path(args.prompt_packets))
    tasks = []
    for packet in packets:
        if packet.get("category") not in (None, "generation"):
            continue
        for repetition in range(1, args.repetitions + 1):
            tasks.append((packet, repetition))

    if args.dry_run:
        print(
            json.dumps(
                {
                    "plan": "deepseek_generation_corpus",
                    "provider": PROVIDER,
                    "model": MODEL,
                    "temperature": TEMPERATURE,
                    "top_p": TOP_P,
                    "seed_supported": SEED_SUPPORTED,
                    "concurrency": CONCURRENCY,
                    "packets": len(packets),
                    "calls": len(tasks),
                    "budget_cap_usd": args.budget_cap_usd,
                    "endpoint": BASE_URL,
                    "run_output": args.run_output,
                    "corpus_output": args.corpus_output,
                },
                indent=2,
            )
        )
        return 0

    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key and not args.fixture_dir:
        print("DEEPSEEK_API_KEY is not set in the environment", file=sys.stderr)
        return 2

    corpus_dir = Path(args.corpus_output) if args.corpus_output else None
    if corpus_dir:
        corpus_dir.mkdir(parents=True, exist_ok=True)
    fixture_dir = Path(args.fixture_dir) if args.fixture_dir else None

    responses: list[dict] = []
    spent = 0.0

    def run_one(packet: dict, repetition: int) -> dict:
        attempt_id = f"{packet['task_id']}-r{repetition:02d}"
        started = time.monotonic()
        if fixture_dir:
            fixture = fixture_dir / f"{attempt_id}.json"
            if not fixture.exists():
                raise SystemExit(f"missing fixture response: {fixture}")
            body = json.loads(fixture.read_text(encoding="utf-8"))
            raw = body["choices"][0]["message"]["content"]
            usage = body.get("usage") or {}
            input_tokens = int(usage.get("prompt_tokens", 0))
            output_tokens = int(usage.get("completion_tokens", 0))
        else:
            raw, input_tokens, output_tokens = call_model(
                api_key,
                packet.get("system_prompt", ""),
                packet.get("user_prompt", ""),
            )
        latency_ms = int((time.monotonic() - started) * 1000)
        cost = cost_of(input_tokens, output_tokens)
        entry = {
            "attempt_id": attempt_id,
            "task_id": packet["task_id"],
            "repetition": repetition,
            "raw_output": raw,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "cost": round(cost, 6),
            "latency_ms": latency_ms,
        }
        if corpus_dir:
            candidate = corpus_dir / f"{attempt_id}.sico"
            candidate.write_text(raw, encoding="utf-8", newline="\n")
            entry["corpus_file"] = candidate.name
            entry["corpus_sha256"] = sha256_file(candidate)
        return entry

    with concurrent.futures.ThreadPoolExecutor(max_workers=CONCURRENCY) as pool:
        futures = [pool.submit(run_one, packet, repetition) for packet, repetition in tasks]
        for future in concurrent.futures.as_completed(futures):
            entry = future.result()
            responses.append(entry)
            spent += entry["cost"]
            if spent > args.budget_cap_usd:
                print(
                    f"budget cap reached: ${spent:.4f} > ${args.budget_cap_usd:.2f}; stopping",
                    file=sys.stderr,
                )
                for pending in futures:
                    pending.cancel()
                break

    responses.sort(key=lambda item: (item["task_id"], item["repetition"]))
    run = {
        "schema_version": 1,
        "protocol_id": "sico-ai-eval-v1",
        "run": {
            "run_id": f"deepseek-item3-{time.strftime('%Y-%m-%d')}",
            "kind": "model",
            "synthetic": False,
            "scope": "corpus",
            "task_ids": sorted({item["task_id"] for item in responses}),
            "repetitions": args.repetitions,
            "model": {
                "provider": PROVIDER,
                "name": MODEL,
                "version": MODEL,
            },
            "date": time.strftime("%Y-%m-%d"),
            "parameters": {
                "temperature": TEMPERATURE,
                "top_p": TOP_P,
                "seed_supported": SEED_SUPPORTED,
                "seed": None,
            },
            "token_accounting": "provider-reported",
            "cost_currency": "USD",
            "cost_total": round(spent, 6),
            "rate_limits": "provider default; 4 in-flight requests; exponential backoff on 429",
            "prompt_packet_sha256": sha256_file(Path(args.prompt_packets)),
        },
        "responses": responses,
    }

    if args.run_output:
        run_path = Path(args.run_output)
        run_path.parent.mkdir(parents=True, exist_ok=True)
        run_path.write_text(json.dumps(run, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"run file: {run_path} sha256={sha256_file(run_path)}")

    if corpus_dir:
        manifest = {
            "schema": "sico.bare-literal-corpus.v0",
            "provider": PROVIDER,
            "model": MODEL,
            "date": run["run"]["date"],
            "prompt_packet_sha256": run["run"]["prompt_packet_sha256"],
            "cost_total_usd": round(spent, 6),
            "files": [
                {
                    "file": item["corpus_file"],
                    "task_id": item["task_id"],
                    "repetition": item["repetition"],
                    "sha256": item["corpus_sha256"],
                }
                for item in responses
                if "corpus_file" in item
            ],
        }
        manifest_path = corpus_dir / "manifest.json"
        manifest_path.write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        print(f"corpus manifest: {manifest_path} files={len(manifest['files'])}")

    print(f"calls={len(responses)} cost_usd={spent:.4f} cap_usd={args.budget_cap_usd:.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
