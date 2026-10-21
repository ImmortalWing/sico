# Report: item 3 AI corpus leg — live-model measurement (DeepSeek)

> - protocol: `sico-ai-eval-v1`, scope=corpus, 10 generation tasks (A0) × 10 repetitions
> - run date: 2026-10-21
> - evidence level: **measured** (`kind: model`, `synthetic: false`, provider-billed)
> - owner authorization: DeepSeek API key supplied by the owner on 2026-10-21
>   for the M23 item 3 AI-corpus leg; the numeric budget cap applied by the
>   adapter was **USD 10** (hard stop, not approached)
> - scope of this report: the AI-corpus leg only. The selfhost leg is
>   recorded in [`STEP-0490`](../../docs/steps/STEP-0490-m23-item3-bare-literal-preparation.md)
>   and in the M23 plan §10.1.
> - **This report does not decide D3.** The corpus is still under owner
>   review (M23 plan §8.3), and no grammar change may precede the decision
>   record.

## 1. Model identity and run parameters

| field | value |
|---|---|
| provider | `deepseek` (https://api.deepseek.com, OpenAI-compatible chat completions) |
| model name | `deepseek-chat` |
| served version | `deepseek-chat` (the provider reports the alias only; no dated snapshot string is exposed — recorded exactly as reported) |
| temperature | 0 |
| top_p | 1 |
| seed | not supported by the API → `seed_supported: false`, `seed: null` |
| concurrency | 4 in-flight requests; exponential backoff on 429 (no 429 observed in the live run) |
| prompt packets | `target/ai-eval-experiments/item3/prompt-packets.json`, SHA-256 `438c0bab18e6272c45e69a73a1cb7c3e9d3f40cde2fcaf8e24f5297986d6f34a` (10 A0 `generation` tasks of the frozen `sico-ai-eval-v1` protocol; B/C are alternative candidate syntaxes and are not part of a current-Sico ceremony count) |
| adapter | `tools/ai-eval/deepseek_generation_corpus.py` (versioned; standard library only; key read from `DEEPSEEK_API_KEY` and never persisted) |

The identity and parameters are the M13 ones
([`ai-eval-live-model-v1.md`](./ai-eval-live-model-v1.md)): `deepseek-chat`,
temperature 0, top_p 1, seed unsupported, 4 in-flight requests, cost computed
from provider-reported tokens at the published DeepSeek rates
(0.27 / 1.10 USD per 1M input / output tokens).

Before the billed run, the adapter's **live HTTP path** was pre-flighted
against a local mock (no key, no network): request construction, response
parsing, the 429 backoff branch, corpus writing with per-file SHA-256, the
manifest, the run-file shape and the cost accounting were all exercised
(14/14 checks, 21 HTTP requests for 20 calls, i.e. the retry branch was
actually taken).

## 2. Cost, tokens and latency

| metric | value |
|---|---|
| calls | 100 (10 tasks × 10 repetitions), 0 failed, 0 dropped |
| input tokens | 75,830 (provider-reported) |
| output tokens | 6,116 (provider-reported) |
| total cost | **USD 0.027196** (adapter-computed at the published rates; USD 10 cap not approached) |
| latency | p50 875 ms, p95 1281 ms, min 405 ms, max 1906 ms per call; ~40 s wall clock at 4 workers |

Raw run evidence (controlled experiment store, **not** committed — see
[`ai-eval/runs/README.md`](../../ai-eval/runs/README.md)):

| artifact | SHA-256 |
|---|---|
| `target/ai-eval-experiments/item3/run.json` | `2bc2d7683321d8ff2f54e59f6064a54ffda3e2e581a95b8089ec8667a3147b33` |
| `target/ai-eval-experiments/item3/prompt-packets.json` | `438c0bab18e6272c45e69a73a1cb7c3e9d3f40cde2fcaf8e24f5297986d6f34a` |
| `target/ai-eval-experiments/item3/corpus/manifest.json` | `f27b1b1628b51a63ef4df76bff33366d0ab24c76cd5955891fa7fd1e35799a8d` |
| corpus file list (100 × `sha256  name`) | `dad8fef7306f53d4e56318d8f99dfac9d2351a246597bdc8501c5def76ddbe7a` |

## 3. Corpus shape

100 candidate programs, 22,091 bytes, 1,190 lines, 560 expression lines,
350 call sites.

| task | files | distinct outputs | bytes | bare integers | literal-constructor calls |
|---|---:|---:|---:|---:|---:|
| `GEN-A0-CAP-001` | 10 | 2 | 2,583 | 0 | 0 |
| `GEN-A0-COMP-001` | 10 | 1 | 1,150 | 0 | 0 |
| `GEN-A0-MATCH-001` | 10 | 1 | 1,860 | 0 | 0 |
| `GEN-A0-NOM-002` | 10 | 2 | 1,213 | 0 | 0 |
| `GEN-A0-NUM-003` | 10 | 1 | 1,270 | 10 | 0 |
| `GEN-A0-RES-002` | 10 | 1 | 2,300 | 0 | 0 |
| `GEN-A0-RESULT-002` | 10 | 2 | 3,833 | 10 | 0 |
| `GEN-A0-REV-001` | 10 | 1 | 3,660 | 0 | 0 |
| `GEN-A0-STREAM-002` | 10 | 1 | 1,840 | 10 | 0 |
| `GEN-A0-TASK-002` | 10 | 2 | 2,382 | 30 | 0 |
| **total** | **100** | **14** | **22,091** | **60** | **0** |

**Only 14 distinct programs.** At temperature 0 the provider is
near-deterministic, so the 10 repetitions per task are largely duplicates
(one or two outputs per task). The corpus is therefore 10 distinct
programs (14 variants), not 100 independent samples. This is a property of
the M13 identity the owner required, not a defect of the run, but it caps
the statistical value of the leg and must not be reported as 100 samples.

## 4. The measurement: 0.000 literal-ceremony calls per KB

| metric | AI leg (this report) | selfhost leg (pinned commit `2531427`) |
|---|---:|---:|
| files | 100 | 14 |
| bytes / KB | 22,091 / 21.573 | 1,084,773 / 1,059.349 |
| lines | 1,190 | 24,600 |
| expression lines | 560 | 18,347 |
| call sites | 350 | 17,291 |
| literal-constructor calls | **0** | **3,236** |
| per KB | **0.000** | 3.055 |
| per 100 expression lines | **0.000** | 17.638 |
| per 100 call sites | **0.000** | 18.716 |
| hypothetical typed-literal form | saves 0 B (0.00%) | saves 42,042 B (3.88%) |

The two tools that measure this agree: the repository measurement tool
(`tools/measure-bare-literal-ceremony.ps1`) and an independent recount
(pattern `\\b(?:I64|U64|F64|Bool)\\.literal\\(` over the same files, plus a
byte/line comparison against the run file's `raw_output` fields) both report
**0**, and the run file's raw outputs match the corpus files byte-for-byte
(22,091 bytes on both sides).

One denominator caveat, recorded because it shows the same tooling friction
as the syntax mismatch below: the repository tool counts 560 expression
lines for this corpus, while the independent recount — which additionally
treats A0's `fn` header as a declaration — counts 400. The tool's
declaration rule names `function`, not `fn`, so A0-shaped headers land in
its expression-line bucket. The ceremony count is **0 under both
denominators**; the density conclusion does not depend on which is used.

**The zero is a property of the surface syntax, not of AI-written code.**
The A0 candidate syntax writes integer literals plainly — the numeric task
produces `return keep_size(Bytes(16))` — and the A0 guide
([`ai-eval/guides/a0.md`](../../ai-eval/guides/a0.md)) defines no
explicit-width literal constructor at all. The ceremony form
`I64.literal(n)` belongs to the *current* Sico surface, which the frozen
A0 prompts do not use. A0 is also a **retained control syntax** from the M1
syntax-comparison era: `syntax-candidates/README.md` records it as
`not selected; retained control` and states that **no compiler can execute
those candidates** (the same is true of the B and C mirrors).

Consequently the AI leg, as executed under the frozen protocol, **cannot
exhibit the phenomenon it was asked to measure**. Its measured 0/KB is
degenerate and is not evidence that AI-written Sico is ceremony-free.

## 5. Why the ceremony exists at all — type-system facts

Measured against the current compiler (`sico check --json`):

| source | context | result |
|---|---|---|
| `return 0` | function returns `Int` | **accepted** (syntax ok, type checker ok, 0 diagnostics) |
| `return I64.literal(0)` | function returns `Int` | `E2001 TYPE_MISMATCH` — `expected Int, found I64` |
| `return 0` | function returns `I64` | `E2002` — `expected I64, found Int; convert explicitly` |
| `return I64.literal(0)` | function returns `I64` | **accepted** |

So the explicit-width constructor is not a stylistic wart that a "bare
literal" change would simply delete: `Int` and `I64` are distinct types
with no implicit numeric conversion, and the only way to write an `I64`
value is the explicit constructor. Bare integer literals are *already*
accepted wherever an `Int` is expected — which is exactly what the A0
corpus contains. The selfhost's 3.055/KB therefore reflects how
`I64`/`U64`-heavy that code is (byte offsets, counters, u64 arithmetic), not
a ceremony the language imposes on all integer use.

The narrowest question D3 can be asked against is therefore: *is there a
literal form that yields an `I64`/`U64` value without an explicit
constructor call* (context-inferred width), and is it worth a language
change? This is narrower than the original "typed-literal form" framing and
it is what the measured evidence points at.

## 6. Compiler status of the AI corpus

`sico check --json` on all 100 candidates: **0 clean, 100 × `E1013
SYNTAX_UNEXPECTED_TOP_LEVEL`** (`top level accepts declarations only; use
an explicit main function`).

Cause: the frozen generation prompts ask for declaration-only programs
(the A0 guide's own example has no `main`), while the current front-end
requires an explicit `main` function at top level. The mismatch is between
the M1-era control syntax and the current language, not a model failure: the
generated programs match their task specification exactly (e.g.
`GEN-A0-CAP-001` returns precisely the requested `ConsoleError` /
`Console` / `announce` trio in the requested order).

No A0 candidate file in `syntax-candidates/` has a `main` function either,
and only 18 of the 58 `semantic-cases/` programs have one — the same
split. The AI corpus is therefore a **fragment corpus in a control syntax**,
which is consistent with how the frozen protocol uses it (byte comparison
against canonical references), but it means the AI leg is textual evidence
only.

## 7. What this evidence does and does not establish

- **Does**: the AI leg was executed with the M13 provider identity on the
  frozen protocol's 10 A0 generation tasks, for 100 billed calls at
  USD 0.0272, with full raw-run provenance; the resulting corpus contains
  **no** explicit-width literal ceremony, and the reason is measured and
  reproducible (A0 has no such form; `Int` vs `I64` has no implicit
  conversion).
- **Does not**: corroborate or refute the selfhost's 3.055/KB ceremony
  density. The AI leg is **not commensurable** with the selfhost leg
  because the two corpora are written in different surface syntaxes, and
  the frozen protocol cannot produce current-Sico surface code.
- **Does not**: decide D3. The corpus still needs owner review (M23 plan
  §8.3) and the decision record has not been written. No grammar change is
  proposed or implied here.
- **Does not**: claim an `external-pilot` or production result. Evidence
  level is `measured` (provider-billed live model) on a controlled protocol.

## 8. Reproduction

```powershell
# 1. the frozen prompt packets (10 A0 generation tasks)
powershell -NoProfile -ExecutionPolicy Bypass -File tools\prepare-ai-eval.ps1 `
  -OutputPath target\ai-eval-experiments\item3\prompt-packets.json `
  -Category generation -Syntax A0

# 2. the billed run (key from the environment only, never persisted)
$env:DEEPSEEK_API_KEY = '<key>'
py -3 tools\ai-eval\deepseek_generation_corpus.py `
  --prompt-packets target\ai-eval-experiments\item3\prompt-packets.json `
  --repetitions 10 --budget-cap-usd 10 `
  --run-output target\ai-eval-experiments\item3\run.json `
  --corpus-output target\ai-eval-experiments\item3\corpus

# 3. the measurement (identical tool as the selfhost leg)
powershell -NoProfile -ExecutionPolicy Bypass -File tools\measure-bare-literal-ceremony.ps1 `
  -CorpusPath target\ai-eval-experiments\item3\corpus
```

The raw model outputs stay in the controlled experiment store; per
`ai-eval/runs/README.md` they are not committed to Git unless a separate
audit step decides that they carry long-term regression value. This report
is the committed record, with the hashes above.
