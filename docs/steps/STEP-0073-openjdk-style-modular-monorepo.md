# STEP-0073: OpenJDK-style modular monorepo

> - status: complete
> - date: 2026-07-16
> - scope: repository architecture, CLI ownership, release documentation
> - decision: [`ADR-0007`](../adr/ADR-0007-openjdk-style-modular-monorepo.md)
> - archive branch: `codex/archive-v0.0.1-development-history`

## Goal

Preserve the complete v0.0.1 development process and then change `main` from one cross-layer CLI into an OpenJDK-style modular monorepo: one atomic repository with machine-enforced module boundaries and separately owned compiler, application Runtime and platform Host commands.

## Archive baseline

The archive branch was created at annotated tag `v0.0.1`, commit `dd5df41656cd2fbdabccc54a297eec45f79c348b`, before any modularization change. The same branch was pushed to `origin` and `github`. It retains all STEP documents, examples, validators and the original `sico build/run/inspect` workflow.

## Changes

1. Accepted ADR-0007 and added a machine-readable eight-module ownership map.
2. Added a validator that assigns every workspace package exactly once and rejects forbidden normal dependencies.
3. Removed `sico-package` and `sico-runtime` from the normal dependencies of `sico-cli`.
4. Limited `sico` to `check`, `format`, `outline` and Component `build`.
5. Added `sico-app` with explicit `.sapp` `pack`, `inspect` and `run` commands.
6. Removed implicit source compilation/cache from the Runtime-facing command path.
7. Updated Windows Desktop Host measurement and prior CLI validators to use the explicit compiler → package → Runtime pipeline.
8. Updated the root README and detailed user manuals for `0.0.2-dev` while preserving `v0.0.1` reproduction instructions.

## Validation

- `cargo fmt --all -- --check`;
- strict full-workspace Clippy with `-D warnings`;
- full-workspace tests across all targets and features;
- `MODULE_BOUNDARIES_OK model=openjdk-style packages=22`;
- STEP-0020 language CLI contract;
- STEP-0036 real Wasmtime `42/true/()` modular pipeline;
- STEP-0044 application CLI trust and separation contract;
- STEP-0072 user-manual link and command contract;
- Windows Desktop Host release smoke using a signed package produced by `sico-app`.

## Evidence boundary

This architecture step changes source ownership and development commands. It does not upgrade Android, HarmonyOS/OpenHarmony, Linux-native, production publisher, public registry, live-model or independent third-party evidence.

```text
STEP_0073_OK archive=v0.0.1 remotes=origin,github model=openjdk-style packages=22 language_cli=sico app_cli=sico-app host_cli=sico-desktop-host workspace=fmt,clippy,test runtime=wasmtime-46.0.1 docs=linked version=0.0.2-dev
```
