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
| [`M7 ecosystem, tooling and release`](./M7-ecosystem-release.md) | local-complete, blocked-external-evidence | STEP-0062–0069 complete locally; product exit awaits third-party/production/live-model/platform evidence |
| [`M8 Script Profile v0`](./M8-script-profile.md) | GO; STEP-0075–0084 complete | bounded args/stdin/stdout, general executable codegen, Script WIT/adapter/runner, cache and minimal useful standard library |
| [`M9 streaming, async and interactive scripting`](./M9-streaming-async-interactive.md) | GO; STEP-0085–0094 complete | streaming resources, Task/Future/Stream backend, scoped HTTP, persistent runner, watch, REPL, syntax/tooling integration and exit audit |
| [`M10 Runtime observability and debugging`](./M10-runtime-observability-debugging.md) | complete / GO; STEP-0095–0102 | source/debug identity, Runtime frames, typed cancellation, bounded events, DAP and client integration |
| [`M11 bounded structured-concurrency Runtime`](./M11-structured-concurrency-runtime.md) | complete / GO; STEP-0103–0110 | Store/arena ADR, cooperative scheduler, structured tasks, bounded queues, authority-neutral parallelism and Windows/Linux native exit |
| [`M12 Secure HTTP Provider and Automation SDK`](./M12-secure-http-automation-sdk.md) | GO-core; full integration pending | mature TLS, exact endpoint/DNS policy, streaming bodies, redirects, opaque secrets, SDK/tooling and exit audit |
| [`M13 AI tooling closure`](./M13-ai-tooling-closure.md) | parallel support track; STEP-0119–0121 complete, STEP-0122–0123 reserved | measured generation quality, quality budget, MCP integration, semantic-index/error measurements and closure audit |
| [`M14 application-ready language baseline`](./M14-application-ready-language.md) | planned; no STEP numbers reserved | executable source/backend parity, general algorithms, dynamic collections, modules/packages/WIT and representative application corpus |
| [`M15 Web platform and UI controls`](./M15-web-ui-platform.md) | planned; no STEP numbers reserved | Web Host decision, compiler-facing UI binding, controls/events/DOM/storage/accessibility and browser evidence |
| [`M16 Native Automation Host`](./M16-native-automation-host.md) | planned; no STEP numbers reserved | scoped window capture/input, preview/commit, observation revision, audit/stop and native platform evidence |
| [`M17 Vision and model package ecosystem`](./M17-vision-ml-ecosystem.md) | planned; no STEP numbers reserved | image contracts, deterministic CV packages, optional accelerated providers and bounded model inference |
| [`M18 representative AI applications and external pilots`](./M18-ai-application-pilots.md) | planned; no STEP numbers reserved | API/data/Web/native-vision applications, block-game acceptance path and honest external evidence classes |
