# Sico project status

> - updated: 2026-07-17
> - phase: M8 Script Profile backend foundation; M7 public deployment and M6/mobile deferred
> - phase status: in-progress
> - current step: STEP-0077 fixed-width dynamic scalars (planned)
> - last completed active step: STEP-0076 (M8 composition architecture gate complete)
> - last completed support step: STEP-0074 (production origin and one-command workflow, local complete)
> - next step: start STEP-0077 by freezing explicit I64/U64 semantic and verified-IR operations, then implement dynamic scalar parameters/locals/results without narrowing Int

## 0. M6 exit state

STEP-0054–0061 host-side work, 8,192 security properties, Desktop result `42`, Mobile Host metadata parity and full workspace regression are complete. M6 remains NO-GO because this environment has no Android build toolchain, buildable Host or runner; no Android Runtime, native touch/IME/TalkBack or startup evidence is claimed. Android and Harmony tracks are deferred. M7 STEP-0062–0069 is complete locally, including a two-release Wasmtime pilot; production, third-party, live-model, mobile/Linux-native and final product exit claims remain gated.

## 1. Current objective

仓库所有者已于 2026-07-17 授权新的生产部署路线。STEP-0074 已建立可部署但不持有签名密钥的只读 registry origin、独立 operator bundle、Windows release 集成和 `sico-app dev` 单命令源码流程。由于当前没有 VPS，公网部署继续等待真实域名/hosting、生产身份与密钥托管输入；当前主动目标切换为 M8 Script Profile，先解决本地 AI 自动化所需的 args/stdin/stdout、可执行后端、Script ABI、runner、缓存与最小标准库。Android、Harmony 与 Linux native 路径保留为后置平台轨。

## 2. Current step

[`STEP-0069`](./steps/STEP-0069-third-party-pilot-m7-exit.md) 已用独立于 `examples/` 的两版洁净室应用完成 LSP/AI、Component、签名包、本地 registry、Host upgrade 和 Wasmtime `42`，并拒绝四类攻击。[`M7 exit audit`](./reports/m7-exit-audit.md) 结论为 `blocked-external-evidence`。[`STEP-0070`](./steps/STEP-0070-mobile-platform-development-handbooks.md) 与 [`STEP-0071`](./steps/STEP-0071-linux-development-handbook.md) 只保存平台手册，没有改变平台证据状态。

[`STEP-0072`](./steps/STEP-0072-user-manual-information-architecture.md) 已把根 README 改为用户入口，将原开发状态内容迁入独立开发手册，并建立安装、入门、语言、CLI、Runtime、trust、LSP、AI、排障与限制的细分用户手册；文档变化不提高任何平台或 production 证据等级。

[`STEP-0073`](./steps/STEP-0073-openjdk-style-modular-monorepo.md) 已在两个远端归档完整 `v0.0.1` 开发流程，并按 OpenJDK 模式保持单仓库、拆分 `sico` 与 `sico-app` 命令、建立 22 个 package 的机器依赖边界。该步骤不提高任何平台或 production 证据等级。

[`STEP-0074`](./steps/STEP-0074-production-deployment-origin-and-ux.md) 新增第 23 个 workspace package `sico-registry-server`，通过 loopback HTTP origin、operator ZIP、release composition 和真实 Wasmtime `42` 完成本地生产部署基线。它没有创建公网域名、TLS、生产发布者或真实密钥，证据等级为 `complete-local`。

[`STEP-0075`](./steps/STEP-0075-script-profile-contract.md) 已冻结 M8 Script Profile 契约与纵向原型 gate。[`STEP-0076`](./steps/STEP-0076-script-profile-vertical-prototype.md) 已完成：direct 与显式 Program/Adapter composition 两条路径连续 20 次均通过 6/6，915-byte Adapter digest 稳定，composed cold P95 15.8033 ms、worst recorded warm P95 0.2038 ms，裁决为 `composition-go`，direct path 保留为诊断回退。下一项 [`STEP-0077`](./steps/STEP-0077-fixed-width-dynamic-scalars.md) 将贯通独立 `I64/U64`，不得静默缩窄 `Int`。proposed [`RFC-0029`](./rfc/RFC-0029-script-profile-v0.md) 与 [`ADR-0009`](./adr/ADR-0009-script-adapter-runner.md) 仍等待后续 compiler/package/runner/security gate；[`M9 plan`](./plans/M9-streaming-async-interactive.md) 仅规划 M8 后的 streaming/async/HTTP/watch/REPL 路径。

