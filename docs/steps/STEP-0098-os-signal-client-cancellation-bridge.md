# STEP-0098: OS signal and client cancellation bridge

> - status: complete
> - phase: M10
> - started: 2026-07-19
> - completed: 2026-07-19
> - owners: autonomous-agent

## 1. Objective

Route observed console controls, bounded persistent-client requests, timers and Host cancellation through one typed cancellation path and one terminal winner. Guarantee exit 123 only when the runner itself observes and commits cancellation; never relabel an external process kill as cancellation.

## 2. Implementation

- `CancellationSource` distinguishes timer, signal, client, debugger and Host requests.
- A per-run `TerminalArbiter` implements `running → cancellation-requested → terminal`. The first committed terminal wins; repeated cancellation and repeated terminal attempts return the original decision.
- `CancelToken` binds to exactly one active run, records the first typed source and wakes busy guest code through Wasmtime epochs. The same token remains visible to blocked stream/HTTP polling boundaries.
- The safe `ctrlc` abstraction installs one process handler. Scoped registration guards route Windows console controls to the current one-shot run, current watch generation and the idle watch session without unsafe code in Sico.
- `sico.cancel-request.v0` is a strict canonical 4 KiB protocol carrying exact `run_id`, `generation_id` and `client|debugger` cause. Stale/malformed/replayed requests do not touch the token.
- CLI `--cancel-request-file PATH` is a caller-owned control transport. It polls at 5 ms, reads bounded bytes and targets the current one-shot or watch generation. The path is never exposed to the guest and grants no capability.
- CLI timer cancellation now records `Timer` instead of toggling an untyped boolean. `ObservedRun` retains the winning cancellation source for STEP-0099 event production.
- `--fuel` exposes an existing resource ceiling for deterministic runner fixtures; it changes quota only and grants no authority.

## 3. Race and negative rules

The frozen eight-row race matrix is executable in both orders:

- completion/client;
- timeout/signal;
- Host failure/client;
- guest failure/timeout.

The arbiter produces one terminal decision for each pair. Double cancellation is idempotent and preserves the first source. A timeout committed before a signal stays timeout; a signal observed first becomes cancelled. External process termination does not pass through this API and therefore cannot emit typed exit 123.

## 4. Runtime evidence

- A Python-created Windows process group sends a real `CTRL_BREAK_EVENT` to a native MSVC runner blocked in `sico.stream.read`; the runner exits 123 with class `cancelled`.
- A second native fixture sends the same console control while watch is idle after generation 1; the session exits 123 and leaves no child process.
- Canonical client-request files cancel a busy one-shot run and a generation-bound watch run. A stale generation is proved unable to cancel.
- STEP-0088 regression proves the same token reaches busy guest, blocked read, pump and blocked write paths promptly.
- STEP-0089 regression proves cancellation reaches a pending HTTP worker and that the abandoned Store refuses additional HTTP work.
- Unit evidence covers all eight race rows, double-cancel idempotence, bounded/canonical requests and typed source retention.

## 5. Validation

```powershell
./tools/validate-step-0098.ps1
```

Expected summary:

```text
STEP_0098_OK signal=windows-console-real client=one-shot+watch busy-read-pump-write-http=123 races=8/8 terminal=1 double-cancel=idempotent request=4096/4097-refused external-kill=not-cancelled authority=unchanged m11-adr=unlocked next=STEP-0099+STEP-0103
```

## 6. Boundary and next action

This step does not add execution-event transport or DAP. The control file is a minimal bounded client transport, not a general RPC server. Platform-specific OS mechanics are proven only on Windows x64 here; Linux native signal evidence remains an M10 exit requirement.

STEP-0099 now implements bounded task-aware execution events and redaction. In parallel, documentation-only M11 STEP-0103 is unlocked and must settle the Store/arena scheduler ADR before any M11 code.

## 7. Audit links

- [`report`](../reports/typed-cancellation-bridge-v0.md)
- [`handoff`](../handoffs/M10-STEP-0098.md)
- [`M10 plan`](../plans/M10-runtime-observability-debugging.md)
- [`M11 plan`](../plans/M11-structured-concurrency-runtime.md)
- [`validator`](../../tools/validate-step-0098.ps1)
