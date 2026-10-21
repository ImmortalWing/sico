# Frozen run: item 3 AI corpus (M23, 2026-10-21)

This directory is the **frozen, owner-reviewed** record of the M23 plan §8.3
item-3 AI-corpus leg. It is committed under an explicit audit exception to
the default policy in [`../README.md`](./../README.md) ("real model raw
output is not committed to Git").

## Why this exception exists

The default policy keeps raw provider output out of Git. The owner reviewed
the corpus on 2026-10-21 and decided that this run carries long-term
regression value: it is the *only* AI-generated Sico corpus in the
repository, it is small (22,091 bytes of source), every file is addressed by
a SHA-256, and it is the corpus the RFC-0046 D3 decision record is written
against. The audit step that the policy requires is this owner's decision,
recorded here and in the STEP record.

## Run identity

| field | value |
|---|---|
| protocol | `sico-ai-eval-v1`, scope=corpus |
| provider / model | `deepseek` / `deepseek-chat` (the provider reports the alias only; no dated snapshot string is exposed) |
| parameters | temperature 0, top_p 1, `seed_supported: false`, 4 in-flight requests, exponential backoff on 429 |
| calls | 100 (10 A0 `generation` tasks × 10 repetitions), 0 failed |
| tokens | 75,830 input / 6,116 output (provider-reported) |
| cost | USD 0.027196 (adapter-computed at the published rates 0.27 / 1.10 USD per 1M in/out; USD 10 hard cap, not approached) |
| latency | p50 875 ms, p95 1281 ms, min 405 ms, max 1906 ms |
| owner authorization | DeepSeek credential supplied by the owner on 2026-10-21 for this leg; read from the environment only, never persisted |

## Artifacts and hashes

| artifact | SHA-256 |
|---|---|
| `run.json` | `2bc2d7683321d8ff2f54e59f6064a54ffda3e2e581a95b8089ec8667a3147b33` |
| `prompt-packets.json` | `438c0bab18e6272c45e69a73a1cb7c3e9d3f40cde2fcaf8e24f5297986d6f34a` |
| `corpus/manifest.json` | `f27b1b1628b51a63ef4df76bff33366d0ab24c76cd5955891fa7fd1e35799a8d` |
| corpus file list (100 × `sha256  name`) | `dad8fef7306f53d4e56318d8f99dfac9d2351a246597bdc8501c5def76ddbe7a` |

`corpus/manifest.json` carries the per-file SHA-256, the task id and the
repetition index of every candidate.

## What the corpus is, and what it is not

- It is **AI-generated Sico in candidate syntax A0** — the 10 frozen
  `generation` tasks of the `sico-ai-eval-v1` protocol, i.e. the STEP-0175-era
  DX-audit method.
- A0 is an **M1-era retained control syntax** (`syntax-candidates/README.md`:
  "not selected; retained control"), and **no compiler can execute those
  candidates**. All 100 files are rejected by the current front-end with
  `E1013 SYNTAX_UNEXPECTED_TOP_LEVEL`, because the frozen prompts ask for
  declaration-only programs while the current language requires an explicit
  `main` function at top level.
- The A0 guide defines **no explicit-width literal form**, so the corpus
  contains **0 literal-constructor calls** (0.000/KB) against the selfhost
  leg's 3,236 calls (3.055/KB). The zero is a surface-syntax artifact and
  makes the AI leg **not commensurable** with the selfhost leg.
- At temperature 0 the provider is near-deterministic: the 100 calls yield
  **11 distinct programs** (10 tasks; 9 of them produced a single
  program across their 10 repetitions, and the four tasks that show two raw
  variants differ only in the trailing newline), not 100 independent
  samples.

The measured facts, the full analysis and the RFC-0046 D3 decision record
live in [`docs/reports/ai-eval-item3-live-model-2026-10-21.md`](../../../docs/reports/ai-eval-item3-live-model-2026-10-21.md).

## Reproduction

```powershell
py -3 tools\ai-eval\deepseek_generation_corpus.py `
  --prompt-packets ai-eval\runs\item3-2026-10-21\prompt-packets.json `
  --repetitions 10 --budget-cap-usd 10 `
  --run-output <run.json> --corpus-output <corpus dir>

powershell -NoProfile -ExecutionPolicy Bypass -File tools\measure-bare-literal-ceremony.ps1 `
  -CorpusPath ai-eval\runs\item3-2026-10-21\corpus
```

## Retention and access

Retained with the repository as the frozen item-3 evidence. The corpus
contains provider output only — no credentials, no personal data, no
internal endpoints. The credential used for the run was read from the
environment and is not present in any committed artifact.
