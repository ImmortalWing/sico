# M8 plan: Script Profile v0

> - status: complete (GO 2026-07-18); STEP-0075–0084 complete
> - created: 2026-07-17
> - phase: M8
> - entry requirement: M3/M4 Component, package, trust and Runtime contracts; STEP-0074 one-command development baseline
> - execution boundary: local compiler/Runtime work may proceed without public deployment, mobile runners or production signing identity

## 1. Outcome

Turn Sico from a scalar Component proof into a useful, bounded local scripting language for AI-generated automation. An M8-complete SDK must run one source file with arguments and binary stdin, return separated stdout/stderr and an exit status, provide deterministic cache behavior, and support a minimal text/bytes/list/JSON/scoped-file standard library without granting ambient host authority.

The target user flow is:

```powershell
sico run task.sico -- input.json
Get-Content data.json | sico run transform.sico > result.json
sico eval "40 + 2"
```

M8 retains static typing, WebAssembly Components, `.sapp` verification, capability closure and Runtime limits. It does not turn Sico into a dynamically typed interpreter.

## 2. Baseline and planning correction

STEP-0074 proved `sico-app dev SOURCE.sico`, but every run still launches the compiler, writes a Component, builds and verifies a temporary `.sapp`, writes another temporary Component and launches an external Wasmtime process with machine-code caching disabled.

The current backend is narrower than the frontend and IR: it emits scalar constants, proven `i64` addition and simple return branches, while general `Call`, `Construct`, `Project`, `Variant`, `Match`, strings, records, `Result`, resources and async operations remain refused. M8 therefore includes a general executable codegen foundation; CLI aliases alone would not create a useful script language.

The 2026-07-17 Windows release baseline for a 114-byte minimal Component, after two warmups, was:

| Stage | Median | P95 |
|---|---:|---:|
| `sico` process startup/version | 14.97 ms | 17.58 ms |
| compile and write Component | 32.89 ms | 48.90 ms |
| pack `.sapp` | 24.68 ms | 27.61 ms |
| verify and external Runtime | 65.38 ms | 95.37 ms |
| complete `sico-app dev` | 104.42 ms | 141.96 ms |

These are non-SLA local measurements. The benchmark must be captured in an M8 report before being used as an acceptance comparison.

## 3. Contract decisions to freeze in STEP-0075

- Script v0 is a bounded batch profile, not an interactive terminal profile.
- Source stdin and guest stdin cannot consume the same stream: `sico run -` reads source and supplies empty guest stdin.
- `Text` is strict UTF-8; `Bytes` carries arbitrary octets.
- Script v0 has `I64`/`U64` for sizes, exit values and dynamic machine arithmetic. Existing arbitrary-precision `Int` semantics are not silently weakened; general Runtime `Int` remains a separate implementation item.
- The first Component boundary is the closed `ScriptInput → result<ScriptOutput, ScriptError>` WIT contract; guest exit values are `0..=119` and M8 tool outcomes occupy 120–127.
- Guest input/output is bounded in M8; streaming resources are M9.
- Args and bounded stdio are verified `script.args`/`script.stdio` capabilities auto-granted only for an explicit local run. Environment and scoped files remain explicit; network is excluded from M8.
- Manifest v0 continues to mean scalar `main()`. Script packages use a new strict manifest schema and record exact semantics, WIT and adapter identities.
- Cache entries are content-addressed performance artifacts, never trust roots, and are reverified on every use.
- Top-level statements, stateful REPL, HTTP, process execution, async source lowering and streaming I/O are excluded from M8.

## 4. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0075 | M8/RFC/ADR/WIT contract and honest baseline | accepted contract or explicit unresolved blockers; reproducible benchmark method; no invented implementation claims |
| STEP-0076 | hard-coded Program + Adapter vertical prototype | complete: direct/composed 20 × 6/6; composed cold P95 15.8033 ms, worst warm P95 0.2038 ms; `composition-go` |
| STEP-0077 | fixed-width dynamic scalar representation | complete: `I64/U64`, typed overflow/underflow, dynamic parameters/locals/results, 2,048 × 8 Core oracle and 11 Wasmtime Component cases |
| STEP-0078 | general executable control and function codegen | complete: direct calls; dispatcher CFG with back-edges; internal construct/project/variant; 7 Wasmtime cases plus bounded-fuel loop trap |
| STEP-0079 | Script aggregate Canonical ABI | complete: WIT-driven layout table; bounded arena with checked arithmetic; Text/Bytes/list/record/Result lift-lower; 10,000 seeded Wasmtime roundtrips; 6 malicious-memory fail-closed fixtures |
| STEP-0080 | compiler Script profile, adapter composition and manifest v1 | complete: `sico build --profile script-v0`; adapter digest `185ebf1d…502c7`; composed WASI 0.2.12 command echo/reject verified; strict manifest v1 with exact identities and closure |
| STEP-0081 | structured in-process `sico-runner` | complete: typed outcomes for every RFC-0029 exit class; exact guest exits; fuel/timeout/memory/cancel; malicious-guest host survival; MSVC-only crate note |
| STEP-0082 | unified `sico run`, `eval` and safe caches | complete: frozen cache identity with reuse/fresh/corrupt-closed evidence; stdout purity; JSON diagnostics; engine cache deferred (offline dep) |
| STEP-0083 | minimal useful script standard library | complete: three-layer frozen intrinsic registry (text/bytes/list/JSON/fs), helper aggregate ABI, full-width Result + `case ok(x)`, scoped fs dual interface with typed refusals; four pilots green via `sico run`; `script-standard-library-v0` |
| STEP-0084 | performance, security, platform and M8 exit audit | complete: §11 gate 8/8 verified; original benchmark P95 46.96/44.82/29.00 ms, quality reruns preserve 20-sample raw measurement and non-SLA comparison; WSL Linux runner evidence + pilots; honest GO; `m8-exit-audit-v0` |

