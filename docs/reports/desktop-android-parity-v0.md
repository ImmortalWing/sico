# Desktop and Android parity app v0 review

> - status: partial-runtime-evidence
> - date: 2026-07-16
> - phase: M6

## Result

One in-memory development-signed `.sapp` is built once and its exact bytes are supplied to both paths. The Windows/Desktop path installs, reverifies and executes the package with Wasmtime 46.0.1, producing `42`. The Mobile Host bridge installs and reopens those same bytes and produces byte-for-byte equal installed metadata: app identity, signer identity, revision digest, capability fingerprint and package length. Mutated bytes and fabricated identities fail closed.

The test does not execute the Component on Android. This environment has neither a licensed Android SDK/NDK nor an emulator/device/ADB runner, so the same-digest Android result and native permission behavior required by the M6 exit gate remain unproven.
