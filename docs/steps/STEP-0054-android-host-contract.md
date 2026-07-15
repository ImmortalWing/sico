# STEP-0054: Android Host threat, lifecycle and packaging contract

> - status: complete
> - phase: M6
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## Objective

审计当前 Android 环境和上游支持，冻结 Intent/URI、JNI、identity/storage、Activity/process lifecycle、ABI/packaging 与证据边界。

## Changes

- 30-case Android threat matrix；
- accepted ADR-0005 and RFC-0021；
- installed Rust arm64/x86_64 Android std targets；
- frozen minSdk 28, targetSdk 36, NDK r27d LTS, arm64-v8a/x86_64；
- recorded SDK/NDK/ADB/Emulator/device unavailable and license not accepted；
- evidence ceiling: host-tested + cross-compile-check, not runtime-verified。

## Validation

```text
STEP_0054_OK threats=30 rust_targets=aarch64,x86_64 sdk=unavailable ndk=unavailable runner=unavailable bridge=typed-json uri=copy-then-verify lifecycle=descriptor-restore evidence_ceiling=cross-compile-check next=STEP-0055
```

## Links

- [`ADR-0005`](../adr/ADR-0005-android-host-runtime-lifecycle-boundary-v0.md)
- [`RFC-0021`](../rfc/RFC-0021-android-intent-jni-contract-v0.md)
- [`review`](../reports/android-host-contract-v0.md)

