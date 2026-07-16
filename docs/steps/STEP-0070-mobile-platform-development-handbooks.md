# STEP-0070: Android and Harmony development handbooks

> - status: complete
> - phase: M6/M7 support documentation
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

Preserve detailed, executable Android and Harmony development handbooks so deferred mobile work can resume without losing security, lifecycle, FFI, build, test, signing or evidence boundaries.

## 2. Context and evidence

The Android contract and Kotlin adapters exist, but Gradle/JNI/HostActivity/APK/device evidence do not. Harmony has no selected platform variant, SDK baseline, ADR/RFC, project or runtime evidence. The 2026-07-16 environment recheck confirms that both platform toolchains and runners are unavailable.

Primary references reviewed on 2026-07-16 include Android SDK/NDK, storage, lifecycle, permissions, JNI, testing and accessibility documentation; Huawei HarmonyOS Stage/ArkUI/application documentation; OpenHarmony Stage, package, Node-API, Want, permission, file sharing and arkXtest documentation; Rust Android/OpenHarmony target support; and Wasmtime platform tiers.

## 3. Scope

Included: truthful current-state audit; prerequisites; target project layouts; Gradle/hvigor and Rust cross-build paths; JNI/C ABI/Node-API ownership; file ingress; permission/storage; lifecycle; native UI; Runtime backend probes; test matrices; evidence; signing/release boundaries; troubleshooting and official references.

Excluded: installing or accepting platform SDK licenses, choosing the HarmonyOS/OpenHarmony product target, implementing either Host, creating production credentials, signing a release, claiming Runtime support or changing M6 NO-GO.

## 4. Decision

Android documentation follows accepted ADR-0005/RFC-0021 and freezes minSdk 28, compile/target SDK 36, NDK r27d, arm64-v8a/x86_64 and the existing limits.

Harmony documentation remains proposed. It requires a platform-variant decision and new ADR/RFC before code. Its recommended first probe uses Stage/ArkUI, an ArkTS-to-Node-API C++ shim, a stable C ABI Rust cdylib and `sico-mobile-host-core`; it does not claim HarmonyOS and OpenHarmony binary compatibility or Wasmtime support.

## 5. Changes

- Added `docs/platforms/ANDROID-DEVELOPMENT.md`.
- Added `docs/platforms/HARMONY-DEVELOPMENT.md`.
- Added the shared implementation/evidence checklist.
- Added a machine-readable documentation/status contract.
- Added a validator and documentation indexes.

STEP-0070 was intentionally an out-of-order documentation support step requested before the STEP-0063–0069 implementation sequence completed. Those sequential steps retained their allocated meanings and are now closed locally; the handbooks still do not change mobile evidence status.

## 6. Validation

```powershell
& .\tools\validate-step-0070.ps1
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:__COMPAT_LAYER = 'RunAsInvoker'
& "$HOME/.cargo/bin/cargo.exe" fmt --all -- --check
& "$HOME/.cargo/bin/cargo.exe" clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& "$HOME/.cargo/bin/cargo.exe" test --offline --locked --workspace --all-targets --all-features
git diff --check
```

No Android or Harmony build command is claimed as executed. The validator freezes documentation completeness and the truthful blocked/proposed status only.

Latest regression result: `STEP_0070_OK android=blocked-not-implemented harmony=proposed-not-implemented android_links=23 harmony_links=28 sequence=closed-local next=external-evidence-or-STEP-0072`. Local links, `git diff --check`, Rust formatting, Clippy with warnings denied and the complete offline locked workspace test suite passed. The original out-of-order documentation sequence is retained in Git history rather than presented as the current step.

## 7. Risks and follow-ups

- Android plugin/tool versions and platform policies must be re-audited on the day implementation resumes.
- HarmonyOS vs OpenHarmony, SDK/API, device matrix and signing/distribution must be selected before implementation.
- Exact hvigor/hdc tasks come from the selected generated project and SDK help; the handbook deliberately does not invent a universal command.
- Platform Runtime status remains NO-GO until arm64 device evidence exists.

## 8. Audit links

- [platform documentation index](../platforms/README.md)
- [Android handbook](../platforms/ANDROID-DEVELOPMENT.md)
- [Harmony handbook](../platforms/HARMONY-DEVELOPMENT.md)
- [shared checklist](../platforms/MOBILE-SHARED-CHECKLIST.md)
- [machine-readable contract](../../tests/platform/mobile-documentation-contract.json)
- [environment recheck](../../tests/platform/environment-2026-07-16-recheck.json)