## 3. Verified repository facts

基线检查时间：2026-07-17。

| 事实 | 结果 | 状态 |
|---|---:|---|
| `.sico` 文件 | 214 | measured |
| 代表性程序 | 10 | verified by `examples/` |
| P0 A0 语义案例 | 54 | verified by semantic case validator |
| 候选 B 案例 | 54 | verified by semantic case validator |
| 候选 C 案例 | 54 | verified by semantic case validator |
| 单点语法错误变体 | 36 | verified by mutation validator |
| 固定 AI 评测任务 | 96 | verified by AI evaluation validator |
| 稳定诊断 code/key | 36（其中 12 个由 parser 产生） | verified by diagnostic validator |
| 已映射非法 case | 29 | verified by diagnostic validator |
| Rust `.rs` 文件 | 89 | measured |
| `Cargo.toml` | 29 | measured |
| 数值原型单元测试 | 10 passed | verified |
| resource/async 动态测试 | 10 passed | verified |
| WASI 0.3 WIT parser 测试 | 1 passed | verified |
| 真实 WebAssembly Component | 2 | verified |
| Component sync/async 重跑 | 2 + 2 passed | verified |
| 4096-bit/Decimal ABI roundtrip | 512 bytes + true | verified |
| 正式编译器 workspace | 12 crates, build/test pass | verified |
| source/line index | 6 tests passed | verified |
| lossless lexer | 8 tests; B 54/54 | verified |
| B happy-path parser | 54/54 + 54 snapshots | verified |
| parser recovery/E1xxx | 12/12 mutation；E1001–E1012 | verified |
| canonical formatter | 54/54 AST stable + idempotent | verified |
| `sico` CLI | 6 commands；8 integration tests；`.sapp` build/run/inspect | verified |
| deterministic frontend properties | 8,192 inputs | verified |
| parser limits | depth 256；diagnostics 100 | verified |
| M1 performance | median 1061 ms / 4.190 MiB/s；no SLA | measured |
| 实际 `.sico` core/nominal type-check | 7 valid pass；9 invalid exact primary | verified |
| 实际 `.sico` match/Result check | 6 valid pass；8 invalid exact primary | verified |
| 实际 `.sico` capability/Component check | 4 valid pass；4 invalid exact primary | verified |
| 实际 `.sico` resource/task/stream check | 6 valid pass；6 invalid exact primary | verified |
| 实际 `.sico` revision check | 2 valid pass；2 invalid exact primary | verified |
| compiler-produced Semantic Index | 10 complete B modules；10 honest partial A modules；5 queries | verified |
| semantic CLI full oracle | 25/25 valid；29/29 exact invalid；text/JSON | verified |
| deterministic semantic properties/limits | 2,048 inputs；diagnostics 100；locals 2,000；depth 200 | verified |
| M2 performance | median 1242.186 ms / 2.808 MiB/s；no SLA | measured |
| typed Sico IR v0 | 14 types；19 operations；5 terminators | verified |
| independent IR verifier | 9 mutation classes；diagnostic cap 100 | verified |
| deterministic core lowering | 12 valid lowered；13 valid typed-refused；29 invalid blocked | verified |
| effect/resource/revision lowering | 5 flow cases；cumulative valid 17/25；4 verifier mutations | verified |
| deterministic Core Wasm backend | 2 byte-identical artifacts；wasmparser + Node engine；42/7/9 | verified |
| compiler-generated Component | 2 byte-identical artifacts；wasmparser + Wasmtime 46.0.1；42 | verified |
| WIT Result/record/resource boundary | Ok/Err；512-byte Int；Decimal；owned/borrowed/drop；host output 50 | verified |
| future/stream Runtime contract | 3 Components；roundtrip/close/cancel；capacity 1/5；stable SHA-256 | verified |
| async compiler boundary | Task/Future/Stream 3 feature-specific refusals | verified |
| end-to-end CLI | file/stdin build；Int/Bool/Unit Wasmtime run；exit 0/1/2；failure no artifact | verified |
| M3 deterministic quality | 2,048 scalar sources；1,000-function Component；3 Runtime cases | verified |
| M3 performance | median 51.055 ms / 3.213 MiB/s；3 × 3,000 builds；no SLA | measured |
| M4 Runtime limits | 11 dimensions；7 fault classes；6 tests；真实 infinite-loop + host survival | verified |
| M4 package CLI/cache | deterministic `.sapp`；signed inspect/trusted run；corrupt cache refusal | verified |
| M4 security properties | 2,048 signed mutations；1,024 Component mutations；256 resource permutations | verified |
| M4 performance | build/verify/signature median 5.459/8.571/48.177 ms；3 × 1,000；no SLA | measured |
| M5 Host install/open | signed-only；app+signer/revision/capability hashes；3 integration tests | verified |
| M5 Windows Desktop Host | signed install/open/Wasmtime `42`/uninstall；native Forms；zip smoke | runtime-verified |
| M5 desktop adapters | Windows runtime；macOS/Linux generated association artifacts | contract-verified where no runner |
| M5 host properties | 10,240 identity/capability/UI/open inputs | verified |
| M5 startup performance | median mean 32.344 ms；3 × 20 release launches；no SLA | measured |

