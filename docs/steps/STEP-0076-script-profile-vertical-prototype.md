# STEP-0076: Script Profile Program/Adapter vertical prototype

> - status: in-progress
> - phase: M8
> - started: 2026-07-17
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

Not yet covered:

- Program/Adapter composition and the composed-vs-direct comparison;
- compiler-generated guests and randomized Canonical ABI roundtrips;
- WASI CLI adaptation;
- package schema and capability closure;
- cache identity and corruption behavior;
- production Runtime resource enforcement.

`cargo check --locked` succeeds for this standalone prototype, and the release binary passes 6/6 machine cases under the MSVC toolchain (the host's Windows GNU toolchain still lacks the assembler required to link Wasmtime; see the evidence report for the exact target metadata). The direct-runner fallback now has runtime evidence but the composition decision is still open, so ADR-0009 remains proposed.

## 3. Remaining gate

1. ~~Execute the direct-runner fixture on a host with a complete linker and retain raw JSON.~~ Done: MSVC toolchain run, raw JSON in the evidence report.
2. ~~Build a hard-coded guest Program with real Canonical ABI lift/lower behavior.~~ Done: four hard-coded WAT guests with guest `memory`/`realloc`.
3. Compose the Program with the versioned Adapter candidate.
4. Execute the same cases through Wasmtime 46.0.1 on the composed path and compare against the direct path (the direct path already passes 6/6).
5. ~~Capture cold/warm latency, peak RSS and artifact sizes with raw environment metadata.~~ Done for the direct path; repeat for the composed path.
6. Compare the composed and direct paths against the frozen thresholds.
7. Accept or revise RFC-0029 and ADR-0009, then update STEP-0077 onward from measured evidence.

## 4. Audit links

- [`prototype`](../../prototypes/script-profile/README.md)
- [`machine cases`](../../prototypes/script-profile/cases.json)
- [`runtime evidence`](../reports/script-profile-prototype-v0.md)
- [`STEP-0075`](./STEP-0075-script-profile-contract.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
