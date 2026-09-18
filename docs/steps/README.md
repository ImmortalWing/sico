# Step records

每个非平凡执行步骤使用稳定 `STEP-xxxx` 编号，并链接实现、验证、RFC、ADR 和报告。

| Step | 状态 | 阶段 | 标题 |
|---|---|---|---|
| [STEP-0061](./STEP-0061-m6-quality-exit-audit.md) | complete-audit / blocked | M6 | security, performance and NO-GO Android runner audit |
| [STEP-0060](./STEP-0060-desktop-android-parity-app.md) | partial-runtime-evidence | M6 | same signed package Desktop/Mobile parity; Android runner pending |
| [STEP-0059](./STEP-0059-android-native-ui-adapter.md) | complete-cross-check | M6 | Android native touch, IME and accessibility adapter |
| [STEP-0058](./STEP-0058-android-runtime-lifecycle.md) | complete-cross-check | M6 | Wasmtime Android cross-check and lifecycle supervision |
| [STEP-0057](./STEP-0057-android-permission-storage.md) | complete | M6 | Android permission, URI grant and app-private storage mapping |
| [STEP-0056](./STEP-0056-android-intent-package-ingest.md) | complete | M6 | content URI package/open/share/deep-link adapter |
| [STEP-0055](./STEP-0055-mobile-host-jni-boundary.md) | complete | M6 | shared mobile Host core and typed JNI boundary |
| [STEP-0054](./STEP-0054-android-host-contract.md) | complete | M6 | Android threat/lifecycle/platform/packaging contract |
| [STEP-0053](./STEP-0053-m5-quality-exit-audit.md) | complete | M5 | representative app, properties, startup baseline and M5 GO |
| [STEP-0052](./STEP-0052-desktop-platform-adapters-parity.md) | complete | M5 | Windows/macOS/Linux adapter artifacts and parity evidence |
| [STEP-0051](./STEP-0051-windows-desktop-host-integration.md) | complete | M5 | Windows install/open/uninstall, native UI and package smoke |
| [STEP-0001](./STEP-0001-project-state-audit.md) | complete | M0 | 建立审计基线与项目差距审计 |
| [STEP-0002](./STEP-0002-sico-host-terminology.md) | complete | M0 | 统一应用宿主命名为 Sico Host |
| [STEP-0003](./STEP-0003-syntax-error-injection-v0.md) | complete | M0 | 建立候选语法单点错误注入集 v0 |
| [STEP-0004](./STEP-0004-ai-evaluation-protocol-v0.md) | complete | M0 | 建立 AI 评测协议与离线执行器 v0 |
| [STEP-0005](./STEP-0005-remaining-p0-semantic-cases.md) | complete | M0 | 补齐剩余 P0 语义正反例 |
| [STEP-0006](./STEP-0006-diagnostics-protocol-v0.md) | complete | M0 | 建立诊断协议 v0 |
| [STEP-0007](./STEP-0007-semantic-query-json-v0.md) | complete | M0 | 建立语义索引与查询 JSON v0 |
| [STEP-0008](./STEP-0008-numeric-representation-prototypes.md) | complete | M0 | 验证 Int 与 Decimal 表示原型 |
| [STEP-0009](./STEP-0009-resource-async-wit-prototypes.md) | complete | M0 | 验证 resource、async 与 WIT 映射 |
| [STEP-0010](./STEP-0010-component-runtime-host-call.md) | complete | M0 | 验证 Component Runtime WIT host call 实链 |
| [STEP-0011](./STEP-0011-runtime-desktop-android-feasibility.md) | complete | M0 | 比较 Component Runtime 桌面与 Android 可行性 |
| [STEP-0012](./STEP-0012-syntax-evidence-completion.md) | complete | M0 | 补齐第二轮语法静态、mutation 与 AI 离线证据 |
| [STEP-0013](./STEP-0013-syntax-baseline-decision.md) | complete | M0 | 选择 B labeled-block 作为 M1 唯一语法基线 |
| [STEP-0014](./STEP-0014-m0-exit-audit.md) | complete | M0 | 完成 M0 退出审计并授权进入 M1 |
| [STEP-0015](./STEP-0015-compiler-workspace-lexical-source.md) | complete | M1 | 建立正式 Rust workspace 并冻结 lexical/source 契约 |
| [STEP-0016](./STEP-0016-source-span-lossless-lexer.md) | complete | M1 | 实现统一 source/span、line index 与无损 lexer |
| [STEP-0017](./STEP-0017-b-grammar-lossless-parser.md) | complete | M1 | 实现 B grammar happy-path rowan tree 与 AST shape |
| [STEP-0018](./STEP-0018-parser-recovery-syntax-diagnostics.md) | complete | M1 | 实现 parser recovery 与 E1xxx syntax diagnostics |
| [STEP-0019](./STEP-0019-canonical-formatter.md) | complete | M1 | 实现 parse-success-only canonical formatter |
| [STEP-0020](./STEP-0020-cli-check-format-outline.md) | complete | M1 | 实现 `sico check`、`format` 与 `outline` CLI contract |
| [STEP-0021](./STEP-0021-fuzz-performance-m1-exit.md) | complete | M1 | 完成 frontend fuzz/limits/performance 与 M1 exit audit |
| [STEP-0022](./STEP-0022-full-hir-name-prelude-contract.md) | complete | M2 | 建立 full B HIR、source maps 与 name/prelude contract |
| [STEP-0023](./STEP-0023-core-nominal-types.md) | complete | M2 | 实现 core/nominal types |
| [STEP-0024](./STEP-0024-match-result-control-flow.md) | complete | M2 | 实现 match/Result control flow |
| [STEP-0025](./STEP-0025-effects-capabilities-component.md) | complete | M2 | 实现 effects/capabilities/Component semantics |
| [STEP-0026](./STEP-0026-resources-async-streams.md) | complete | M2 | 实现 resource/task/stream flow |
| [STEP-0027](./STEP-0027-revision-contract-dataflow.md) | complete | M2 | 实现 revision contract dataflow |
| [STEP-0028](./STEP-0028-compiler-semantic-index.md) | complete | M2 | 实现 compiler Semantic Index |
| [STEP-0029](./STEP-0029-semantic-cli-fuzz-m2-exit.md) | complete | M2 | 完成 semantic CLI、quality 与 M2 exit |
| [STEP-0030](./STEP-0030-typed-sico-ir-contract-verifier.md) | complete | M3 | 冻结 typed Sico IR/verifier |
| [STEP-0031](./STEP-0031-core-lowering-evaluation-order.md) | complete | M3 | 实现 core lowering/evaluation order |
| [STEP-0032](./STEP-0032-effect-resource-revision-flow.md) | complete | M3 | 实现 effect/resource/revision IR flow |
| [STEP-0033](./STEP-0033-deterministic-core-wasm-backend.md) | complete | M3 | 实现 deterministic Core Wasm backend |
| [STEP-0034](./STEP-0034-component-wit-boundary.md) | complete | M3 | 实现 Component/WIT boundary |
| [STEP-0035](./STEP-0035-async-task-stream-backend.md) | complete | M3 | 验证 async/task/stream backend boundary |
| [STEP-0036](./STEP-0036-minimal-end-to-end-cli.md) | complete | M3 | 实现最小 source→Component→Runtime CLI |
| [STEP-0037](./STEP-0037-m3-quality-exit-audit.md) | complete | M3 | 完成 M3 quality/exit audit |
| [STEP-0038](./STEP-0038-sapp-threat-model-package-contract.md) | complete | M4 | 冻结 `.sapp` threat/package contract |
| [STEP-0039](./STEP-0039-deterministic-sapp-builder-loader-inspect.md) | complete | M4 | 实现 deterministic builder/strict loader |
| [STEP-0040](./STEP-0040-development-signing-trust-policy.md) | complete | M4 | 实现 development signing/trust policy |
| [STEP-0041](./STEP-0041-capability-closure-permission-intersection.md) | complete | M4 | 实现 capability closure/permission intersection |
| [STEP-0042](./STEP-0042-wasi-capability-host-isolated-storage.md) | complete | M4 | 实现 WASI grants/isolated storage |
| [STEP-0043](./STEP-0043-runtime-limits-fault-taxonomy.md) | complete | M4 | 实现 Runtime limits/fault taxonomy |
| [STEP-0044](./STEP-0044-package-cli-source-cache.md) | complete | M4 | 实现 package CLI/source cache contract |
| [STEP-0045](./STEP-0045-m4-security-quality-exit.md) | complete | M4 | 完成 M4 security/quality/exit audit |
| [STEP-0046](./STEP-0046-desktop-host-threat-lifecycle-contract.md) | complete | M5 | 冻结 Desktop Host threat/lifecycle contract |
| [STEP-0047](./STEP-0047-shared-host-install-open.md) | complete | M5 | 实现共享 Host install/open |
| [STEP-0048](./STEP-0048-permission-records.md) | complete | M5 | 实现权限记录与 prompt contract |
| [STEP-0049](./STEP-0049-lifecycle-process-supervision.md) | complete | M5 | 实现 lifecycle/process supervision |
| [STEP-0050](./STEP-0050-minimal-ui-wit-renderer.md) | complete | M5 | 实现最小 UI WIT/renderer |
| [STEP-0051](./STEP-0051-windows-desktop-host-integration.md) | complete | M5 | 集成 Windows Desktop Host |
| [STEP-0052](./STEP-0052-desktop-platform-adapters-parity.md) | complete | M5 | 固定 desktop platform adapters/parity |
| [STEP-0053](./STEP-0053-m5-quality-exit-audit.md) | complete | M5 | 完成 M5 quality/exit audit |
| [STEP-0054](./STEP-0054-android-host-contract.md) | complete | M6 | 冻结 Android Host contract |
| [STEP-0055](./STEP-0055-mobile-host-jni-boundary.md) | complete | M6 | 实现 Mobile Host/JNI boundary contract |
| [STEP-0056](./STEP-0056-android-intent-package-ingest.md) | complete | M6 | 实现 Android Intent/package ingestion contract |
| [STEP-0057](./STEP-0057-android-permission-storage.md) | complete | M6 | 映射 Android permission/storage |
| [STEP-0058](./STEP-0058-android-runtime-lifecycle.md) | complete | M6 | 冻结 Android Runtime/lifecycle contract |
| [STEP-0059](./STEP-0059-android-native-ui-adapter.md) | complete | M6 | 适配 Android native UI contract |
| [STEP-0060](./STEP-0060-desktop-android-parity-app.md) | partial-runtime-evidence | M6 | 验证 Desktop/Mobile core parity；Android runner deferred |
| [STEP-0061](./STEP-0061-m6-quality-exit-audit.md) | complete / NO-GO | M6 | 完成 M6 host audit，保留外部 runner 阻塞 |
| [STEP-0062](./STEP-0062-ecosystem-release-contract.md) | complete | M7 | 冻结 ecosystem/release trust 与 compatibility contract |
| [STEP-0063](./STEP-0063-production-publisher-identity-key-lifecycle.md) | complete | M7 | 实现 production publisher policy 与 key lifecycle 本地闭环 |
| [STEP-0064](./STEP-0064-signed-local-registry.md) | complete | M7 | 实现 signed namespace/release/channel/checkpoint 与本地 publish/discover/download 闭环 |
| [STEP-0065](./STEP-0065-secure-update-rollback.md) | complete | M7 | 实现 monotonic update、staged activation、advisory 与显式 recovery 闭环 |
| [STEP-0066](./STEP-0066-dependency-standard-library-stability.md) | complete | M7 | 实现 source-aware resolution、canonical lock、compatibility 与 capability closure |
| [STEP-0067](./STEP-0067-language-server-editor-workflow.md) | complete | M7 | 实现 bounded stdio LSP、compiler diagnostics/index、format 与 shell-free run/debug contract |
| [STEP-0068](./STEP-0068-ai-tooling-measured-evaluation.md) | complete-offline | M7 | 实现 compiler-backed inspect/fix、54-source/12-fix 测量与 live-model gate |
| [STEP-0069](./STEP-0069-third-party-pilot-m7-exit.md) | complete-local / blocked-external-evidence | M7 | 完成两版洁净室 pilot/release drill 与退出审计，保留七项外部 gate |
| [STEP-0070](./STEP-0070-mobile-platform-development-handbooks.md) | complete | M6/M7 docs | 保存 Android 与鸿蒙详细开发/验收手册，不改变移动 NO-GO |
| [STEP-0071](./STEP-0071-linux-development-handbook.md) | complete | M5/M7 docs | 保存 Linux Desktop Host 详细开发/验收手册，不改变 Linux 证据等级 |
| [STEP-0072](./STEP-0072-user-manual-information-architecture.md) | complete | release docs | 根 README 改为用户入口，迁移开发手册并建立十项细分用户手册与文档契约 |
| [STEP-0073](./STEP-0073-openjdk-style-modular-monorepo.md) | complete | architecture | 归档 v0.0.1，按 OpenJDK 模式拆分语言、应用 Runtime 与 Host 命令并强制依赖边界 |
| [STEP-0074](./STEP-0074-production-deployment-origin-and-ux.md) | complete-local | M7 production | 建立只读 registry origin、生产部署包与单命令源码工作流 |
| [STEP-0075](./STEP-0075-script-profile-contract.md) | complete-contract | M8 | 冻结 Script Profile/WIT/adapter/runner 契约并定义纵向原型 gate |
| [STEP-0076](./STEP-0076-script-profile-vertical-prototype.md) | complete | M8 | direct/composed 各 20 × 6/6，实测选择 versioned Adapter composition，保留 direct fallback |
| [STEP-0077](./STEP-0077-fixed-width-dynamic-scalars.md) | complete | M8 | 独立 I64/U64、typed overflow/underflow、dynamic Core/Component 与 runtime oracle 已完成 |
| [STEP-0078](./STEP-0078-general-executable-control-codegen.md) | complete | M8 | direct Call、dispatcher CFG、match/back-edge 与内部 aggregate codegen，不扩张 Component ABI |
| [STEP-0079](./STEP-0079-script-aggregate-canonical-abi.md) | complete | M8 | WIT 驱动 layout 表、bounded arena、Text/Bytes/List/record/Result Canonical ABI、10,000 seeded roundtrip 与恶意内存 fail-closed |
| [STEP-0080](./STEP-0080-compiler-script-profile-adapter-manifest.md) | complete | M8 | `sico build --profile script-v0`、versioned Adapter composition（WASI 0.2.12 实跑 echo/reject）与严格 manifest v1 |
| [STEP-0081](./STEP-0081-in-process-script-runner.md) | complete | M8 | in-process sico-runner：typed outcomes、fuel/timeout/memory/cancel、精确 guest exit 与 host survival |
| [STEP-0082](./STEP-0082-unified-run-eval-caches.md) | complete | M8 | unified `sico run`/`eval`、冻结 cache identity、corrupt/conflict fail-closed 与 stdout purity |
| [STEP-0083](./STEP-0083-script-standard-library.md) | complete | M8 | 三层冻结 intrinsic registry（text/bytes/list/JSON/fs）、helper aggregate ABI、scoped fs 双接口与四个 pilots |
| [STEP-0084](./STEP-0084-m8-exit-audit.md) | complete | M8 | §11 门禁 8/8、性能矩阵、WSL Linux 证据与 honest GO |
| [STEP-0085](./STEP-0085-streaming-script-rfc.md) | complete | M9 | RFC-0030 streaming channel：streams WIT、ownership、backpressure、cancellation 矩阵 |
| [STEP-0086](./STEP-0086-stream-resource-abi-codegen.md) | complete | M9 | streams resource Canonical ABI、exact drop、mixing rule、256 MiB bounded-RSS passthrough |
| [STEP-0087](./STEP-0087-task-future-stream-source-backend.md) | complete | M9 | sequential executor：structured task scope、E5101/E5102（含间接逃逸）、cancellation edge 123 |
| [STEP-0088](./STEP-0088-async-runner-streaming-stdio.md) | complete | M9 | worker-thread 可取消 IO、blocked read/pump/write 取消 120–134ms、bounded queues |
| [STEP-0089](./STEP-0089-scoped-http-component-provider.md) | complete | M9 | scoped HTTP Component provider：精确 endpoint grant、默认拒绝、loopback、redirect/limit/timeout/cancel 证据 |
| [STEP-0090](./STEP-0090-persistent-runner-watch.md) | complete | M9 | persistent Engine/Component/Linker、per-run Store isolation、watch coalescing/failure recovery、5.8–6.1 ms warm median |
| [STEP-0091](./STEP-0091-bounded-repl-session.md) | complete | M9 | bounded expression REPL、deterministic ID、rollback/replay/reset/export、4 KiB/256/16 KiB limits |
| [STEP-0092](./STEP-0092-top-level-script-syntax-decision.md) | complete | M9 | explicit typed `main` accepted；top-level statements 与 `script:` rejected；E1013 fail-closed |
| [STEP-0093](./STEP-0093-editor-ai-execution-integration.md) | complete | M9 | shared LSP/AI execution plan；direct argv；bounded logs/cancel/source-map/debug honesty |
| [STEP-0094](./STEP-0094-m9-exit-audit.md) | complete / GO | M9 | aggregate M8/M9 security/performance regression、Windows-only Runtime matrix 与 M10 plan |
| [STEP-0095](./STEP-0095-observability-debug-contract.md) | complete-contract | M10 | RFC-0035、strict identity/map/fault/event schemas、exact DAP allowlist、cancellation races 与双 Component identity evidence |
| [STEP-0096](./STEP-0096-deterministic-compiler-debug-map.md) | complete | M10 | deterministic debug triplet、真实 Core offset/source mapping、generated rows、atomic CLI/package integration |
| [STEP-0097](./STEP-0097-structured-runtime-faults-source-frames.md) | complete | M10 | typed Runtime fault classes、verified exact source frames、256-frame bound、JSON/text CLI 与 stale/missing map fail-closed |
| [STEP-0098](./STEP-0098-os-signal-client-cancellation-bridge.md) | complete | M10 | real Windows console control、canonical client/watch cancellation、single terminal winner 与 M9 blocked-I/O/HTTP regression |
| [STEP-0099](./STEP-0099-bounded-execution-events.md) | complete-core | M10 | bounded canonical events、binary chunking、mandatory redaction、overflow marker 与 terminal reservation |
| [STEP-0100](./STEP-0100-minimal-dap-implementation.md) | complete / GO | M10 | exact DAP allowlist、real entry/source breakpoints、safe pause、nested stack/scalars、bounded stdio 与 owned teardown |
| [STEP-0101](./STEP-0101-editor-ai-execution-feedback.md) | complete / GO | M10 | shell-free debug launch plan、LSP handoff 与 data-only AI redacted execution summaries |
| [STEP-0102](./STEP-0102-m10-exit-audit.md) | complete / GO | M10 | M0–M9 aggregate regression、M10 security/performance/platform audit 与 external-gate recheck |
| [STEP-0104](./STEP-0104-semantic-ir-structured-concurrency.md) | complete | M11 | RFC-0036 task scope 语义/IR 契约、忠实 lowering、sequential-v1 codegen 投影与 collect_tasks 执行证据 |
| [STEP-0105](./STEP-0105-scheduler-core.md) | complete | M11 | single-Store cooperative scheduler core：有界任务表/scope 树/FIFO 队列、identity-checked Host completion ingress、canonical turn order、typed limits 与 teardown 稳定性证据 |
| [STEP-0106](./STEP-0106-cancellation-race-select.md) | complete | M11 | downward cancellation tree、scheduler 级 race/select（≤256 operands、canonical winner、loser cancel+abandon）、timer readiness、typed deadlock 与 1,024 链式取消规模证据 |
| [STEP-0107](./STEP-0107-bounded-channels-streams.md) | complete | M11 | task-aware bounded channels：0..=1,024 items + 显式字节预算、rendezvous、FIFO 反压（suspend 不 spin）、affine close/move、取消失败传播与 1 GiB relay RSS 平稳证据 |
| [STEP-0108](./STEP-0108-persistent-watch-repl-dap-task-integration.md) | complete | M11 | debug worker 统一 teardown、watch 世代隔离、REPL 纯常量无 Store、100 次变权限运行零泄漏、DAP terminate 无滞留、AI 工具面 data-only |
| [STEP-0109](./STEP-0109-cross-platform-runner-parity.md) | complete | M11 | Linux x64 原生 parity（WSL2 Ubuntu 24.04）：scheduler/runner 全套语料绿、fd/RSS 平稳、证据归档 |
| [STEP-0110](./STEP-0110-m11-exit-audit.md) | complete / GO | M11 | M0–M10 聚合回归 + M11 链全绿；Windows x64 + Linux x64 双平台实证；10/10 gates GO；M12 解锁 |
| [STEP-0111](./STEP-0111-secure-http-rfc.md) | complete-contract | M12 | 冻结 secure HTTP provider authority/transport contract |
| [STEP-0112](./STEP-0112-tls-transport.md) | complete | M12 | 成熟 TLS stack、证书与 hostname 验证 |
| [STEP-0113](./STEP-0113-endpoint-authority-dns.md) | complete | M12 | endpoint authority、DNS pinning 与地址策略 |
| [STEP-0114](./STEP-0114-streaming-bodies.md) | complete | M12 | strict framing、bounded streaming 与 backpressure |
| [STEP-0115](./STEP-0115-redirect-policy.md) | complete | M12 | redirect reauthorization 与跨 origin secret stripping |
| [STEP-0116](./STEP-0116-secret-provider-redaction.md) | complete | M12 | Host-owned secret provider 与全链路 redaction |
| [STEP-0117](./STEP-0117-connection-lifecycle-sdk.md) | complete-core | M12 | per-Store HTTP engine 与 connection lifecycle SDK core |
| [STEP-0118](./STEP-0118-m12-exit-audit.md) | complete / GO-core | M12 | secure HTTP core audit；guest runner integration 与 Linux provider parity 待完成 |
| [STEP-0119](./STEP-0119-ai-generation-quality-baseline.md) | complete | M13 | generation 失败归因、prompt/guide canonical 修正、fixture 修复与重测基线 0.8846→0.9744 |
| [STEP-0120](./STEP-0120-ai-quality-budget-adr.md) | complete | M13 | ADR-0011 AI 质量预算：floor 非回归层（≤ 实测基线）与 target 完成门（proof 待 live-model，blocked-external-evidence） |
| [STEP-0121](./STEP-0121-mcp-agent-integration.md) | complete | M13 | MCP stdio 接入层（sico-mcp-server，第 26 包）：四工具 JSON Schema 注册、budget 服务端持有不扩权、512 变异穿透 MCP fail-closed、真实会话 inspect→validate_fix roundtrip |
| [STEP-0111](./STEP-0111-secure-http-rfc.md) | complete | M12 | RFC-0037 accepted：http@0.2.0 身份、endpoint authority、rustls 选型（含本机 roundtrip 证据）、redirect 矩阵、secret 模型、全部 bounds |
| [STEP-0112](./STEP-0112-tls-transport.md) | complete | M12 | sico-http-provider（第 27 包）：确定性 rcgen CA + rustls 服务器/验证客户端；valid-accept + wrong-host/untrusted-issuer/malformed-PEM 拒绝；无 danger 路径 |
| [STEP-0113](./STEP-0113-endpoint-authority-dns.md) | complete | M12 | 严格 authority 规范化（IPv4 文本欺骗/IPv4-mapped/zone/大小写/percent 全拒绝）、IDNA 只认 canonical ASCII、per-use 地址门（+private 开发 scheme） |
| [STEP-0114](./STEP-0114-streaming-bodies.md) | complete | M12 | 严格传输分帧：重复/混合/非规范 Content-Length 拒绝、chunked 读取器（扩展拒绝、64KiB 上限、预算门）、trailer 界限；request-smuggling 语料绿 |
| [STEP-0115](./STEP-0115-redirect-policy.md) | complete | M12 | opt-in ≤5 跳 redirect 引擎：冻结状态码矩阵、同 URL 环检测、HTTPS 降级拒绝、跨源结构性剥除 authorization/secret 头 |
| [STEP-0116](./STEP-0116-secret-provider-redaction.md) | complete | M12 | Host 侧 opaque secret 注册表：name+endpoint+policy 精确三元的 typed 拒绝、header-only 注入（bearer/basic/header）、确定性指纹 redaction（canary 证明） |
| [STEP-0117](./STEP-0117-connection-lifecycle-sdk.md) | complete-core | M12 | per-Store HttpEngine：authority→pinning→地址门→grant 检查→TLS/明文交换→redirect 链全组合；16 in-flight typed cap；每次请求单一终态；full roundtrip 测试含服务端 secret 注入 |
| [STEP-0118](./STEP-0118-m12-exit-audit.md) | complete / GO-core | M12 | 10 项 exit gate：1-6/8 GO-core（合同+五层全部冻结并测试）；7 partial（pooling/retry 在 runner 集成）；自动 NO-GO 清零 |
| [STEP-0122](./STEP-0122-measurement-completion.md) | complete | M13 | 语义索引 accuracy/latency 基准（median 776µs/p95 1927µs，8/8 fixture，可复现）+ taxonomy measured frequency（180/2880，含 provenance） |
| [STEP-0123](./STEP-0123-ai-tooling-closure-audit.md) | complete / M13 GO | M13 | §13 收口审计：a/b/c GO，d blocked-external-evidence（ADR-0011 target 待 live-model）；validate-step-0123 机械校验全部声明 |
| [STEP-0124](./STEP-0124-application-platform-roadmap.md) | complete-planning | M14–M18 roadmap | 冻结 application-ready language → Web/UI 与 Native Automation → vision/model → application pilots 路线，不预留实现 STEP |
| [STEP-0125](./STEP-0125-http2-runner-integration.md) | complete | M12 | guest-visible http@0.2.0：provider 流式/池/重试/上传 + runner 链接（缓冲/流式/上传/secret/per-Store 池）+ 6 个手编 guest Component 测试；CLI `--allow-endpoint/--http-trust-roots/--secret-file` |
| [STEP-0126](./STEP-0126-linux-provider-corpus.md) | complete | M12 | Linux x64 native（WSL2 Ubuntu-24.04）provider 语料 48/48 + runner 37/6/35 + STEP-0089 oracle 13/13 全绿；证据 target/evidence/step-0126/linux/ |
| [STEP-0127](./STEP-0127-m12-full-go-audit.md) | complete / GO | M12 | M12 完整 GO 复审：10/10 exit gates GO，自动 NO-GO 清零；源码层 0.2.0 emission 留给 M14 |
| [STEP-0128](./STEP-0128-live-model-evaluation.md) | complete | M13 | 权威 live-model 评测（DeepSeek deepseek-chat，96×30，$0.78）：实测 0.9095，ADR-0012 预算下达且达标→§13 (d) = 实测 GO（B-repair 0.833 为既定缺口）；报告 docs/reports/ai-eval-live-model-v1.md |
| [STEP-0129](./STEP-0129-m14-inventory-profile-rfc.md) | complete (RFC-0038 accepted) | M14 | M14 盘点：check/build/run 实测矩阵（递归过 check 但 match 全 return 臂封死；无 loop 语法）、应用 profile 与出口语料冻结 RFC |
| [STEP-0130](./STEP-0130-general-control-flow.md) | complete | M14 | 通用控制流落地：while/if-else/break/continue/set + IR cells + 通用 CFG 降级，冻结形状字节不变；端到端 3 测试 + 修复 watch bridge 抢跑 bug |

