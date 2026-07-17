# STEP-0075: Script Profile contract and vertical-prototype gate

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - owners: autonomous-agent

## 1. Objective

Freeze a reviewable Script Profile v0 contract and define the exact Program/Adapter/Wasmtime prototype gate that STEP-0076 must execute before M8 commits to Component composition or the documented direct-runner fallback.

## 2. Context and evidence

- STEP-0074 made trusted local source executable through one `sico-app dev` command.
- Current user documentation records no application args or guest stdin and a scalar `main()` Runtime boundary.
- Current Wasm codegen rejects non-scalar locals/results and most general IR operations.
- The 2026-07-17 local benchmark measured complete minimal `dev` median 104.42 ms and P95 141.96 ms; the method and raw evidence still require a formal report.
- RFC-0029 and ADR-0009 are proposed, not accepted.
- Public deployment remains independently blocked on owner-controlled external inputs and is not required for local M8 work.

## 3. Scope

Included:

- M8/M9 plans and index/status synchronization;
- Script WIT draft;
- exact CLI/stdin/channel/limit/cache/capability decisions;
- reproducible prototype fixtures, benchmark method and GO/fallback thresholds.

Excluded:

- claiming Script Profile implementation;
- implementing or executing the STEP-0076 Program/Adapter prototype;
- changing the production package schema before contract acceptance;
- general aggregate codegen;
- embedded runner implementation;
- M9 streaming, HTTP, REPL or top-level syntax;
- public production rollout.

## 4. Options and decision

The primary candidate is a Program Component composed with a versioned WASI CLI adapter. The fallback is a separate in-process runner that calls the same Program export directly. A compiler/runtime monolith and unbounded/ambient host interfaces are rejected.

The contract and gate are frozen for STEP-0076. RFC-0029 and ADR-0009 remain proposed until the prototype chooses composition or the direct-runner fallback.

## 5. Plan

1. Register M8, M9, RFC-0029, ADR-0009, STEP-0075 and the WIT draft.
2. Resolve fixed-width numbers, exit range, exact limits and manifest identity.
3. Specify hard-coded Program and Adapter fixtures outside the production compiler.
4. Freeze composition/import validation and Wasmtime 46.0.1 commands.
5. Freeze Unicode args, binary/empty/1 MiB stdin, separated channels, exit/error and limit cases.
6. Freeze cold/warm latency, peak-memory and artifact-size measurement methods and thresholds.
7. Hand the accepted/revised contract to STEP-0076 for prototype execution and its GO/fallback decision.

## 6. Changes

Planning records, proposed contracts and the WIT draft were added. The existing codegen WIT parser test now includes the Script draft. No compiler, package or Runtime behavior is changed by this record.

## 7. Validation

Completed for the planning baseline:

```powershell
cargo test --locked -p sico-codegen-wasm --test backend repository_wit_contracts_parse_with_expected_namespace
cargo fmt --all -- --check
.\tools\validate-module-boundaries.ps1
```

The WIT parser test passed, formatting is canonical and the existing 23-package module boundary remains valid. The contract fixes explicit `I64/U64`, guest exit 0–119/tool exit 120–127, argument/input/output/error/memory limits, `script.args`/`script.stdio` baseline capability identities, scoped env/file rules and the length-prefixed source-cache key. Prototype, Component, Runtime and formal performance validation belong to STEP-0076.

## 8. Metrics

Current figures are planning baselines only: minimal complete `dev` median 104.42 ms/P95 141.96 ms; direct raw Wasmtime median 37.48 ms/P95 53.69 ms on the measured Windows environment. The measurement method and frozen prototype thresholds are recorded in the contract report; STEP-0076 must produce immutable raw prototype evidence.

## 9. Risks and follow-ups

- Current aggregate/control codegen is much narrower than the Script source contract.
- Exact WIT may change while RFC-0029 remains proposed.
- Composition may be replaced by direct runner invocation without changing source semantics.
- M8 schedule is dominated by backend work, not this contract step.
- Public rollout and mobile runner blockers remain separate and must not be relabeled complete.

## 10. Audit links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`M9 plan`](../plans/M9-streaming-async-interactive.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`ADR-0009`](../adr/ADR-0009-script-adapter-runner.md)
- [`Script WIT draft`](../../wit/script-profile-v0/world.wit)
- [`contract report`](../reports/script-profile-contract-v0.md)
