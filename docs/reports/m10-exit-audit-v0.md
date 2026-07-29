# M10 security, performance and exit audit

> - status: accepted
> - date: 2026-07-24
> - phase: M10
> - decision: GO

## Decision

M10 is **GO** for the platform and authority scope actually exercised. STEP-0095–0101 establish an end-to-end identity-bound path from compiler debug artifacts through typed Runtime faults, cancellation and bounded execution events to a real minimal DAP adapter and data-only editor/AI consumers. STEP-0102 reruns the complete M0–M9 aggregate before rerunning every M10 validator.

This is Windows x64 GNU Runtime evidence. It is not Linux, macOS, Android, Harmony, production deployment, independent third-party or live-model evidence.

## Exit-gate matrix

| Gate | Evidence | Result |
|---|---|---|
| Identity continuity | strict source/compiler/Component/map schemas; deterministic digests; mixed and stale artifacts refused | pass |
| Deterministic mappings | exact real Core offsets, generated rows, atomic Component/map/identity triplet | pass |
| Typed faults | nine stable classes, exact verified frames, 256-frame bound, no engine-prose classification | pass |
| Cancellation | real Windows console control, canonical client requests, blocked/busy/HTTP paths return 123, one terminal winner | pass |
| Event transport | 64 KiB chunks, 1 MiB captures, 256-item/4 MiB queue, explicit overflow and reserved terminal | pass |
| Exact DAP | 12 supported requests, 20 typed refusals and 6 supported events; real entry/source breakpoint, pause, stack, locals and teardown | pass |
| Editor/AI boundary | direct argv, `shell:false`, identity-bound launch plan, redacted data-only summaries, no execution authority | pass |
| Session isolation | 100 sequential DAP sessions with stable handle count and bounded RSS; poisoned/trapped sessions do not leak state | pass |
| M0–M9 regression | STEP-0094 aggregate, workspace format/Clippy/tests and prior milestone validators | pass |
| Platform honesty | native execution limited to Windows x64 GNU; other platforms remain unclaimed | pass |

## Security and adversarial evidence

- Debug maps, identities, events, faults and DAP frames reject unknown fields, stale digests, truncation, oversized input and cross-session mixing.
- Source bytes must match the verified document digest and byte length before breakpoint or frame mapping.
- DAP has a sole machine-readable allowlist. Unknown and refused requests receive typed failures, and the adapter cannot emit unclaimed events.
- Output is bounded and redacted before it leaves the runner. Binary bytes use explicit base64 representation; fault summaries omit raw messages.
- Pause, continue, disconnect and terminate operate only on the session-owned Store and do not widen filesystem, network, package or Host grants.
- Console, client, timer and Host cancellation converge on one terminal arbiter. External process death is not forged into typed cancellation.
- The audit repaired two validation hazards rather than accepting false evidence: Windows PowerShell 5.1's `NativeCommandError` treatment of ordinary Cargo stderr, and parallel tests sharing process-global Windows console state.

No unresolved M10 P0 correctness, isolation or authority-expansion issue was found.

## Performance and resource evidence

All latency figures are local non-SLA observations. Hard byte, item, identity and teardown bounds remain correctness gates.

| Measurement | Observed result | Assessment |
|---|---:|---|
| Source lookup over 100,000 mappings | P95 2.628 µs per lookup | candidate goal met |
| DAP pause | median 12.146 ms; P95 54.793 ms | measured, spawn excluded |
| DAP continue | median 5 µs; P95 6 µs | measured |
| Debug build | median 4.576 ms vs 2.333 ms normal; P95 7.869 vs 7.657 ms | measured overhead, no SLA |
| 100 sequential DAP sessions | handles 92 → 92; RSS 28,377,088 → 28,442,624 bytes | bounded/no handle growth |
| 256 MiB stream passthrough | peak runner RSS 11,997,184 bytes | hard memory behavior pass |
| HTTP cancellation | 186 ms total with a 100 ms test timer | cancellation delivery pass |
| Persistent warm rerun | median 6.983 ms | candidate goal met |
| REPL 256-cell growth | 110 ms; 7.8 MiB peak RSS | bounded |
| Latest M8 cold `sico run` P95 | 691.2816 ms, 20 samples | non-SLA 200 ms goal miss |
| Latest M8 warm `sico run` P95 | 138.9047 ms, 20 samples | non-SLA 120 ms goal miss |
| Latest runner-only P95 | 59.4781 ms, 20 samples | contextual measurement |

The cold and warm launch misses are recorded, not converted into a correctness failure or silently replaced with a faster earlier run. The runner-only result and host-load sensitivity show that these figures require broader environment sampling before any SLA can be set.

## Platform matrix

| Platform | M10 native evidence | Claim |
|---|---|---|
| Windows x64 GNU | compiler, runner, console cancellation, events, fault maps, DAP, editor/AI and performance | runtime-verified for M10 |
| Linux/WSL | no M10 native rerun yet | not M10-verified |
| macOS | none | not verified |
| Android/Harmony | separate partial/deferred tracks | not verified |

WSL2 may be added as independent Linux evidence later. It cannot replace Windows console-control evidence and does not retroactively widen this matrix.

## External-gate recheck

No new production deployment, legal publisher identity, public registry authority, independent third-party pilot, live-model authorization/credentials, Android runner or Harmony runner evidence was supplied during M10. Therefore:

- project/product completion remains `blocked-external-evidence`;
- M7 production and independent-user gates remain NO-GO;
- live-model evaluation remains `live-model-not-authorized`;
- Android remains partial Runtime evidence and Harmony remains deferred;
- Linux/macOS remain unsupported by M10 native evidence.

Internal M10 GO does not alter those external decisions.

## Residual risks and handoff

- DAP intentionally excludes stepping, evaluate, mutation, memory access, attach and reverse execution.
- Wasmtime 47.0.2 is pinned for correct unaligned debug-frame reads; future engine upgrades must rerun exact DAP and mapping evidence.
- Process-global console tests must remain serialized.
- Launch latency needs repeated measurements on additional hosts before promotion from a candidate goal.
- Linux signal/DAP behavior should be rerun natively when a WSL2 or Linux host is available.

M11 implementation may proceed from accepted ADR-0010 and STEP-0103. M12 remains sequenced after structured-concurrency ownership and cancellation semantics are stable.

## Reproduction

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0102.ps1
```
