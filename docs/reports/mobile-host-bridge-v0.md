# Shared mobile Host bridge v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M6

## Result

GO for STEP-0055. `sico-mobile-host-core` reuses `HostStore` for signed install/open/uninstall and exposes a strict 64 KiB envelope with package bytes transported separately. Unknown input and missing packages fail closed; panics map to `NATIVE_PANIC` before the platform boundary.

The Kotlin declaration copies both byte arrays and contains no raw pointer, reflection, Java object persistence or shell path. It is contract-reviewed but not Kotlin-compiled because the licensed Android SDK/Gradle environment is absent.

