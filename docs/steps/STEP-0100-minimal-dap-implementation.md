# STEP-0100: minimal DAP implementation

> - phase: M10
> - status: complete / GO
> - date: 2026-07-23
> - platform evidence: Windows x64 only

## Result

The repository now has a bounded stdio DAP adapter backed by Wasmtime guest-debug Stores. It implements exactly the machine allowlist in `observability/contracts/dap-claimed-subset-v0.json`: 12 supported requests, 20 typed-refused requests and 6 supported events. No request widens filesystem or network grants, and disconnect/terminate owns and cancels its launch.

Source lines are converted to UTF-8 byte offsets only from the exact source bytes whose SHA-256 and length match the verified debug map. Entry and user breakpoints patch real guest module PCs. A busy Component pauses at a real epoch safe point. Nested guest calls return at most 256 source-mapped frames in innermost-first order. Arguments and Wasmtime scalar locals are read-only and bounded to 256 values; references and unavailable values are never dereferenced or formatted as engine addresses.

Output is derived from `sico.execution-event.v0`, passes a mandatory fail-closed DAP redactor, and is emitted before the one terminated/exited pair. The stdio server accepts only strict CRLF `Content-Length` frames with a 1 MiB body and 8 KiB header bound. It polls natural Runtime completion while waiting for client input, so terminal events do not require a later request.

## Wasmtime safety correction

The first nested-local fixture found that Wasmtime 46.0.1 directly dereferenced packed frame-state slots and could abort on an unaligned I64 load. The runner is pinned to Wasmtime 47.0.2, whose upstream implementation uses unaligned reads for every frame value. The formerly aborting nested fixture and a scalar-local fixture both pass after the upgrade. This pin is part of the validator; downgrading reopens the process-abort defect.

## Evidence

- `crates/sico-tooling-protocol/src/lib.rs`: exact framing, sole-allowlist session state and asynchronous event polling.
- `runner/sico-runner/src/dap.rs`: identity-bound runner backend, breakpoint/stack/scope/variable/control and terminal mapping.
- `runner/sico-runner/src/bin/sico-dap.rs`: bounded stdio server.
- `runner/sico-runner/tests/runner.rs`: real entry and source breakpoints, safe pause, nested stack, scalar values, redacted output, stale identity, typed breakpoint refusal, teardown and process-level framing.
- `tools/validate-step-0100.ps1`: aggregate contract and real-Component validator.

The validator iterates the sole claim file counts and the tooling tests iterate every refused row. Supported rows/events are covered across the seven real-DAP integration fixtures; a mock response is not accepted as Runtime evidence.

## Reproduction

```powershell
./tools/validate-step-0100.ps1
```

Expected final line:

```text
STEP_0100_OK dap=requests:12/refused:20/events:6 frame=1048576 real=entry,breakpoint,pause,nested-stack,scalar-locals,output,terminal stdio=bounded identity=sha256 teardown=owned wasmtime=47.0.2 platform=windows-x64 next=STEP-0101
```

## Evidence boundary

This step proves the Runtime/DAP subset on the actual Windows x64 host used for the run. It does not claim Linux, macOS, Android or Harmony execution, production deployment, third-party adoption, or external credentials. STEP-0101 still owns editor and data-only AI integration; STEP-0102 still owns the aggregate M10 exit decision.