## 5. Architecture

The preferred portable path is:

```text
source.sico
  -> sico compiler: Program Component implementing sico:script/program@0.1.0
  -> sico-app: compose a versioned Script Adapter
  -> WASI CLI Command Component
  -> strict script `.sapp`
  -> sico-runner / Wasmtime
```

The adapter owns WASI argument/stdin/stdout/stderr translation. The compiler owns only language-to-Component codegen. `sico-app` owns composition, package, trust and capability gates. `sico-runner` owns Wasmtime/WASI execution. `sico run` may delegate across executable boundaries but must not introduce a compiler-to-Runtime crate dependency.

If STEP-0076 shows composition to be unstable or too costly, the fallback is a direct `sico-runner` host that calls the same Program export. The language WIT, cache identity and package compatibility fields remain unchanged.

## 6. Representation and safety rules

- One Wasm instance owns one bounded arena; dropping the Store releases the entire instance allocation.
- Script v0 does not claim general GC, cross-instance references or arbitrary nested generic ABI support.
- Text validation occurs at every untrusted boundary; invalid UTF-8 remains Bytes or fails explicitly.
- Pointer, length, alignment, multiplication and addition are checked before memory access.
- Bounds are 1,024 arguments, 64 KiB per argument, 1 MiB total argument bytes, 8 MiB each for stdin/stdout/stderr, 64 KiB ScriptError text and 64 MiB profile memory before stricter package/Host limits.
- Output overflow is a resource-limit failure, never silent truncation.
- Unknown manifest, WIT, adapter, capability or Runtime contract values fail closed.

## 7. Cache contract

The source cache key uses domain `SICO-SCRIPT-SOURCE-CACHE-V0\0`, fixed-order fields and `u64` little-endian length prefixes for variable bytes. It hashes:

```text
compiler executable digest
compiler build ID
language semantics version
script WIT version
adapter Component digest
exact source bytes
app identity/version
codegen profile/options
manifest schema
```

Cache writes use create-new temporary artifacts and atomic commit where supported. Concurrent writers may reuse a byte-identical completed entry but may not overwrite an ambiguous or corrupt entry. A hit is structurally, cryptographically and capability reverified before execution. Wasmtime machine-code caching is separately keyed by Component digest, Wasmtime version, target and engine configuration.

## 8. Standard-library minimum

M8 is not useful until it can implement representative automation without native escape hatches. The minimum is:

- `sico.text`: length, concatenate, trim, contains, prefix, split/split-lines and replace;
- `sico.bytes`: length, slice, concatenate, UTF-8 encode/decode;
- `sico.list`: length, checked get and append; map/filter/fold require either closures or an explicit deferred decision;
- `sico.json`: bounded parse/stringify with a documented number-precision contract;
- `sico.fs`: scoped read/write/exists/list through Host-granted roots.

M8 does not expose arbitrary process launch, full environment inheritance, registry access, dynamic libraries or unrestricted filesystem/network APIs.

## 9. Performance gates

The M8 report must measure cold/warm startup, cache hit/miss, compiler phases, Runtime compilation, peak RSS, artifact sizes, 1/8 MiB I/O, word count, JSON and malicious guests. Initial non-SLA goals on the baseline Windows machine are:

- minimum warm script median at most 80 ms and P95 at most 120 ms;
- cache-hit validation at most 20 ms median;
- correct 1 MiB binary roundtrip within declared memory bounds;
- deterministic Component and `.sapp` bytes for identical inputs;
- Runtime host remains healthy after timeout, fuel, memory and trap cases.

If a one-shot in-process runner misses these goals, STEP-0084 may recommend a persistent runner for M9; it must not silently add a daemon to the M8 contract.

## 10. Estimated schedule and gates

- STEP-0075–0076: approximately one week; architecture GO/NO-GO.
- STEP-0077–0079: approximately five to seven weeks; compiler/backend critical path.
- STEP-0080–0082: approximately two to three weeks; composition, runner, CLI and cache.
- STEP-0083–0084: approximately three to six weeks; useful library and stabilization.

A focused single-line implementation is therefore estimated at 10–16 weeks for a useful M8, with a narrower transport-only Script MVP possible around weeks 8–10. Estimates are planning ranges, not delivery promises.

## 11. M8 exit gate

M8 is GO only when all of the following are true:

1. exact Script WIT, manifest and adapter identities are versioned and validated;
2. dynamic non-constant code, functions, control flow and required aggregates execute through real Wasmtime;
3. args, binary stdin, stdout, stderr, errors and exit status satisfy channel and limit contracts;
4. cache corruption, stale identity, permission expansion and malformed Component cases fail closed;
5. default execution exposes no env, file, network or process authority;
6. representative text, word-count, JSON and scoped-file programs run without compiler/Runtime modifications;
7. performance/security reports distinguish measured evidence from goals;
8. all existing workspace regressions remain green.

M8 completion does not imply M9 streaming/async/interactive support or public production deployment.
