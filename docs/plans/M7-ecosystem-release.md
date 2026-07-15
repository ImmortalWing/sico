# M7 plan: Ecosystem, tooling and release

> - status: planned, blocked until M6 GO
> - created: 2026-07-16
> - phase: M7
> - entry requirement: M6 Android runner exit gate GO
> - execution boundary: stable Desktop and Android Host package/runtime contracts

## 1. Outcome

Enable an external developer to develop, check, build, publish, discover, install, update, run and debug a Sico application without modifying the compiler or Runtime. Production identity, distribution and update security are established before public ecosystem flows.

## 2. Entry and decision gates

- Do not start implementation until the same exact signed `.sapp` runs on verified Desktop and Android runners.
- Production publisher identity, key custody, rotation and revocation require repository-owner decisions and must not reuse development keys.
- Registry publishing, legal terms, public namespaces and security disclosure channels require explicit authorization.
- Package/update metadata remains deterministic, signed and rollback-safe; registry data never weakens local Host verification.
- LSP and AI tools consume compiler diagnostics/index contracts instead of reimplementing language semantics.

## 3. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0062 | ecosystem/release threat model and compatibility contract | publisher, namespace, registry, update, dependency, disclosure and rollback matrix; ADR/RFC accepted |
| STEP-0063 | production publisher identity and key lifecycle | offline/root vs online signing, rotation, revocation, recovery and audit fixtures |
| STEP-0064 | signed registry publish/discovery/download | deterministic metadata, namespace ownership, transparency and end-to-end local reverify |
| STEP-0065 | secure update and rollback | signed channels, monotonic policy, rollback recovery, partial/corrupt update corpus |
| STEP-0066 | standard-library and package dependency stability | version/compatibility policy, deterministic resolution and capability closure across dependencies |
| STEP-0067 | LSP, editor and debugging workflow | diagnostics/index-driven completion, navigation, format, run/debug and bounded protocol tests |
| STEP-0068 | AI tooling protocol and measured evaluation | structured inspect/fix APIs, offline corpus, explicitly authorized live evaluation |
| STEP-0069 | third-party Component and real-app pilot, M7 exit audit | external workflow, release drill, security/performance/regression and project-completion review |

## 4. Immediate next action

M7 is not executable yet. Provision a licensed Android SDK/NDK plus x86_64 emulator and arm64 device, resume STEP-0060 device parity, then repeat STEP-0061. A GO result unlocks STEP-0062.