M0/M1/M2 已完成。STEP-0022–0029 证明完整 B HIR、25/25 valid、29/29 exact invalid、semantic CLI、compiler index/query 与有界质量基线；[`M2 exit audit`](./reports/m2-exit-audit.md) 已授权进入 M3 STEP-0030。

## 4. Completed assets

- 方向与非目标：[`DIRECTION.md`](../DIRECTION.md)；
- 工程与阶段架构：[`DEVELOPMENT.md`](../DEVELOPMENT.md)；
- 核心语义草案：[`SEMANTICS.md`](../SEMANTICS.md)；
- 候选语法与评测方法：[`SYNTAX.md`](../SYNTAX.md)；
- 10 个代表性程序与问题矩阵：[`examples/`](../examples/README.md)；
- 候选 A 跨样本语义审计：[`examples/SEMANTICS-AUDIT.md`](../examples/SEMANTICS-AUDIT.md)；
- 54 个 P0 正反例：[`semantic-cases/`](../semantic-cases/README.md)；
- A0/B/C 的 162 个一一对应案例与完整静态指标：[`syntax-candidates/`](../syntax-candidates/README.md)；
- 36 个可复现单点结构错误变体：[`syntax-mutations/`](../syntax-mutations/README.md)；
- 96 项 AI v1 评测协议与离线执行器：[`ai-eval/`](../ai-eval/README.md)；
- 36 个稳定诊断（12 个真实 syntax、24 个 M2 设计）、12 个 syntax mutation 与 29 个 semantic case 映射、JSON Schema 和校验器：[`diagnostics/`](../diagnostics/README.md)；
- 10 模块 Semantic Index fixture、五类查询 JSON v0 与离线校验器：[`semantic-index/`](../semantic-index/README.md)；
- `Int`/Decimal Rust 原型、WIT 候选、RFC 和可复现报告：[`prototypes/numeric/`](../prototypes/numeric/README.md)；
- resource/async Rust 原型、compile-fail、WASI 0.3 WIT、RFC 和报告：[`prototypes/resource-async/`](../prototypes/resource-async/README.md)；
- 真实 Component host call、resource、数值记录和原生 async 往返：[`prototypes/component-host-call/`](../prototypes/component-host-call/README.md)；
- 桌面/Android Runtime 基线、发布政策与 Android 最小探针：[`ADR-0002`](./adr/ADR-0002-runtime-platform-baseline.md)、[`runtime report`](./reports/runtime-desktop-android-v0.md)；
- M1 B labeled-block 语法基线、候选取舍与复审门槛：[`RFC-0005`](./rfc/RFC-0005-labeled-block-syntax-baseline.md)；
- 12 类 AI-oriented 错误 taxonomy（真实频率未测量）：[`error-taxonomy.json`](../ai-eval/error-taxonomy.json)；
- M0 requirement-by-requirement GO 结论与递延登记：[`M0 exit audit`](./reports/m0-exit-audit.md)；
- STEP-0015–0021 前端工程计划：[`M1 compiler frontend`](./plans/M1-compiler-frontend.md)；
- 正式 Rust workspace、lexical/source v0 与 21 个 contract case：[`STEP-0015`](./steps/STEP-0015-compiler-workspace-lexical-source.md)、[`RFC-0006`](./rfc/RFC-0006-lexical-source-contract-v0.md)；
- strict source/span、line index 与 54-file lossless lexer：[`STEP-0016`](./steps/STEP-0016-source-span-lossless-lexer.md)；
- B happy-path lossless parser 与 AST shape：[`STEP-0017`](./steps/STEP-0017-b-grammar-lossless-parser.md)；
- parser recovery、E1001–E1012 与 text/JSON span：[`STEP-0018`](./steps/STEP-0018-parser-recovery-syntax-diagnostics.md)；
- canonical formatter、comment/trivia policy 与 error-tree refusal：[`STEP-0019`](./steps/STEP-0019-canonical-formatter.md)；
- `sico check`/`format`/`outline`、退出码与 M2 capability boundary：[`STEP-0020`](./steps/STEP-0020-cli-check-format-outline.md)；
- frontend fuzz/limits/performance 与 M1 GO：[`STEP-0021`](./steps/STEP-0021-fuzz-performance-m1-exit.md)、[`M1 exit audit`](./reports/m1-exit-audit.md)；
- STEP-0022–0029 静态语义执行计划：[`M2 static semantics`](./plans/M2-static-semantics.md)；
- full B HIR/name/prelude contract：[`STEP-0022`](./steps/STEP-0022-full-hir-name-prelude-contract.md)、[`review report`](./reports/full-hir-name-prelude-v0.md)；
- core/nominal type checker：[`STEP-0023`](./steps/STEP-0023-core-nominal-types.md)、[`review report`](./reports/core-nominal-types-v0.md)；
- match/Result control flow：[`STEP-0024`](./steps/STEP-0024-match-result-control-flow.md)、[`review report`](./reports/match-result-control-flow-v0.md)；
- effects/capabilities/Component boundary：[`STEP-0025`](./steps/STEP-0025-effects-capabilities-component.md)、[`review report`](./reports/effects-capabilities-component-v0.md)；
- resource/task/stream flow：[`STEP-0026`](./steps/STEP-0026-resources-async-streams.md)、[`review report`](./reports/resources-async-streams-v0.md)；
- revision contract dataflow：[`STEP-0027`](./steps/STEP-0027-revision-contract-dataflow.md)、[`review report`](./reports/revision-contract-dataflow-v0.md)；
- compiler Semantic Index/query：[`STEP-0028`](./steps/STEP-0028-compiler-semantic-index.md)、[`review report`](./reports/compiler-semantic-index-v0.md)；
- semantic CLI、quality baseline 与 M2 GO：[`STEP-0029`](./steps/STEP-0029-semantic-cli-fuzz-m2-exit.md)、[`review report`](./reports/semantic-cli-fuzz-performance-v0.md)、[`M2 exit audit`](./reports/m2-exit-audit.md)；
- STEP-0030–0037 IR/Component 执行计划：[`M3 Sico IR and Component`](./plans/M3-sico-ir-component.md)；
- typed Sico IR contract/verifier：[`STEP-0030`](./steps/STEP-0030-typed-sico-ir-contract-verifier.md)、[`RFC-0008`](./rfc/RFC-0008-typed-sico-ir-contract-v0.md)、[`review report`](./reports/typed-sico-ir-contract-v0.md)；
- core lowering/evaluation order：[`STEP-0031`](./steps/STEP-0031-core-lowering-evaluation-order.md)、[`RFC-0009`](./rfc/RFC-0009-core-lowering-evaluation-order-v0.md)、[`review report`](./reports/core-lowering-evaluation-order-v0.md)；
- effect/resource/revision IR flow：[`STEP-0032`](./steps/STEP-0032-effect-resource-revision-flow.md)、[`RFC-0010`](./rfc/RFC-0010-effect-resource-revision-ir-flow-v0.md)、[`review report`](./reports/effect-resource-revision-ir-flow-v0.md)；
- deterministic Core Wasm backend：[`STEP-0033`](./steps/STEP-0033-deterministic-core-wasm-backend.md)、[`RFC-0011`](./rfc/RFC-0011-deterministic-core-wasm-backend-v0.md)、[`review report`](./reports/deterministic-core-wasm-backend-v0.md)；
- Component/WIT boundary：[`STEP-0034`](./steps/STEP-0034-component-wit-boundary.md)、[`RFC-0012`](./rfc/RFC-0012-component-wit-boundary-v0.md)、[`review report`](./reports/component-wit-boundary-v0.md)；
- async/task/stream backend contract：[`STEP-0035`](./steps/STEP-0035-async-task-stream-backend.md)、[`RFC-0013`](./rfc/RFC-0013-async-backend-support-v0.md)、[`review report`](./reports/async-task-stream-backend-v0.md)；
- minimal end-to-end CLI：[`STEP-0036`](./steps/STEP-0036-minimal-end-to-end-cli.md)、[`RFC-0014`](./rfc/RFC-0014-minimal-build-run-cli-v0.md)、[`review report`](./reports/minimal-build-run-cli-v0.md)；
- M3 quality/exit GO 与 M4 execution plan：[`STEP-0037`](./steps/STEP-0037-m3-quality-exit-audit.md)、[`M3 exit audit`](./reports/m3-exit-audit.md)、[`M4 plan`](./plans/M4-sapp-runtime.md)；
- `.sapp` format/builder/trust/capability/storage/limits：[`STEP-0038–0043`](./steps/README.md)、[`RFC-0015–0018`](./rfc/README.md)、[`ADR-0003`](./adr/ADR-0003-isolated-storage-wasi-host-v0.md)；
- package CLI/cache 与 M4 GO：[`STEP-0044`](./steps/STEP-0044-package-cli-source-cache.md)、[`STEP-0045`](./steps/STEP-0045-m4-security-quality-exit.md)、[`M4 exit audit`](./reports/m4-exit-audit.md)；
- STEP-0046–0053 Desktop Host execution plan：[`M5 plan`](./plans/M5-desktop-host.md)；
- 长期自治执行目标：[`AGENT_GOAL.md`](../AGENT_GOAL.md)。

