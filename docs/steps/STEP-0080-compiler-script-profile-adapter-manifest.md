# STEP-0080: compiler Script profile, adapter composition and manifest v1

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - evidence: [`compiler-script-profile-adapter-manifest-v0`](../reports/compiler-script-profile-adapter-manifest-v0.md), `tools/validate-step-0080.ps1`

## 1. Objective

Wire the STEP-0079 aggregate Canonical ABI into the compiler CLI so `sico build --profile script-v0` turns one source file into a deterministic `sico:script/program@0.1.0` Program Component, compose it with the versioned Script Adapter selected by STEP-0076, and introduce strict manifest v1 recording exact entry, semantics, WIT, adapter and Component identities.

## 2. Required order

1. Source surface: `main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]` lowers through semantics and IR, including `error(...)` construction and profile-owned `Bytes`/`List[Text]` types with user-type shadowing preserved.
2. `sico build --profile script-v0`: declaration shapes are validated against the frozen ABI (fail closed on mismatch), source `main` maps to the boundary `run`, and the Program Component is deterministic.
3. Adapter composition with exact WIT/adapter digests recorded; direct Program invocation remains the tested fallback.
4. Manifest v1: strict schema with entry kind/world, language semantics, Script WIT, adapter and final Component identities; unknown values fail closed; composed command imports and package closure verified.
5. Deterministic artifact snapshots and workspace regression before handoff to STEP-0081.

## 3. Included

- `Bytes`/`List[Text]` profile types and `error(...)` lowering;
- ABI-shape validation of the four Script declarations;
- the `--profile script-v0` build path and its deterministic snapshots;
- Adapter composition and the direct fallback with recorded digests;
- manifest v1 with exact identities and import/capability closure.

## 4. Excluded

- in-process runner and WASI host policy (STEP-0081);
- unified `sico run`/`eval` and caches (STEP-0082);
- standard library and list element access lowering (STEP-0083);
- streaming, async, HTTP, watch and REPL (M9);
- removing the explicit Script declarations via a prelude (later RFC).

## 5. Exit gate

- `sico build --profile script-v0` produces byte-identical Components for identical inputs;
- wrong Script declaration shapes, wrong entry signatures and unknown manifest/WIT/adapter values fail closed;
- composed Adapter path and direct fallback execute the same values through Wasmtime with recorded digests;
- manifest v1 rejects v0 scalar entries under the script profile and verifies import/capability closure;
- STEP-0077–0079 evidence and the full workspace regression remain green.

## 6. Links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`STEP-0079 evidence`](../reports/script-aggregate-canonical-abi-v0.md)
- [`layout table`](../development/script-canonical-abi-layout-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`script WIT`](../../wit/script-profile-v0)
