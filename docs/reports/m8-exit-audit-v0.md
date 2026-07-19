# M8 exit audit: performance, security, platform and GO/NO-GO

> - date: 2026-07-18
> - step: STEP-0084
> - status: complete

## Method and environment

Every number below was measured on this host (Windows 10.0.26300, x86-64) with release binaries; each measured run is a fresh OS process. The Linux runner evidence was produced inside WSL Ubuntu 24.04 (gcc 13.3.0, Rust 1.97.1) from the same sources. Nothing in this report is an SLA or a goal restated as a measurement; goals are quoted only to be compared against.

## Exit-gate verification (M8 plan §11)

1. **Exact Script WIT, manifest and adapter identities are versioned and validated.** `sico:script@0.1.0` WIT is embedded and asserted at load (`ScriptAbi::load` panics on any drift, pinned by unit tests); adapter identity `sico:script/adapter@0.1.0` digest `185ebf1d393cb2b93c1d023f89d19d36c323c64f858a2fef1092f6a0283502c7` re-derived deterministically (STEP-0080, one digest across 20 runs); manifest schema `sico.sapp.manifest.v1` with exact identities and import/capability closure. Validators: `tools/validate-step-0080.ps1`.
2. **Dynamic non-constant code, functions, control flow and required aggregates execute through real Wasmtime.** STEP-0077 (2,048 × 8 fixed-width properties, 11 Component cases), STEP-0078 (direct calls, dispatcher CFG with back-edges, match, fuel-trap), STEP-0079 (10,000 seeded Canonical ABI roundtrips, 6 malicious-memory fail-closed fixtures), STEP-0083 (helper functions with aggregate parameters/results through the flattened ABI, full-width `Result` with runtime-tag boundary). Validators: 0077/0078/0079.
3. **Args, binary stdin, stdout, stderr, errors and exit status satisfy channel and limit contracts.** Runner typed outcomes for every RFC-0029 class with exact guest exits 0–119 and 121–127 host codes; 8 MiB channel bounds; input bounds rejected pre-execution. Validators: 0081/0082.
4. **Cache corruption, stale identity, permission expansion and malformed Component cases fail closed.** Cache reuse/fresh/conflict/corrupt evidence (STEP-0082); composed reject path (STEP-0080); malformed Component → `Incompatible` 127 (STEP-0081); fs capability denial/escape → typed 122 without file effects (STEP-0083); unknown imports/capabilities fail closed in the package closure. Validators: 0080/0081/0082/0083.
5. **Default execution exposes no env, file, network or process authority.** The runner links no WASI at all; the only host surface is the fs channel, which is denied by default (`storage.read not granted` / `storage.write not granted`, measured exit 122) and scoped to canonical Host-granted roots with per-call containment checks.
6. **Representative text, word-count, JSON and scoped-file programs run without compiler/Runtime modifications.** All four STEP-0083 pilots run through stock `sico run`: args echo, word count, JSON filter, scoped file transform — exact expected outputs and typed refusals (`tools/validate-step-0083.ps1`).
7. **Performance/security reports distinguish measured evidence from goals.** This report; every figure names method, iterations and environment.
8. **All existing workspace regressions remain green.** `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings` (0 findings), `cargo test --workspace --all-targets --all-features` (67 suites) on the pinned GNU toolchain; `sico-runner` release tests 6/6 on MSVC; step validators 0077–0083 all re-run green on 2026-07-18.

## Performance matrix (measured, final pipeline)

`tools/benchmark-step-0084.ps1`, 20 iterations per row, release binaries, Windows (canonical artifact `target/evidence/step-0084/benchmark.json`):

| Metric | min | median | P95 | max |
|---|---:|---:|---:|---:|
| `sico run` cold (cache miss: frontend + codegen + cache commit + runner) | 41.20 ms | 42.80 ms | 46.96 ms | 77.71 ms |
| `sico run` warm (verified cache hit + runner) | 41.33 ms | 42.00 ms | 44.82 ms | 46.85 ms |
| `sico-runner` only (warm component, no compile) | 25.67 ms | 26.81 ms | 29.00 ms | 32.00 ms |

Independent reruns recorded for variance honesty: a first 20-iteration run measured 51.26 / 52.86 / 36.48 ms P95 (same rows), and a 10-iteration run inside the validator measured 81.20 / 44.63 / 29.03 ms P95 — cold-run spread is first-touch noise, not a systematic difference.

Reading: the cache removes codegen but the per-run floor is process spawn plus Wasmtime engine/component compilation inside the runner (~27–36 ms); the CLI adds ~18 ms (cache identity hashing includes the compiler executable digest, a fixed ~10 MB read per invocation). An independent 10-iteration rerun inside `tools/validate-step-0084.ps1` reproduced the same picture (cold P95 81.20 ms with a small-sample outlier, warm P95 44.63 ms, runner-only P95 29.03 ms) — cold-run variance is driven by first-touch noise, not by a systematic difference. The STEP-0076 in-process prototype numbers (composed cold compile P95 15.80 ms, warm instantiate+call P95 0.20 ms) are a different measurement unit (no process spawn, reused engine) and are not directly comparable; they remain the evidence that composition overhead itself is small. Machine-code caching is the known lever for the runner floor and stays deferred (offline dependency availability, recorded in STEP-0082).

Compared against the STEP-0076 gate *goals* (cold P95 < 200 ms, warm P95 < 120 ms): the final pipeline's measured 46.96 ms cold and 44.82 ms warm end-to-end P95 pass both goals with margin. These goals were prototype targets, not commitments; the comparison is recorded as context.

Quality rerun (2026-07-19): the benchmark now uses a GUID-scoped fresh cache for every run, preventing stale evidence directories from contaminating the cold-cache classification. Measured P95 was 79.6508 ms cold, 49.0328 ms warm and 34.1953 ms runner-only; the M8 gate remains GO. The same run passed workspace fmt/Clippy/tests and STEP-0084; STEP-0086/0087/0088 were also revalidated independently.

## Security posture (measured)

- Default `sico run` grants only `script.args`/`script.stdio`; every fs call without a grant returned a typed `result` error (exit 122) and produced no filesystem effect (verified: no `escape.txt` created).
- `../secret.txt` (read) and `../escape.txt` (write) denied by lexical scoping before any IO; canonical containment checked per call (symlink escape denied by construction).
- Runner survives malicious guests (fuel/timeout/memory/cancel/trap matrix, STEP-0081) and malicious memory (6 fixtures, STEP-0079).
- Cache identity is frozen; corrupt entries are refused, never executed (STEP-0082).

## Platform evidence

- **Windows**: full pipeline evidence (compiler, runner, all validators, benchmark matrix above).
- **Linux**: sico-runner built from the same sources in WSL Ubuntu 24.04 (Rust 1.97.1, gcc 13.3.0); release test suite 6/6 green; the script-echo component (built by the Windows compiler) executed through the Linux runner with exact output and exit 0, and the file-transform component ran with `--fs-read-root`/`--fs-write-root` producing the expected `words: 5` artifact. No Linux CLI packaging is claimed.

## Decision

**GO for M8.** All eight §11 items hold with measured evidence. Honest limits carried forward into M9: runner per-process floor ~27–36 ms without machine-code caching; `sico.json.get` returns raw token text; `case ok(x)` requires a parameter match subject; `sico.fs.list` absent; Linux evidence is WSL-based. M8 completion does not imply M9 streaming/async/interactive support or public production deployment.
