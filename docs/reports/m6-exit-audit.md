# M6 Android Host exit audit

> - status: blocked-external-runner
> - date: 2026-07-16
> - phase: M6

> Sequencing note (updated 2026-07-16): M6 remains NO-GO. ADR-0006 permitted the now-complete repository-local M7 track; this audit still controls Android support, while the M7 exit audit controls product completion.

NO-GO: M6 remains blocked at the Android runner exit gate. Resume STEP-0060 device validation before claiming Android support or M6 completion; the completed platform-neutral M7 local track does not satisfy this gate.

## Requirement audit

| Gate | Evidence | Result |
|---|---|---|
| Android threat and platform contract | 30-threat matrix, accepted ADR-0005 and RFC-0021 | proven |
| shared Host/JNI protocol | typed bounded bridge, owned bytes, panic/error mapping | core proven; JNI load unavailable |
| file/share/link ingestion | content-only URI, one-pass bounded copy, picker-only deep link | contract and Rust tests proven |
| permission and storage isolation | exact five-capability map, ephemeral URI grant, app-private app+signer scope | shared contract proven |
| Runtime and lifecycle | native/Pulley policy, descriptor restore, suspend, bounded duplicate-open, one terminal winner | shared contract proven; Android Runtime unavailable |
| touch, IME and accessibility | native widget mapping, 4 KiB input, 120/s and queue 256, TalkBack order | shared/Kotlin contract proven; device UI unavailable |
| same package parity | exact signed bytes, Desktop Wasmtime result `42`, identical Mobile Host identity/revision/capability metadata | Desktop runtime + mobile core proven; Android result unavailable |
| security/property corpus | bridge, Intent, lifecycle and UI: 2,048 each, 8,192 total | proven on host |
| performance | release Mobile Host bridge probe, 3 x 10,000, median mean 1.260 us, no SLA | host measured; Android startup unavailable |
| regression | formatting, full workspace Clippy/tests and M0-M5 exit GO chain | proven |

## External blocker

The current environment has no Android SDK, accepted SDK terms, NDK, ADB, AVD or connected device. The project does not accept legal terms autonomously. M6 therefore cannot satisfy its same-digest Android execution, lifecycle, native touch/IME/TalkBack or cold/warm startup gates.

The exact required matrix is recorded in `tests/android-host/runner-gate.json`: a licensed SDK/NDK, x86_64 emulator and arm64 device, followed by STEP-0060 device parity and a repeat of this audit.

## M7 disposition

The original audit blocked all M7 implementation. ADR-0006 later allowed STEP-0062–0068 platform-neutral work, and repository-owner direction subsequently allowed STEP-0069's local clean-room drill. Those local steps are complete; M6, Android support and the final M7/product exit remain blocked on real external/platform evidence.
