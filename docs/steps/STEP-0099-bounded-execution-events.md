# STEP-0099: bounded execution events and logs

> - phase: M10
> - status: complete-core
> - date: 2026-07-23

## Result

The observability boundary now owns strict canonical `sico.execution-event.v0` decoding, binary-safe Base64 output chunks, monotonically increasing run/generation-bound sequences, an explicit redaction hook, and a queue capped at 256 items and 4 MiB. The runner converts typed outcomes into accepted/started, stdout/stderr, truncation, cancellation and one terminal/fault record without parsing engine text.

Output capture is capped at 1 MiB per channel and 64 KiB per chunk. Queue overflow is explicit and space is reserved for the dropped marker and terminal record. The single M10 logical task uses `task-0`/`scope-0`; M11 may add task relationships without replacing the transport.

## Evidence

- `crates/sico-observability/src/lib.rs`
- `runner/sico-runner/src/lib.rs`
- `tools/validate-step-0099.ps1`

The core producer/decoder and adversarial bounds are accepted. Process/editor transport integration remains owned by STEP-0101; this step does not claim DAP support.

## Reproduction

```powershell
./tools/validate-step-0099.ps1
```

Expected summary begins with `STEP_0099_OK`.

## Next

STEP-0100 must use Wasmtime guest-debug hooks with the exact machine allowlist. No DAP row is supported until a real Component fixture proves it.
