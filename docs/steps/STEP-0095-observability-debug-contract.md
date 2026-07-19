# STEP-0095: observability/debug/source-identity contract

> - status: complete
> - phase: M10
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Freeze the M10 identity, debug-map, Runtime fault/frame, task-aware event, redaction, cancellation-race and exact minimal DAP contracts before compiler, runner or adapter implementation begins.

## 2. Context and evidence

M9 GO provides real Windows x64 Component execution, typed runner outcomes, persistent per-run Store isolation and `sico.execution-plan.v0`, but it intentionally exposes compile-only coordinates and refuses `sico.debug`. The M10 plan requires a machine allowlist so STEP-0100 cannot select its own acceptance surface after implementation.

The contract builds on:

- [`RFC-0018`](../rfc/RFC-0018-runtime-limits-fault-taxonomy-v0.md) typed Runtime classes;
- [`RFC-0034`](../rfc/RFC-0034-tooling-execution-plan-v0.md) shell-free bounded execution plans and honest debug refusal;
- [`M8 STEP-0079`](./STEP-0079-script-aggregate-canonical-abi.md) bounded arena and exact Component identity discipline;
- [`M9 STEP-0090`](./STEP-0090-persistent-runner-watch.md) fresh Store per run/generation.

## 3. Scope

Included:

- RFC-0035 and three strict JSON schemas;
- four normative protocol identities;
- exact 12 supported request, 20 refused request and 6 supported event DAP rows;
- seven terminal inputs and eight both-order race cases;
- positive and adversarial fixtures;
- real compiler evidence that different sources produce distinct Components and repeat builds are deterministic.

Excluded:

- debug-map emission, Component custom-section implementation or cache integration (STEP-0096);
- Runtime source frames (STEP-0097);
- OS signal/client cancellation bridge (STEP-0098);
- event producer, DAP server, editor or AI implementation (STEP-0099–0101);
- any new syntax, capability or platform support claim.

## 4. Options and decision

### Debug-map placement

- Full embedded map: easy single-file transport, but inflates every Component and couples distribution to up to 16 MiB of tooling data.
- Unbound sidecar: compact Component, but filename/path selection cannot prove identity.
- Digest-bound sidecar plus compact custom-section link: independently inspectable and removable while cryptographically tied to the Component.

Decision: accept the third option with a non-circular chain: hash executable Component code before `sico.debug-*` sections, bind that code hash into the map, embed the map digest in `sico.debug-link.v0`, then hash the final linked Component.

### Cancellation fallback

Decision: only runner-observed signal/client cancellation may produce class `cancelled` and exit 123. Forced process-tree termination is `external-termination` with no forged exit code.

### DAP claim

Decision: a checked-in machine file is the sole allowlist. Unsupported requests return a typed error; unclaimed events are forbidden. Each claimed row has a unique evidence ID consumed by STEP-0100.

## 5. Plan

1. Accept RFC-0035 and exact limits/ownership.
2. Add strict schemas and normative DAP/cancellation contracts.
3. Add positive/negative fixtures for required failure classes.
4. Implement an offline strict validator.
5. Generate two real Components, compare source/compiler/Component identities and repeat determinism.
6. Update M10 status and hand off only the debug-map slice to STEP-0096.

## 6. Changes

- [`RFC-0035`](../rfc/RFC-0035-runtime-observability-debug-v0.md) freezes identity, sidecar/link, coordinates, Runtime classes, events, redaction, cancellation and DAP semantics.
- [`observability/`](../../observability/README.md) is the machine-readable source of truth.
- `observability/schema/` contains strict observability, DAP and cancellation schemas.
- `observability/contracts/` contains the exact DAP allowlist and terminal race matrix.
- `observability/fixtures/contract-cases-v0.json` contains one accepted bundle and six exact rejection cases.
- [`validate-step-0095.ps1`](../../tools/validate-step-0095.ps1) validates schemas/contracts/fixtures and generates identity evidence through the real compiler.

No Rust, WIT, compiler, runner or tooling implementation changed.

## 7. Validation

Executed from repository root:

```powershell
& .\tools\validate-step-0095.ps1
```

Result:

```text
STEP_0095_OK schemas=3 identities=4 fixtures=7 accepted=1 rejected=6 dap_supported_requests=12 dap_refused_requests=20 dap_events=6 cancellation_cases=7 races=8 components=2 deterministic=true source_a=006ecd7e2f512701 source_b=ce083b6302bfa558 component_a=3e41db2f7f30d51f component_b=cbf333eea4fa69cb compiler=94b75cb4547bc13e authority=unchanged implementation=not-claimed next=STEP-0096
```

The validator proved:

- strict unknown-field rejection;
- duplicate identity and duplicate mapping rejection;
- unsafe integer rejection above `2^53 - 1`;
- stale source/compiler/Component binding rejection;
- DAP frame limit+1 rejection;
- exact DAP supported/refused/event sets and unique evidence IDs;
- observed cancellation is typed while external kill is not;
- both observation orders in four terminal race pairs produce exactly one declared winner;
- two different real sources produce different Components; rebuilding the same source is byte-identical.

Repository documentation/link validation is run before commit and recorded in the final handoff.

## 8. Metrics

| Metric | Result | Status |
|---|---:|---|
| normative schemas | 3 | verified |
| protocol document identities | 4 | verified |
| fixtures | 7 (1 accept, 6 reject) | verified |
| DAP rows | 38 (12 supported requests, 20 refused requests, 6 events) | verified |
| cancellation cases/races | 7 / 8 | verified |
| real distinct Components | 2 | measured |
| repeat Component determinism | byte-identical | measured |
| new authority | 0 | verified by contract boundary |
| Runtime debugger implementation | 0 | explicitly not claimed |

Latency, Runtime source lookup, cancellation response and DAP session performance were not measured because their implementation belongs to later steps.

## 9. Risks and follow-ups

- The schemas and fixtures prove contract strictness, not compatibility with a future Runtime implementation.
- STEP-0096 must define canonical serialization bytes and the exact compact custom-section encoding; arbitrary JSON reserialization cannot be used as an artifact digest.
- `pause` and source breakpoint claims remain mandatory STEP-0100 gates. If the selected Runtime lacks safe hooks, M10 records NO-GO.
- Linux/macOS signal and DAP behavior remains unclaimed until native execution.
- M11 STEP-0103 remains locked until STEP-0098 accepts typed cancellation.

The next executable step is STEP-0096 deterministic compiler debug-map artifact.

## 10. Audit links

- [`M10 plan`](../plans/M10-runtime-observability-debugging.md)
- [`RFC-0035`](../rfc/RFC-0035-runtime-observability-debug-v0.md)
- [`observability contracts`](../../observability/README.md)
- [`STEP-0095 validator`](../../tools/validate-step-0095.ps1)