## 5. Incomplete M0 work

无。真实 AI、Android、Future/Stream Runtime 与 proposed RFC 接受条件已登记为后续/外部证据，不是 M0 完成声明。

## 6. Blockers

M6 当前被外部 runner 阻塞：本机没有已授权 Android SDK/NDK、ADB、x86_64 emulator/AVD 或 arm64 device。仓库也尚无可构建 Gradle Android Host、`HostActivity`、JNI `cdylib` 或 APK。用户接受许可并提供 runner 后，仍需先补齐这些实现，不能直接把 cross-check 视为 Runtime evidence。

2026-07-16 复查还确认当前 stable Rust 1.97.0 未安装 Android targets；仓库固定的 version-named toolchain 未单独安装，但同版本 stable toolchain 可用于平台无关工作。Harmony 没有 SDK、DevEco、仓库计划或实现；当前状态是 deferred scope，而不是已完成或已验证的目标。

真实 AI API 批量评测仍需要模型凭据和成本授权；协议和离线工具已经完成，因此该条件不阻塞 STEP-0015/M1。没有真实调用前不产生模型分数。

## 7. Risks

- B frontend 与确定性 fuzz/limit 已完成，但 coverage-guided 长期 fuzz 和跨平台性能仍是后续质量工作；
- `Int`/Decimal 记录已通过 Rust 原型和真实 Component 往返，但 RFC-0003 仍待非 Windows 重现与稳定限额诊断分类；contract invariant 和 Result 表层写法仍是草案；
- STEP-0023–0029 已有真实 checker/index/CLI；仍无真实模型实测；
- WIT 0.253 已真实往返 resource、数值记录、`async func`、`future<T>` 与 `stream<T>`；compiler async IR 尚缺完整 task scope/cancel/bound，所以 codegen 保持专用 refusal；
- Core Wasm、compiler-generated Component 与同步 scalar CLI 已由独立 engine/selected Wasmtime 执行；一般 aggregate/import adapter、package capability host 与 sandbox 留在 M4；
- Wasmtime Android aarch64/x86_64 仍是 Tier 3；Pulley/真机/JNI/商店政策只有 M0 选择，尚无仓库实测；
- E1xxx 与 catalog-mapped E2–E7 已有真实 compiler code、跨度、bounded cascade 与 CLI text/JSON；
- 语义查询已有 compiler producer 与五类直接查询；跨包/IR facts、accuracy/latency 和真实模型收益仍未测量。

## 8. Next step

下一项执行 STEP-0077：先冻结显式 `I64/U64` semantic/IR JSON/checked arithmetic 与 comparison operation set，再实现 dynamic parameters/locals/results 和 Wasmtime oracle/property tests；`Int` 保持任意精度方向且不得被后端静默缩窄。完成后按 STEP-0078–0084 推进 general control、Canonical ABI、runner、CLI、标准库和退出审计。公网 rollout 继续等待 hostname/hosting/production identity/key-custody 输入；输入到位时使用新的未分配 STEP，且不得把 loopback/operator bundle 改标为 public production evidence。
