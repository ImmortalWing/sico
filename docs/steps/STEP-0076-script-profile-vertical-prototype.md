# STEP-0076: Script Profile Program/Adapter vertical prototype

> - status: in-progress
> - phase: M8
> - started: 2026-07-17
> - owners: autonomous-agent

## 1. Objective

Execute the prototype gate frozen by STEP-0075 and choose either versioned Program/Adapter composition or the documented direct-runner fallback using reproducible correctness, compatibility and performance evidence.

## 2. First implementation slice

The first executable candidate generates a Component with the exact rich Script value shape, imports a dynamically typed `host-run`, re-exports it as `run`, and defines machine-readable fixtures for in-process Wasmtime 46.0.1 execution.

Implemented and type-checked:

- Unicode arguments;
- arbitrary binary input including invalid UTF-8;
- separated stdout and stderr;
- exact guest exit propagation;
- structured ScriptError;
- 1 MiB binary roundtrip;
- refusal above the 8 MiB stdin limit;
- cold Component compilation and repeated instantiate/call timing instrumentation.

Not yet covered:

- guest Core Wasm and Canonical ABI memory allocation;
- Program/Adapter composition;
- WASI CLI adaptation;
- package schema and capability closure;
- cache identity and corruption behavior;
- production Runtime resource enforcement.

`cargo check --locked` succeeds for this standalone prototype. Runtime execution is not yet evidence on the current host because its Windows GNU Rust installation lacks the assembler required to link the Wasmtime library. The installed external Wasmtime CLI cannot supply the prototype's typed `host-run` import. The direct-runner fallback therefore remains a candidate, not a validated decision, and ADR-0009 remains proposed.

## 3. Remaining gate

1. Execute the direct-runner fixture on a host with a complete linker and retain raw JSON.
2. Build a hard-coded guest Program with real Canonical ABI lift/lower behavior.
3. Compose the Program with the versioned Adapter candidate.
4. Execute the same cases through Wasmtime 46.0.1.
5. Capture cold/warm latency, peak RSS and artifact sizes with raw environment metadata.
6. Compare the composed and direct paths against the frozen thresholds.
7. Accept or revise RFC-0029 and ADR-0009, then update STEP-0077 onward from measured evidence.

## 4. Audit links

- [`prototype`](../../prototypes/script-profile/README.md)
- [`machine cases`](../../prototypes/script-profile/cases.json)
- [`STEP-0075`](./STEP-0075-script-profile-contract.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
