# Runtime fault and source-frame report v0

> - status: verified Runtime observation slice
> - date: 2026-07-19
> - scope: STEP-0097 only

## Result

The native runner can now produce canonical `sico.runtime-fault.v0` records from typed execution state and Wasmtime frame metadata. With an exact verified debug triplet, a real trap resolves to the original UTF-8 source byte range. A stale triplet is rejected before execution and a missing map produces explicit unavailable locations.

The stable class/message layer never examines engine display text. Raw trap text remains only in the legacy `RunOutcome` diagnostic path for non-observed callers. CLI debug mode renders JSON or text from the fault object itself, so the two presentations cannot invent different classifications.

## Bounds and separation

- 256 captured/validated frames maximum; 257 is rejected;
- 65,536 bytes maximum for the stable message;
- 16 MiB map and 1 MiB identity CLI read ceilings;
- exact Component-file Core offsets only, with no nearest-span fallback;
- Host/provider failure uses a typed provider marker and `provider_id`, not a guest source guess;
- watch mode refuses fixed debug sidecars until generation-specific identity transport exists.

## Coordinate correction

The first Runtime trace showed that Wasmtime's embedded Core `module_offset` is absolute within the Component file. STEP-0096 had emitted Core-module-relative offsets. Codegen now adds the parsed embedded guest-module base and maps generated dispatcher gaps. The corrected map remains deterministic and its complete digest chain is revalidated by STEP-0096 regression.

## Evidence summary

```text
STEP_0097_OK classes=9 exact_source=true stale_refusal=true missing_map=unavailable frames=256/257-refused provider=typed cli=json+text engine_text_classification=false authority=unchanged next=STEP-0098
```

This report does not claim signals, task events, DAP, editor integration or cross-platform M10 GO.
