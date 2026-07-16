# Report: Ecosystem and release contract v0

> - status: complete
> - date: 2026-07-16
> - related-step: STEP-0062
> - environment: Windows x86_64; Rust 1.97.0 stable GNU; no mobile SDK/runner

## 1. Question

Can Sico safely advance platform-independent ecosystem design while Android and Harmony tracks are deferred, without weakening the existing package/Host trust boundary or overstating platform completion?

## 2. Method

Audit RFC-0015–0021, `sico-package`, `sico-host-core`, the M6 runner gate and local tools. Compare their gaps with current TUF, Sigstore, OCI Distribution and SemVer primary specifications. Encode every accepted invariant in bounded JSON matrices and validate coverage mechanically.

## 3. Reproduction

```powershell
& .\tools\validate-step-0062.ps1
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:__COMPAT_LAYER = 'RunAsInvoker'
```

Full Rust regression commands are recorded in the step document and run using the installed `stable-x86_64-pc-windows-gnu` toolchain.

## 4. Raw evidence

- [`environment recheck`](../../tests/platform/environment-2026-07-16-recheck.json)
- [`threat matrix`](../../tests/ecosystem/threat-matrix.json)
- [`compatibility matrix`](../../tests/ecosystem/compatibility-matrix.json)
- existing [`Android runner gate`](../../tests/android-host/runner-gate.json)

## 5. Results

| Result | Evidence class |
|---|---|
| Android SDK, NDK, ADB, AVD, emulator, Gradle, Java and connected device absent | measured local command/path/device audit |
| repository Android Host lacks Gradle, HostActivity and JNI crate | verified repository audit |
| installed Rust stable compiler is 1.97.0; Android Rust targets are absent | measured toolchain audit |
| no Harmony references, plan, SDK or implementation exist | verified repository/environment audit |
| package bytes and local trust gates can remain platform-independent | verified current code/contract |
| 32 threats and 12 compatibility surfaces are covered | verified validator |
| production signing/registry/update behavior | proposed for STEP-0063–0066, not implemented |

## 6. Interpretation

Android is a real implementation-and-runner blocker, not a blocker for threat modeling, LSP, AI tooling or other platform-neutral contracts. Harmony is not blocked implementation; it is currently absent scope and must not be presented as supported. A split-track roadmap is therefore truthful if M6 and final mobile/product exit claims remain blocked.

The older Android environment snapshot recorded two Android Rust targets and Java. The new recheck intentionally does not rewrite that historical record; it captures current state and corrects current-status documentation.

## 7. Limitations

No mobile build or execution was attempted. No production identity, external registry, transparency log, namespace, key ceremony or live network protocol was tested. The standards comparison supports design choices but is not implementation evidence.

## 8. Decision impact

STEP-0062 is complete. STEP-0063 is next for platform-neutral work, but it must stop at fixtures and local policies wherever owner identity, private-key custody, legal terms or public operations are required.

## 9. Links

- [`ADR-0006`](../adr/ADR-0006-ecosystem-release-trust-boundary-v0.md)
- [`RFC-0022`](../rfc/RFC-0022-ecosystem-compatibility-contract-v0.md)
- [TUF specification v1.0.35](https://theupdateframework.github.io/specification/v1.0.35/)
- [Sigstore overview](https://docs.sigstore.dev/)
- [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md)
- [Semantic Versioning 2.0.0](https://semver.org/)
