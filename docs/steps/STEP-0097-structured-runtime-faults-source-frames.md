# STEP-0097: structured Runtime faults and source frames

> - status: complete
> - phase: M10
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Convert typed runner outcomes and Wasmtime trap metadata into the strict `sico.runtime-fault.v0` contract. Resolve source locations only through the exact verified Component/map/identity triplet, bound all messages and stacks, and derive both CLI renderings from the typed record without classifying engine prose.

## 2. Implemented boundary

- `sico-observability` now parses and validates Runtime faults with a 65,536-byte message ceiling, at most 256 frames, strict class/identity rules and mutually exclusive source/unavailable locations.
- The runner enables Wasmtime address maps and caps captured backtraces at 256 frames. It reads typed `WasmBacktrace` frame metadata rather than parsing display text.
- `prepare_program_with_debug` verifies the complete debug triplet before compilation or source lookup. A stale, malformed or mixed sidecar becomes `incompatible-artifact`; it never supplies a partial map.
- Wasmtime reports embedded Core offsets in Component-file coordinates. STEP-0097 evidence therefore corrected STEP-0096 codegen to add the embedded guest module base and added generated dispatcher-gap mappings. This is an interoperability repair to the already accepted map contract.
- `run_observed` maps domain, cancellation, timeout, fuel, memory, trap, Host-provider, launch and incompatible outcomes to stable codes/messages. Raw engine details remain outside the stable fault record.
- Stream worker disconnect or OS I/O failure crosses a typed `ProviderFault` boundary and becomes `host-provider-failure` with a bounded provider identity. Expected guest-visible stream errors remain WIT results.
- CLI `--debug-map` and `--debug-identity` are all-or-none bounded inputs. JSON is canonical contract JSON; text is rendered from the same record. A fixed sidecar is refused in watch v0 because it cannot safely follow changing generations.

No signal bridge, execution-event transport, DAP behavior or new capability is introduced by this step.

## 3. Source-frame rules

The resolver accepts only an exact module/function/half-open instruction-range match. It may infer a module only when exactly one verified module owns that function index. Generated rows remain labeled generated. Every other frame has an explicit reason such as `debug-map-unavailable`, `instruction-offset-unavailable` or `mapping-unavailable`; it never receives a nearby or guessed source span.

Host-provider failures carry `provider_id` and no fabricated guest frame. Engine wrapper frames that do not match the guest map remain unavailable.

## 4. Evidence

Real Component tests cover:

- an explicit guest trap mapped to the exact `doc.trap` UTF-8 range `10..20`;
- JSON and text CLI output from the linked Component/map/identity triplet;
- stale identity refusal with exit 127 and no source frames;
- debug-map absence producing explicit unavailable frames;
- domain, cancellation, timeout, fuel, memory, malformed guest, launch, incompatible and Host-provider class records;
- exact 256-frame acceptance, limit+1 rejection, oversized messages and conflicting source/unavailable fields;
- host survival after malicious/trapping guests;
- stable text that does not leak Wasmtime's raw `unreachable` prose.

The HTTP loopback fixture was also hardened to consume the complete request before closing, removing a Windows TCP segmentation race exposed by repeated validation.

## 5. Validation

```powershell
./tools/validate-step-0097.ps1
```

Expected summary:

```text
STEP_0097_OK classes=9 exact_source=true stale_refusal=true missing_map=unavailable frames=256/257-refused provider=typed cli=json+text engine_text_classification=false authority=unchanged next=STEP-0098
```

## 6. Honest boundary and next action

STEP-0097 proves post-fault typed observation. It does not yet prove Ctrl+C, client cancellation requests, a single terminal winner, event replay, breakpoints or pause. Ordinary runs without debug sidecars retain the previous outcome interface; callers that need strict faults use `run_observed` or the paired CLI sidecar flags.

The next executable slice is STEP-0098: one typed cancellation bridge shared by timer, Windows console control and persistent-client requests. After STEP-0098 is accepted, the documentation-only M11 STEP-0103 ADR lane may start while M10 continues.

## 7. Audit links

- [`implementation report`](../reports/runtime-fault-source-frames-v0.md)
- [`handoff`](../handoffs/M10-STEP-0097.md)
- [`M10 plan`](../plans/M10-runtime-observability-debugging.md)
- [`RFC-0035`](../rfc/RFC-0035-runtime-observability-debug-v0.md)
- [`validator`](../../tools/validate-step-0097.ps1)
