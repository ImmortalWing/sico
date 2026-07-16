# M7 plan: Ecosystem, tooling and release

> - status: in-progress, platform-independent track
> - created: 2026-07-16
> - phase: M7
> - entry requirement: M4/M5 package and Desktop Host contracts; repository-owner authorization to defer mobile tracks
> - execution boundary: platform-independent work may proceed; mobile and final exit claims remain gated

## 1. Outcome

Enable an external developer to develop, check, build, publish, discover, install, update, run and debug a Sico application without modifying the compiler or Runtime. Production identity, distribution and update security are established before public ecosystem flows.

## 2. Entry and decision gates

- STEP-0062–0068 may implement platform-independent contracts, fixtures and tools without claiming Android or Harmony support.
- M6 remains `blocked-external-runner`; the same exact signed `.sapp` still requires verified Desktop and Android execution before Android support or M6 completion is claimed.
- HarmonyOS/OpenHarmony has no accepted plan or implementation. It is deferred scope, not a supported or blocked-complete platform.
- Production publisher identity, key custody, rotation and revocation require repository-owner decisions and must not reuse development keys.
- Registry publishing, legal terms, public namespaces and security disclosure channels require explicit authorization.
- Package/update metadata remains deterministic, signed and rollback-safe; registry data never weakens local Host verification.
- LSP and AI tools consume compiler diagnostics/index contracts instead of reimplementing language semantics.

## 3. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0062 | ecosystem/release threat model and compatibility contract | **complete**: 32-threat/12-surface matrices; ADR-0006 and RFC-0022 accepted |
| STEP-0063 | production publisher identity and key lifecycle | **complete**: canonical policy, five role thresholds, exact claims, rotation/revocation/recovery and 2,048 mutations |
| STEP-0064 | signed registry publish/discovery/download | **complete**: four signed schemas, namespace lifecycle, immutable local transport, 3 checkpoints, 1,231 mutations and strict download reverify |
| STEP-0065 | secure update and rollback | signed channels, monotonic policy, rollback recovery, partial/corrupt update corpus |
| STEP-0066 | standard-library and package dependency stability | version/compatibility policy, deterministic resolution and capability closure across dependencies |
| STEP-0067 | LSP, editor and debugging workflow | diagnostics/index-driven completion, navigation, format, run/debug and bounded protocol tests |
| STEP-0068 | AI tooling protocol and measured evaluation | structured inspect/fix APIs, offline corpus, explicitly authorized live evaluation |
| STEP-0069 | third-party Component and real-app pilot, M7 exit audit | external workflow, release drill, security/performance/regression and project-completion review |

## 4. Mobile platform gates

- Android: retain the STEP-0060/0061 blocker until a buildable Gradle/JNI Host, licensed SDK/NDK, x86_64 emulator and arm64 device provide same-digest Runtime/UI/lifecycle evidence.
- Harmony: require a future audited feasibility/contract step before implementation; do not assume Android adapters or evidence transfer.
- Final M7/project exit: require every platform named as supported to have its own build/runtime evidence. Platform-neutral work cannot satisfy this gate.

## 5. Immediate next action

Proceed to STEP-0065 secure update/rollback using STEP-0064 immutable records, local filesystem transport and test-only policies. Implement persisted monotonic trust, freeze/rollback checks, partial/corrupt update recovery and atomic activation without public operations.
