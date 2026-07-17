# STEP-0076: Script Profile Program/Adapter vertical prototype

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - owners: autonomous-agent

## 1. Objective

Execute the prototype gate frozen by STEP-0075 and choose either versioned Program/Adapter composition or the documented direct-runner fallback using reproducible correctness, compatibility and performance evidence.

## 2. First implementation slice

The first executable candidate generates Program Components with the exact rich Script value shape: a hard-coded guest Core Wasm module per fixture mode, lifted with real Canonical ABI options (guest `memory` + `realloc`), with Script types named through a local `sico:script/types@0.1.0` types-only instance import.

Implemented and executed on Wasmtime 46.0.1 (runtime evidence: [`script-profile-prototype-v0.md`](../reports/script-profile-prototype-v0.md)):

- Unicode arguments;
- arbitrary binary input including invalid UTF-8;
- separated stdout and stderr;
- exact guest exit propagation;
- structured ScriptError;
- 1 MiB binary roundtrip;
- refusal above the 8 MiB stdin limit;
- cold Component compilation and repeated instantiate/call timing instrumentation.

The original "import a dynamically typed `host-run` and re-export it as `run`" design was abandoned after execution showed that (a) component-level functions may only reference named types and (b) Wasmtime 46.0.1 rejects re-exporting an imported function. The prototype therefore uses real guests, which also satisfies the gate's second item ahead of schedule.

Not covered by this completed architecture gate:

- compiler-generated guests and randomized Canonical ABI roundtrips;
- WASI CLI adaptation;
- package schema and capability closure;
- cache identity and corruption behavior;
- production Runtime resource enforcement.

`cargo check --locked` succeeds for this standalone prototype. Under the MSVC toolchain, direct and composed paths each passed 6/6 in 20 independent release runs. The deterministic 915-byte Adapter has SHA-256 `fc049155cbf37bb727425461f0cdf38ba2d3440d839a3c5137c9425aed675f8f`; composed cold P95 was 15.8033 ms and the worst recorded per-run warm P95 was 0.2038 ms. The decision is `composition-go`, with direct invocation retained as a tested fallback. RFC-0029 and ADR-0009 remain proposed because their compiler/package/runner/security acceptance gates extend beyond this architecture prototype.

## 3. Completed gate

1. ~~Execute the direct-runner fixture on a host with a complete linker and retain raw JSON.~~ Done: MSVC toolchain run, raw JSON in the evidence report.
2. ~~Build a hard-coded guest Program with real Canonical ABI lift/lower behavior.~~ Done: four hard-coded WAT guests with guest `memory`/`realloc`.
3. ~~Compose the Program with the versioned Adapter candidate.~~ Done: explicit nested instantiation plus canonical lower/core trampoline/lift.
4. ~~Execute the same cases through Wasmtime 46.0.1 on the composed path and compare against direct.~~ Done: 20 × 6/6 on each path.
5. ~~Capture cold/warm latency, peak RSS and artifact sizes with raw environment metadata.~~ Done for both paths; combined-process sampled RSS max 98.72 MiB.
6. ~~Compare both paths against the frozen thresholds.~~ Done: all prototype gates pass with large latency margin.
7. ~~Choose the architecture and update later M8 work.~~ Done: composition preferred, direct tested fallback; STEP-0077 is fixed-width dynamic scalars.

## 4. Audit links

- [`prototype`](../../prototypes/script-profile/README.md)
- [`machine cases`](../../prototypes/script-profile/cases.json)
- [`runtime evidence`](../reports/script-profile-prototype-v0.md)
- [`composition evidence`](../reports/script-profile-composition-v0.md)
- [`STEP-0075`](./STEP-0075-script-profile-contract.md)
- [`STEP-0077`](./STEP-0077-fixed-width-dynamic-scalars.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
