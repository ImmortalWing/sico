# M7 plan: Ecosystem, tooling and release

> - status: local-complete, blocked-external-evidence
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
| STEP-0065 | secure update and rollback | **complete**: consistent snapshots, monotonic journal, staged activation, advisories, explicit recovery and 1,592 mutations |
| STEP-0066 | standard-library and package dependency stability | **complete**: source-aware resolver, canonical lock, SemVer, compatibility, capability closure, yank/revocation and 1,140 mutations |
| STEP-0067 | LSP, editor and debugging workflow | **complete**: bounded stdio LSP, compiler diagnostics/index, UTF-16 navigation, format, shell-free run and explicit debug refusal |
| STEP-0068 | AI tooling protocol and measured evaluation | **complete-offline**: compiler-backed inspect/fix, 54-source/12-fix measurements, 96-task regression; live model not authorized |
| STEP-0069 | third-party-style Component and real-app pilot, M7 exit audit | **complete-local / blocked-external-evidence**: two releases, Wasmtime `42`, four refusals, 24 cases and project audit; actual third party remains external |

## 4. Mobile platform gates

- Android: retain the STEP-0060/0061 blocker until a buildable Gradle/JNI Host, licensed SDK/NDK, x86_64 emulator and arm64 device provide same-digest Runtime/UI/lifecycle evidence.
- Harmony: require a future audited feasibility/contract step before implementation; do not assume Android adapters or evidence transfer.
- Final M7/project exit: require every platform named as supported to have its own build/runtime evidence. Platform-neutral work cannot satisfy this gate.

## 5. External resumption action

The repository-local sequence is closed. Resume only when an independent participant, production identity/public service authorization, live-model credentials/budget or a target-platform runner is available. Follow the M7 exit audit and saved platform handbooks; never promote clean-room fixtures into external evidence.
