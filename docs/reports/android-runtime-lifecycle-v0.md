# Android Runtime and lifecycle v0 review

> - status: accepted-cross-check
> - date: 2026-07-16
> - phase: M6

## Result

STEP-0058 shared/cross-compile work is complete. The Android-neutral Host core compiles for Rust arm64 and x86_64 Android targets. The pinned Wasmtime 46.0.1 integration contract selects native Cranelift only after an executable-memory probe and otherwise selects the `pulley64` portable backend. Building or running Wasmtime for Android remains a device/NDK gate and is not claimed by this report.

Activity lifecycle stores only app/revision digests, suspends background work, bounds duplicate opens to 256, accepts one terminal outcome and restores process death as non-running state requiring reverify. No Android Runtime execution is claimed because no licensed SDK/NDK runner exists in this environment.
