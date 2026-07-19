# STEP-0082: unified sico run, eval and safe caches

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - evidence: [`unified-run-eval-caches-v0`](../reports/unified-run-eval-caches-v0.md), `tools/validate-step-0082.ps1`

## 1. Objective

Deliver the normal local Script workflow: `sico run FILE.sico -- ARGS` and `sico eval "EXPR"` with exact args/stdin/stdout/stderr/exit behavior, stdout purity, JSON diagnostics, and a content-addressed source cache that is reverified on every hit and fails closed on corruption, plus machine-code caching through the runner.

## 2. Required order

1. `sico run`: compile through the script-v0 profile, cache the Program Component under the frozen `SICO-SCRIPT-SOURCE-CACHE-V0` identity, and execute through `sico-runner` across the executable boundary (no compiler-to-Runtime crate dependency).
2. Args/stdin contract: `sico run FILE` gives process stdin to the guest; `sico run -` consumes stdin as source and gives empty guest stdin; one stream is never ambiguously consumed twice.
3. `sico eval`: synthesize Candidate A in memory; v0 evaluates compile-time-constant `Int` expressions only and refuses everything else with a typed diagnostic; no new source grammar.
4. Source cache: key fields in frozen order with u64-LE length prefixes; create-new temporary artifacts with atomic commit; hits revalidate structure before execution; corrupt entries fail closed and are never overwritten silently; concurrent writers may reuse byte-identical entries.
5. Machine cache: Wasmtime compilation cache keyed by Component digest/engine/target through the runner's engine cache; record behavior or an explicit deferral with evidence.
6. Exit mapping: compile diagnostics → 120, CLI/IO/cache → 121, runner outcomes pass through (0–119/122–127), missing/incompatible runner → 127.

## 3. Included

- `sico run` and `sico eval` in `sico-cli`;
- source cache with corruption/concurrency refusals;
- runner discovery (`SICO_RUNNER`, sibling, PATH);
- JSON diagnostics for tool failures; stdout purity;
- runner machine-code cache enablement.

## 4. Excluded

- standard library (STEP-0083);
- composed Adapter packaging in the run path (the runner executes Program Components directly);
- a persistent daemon runner (M9 consideration);
- streaming/async (M9).

## 5. Exit gate

- repeated runs of an unchanged source reuse the byte-identical cache entry (content and mtime unchanged, correct output, no rewrite); a changed source produces a fresh entry while the old one stays;
- modified source gets a fresh cache entry; corrupt entries fail closed;
- `sico run -` never feeds source stdin to the guest;
- stdout carries only guest stdout; diagnostics are JSON on stderr;
- exact exit codes pass through end to end;
- STEP-0077–0081 evidence and the full workspace regression remain green.

## 6. Links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`STEP-0081 evidence`](../reports/in-process-script-runner-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
