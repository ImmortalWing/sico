# STEP-0058: Android Runtime and lifecycle supervision

> - status: complete-cross-check
> - phase: M6
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## Evidence

- Android-neutral Host core checks for arm64 and x86_64 Android Rust targets.
- Pinned Wasmtime 46.0.1 integration plan: native with proven executable memory, otherwise Pulley.
- Activity descriptors only; process restore never assumes the guest is running.
- Foreground/background suspend, queue 256 and one terminal winner.
- Five lifecycle/backend tests and reviewed Kotlin contract.
- Android SDK/NDK runtime unavailable; no device execution is claimed.

```text
STEP_0058_OK runtime_contract=wasmtime-46.0.1 android_core_checks=aarch64,x86_64 backends=native-cranelift,pulley lifecycle=descriptor-reverify queue=256 terminal=single-winner tests=5 android_runtime=unavailable next=STEP-0059
```
