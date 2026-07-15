# ADR-0005: Android Host runtime and lifecycle boundary v0

> - status: accepted
> - date: 2026-07-16
> - authors: autonomous-agent
> - scope: M6 Android Host

## Context

Android delivers external documents through Intent/URI and may recreate an Activity or kill its process. A content provider can change bytes between reads, OS runtime permissions may be revoked, and APK/AAB native code is ABI-specific. These facts cannot weaken M5 package identity, permission or lifecycle rules.

Current official evidence accessed 2026-07-16:

- Android recommends Storage Access Framework/content URIs for user-selected documents and app-private internal storage for sensitive app data;
- runtime permissions must be checked at each protected operation and process termination may follow revocation;
- Activity/process death requires durable restoration rather than relying on in-memory state;
- Rust Android targets are Tier 2 and use the NDK; Wasmtime supports Android less well than primary platforms, while its current tier table lists `aarch64-linux-android` with CI;
- current Android platform is API 36; the latest LTS NDK is r27d.

References: [Android intents](https://developer.android.com/guide/components/intents-filters), [Storage Access Framework](https://developer.android.com/training/data-storage/shared/documents-files), [runtime permissions](https://developer.android.com/training/permissions/requesting), [Activity lifecycle](https://developer.android.com/guide/components/activities/activity-lifecycle), [Rust Android targets](https://doc.rust-lang.org/beta/rustc/platform-support/android.html), [Wasmtime platform support](https://docs.wasmtime.dev/stability-platform-support.html), [Wasmtime tiers](https://docs.wasmtime.dev/stability-tiers.html), [NDK downloads](https://developer.android.com/ndk/downloads/).

## Decision

- Android adapter receives only `ACTION_VIEW`, `ACTION_SEND` or explicit picker results with exactly one `content:` URI and expected `.sapp` MIME/name evidence.
- It streams at most the package ceiling into Host-owned app-private staging, closes the provider, then installs/reopens only the copied digest. No external URI is executed directly.
- Shared Rust owns package parsing, trust, app+signer identity, permissions, storage keys and terminal lifecycle. Kotlin/JNI owns Activity, ContentResolver, system permission and native View translation only.
- v0 targets `arm64-v8a` and `x86_64`, Rust `aarch64-linux-android` and `x86_64-linux-android`, `minSdk 28`, `targetSdk 36`, and NDK r27d LTS when a licensed SDK is supplied.
- Activity recreation restores only immutable app/revision descriptors and validated UI state. Process death requires reopening and reverifying installed bytes; it never revives an assumed running guest.
- Runtime backend remains `Wasmtime native when proven, Pulley when executable-memory/native backend is unavailable`. Local evidence may not exceed cross-compile-check until an Android runner exists.

## Consequences

The adapter can be implemented and host-checked without an Android SDK, but M6 cannot claim device/runtime completion until a licensed SDK/NDK plus emulator or device executes the same signed `.sapp`. Production APK signing and Play publishing remain M7.

