# STEP-0055: Shared mobile Host core and JNI boundary

> - status: complete
> - phase: M6
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## Objective

实现 Android-neutral shared Host core 和 typed JNI declaration，使 platform adapter 无法绕过 M5 trust/install/open 语义。

## Evidence

- strict `sico.android.bridge.v0` JSON envelope, 64 KiB；
- separate copied package bytes；
- probe/install/open/uninstall typed operations；
- panic -> `NATIVE_PANIC`；unknown/malformed/oversized fail closed；
- 4 Rust tests；arm64/x86_64 Android cross-compile checks；
- Kotlin declaration contract-reviewed, not SDK-compiled。

```text
STEP_0055_OK crate=sico-mobile-host-core bridge=typed-json bytes=separate-copied panic=NATIVE_PANIC operations=probe,install,open,uninstall tests=4 android_checks=aarch64,x86_64 kotlin=contract-reviewed next=STEP-0056
```

