# Reports

本目录保存可复现的实验、AI 评测、性能、安全、兼容性和 Runtime 对比报告。

现有静态语法指标位于 [`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)。后续新报告使用 [`report template`](../templates/REPORT.md)。

| Report | Status | Related step | 内容 |
|---|---|---|---|
| [`android-intent-ingest-v0`](./android-intent-ingest-v0.md) | accepted | STEP-0056 | one-pass bounded content URI ingestion and picker-only deep link |
| [`mobile-host-bridge-v0`](./mobile-host-bridge-v0.md) | accepted | STEP-0055 | typed bounded mobile bridge over shared Host trust core |
| [`android-host-contract-v0`](./android-host-contract-v0.md) | accepted | STEP-0054 | Android Intent/URI/JNI/lifecycle/ABI threat and evidence boundary |
| [`m5-exit-audit`](./m5-exit-audit.md) | complete | STEP-0053 | M5 Windows runtime, platform parity, quality/performance and M6 GO |
| [`desktop-platform-parity-v0`](./desktop-platform-parity-v0.md) | accepted | STEP-0052 | shared Windows/macOS/Linux adapter contract with explicit evidence labels |
| [`windows-desktop-host-v0`](./windows-desktop-host-v0.md) | accepted | STEP-0051 | Windows signed install/open/uninstall, native UI and zip smoke |
| [`syntax-error-injection-v0`](./syntax-error-injection-v0.md) | complete | STEP-0003 | 18 个单点结构错误变体及可复现性验证 |
| [`ai-evaluation-protocol-v0`](./ai-evaluation-protocol-v0.md) | complete | STEP-0004 | 42 项 AI 任务、运行协议和离线评分器验证 |
| [`remaining-p0-semantic-cases`](./remaining-p0-semantic-cases.md) | complete | STEP-0005 | 10 组、54 个 P0 语义判定及 A0/B/C 镜像 |
| [`diagnostics-protocol-v0`](./diagnostics-protocol-v0.md) | complete | STEP-0006 | 24 个稳定诊断、29 个 case 映射和 JSON v0 fixtures |
| [`semantic-query-json-v0`](./semantic-query-json-v0.md) | complete | STEP-0007 | 10 模块索引、五类查询、可信度/完整性与预算 fixtures |
| [`numeric-representation-v0`](./numeric-representation-v0.md) | complete | STEP-0008 | `Int`/Decimal 运算、规范编码、限额和 WIT 边界原型 |
| [`resource-async-wit-v0`](./resource-async-wit-v0.md) | complete | STEP-0009 | affine resource、Task/Future/Stream 与 WASI 0.3 WIT 原型 |
| [`component-runtime-host-call-v0`](./component-runtime-host-call-v0.md) | complete | STEP-0010 | 真实 Component host call、resource、数值记录与原生 async 往返 |
| [`runtime-desktop-android-v0`](./runtime-desktop-android-v0.md) | complete | STEP-0011 | 桌面/Android Runtime、后端、发布政策与最小探针选择 |
| [`syntax-evidence-v1`](./syntax-evidence-v1.md) | complete | STEP-0012 | 54-case 静态快照、36 mutation 与 96-task AI 离线协议 |
| [`m0-exit-audit`](./m0-exit-audit.md) | complete | STEP-0014 | M0 全门槛、真实 Rust/Component 重跑、递延登记与 M1 GO 结论 |
| [`compiler-workspace-lexical-source-v0`](./compiler-workspace-lexical-source-v0.md) | complete | STEP-0015 | 七 crate workspace、锁定依赖、RFC-0006 与 21 个 lexical/source contract case 审查 |
| [`source-span-lossless-lexer-v0`](./source-span-lossless-lexer-v0.md) | complete | STEP-0016 | strict UTF-8、统一 span/line index、21 contract case 与 54-file lossless lexer 验证 |
| [`b-grammar-lossless-parser-v0`](./b-grammar-lossless-parser-v0.md) | complete | STEP-0017 | 54/54 B lossless parse、AST snapshots 与 M2 semantic reject 隔离 |
| [`parser-recovery-syntax-diagnostics-v0`](./parser-recovery-syntax-diagnostics-v0.md) | complete | STEP-0018 | 12/12 mutation 根因、E1xxx text/JSON span、error node 与有界 recovery |
| [`canonical-formatter-v0`](./canonical-formatter-v0.md) | complete | STEP-0019 | 54/54 AST-stable/idempotent canonical layout、comment policy 与 error-tree refusal |
| [`cli-check-format-outline-v0`](./cli-check-format-outline-v0.md) | complete | STEP-0020 | 真实 binary、file/stdin、exit 0/1/2、RFC-0001 JSON、format 与 outline integration |
| [`m1-exit-audit`](./m1-exit-audit.md) | complete | STEP-0021 | 8,192 property inputs、parser limits、3-run performance 与 M1 requirement-by-requirement GO |
| [`full-hir-name-prelude-v0`](./full-hir-name-prelude-v0.md) | complete | STEP-0022 | 54 deterministic HIR snapshots、stable IDs/source maps、error-tree refusal 与 P0 prelude boundary |
| [`core-nominal-types-v0`](./core-nominal-types-v0.md) | complete | STEP-0023 | core/nominal type checker 与精确诊断 |
| [`match-result-control-flow-v0`](./match-result-control-flow-v0.md) | complete | STEP-0024 | match exhaustiveness、Result/try 与 control-flow diagnostics |
| [`effects-capabilities-component-v0`](./effects-capabilities-component-v0.md) | complete | STEP-0025 | effects、capabilities 与 Component call boundary |
| [`resources-async-streams-v0`](./resources-async-streams-v0.md) | complete | STEP-0026 | affine resource、Task/Future/Stream static flow |
| [`revision-contract-dataflow-v0`](./revision-contract-dataflow-v0.md) | complete | STEP-0027 | revision guard 与 stale-value dataflow |
| [`compiler-semantic-index-v0`](./compiler-semantic-index-v0.md) | complete | STEP-0028 | compiler-produced Semantic Index 与五类查询 |
| [`semantic-cli-fuzz-performance-v0`](./semantic-cli-fuzz-performance-v0.md) | complete | STEP-0029 | semantic CLI、property/limits 与非 SLA 性能基线 |
| [`m2-exit-audit`](./m2-exit-audit.md) | complete | STEP-0029 | M2 requirement-by-requirement GO |
| [`typed-sico-ir-contract-v0`](./typed-sico-ir-contract-v0.md) | complete | STEP-0030 | typed IR、canonical serialization 与独立 verifier |
| [`core-lowering-evaluation-order-v0`](./core-lowering-evaluation-order-v0.md) | complete | STEP-0031 | deterministic core lowering 与 evaluation order |
| [`effect-resource-revision-ir-flow-v0`](./effect-resource-revision-ir-flow-v0.md) | complete | STEP-0032 | effect/resource/revision IR invariants |
| [`deterministic-core-wasm-backend-v0`](./deterministic-core-wasm-backend-v0.md) | complete | STEP-0033 | verified IR 到 deterministic Core Wasm |
| [`component-wit-boundary-v0`](./component-wit-boundary-v0.md) | complete | STEP-0034 | compiler Component lift 与 WIT Result/record/resource boundary |
| [`async-task-stream-backend-v0`](./async-task-stream-backend-v0.md) | complete | STEP-0035 | Future/Stream Runtime contract 与 compiler typed refusal |
| [`minimal-build-run-cli-v0`](./minimal-build-run-cli-v0.md) | complete | STEP-0036 | source→Component→Wasmtime CLI、exit/channel 与无产物失败契约 |
| [`m3-exit-audit`](./m3-exit-audit.md) | complete | STEP-0037 | M3 determinism/property/limits/performance、全阶段 regression 与 GO |
| [`sapp-builder-loader-v0`](./sapp-builder-loader-v0.md) | complete | STEP-0039 | canonical `.sapp` builder、strict loader、hash/path/limit verification |
| [`package-cli-cache-v0`](./package-cli-cache-v0.md) | accepted | STEP-0044 | `.sapp` build/run/inspect、explicit trust、source cache 与 args/stdio |
| [`m4-exit-audit`](./m4-exit-audit.md) | complete | STEP-0045 | M4 security/property/performance、M0–M3 regression 与 M5 GO |
