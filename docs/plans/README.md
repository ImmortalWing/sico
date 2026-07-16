# Execution plans

阶段计划把 ROADMAP 的里程碑拆成有依赖、退出证据和稳定 STEP 编号的可执行序列。计划可以因新证据调整，但调整必须更新状态、理由和后续编号，不能让实现隐式改变 RFC/ADR。

| Plan | Status | Scope |
|---|---|---|
| [`M1 compiler frontend`](./M1-compiler-frontend.md) | complete | Rust source/span、lossless lexer/parser、恢复、formatter、`sico check`、outline 与 M1 audit |
| [`M2 static semantics`](./M2-static-semantics.md) | complete | full HIR、names/types/control/effects/resources、semantic diagnostics/index 与 M2 audit |
| [`M3 Sico IR and Component`](./M3-sico-ir-component.md) | complete | typed IR/verifier、lowering、Core Wasm/Component/WIT、async boundary、CLI 与 M3 audit |
| [`M4 .sapp and secure Runtime`](./M4-sapp-runtime.md) | complete | package/manifest/hash/signature、capability closure、storage/limits、package CLI 与 M4 audit |
| [`M5 Sico Desktop Host`](./M5-desktop-host.md) | complete | threat/lifecycle、install/open、permission UI、UI WIT/SDK、desktop adapters 与 M5 audit |
| [`M6 Sico Android Host`](./M6-android-host.md) | blocked-external-runner | shared Android contracts complete; device Runtime/UI evidence pending |
| [`M7 ecosystem, tooling and release`](./M7-ecosystem-release.md) | in-progress, mobile-gated | STEP-0062 complete; platform-independent publisher/registry/update/LSP/AI work may proceed while mobile/final exit stays gated |
