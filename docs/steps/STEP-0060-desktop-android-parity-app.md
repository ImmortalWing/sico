# STEP-0060: Same signed package Desktop and Android parity app

> - status: partial-runtime-evidence
> - phase: M6
> - started: 2026-07-16
> - completed-host-evidence: 2026-07-16
> - owners: autonomous-agent

## Evidence

- One exact signed package is built once for both paths.
- Desktop install/open executes the Component with Wasmtime 46.0.1 and returns `42`.
- Mobile bridge install/open returns identical identity, revision, signer and capability metadata.
- Package mutation and fabricated identity fail closed.
- Android device execution is unavailable and remains an explicit M6 exit blocker.

```text
STEP_0060_PARTIAL package=same-bytes desktop_result=42 metadata=identical mutation=refused android_runtime=unavailable blocker=licensed-sdk-ndk-runner next=STEP-0061
```