STEP-0062–0069 的仓库本地顺序已闭环；STEP-0070–0074 为后续支持工作。STEP-0075–0084 完成 M8（GO）；STEP-0085–0094 完成 M9（GO）；STEP-0095–0102 完成 M10（GO）；STEP-0103–0110 完成 M11（GO）。M12 STEP-0111–0118 已形成 GO-core；STEP-0125 完成 guest-visible http@0.2.0 runner integration，STEP-0126 关闭 Linux provider parity，STEP-0127 复审发出 M12 完整 GO。STEP-0119–0123 为 M13 AI tooling closure 并行支持轨；STEP-0128 以 DeepSeek 真实运行收口，ADR-0012 依实测基线下达预算且达标——§13 质量预算项 (d) 实测 GO，B-repair 0.833 为既定跟踪缺口。STEP-0124 只冻结 M14–M18 路线与门槛，没有预留或启动任何实现 STEP。

| [STEP-0195](./STEP-0195-m22-s6-exact-ir-signatures.md) | complete | M22 S4 | 精确 typed IR function signatures |
| [STEP-0196](./STEP-0196-m19-fresh-host-ci-repair.md) | complete-support | M19 | fresh-host Python/GNU toolchain CI 修复与 11/11 CI 证据 |
| [STEP-0197](./STEP-0197-m22-s6-statement-shape.md) | complete | M22 S4 | canonical statement shape |
| [STEP-0198](./STEP-0198-m22-bootstrap-architecture.md) | complete-design | M22 | 接受 ADR-0015 A=B=C 自举架构与证据门 |
| [STEP-0199](./STEP-0199-m22-return-expression-shape.md) | complete | M22 S4 | return expression shape |
| [STEP-0200](./STEP-0200-m22-recursive-expression-tree.md) | complete | M22 S4 | recursive expression tree |
| [STEP-0201](./STEP-0201-m22-first-verifier-accepted-ir.md) | complete | M22 S4 | 首个 verifier-accepted canonical IR |
| [STEP-0202](./STEP-0202-m22-parameter-ssa-ir.md) | complete | M22 S4 | parameter SSA IR |
| [STEP-0203](./STEP-0203-m22-bool-text-constant-ir.md) | complete | M22 S4 | Bool/Text constant IR |
| [STEP-0204](./STEP-0204-m22-fixed-width-literal-ir.md) | complete | M22 S4 | fixed-width literal IR |
| [STEP-0205](./STEP-0205-m22-multi-scalar-parameter-ir.md) | complete | M22 S4 | multi-scalar parameter IR |
| [STEP-0206](./STEP-0206-m22-recursive-user-call-ir.md) | complete | M22 S4 | recursive typed user-call IR |
| [STEP-0207](./STEP-0207-m22-cross-function-call-ir.md) | complete | M22 S4 | cross-function call IR |
| [STEP-0208](./STEP-0208-m22-constant-call-argument-ir.md) | complete | M22 S4 | constant call-argument IR |
| [STEP-0209](./STEP-0209-m22-fixed-width-operation-ir.md) | complete | M22 S4 | fixed-width operation IR |
| [STEP-0210](./STEP-0210-m22-fixed-literal-argument-ir.md) | complete | M22 S4 | fixed-width literal call arguments |
| [STEP-0211](./STEP-0211-m22-nested-call-argument-ir.md) | complete | M22 S4 | nested call arguments |
| [STEP-0212](./STEP-0212-m22-fixed-op-literal-operand-ir.md) | complete | M22 S4 | fixed-operation literal operands |
| [STEP-0213](./STEP-0213-m22-interim-evidence-audit.md) | complete-audit / M22 NO-GO | M22 | 校正旧 S1–S5 完成措辞，逐门记录实际可执行证据 |
| [STEP-0214](./STEP-0214-m22-corpus-bundle-baseline.md) | complete | M22 closure sequence 1 | 冻结 215-source 语料与 Rust formatter/checker oracle，落地 ADR-0015 strict canonical source-bundle decoder 和全部 limit+1 |
| [STEP-0215](./STEP-0215-m22-lossless-lexer-corpus.md) | complete | M22 S3 lexer | Sico lossless kind/span/raw-text token stream 在 215/215 冻结源码与 Rust lexer byte-exact 差分绿；按物理行有界执行 |
| [STEP-0216](./STEP-0216-m22-semantic-ast-shape.md) | complete / partial-S3 | M22 S3 AST | Sico parser 的 accepted semantic ModuleAst declaration shape 在 99/99 源码与 Rust byte-exact；syntax/refusal AST 仍开放 |
| [STEP-0217](./STEP-0217-m22-accepted-formatter-parity.md) | complete / partial-S1 | M22 S1 formatter | Sico formatter 在 99/99 Rust-accepted 源码上 byte-exact 且二次格式化幂等；116-source typed refusal gate 仍开放 |
| [STEP-0218](./STEP-0218-m22-formatter-refusal-gate.md) | complete | M22 S1 formatter | 基于源码 token 错误检测关闭 116/116 typed lexical refusal；合并 accepted 99/99 后 S1 全 215-source 完成 |
| [STEP-0219](./STEP-0219-m22-declaration-metadata.md) | complete / partial-S3 | M22 S3 AST | accepted ModuleAst kind/name/range/detail 在 99/99 源码与 Rust byte-exact；expression/statement/recovery/refusal AST 仍开放 |
| [STEP-0220](./STEP-0220-m22-script-build-corpus.md) | complete / oracle baseline | M22 L2 corpus | Script v0 37 accept/178 refuse；accepted Component 37/37 双构建 byte-reproducible，refused 0 artifact，固定 guest parity 目标 |
| [STEP-0221](./STEP-0221-m22-modular-compiler-frontend.md) | complete / frontend integrated | M22 S3→S4 | lossless lexer + declaration parser 链接进 compiler Component；自身 3 源码 token/AST metadata 对齐 Rust，lowering/codegen 仍 partial |
