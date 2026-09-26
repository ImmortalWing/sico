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
| [STEP-0222](./STEP-0222-m22-checked-add-match-ir.md) | complete / partial-S4 | M22 S4 | I64/U64 checked_add 精确 all-return match：3-block canonical IR、spill/project、34 正例 byte-exact；非零 fallback typed refusal |
| [STEP-0223](./STEP-0223-m22-checked-arithmetic-match-ir.md) | complete / partial-S4 | M22 S4 | 同一精确 match 形状扩至 checked_sub/mul/div；I64/U64 六个新增正例，累计 40 个 byte-exact |
| [STEP-0224](./STEP-0224-m22-checked-literal-operand-match-ir.md) | complete / partial-S4 | M22 S4 | checked match 支持同宽定宽字面量操作数；常量先发与 SSA 顺序逐字节一致，累计 42 正例 |
| [STEP-0225](./STEP-0225-m22-checked-nested-call-match-ir.md) | complete / partial-S4 | M22 S4 | checked match 左右操作数支持一层 typed user call；错误返回类型拒绝，累计 44 正例 byte-exact |
| [STEP-0226](./STEP-0226-m22-deep-checked-operand-ir.md) | complete / partial-S4 | M22 S4 | checked operand 证明二层 nested call 与 nested literal 递归发射；累计 46 正例 byte-exact |
| [STEP-0227](./STEP-0227-m22-straight-let-binding-ir.md) | complete / partial-S4 | M22 S4 | 首个通用语句切片：单个定宽运算 let 绑定后 return；纯 SSA alias、literal 先发，累计 48 正例 byte-exact |
| [STEP-0228](./STEP-0228-m22-chained-let-binding-ir.md) | complete / partial-S4 | M22 S4 | 两条直线 let 建立跨语句 SSA 依赖；独立源码范围锚点与同宽拒绝，累计 50 正例 byte-exact |
| [STEP-0229](./STEP-0229-m22-straight-binding-environment.md) | complete / partial-S4 | M22 S4 | 循环式 name/type/value 环境移除 1/2 条绑定上限；4/5 条依赖链与前向引用拒绝，累计 52 正例 byte-exact |
| [STEP-0230](./STEP-0230-m22-long-block-literal-ir.md) | complete / partial-S4 | M22 S4 | 长直线块任意语句支持 I64/U64 literal；const/operation SSA 顺序、溢出拒绝，累计 54 正例 byte-exact |
| [STEP-0231](./STEP-0231-m22-environment-call-ir.md) | complete / partial-S4 | M22 S4 | 长直线块统一 typed user call：嵌套括号分段实参、参数/前绑定/literal 解析与位置类型检查，累计 58 正例 byte-exact |
| [STEP-0232](./STEP-0232-m22-scalar-const-binding-ir.md) | complete / partial-S4 | M22 S4 | 长直线块统一 Int/Bool/Text 标量常量绑定：token 级 range、canonical magnitude 与转义拒绝，累计 64 正例 byte-exact |
| [STEP-0233](./STEP-0233-m22-alias-and-const-return-ir.md) | complete / partial-S4 | M22 S4 | 长直线块统一别名绑定（零指令纯 SSA alias）与裸常量 return；emitted 计数逗号排序，累计 69 正例 byte-exact |
| [STEP-0234](./STEP-0234-m22-literal-call-return-ir.md) | complete / partial-S4 | M22 S4 | 长直线块统一定宽 literal 与 typed call return；实参嵌套调用解析与 value-base 推进，累计 76 正例 byte-exact |
| [STEP-0235](./STEP-0235-m22-fixed-op-return-ir.md) | complete / partial-S4 | M22 S4 | 长直线块统一定宽运算 return：operand 检查、literal 先发与 value/amount 键，全部 return 形状闭环，累计 81 正例 byte-exact |
| [STEP-0236](./STEP-0236-m22-set-mutation-cell-ir.md) | complete / partial-S4 | M22 S4 | set/mutation cell 模式：let/set→write_local、读→read_local、locals 表，与 Rust general CFG 路径逐字节一致，累计 88 正例 |
| [STEP-0237](./STEP-0237-m22-cell-mode-op-call-rhs.md) | complete / partial-S4 | M22 S4 | cell 模式运算/调用 RHS：操作数/实参 cell 读源序 read_local、零参调用，累计 96 正例 byte-exact |
| [STEP-0238](./STEP-0238-m22-cell-mode-nested-call-args.md) | complete / partial-S4 | M22 S4 | cell 模式 call 实参嵌套调用：复用 argument_* 机械、seg_advance 统一推进，累计 100 正例 byte-exact |
| [STEP-0239](./STEP-0239-m22-structured-if-ir.md) | complete / partial-S4 | M22 S4 | 结构化 if/else lowering：branch/jump/unreachable 块、if 头 range、set/return 双形态，累计 107 正例 byte-exact |
| [STEP-0240](./STEP-0240-m22-structured-while-ir.md) | complete / partial-S4 | M22 S4 | while+break/continue lowering：头/体/退出/回边块与 7 块 if 变体、locals 预算内辅助拆分，累计 110 正例 byte-exact |
| [STEP-0241](./STEP-0241-m22-origin-dev-quality-repair.md) | complete / partial-S4 | M22 S4 | origin/dev 合并质量修复：旧路径 local cell 优先、新 cell frontend 同名遮蔽 typed fail-closed、稳定诊断码、all-target clippy 与重复 STEP 记录清理 |
| [STEP-0242](./STEP-0242-m22-checker-lexical-partition.md) | complete / partial-S2 | M22 S2 | checker 复用 integrated lexer；215-source 冻结语料的 116 `LEXICAL` / 99 non-lexical 分区由真实 runner 精确验证，无大型 token 中间串 |
| [STEP-0243](./STEP-0243-m22-checker-e1xxx-syntax-identities.md) | complete / partial-S2 | M22 S2 | integrated parser 按真实块/定界符/声明形状实现 E1001–E1016；12 mutation + 4 shape oracle 全部经真实 runner，215-source 无误判回归保持绿 |
| [STEP-0244](./STEP-0244-m22-checker-core-semantics.md) | complete / partial-S2 | M22 S2 | struct-of-arrays 语义表实现 E2001/E2002/E2010/E2011/E2020；9 个冻结拒绝精确匹配、65 accepted 保持通过，删除 free-form header scanner |
| [STEP-0245](./STEP-0245-m22-checker-frozen-corpus-closure.md) | complete / S2 GO | M22 S2 | 215-source 声明式 checker 子集关闭：116 lexical + 34 精确诊断 identity + 65 accepted + 0 unsupported；outline 复用 STEP-0216/0219 byte-exact 证据，不宣称完整诊断渲染 parity |
| [STEP-0246](./STEP-0246-m22-soa-convergence-baseline.md) | complete / M22 NO-GO | M22 S3/S4 | canonical-LF 语料修复 + checker SHA 门；ADR-0016 冻结 SOA/compilation-unit；parser 模块与 compiler 统一 lowering，删除 exact-source IR fallback；完整 canary/S5/S6 仍开放 |
| [STEP-0247](./STEP-0247-m22-linear-builder-and-intrinsic-prefix.md) | complete / partial-S4; M22 NO-GO | M22 S3/S4 | versioned-capacity List COW 消除完整源码 lexer trap；Text/U64 alias 真实 runner 回归；checked fallback、formatter intrinsic 与 fixed guard-chain prefix canonical IR byte-exact；user-call guard/S5/S6 仍开放 |
| [STEP-0248](./STEP-0248-m22-user-call-guard-chain.md) | complete / partial-S4; M22 NO-GO | M22 S4 | guard-chain 接入 typed user call、literal 参数与 Text return；formatter 前七函数（至 `word_kind`）canonical IR byte-exact；`scan_space`/S5/S6/S7 仍开放 |
| [STEP-0249](./STEP-0249-m22-local-limit-repair-and-frontier-repin.md) | complete / partial-S4; M22 NO-GO | M22 S4 | 256-local 越限修复：`while_bytes_at_rhs_packed` 抽取使 self-host 组件可构建（busiest 238 locals + headroom 回归钉）；验证器/corpus 工具 PS 5.1 兼容化 + 确定性 JSON 序列化 + LF 语料真落地；canary 前沿如实重钉至 `E-SH-IR-CALL-TYPE`；S5/S6/S7 仍开放 |
| [STEP-0250](./STEP-0250-m22-bytes-parameters.md) | complete / partial-S4; M22 NO-GO | M22 S4 | `scalar_type_kind` 接受 `Bytes` 参数（IR/codegen 本就支持），Bytes-parameter while 函数从 `E-SH-IR-PARAMETER-TYPE` 变为可 lowering；canary 前沿推进至 while 体 if/else 语句区域的 `E-SH-IR-STATEMENT`；旧验证器改为钉 typed fail-closed 类，前沿码由最新验证器钉定 |
| [STEP-0251](./STEP-0251-m22-general-while-if-else-regions.md) | complete / partial-S4; M22 NO-GO | M22 S4 | 通用 while 体 if/else 语句区域：`general_while_function_ir` 惰性块纪律（then/else 分支期、join 闭区期、exit 闭环期分配）+ `gw_decimal_value`/`gw_instr_count`/`gw_compare_packed`/`gw_rhs_packed` 支撑；formatter 前八函数（至 `scan_space`）canonical IR byte-exact；canary 前沿推进至 `E-SH-IR-EXPRESSION`；S5/S6/S7 仍开放 |
| [STEP-0252](./STEP-0252-m22-nested-intrinsic-condition.md) | complete / partial-S4; M22 NO-GO | M22 S4 | `gw_condition_packed` 按 Rust oracle 顺序发射 `ident_continue(sico.bytes.at(...))` 的 read/literal/intrinsic/call 链；formatter 前九函数（至 `scan_ident`）canonical IR byte-exact；canary 前沿推进至 `scan_integer` 的 if-without-else `E-SH-IR-STATEMENT`；S5/S6/S7 仍开放 |
| [STEP-0253](./STEP-0253-m22-general-while-no-else-frames.md) | complete / partial-S4; M22 NO-GO | M22 S4 | parent-linked append-only if frame + deferred empty-else block 对齐 nested/sequential no-else CFG，修复单指令与 return-call SSA 计数；formatter 前十三函数（至 `punctuation_kind`）canonical IR byte-exact；canary 前沿推进至 `item` 的 `sico.list.get` match `E-SH-IR-CALL-TARGET`；S5/S6/S7 仍开放 |
| [STEP-0254](./STEP-0254-m22-prepared-compiler-test-reuse.md) | complete / validation infrastructure | M22 support | 差分 harness 复用 PreparedProgram、串行共享 Engine，每次独立 Store；13 项串行实测 801.46s → 66.99s，补拒绝后合法输入恢复；语言与 bootstrap 支持不变 |
| [STEP-0255](./STEP-0255-m22-m24-plan-refinement.md) | complete / planning documentation | M22–M24 planning support | M22 增切片状态表（§3.1）与执行队列（§3.2，item → formatter 尾 → selfhost 全源 → S5 → S6 → S7）；M23 增四项逐项工作协议（§8）与排序依赖图（§9）；M24 增逐 gate 工作分解、pilot 循环能力映射与枚举错误注入语料（§8）；不预留 STEP 编号、不改任何 gate |
| [STEP-0256](./STEP-0256-m22-list-get-match-and-nested-intrinsic-returns.md) | complete / partial-S4; M22 NO-GO | M22 S4 | `list_get_match_ir` 降级 `match sico.list.get(List[Text], U64)` result-match（intrinsic 主题 + project/const_string 臂）+ `nested_intrinsic_return_ir` 降级 `append_pair` 嵌套 `sico.list.append` return + `parameters_ir`/`list_aware_parameter_index` 支持 `List[<scalar>]`；formatter 前十五函数（至 `append_pair`）byte-exact；canary 前沿推进至 `line_tokens` 的多 match `E-SH-IR-CONTROL`；S5/S6/S7 仍开放 |
| [STEP-0257](./STEP-0257-m24-sico-native-ui-library-pilot-target.md) | superseded by STEP-0258 / planning documentation | M24 planning support | owner 指令「方块游戏pilot改为GUI和原生界面库（风格类似winui3）」+「原生界面库是sico语言原生界面库」：曾登记 Sico 语言原生界面库前置工作流（§8.6.1，本记录后由 STEP-0258 保留）与第一方 GUI 方块游戏试点目标；同日复被 STEP-0258 删除 block-game 目标，本记录仅留档指令历史 |
| [STEP-0258](./STEP-0258-m24-pilot-redesignation-gui-format-converter.md) | complete / planning documentation | M24 planning support | owner 指令「方块游戏pilot目标删除，改为带GUI界面的格式转换器，第一步可以先实现图片格式互相转换，例如jpg互转png」：M24 计划更名 vision-closure-gui-application-pilot 并重写——block-game 自动化试点删除（capture→vision→solver→input 闭环移出 M24，M16 证据停在 GO 级别），M18 组合项 4 重定为带 GUI 格式转换器（Sico 语言原生界面库 + 第一步 JPEG↔PNG 双向互转，codec 合同 RFC 先行，如实登记 image-vision@1 无 codec I/O）；§8.4/§8.5 改为转换器工作包与 fail-closed 语料；ROADMAP/M18/M25/AGENT_GOAL/DIRECTION/案例 README 同步；gate 数/证据分级不变、不预留 STEP 编号、无实现声明 |
| [STEP-0259](./STEP-0259-m18-audit-addendum-registration.md) | complete / planning documentation | M18 planning support | owner 确认「M18 修改了是否需重做」= 不需重做：STEP-0164 四项 pilot 证据与 STEP-0165 审计保持有效（重定仅触及从未完成的组合项 4）；登记 M18 出口审计增补协议于 M18 计划 §7——M24 转换器试点过 §3 gate 5 后按 §4 验收路径审计、组合项 4 翻 GO、重跑 §8 gate 7 回归、更新 ROADMAP/STATUS、独立 STEP 留档；外部 pilot NO-GO 不触碰；M18 保持 GO 4/5 直至增补实跑 |
| [STEP-0260](./STEP-0260-m22-line-tokens-general-while-convergence.md) | complete / partial-S4; M22 NO-GO | M22 S4 | general-while 机内收敛 `line_tokens`：通用调用条件、slice 主语逗号扫描、`#match<binding>` 绑定、List 类型 JSON 透传、固定运算 set RHS、break/continue；formatter 前十六函数 byte-exact；canary 推进至 `source_has_lex_error`；S5/S6/S7 仍开放 |
| [STEP-0261](./STEP-0261-m22-nested-while-frames.md) | complete / partial-S4; M22 NO-GO | M22 S4 | general-while 机单槽 while 帧改打包帧栈（`gwh_frames`/`gwh_len`）：嵌套/顺序 while byte-exact、`end while` 按帧弹栈回填 after 块、break 按 `brk_base` 帧作用域、branch 终结子表 `gwt_entries`；配套 `sico.list.length` 参数/cell 主语、`sico.text.split_lines` 非字面量实参、用户调用嵌套调用实参递归打包；formatter 前十七函数 byte-exact（fixture step-0261 117,109 字节钉住）；`general_while_function_ir` 240 locals 恰在界内 |
| [STEP-0262](./STEP-0262-m22-format-code-region.md) | complete / partial-S4; M22 NO-GO | M22 S4 | gw 机 while-less 语句函数最后回退路由（dispatcher last resort）：bare-cell 条件、entry 区域 if/return、按开序编号的 deferred 空 else、`sico.text.concat` intrinsic RHS 含嵌套实参；`call_left`/`format_code`/`repeat_indent` byte-exact，formatter 前二十二函数收敛；canary 推进至 `nearest_match` 的 `Map[Text,U64]` 参数面（`E-SH-IR-UNRESOLVED`）；S5/S6/S7 仍开放 |
| [STEP-0263](./STEP-0263-m26-pdf-milestone-registration.md) | complete / planning documentation | M26 planning support | owner 指令（2026-09-24「确认，要使用sico原生UI库」+「如果原生UI库不完善那就完善M24」）登记 M26 PDF 里程碑：结构化阅读/合并/旋转为 versioned package（RFC 先行、fail-closed 语料、不改核心语言/authority），GUI PDF 工具以 Sico 语言原生界面库编写；M26 排 M25 之后；ROADMAP/双索引/规划校验器/STATUS 同步 |
| [STEP-0264](./STEP-0264-m22-m26-route-replan.md) | complete / planning documentation | M22–M26 planning support | owner 指令（2026-09-24「重新规划路线，审视M22-M26，确保能落实到工程实现」）登记 M22–M26 路线重划片：M22 执行队列外加 R 阶段收敛门（R0 SOA 冻结 ADR vs `List[record]` RFC、R1 canary 覆盖率+消耗燃料预算曲线、R3 预登记止损 4 连零增长）；M23 实现门双出口（Route A：S7 GO 后 / Route B：止损宣告后立即，re-baseline 在 M23 审计后重启）；M24 即刻并行开工盘点；M25 入口要求 M22 行显式；不改能力合同/证据等级，不预留实现 STEP 编号 |
| [STEP-0265](./STEP-0265-m22-restart-assessment.md) | complete / planning documentation | M22 planning (R0 input) | owner 指令（2026-09-24「查看M22已有步骤，是不是需要回退重新开始。规划好之后重新实现M22」）分层盘点判定：不做 git 级回退——语料/harness/oracle 与 S1/S2 无条件保留，S3/S4 通用机器保逻辑、源层数据模型重写挂 R0，legacy lexer/parser 合并债两种选项都偿还；登记在途 WIP（Map 参数面 +228 行）处置要求；R0 两选项（records RFC vs SOA 冻结 ADR）成本表与分波重启计划（R1→R0→W1 合并→W2 重授权层→W3 S5/S6/S7） |
| [STEP-0266](./STEP-0266-r1-canary-and-budget-instrumentation.md) | complete / R1 instrumentation | M22 R-phase | R1 仪器化落地并实测出数：`report-m22-canary.ps1`（canary 30 函数覆盖 22=73.3%，前沿 nearest_match @ E-SH-IR-GWPACK-OTHER——WIP 的 Map 参数面已过关、前沿深入）；`probe-m22-budget.ps1`（1k/4k 消耗∈(1e8,1e9]、8k/16k∈(1e9,5e9] 无悬崖；wall 由复杂度主导：22 formatter 函数 46s vs 565 平凡函数 0.3s，debug runner）；关闭 review P0-3（燃料实测）；S6 预算须申报 5e9 级 cap + release 复测 |
| [STEP-0267](./STEP-0267-r0-adr-0017-draft.md) | complete / contract draft; owner acceptance pending | M22 R-phase (R0) | R0 决策文档草案 [`ADR-0017`](../adr/ADR-0017-selfhost-data-model-architecture-v0.md)（status: proposed）：推荐 Option B——SOA 升级为永久数据模型并补齐 P0-2 三件套（canary/coverage-growth 指标/R3 止损阈值），records RFC 脱钩不阻塞；否决 Option A 的证据 = STEP-0266 燃料无悬崖 + 机器逻辑与语言形状无关；owner 可选 A（本 ADR 拒绝转 records RFC）或 C（拒绝）；revisit 条件三条固定 |
| [STEP-0268](./STEP-0268-r0-recommendation-amendment.md) | complete / decision-criteria evaluation | M22 R-phase (R0) | owner 标准指令（2026-09-24「我只看最终效果和稳定性、长期可维护性」）对 ADR-0017 的推荐修订留痕：三维度重评后推荐由 B 翻转为 **A（records RFC 先行）**——B 的"永久"是假永久（M14/M23 需求压力不消失，B = SOA 维护 + 迟到重写；A = 只重写一次 + 终局单一数据模型）；ADR-0017 全面修订，Option 分析保留可追溯；待 owner 显式接受 |
| [STEP-0269](./STEP-0269-rfc-0047-record-types-draft.md) | complete / contract draft; owner acceptance required | M22 R0 outcome | owner 接受 Option A（2026-09-24「开始」）后起草 [`RFC-0047`](../rfc/RFC-0047-record-types-v0.md) record 类型最小封闭集（声明/具名字面量/字段访问/`List[record]`；词法表零增长；List 边界降级到 ADR-0016 并行数组；排除可变字段/方法/泛型/WIT record）；RFC-0033 四证据 + EC-1..EC-5 出口语料 + 双实测消费者（selfhost 列形状普查 + 应用 profile 普查）；RFC README 补齐 0043–0047 登记 |
| [STEP-0270](./STEP-0270-rfc-0047-acceptance-ec4-census.md) | complete / RFC accepted; EC-4 census frozen | M22 R0 outcome | owner 指令（2026-09-24「接受」）登记 RFC-0047 翻 accepted 并完成接受后第一个实现前 STEP = EC-4 双普查冻结（纯测量零语法改动，`tools/census-ec4-records.ps1`，schema `sico.m22.ec4.v0`，冻结证据 [`m22-ec4-census-2026-09-24.json`](../reports/m22-ec4-census-2026-09-24.json)）：selfhost 16,931 行中 list 仪式行 360（2.13%，SOA 缩减上限代理——选 A 依据是终局单一数据模型而非行数），≥3 List 形参函数 16；应用 e2e 39 文件 3,450 行中 map-as-struct 字面量键仅 14 个去重键（验证 v0 排除 Map/Set record 键值）、≥5 形参宽签名函数 5；下一步 = EC-1..EC-3 语料冻结，WIP 照还，不预留实现 STEP 编号 |
| [STEP-0271](./STEP-0271-rfc-0047-ec1-corpus-frontend-slice.md) | complete / frontend slice; script-v0 ABI next | records implementation (RFC-0047) | owner 指令（2026-09-24「你起草完直接动手就行」）：EC-1 语法语料冻结（9 个 record-*.sico：3 接受 + 5 拒绝 + 1 个 D5 gate）+ 前端实现片——语义/IR 裸字段行收集（`field` 行并存零扰动）、E2021 重复字段、E2022 空 record、点访问未知字段 E2011 堵 unknown 洞；实证 check 3 accept 绿 + 5 拒绝各单诊断、build 链 honest refusal（non-scalar parameter/local）、format 双程不动点；`record_types.rs` 5 拒绝测试即刻生效、3 run 测试 ignore 待 D4 ABI 片（codegen-wasm LocalLayout/参数结果扁平化，下一片不预留编号）；runner 测试本机不可重建（ring 需 gcc），CI 绿为准 |
| [STEP-0272](./STEP-0272-rfc-0047-script-abi-flattening.md) | complete / D4 ABI slice; e2e runs | records implementation (RFC-0047) | owner 指令（2026-09-24「开始」）落地 script-v0 ABI record 扁平化：`Module.records` canonical 表（D4，serde 空表字节不变）+ `ScriptAbi.user_records` 注册（不可扁平化则诚实拒、不猜布局）+ record local cell 字段图 + 链式投影 `seg.b.x`（迭代 Project）；三语料 build+runner 实跑 marker 全过、拒绝面零回归、D5 仍诚实 gate；record-nested 多行字面语料改单行（跨行字面量实参债登记）；三个 run 测试解 ignore（CI 裁决）；后续 = D5 `List[record]` 单态、EC-2/EC-3/EC-5 语料与矩阵行 |
| [STEP-0273](./STEP-0273-rfc-0047-ec2-refusals-ec5-matrix.md) | complete / EC-2 refusal surface + EC-5 matrix | records implementation (RFC-0047) | owner 目标「完成M22，途中发现问题要反馈和回顾」的 records 收口段：实测两处置信缺口修复——`set p.x` 从误导性 E2001 改为 E2023 FIELD_IMMUTABLE（字段不可变+行动提示）、record `==`/`<=` 从静默放行改为 E2024 RECORD_COMPARISON（D3 v0 拒绝）；EC-2 语料 +3（字段 set 拒绝/相等拒绝/COW 别名 run `record-cow-ok`），record_types 测试 11 个全接线；EC-5 矩阵登记 `records-v0-user-records` 行（行级 step，头 step 为冻结锚不动）；D5 `List[record]` 布局决策反馈：ADR-0016 列式 vs 行式，默认列式（RFC D4 字面），下一片实现 |
| [STEP-0274](./STEP-0274-rfc-0047-d5-list-of-records.md) | complete / D5 column-layout monomorphs | records implementation (RFC-0047) | owner 目标「完成M22」的 D5 片：`List[record]` ADR-0016 列式单态 empty/length/get/append 落地（capacity 步距列址、header 同标量方案、get 走 general Result 4 槽形状）；v0 收窄 = 元素记录字段限 I64/U64（其余 typed 拒绝）、越界 error-tag 未定义（写 0，语料只按 tag 匹配）；途中实测修两缺陷（拷贝守卫反置 trap、元素写入被包进守卫致恒零）；`record-list-of-records.sico` run 解锁 `record-list-of-records-ok`，既有 15 语料零回归，三 crate test/clippy/fmt 全绿；records v0 至此全封，下一步 = for 迭代 + W1 合并债（在途 WIP 先落地） |
| [STEP-0275](./STEP-0275-rfc-0047-d5-for-iteration.md) | complete / D5 for-iteration + depth audit | records implementation (RFC-0047) | owner 目标「完成M22」的 D5 尾片：for 迭代 over `List[record]` 落地（语义 executable_element 扩 flat record；IR 反糖单态名开 Named 臂，反糖本体布局泛型未动）；gate 语料扩 for 累加校验（总和 36）实跑 `record-list-of-records-ok`；嵌套深度核对结论 = v0 深度 2 文法层不可构造（E2031，三态保险），无预算债；既有语料零回归、test/clippy/fmt/校验器全绿；records v0 全封（剩 record-nested 多行字面量债 + EC-3 快照），下一步 = W1 合并债 |
| [STEP-0276](./STEP-0276-w1-wip-absorption-and-records-emission.md) | complete / WIP 吸收 + records 发射修复 | M22 compiler self-host — W1 前置 gate | owner 目标「完成M22」的 W1 前置：中断会话在途 WIP（parser.sico +228 行 `Map[K,V]` 参数面 + `count_params` 括号深度 + `dump_nm_region_ir` 探针）吸收登记；先修 STEP-0272 引入的 selfhost 微分回归——guest `scalar_ir` 补 `records` 表发射（6 个新函数，serde kind/data 契约，record 名字典序 = `sico.list.sort` byte-order，空表省略保持冻结字节）；本机 CLI 微分：formatter 前缀 14/16 byte-exact（nearest_match 修复 −393B 实证；close_code/direct_close 的 GWPACK-OTHER 经 HEAD 对照归因为既有非 gated 前沿）、driver --emit-ir 16 代表例 16/16、拒绝面 3 例精确；runner 测试仓本机不可重建，全量断言以 CI 裁决；下一步 = W1 合并债本体 |
| [STEP-0277](./STEP-0277-w1-consolidation-debts-plan.md) | complete / W1 计划登记 + 实测侦察 | M22 compiler self-host — W1 consolidation | owner 目标「完成M22」的 W1 第一刀（评估 §5 items 1-4 登记为可执行计划）：实测侦察四结论——① legacy 三件套（lexer/tokens/declaration_parser，947 行）selfhost 零 import、只被各自单测 + 校验器 0215/0216/0219 + 0245 suite 引用，tokens.sico:431-435 裸 `<` 缺陷分支属实随退役消灭，单位级断言面由集成链 215 全量 + sha 冻结接替；② E7001 已是结构性检查，E7002 是关键词指纹（compiler_semantics.sico:582-590），改写优先、rename-safety 语料守卫；③ sha 门实测已存在（selfhost_checker.rs:159-168，STEP-0246 批落地），本片验证关闭；④ 缩进代理实位 compiler_parser.sico:489/650，0-indent 语料先行定行为；实现分片 A→B→C→D（验证关闭→退役→E7002→缩进），各片 CI 裁决、无中间红态 |
| [STEP-0278](./STEP-0278-w1-legacy-retirement-and-sha-gate.md) | complete / 片A+B 落地（CI 裁决中） | M22 compiler self-host — W1 consolidation | owner 目标「完成M22」的 W1 片 A+B：① sha 门验证关闭——实测 selfhost_checker.rs:159-168 逐入口 bytes+sha256 断言随 STEP-0246 批已落地，评估 §5 item 3 属已还债，零代码关闭；② legacy 三件套退役——删 lexer/tokens/declaration_parser（947 行，selfhost 零 import，tokens.sico 裸 `<` 缺陷分支随灭）+ 三份单位级单测（断言面由集成链 215 全量 + 语料 sha 冻结接替），校验器 0215/0216/0219 改写为退役登记（钉不存在 + 历史锚 + 改跑 selfhost_checker），0243/0244/0245 suite 摘除 selfhost_declaration_parser；本机：引用清扫零活引用、六 ps1 语法检查绿、集成链 check 全 ok；runner 测试仓本机不可重建（gcc/dlltool），215 全量差分以 CI 裁决；下一步 = 片 C（E7002 指纹改结构性） |
| [STEP-0279](./STEP-0279-m14-m26-milestone-audit-route-sync.md) | complete / planning audit | M14–M26 governance | 逐 gate 审查表与现行文档同步；M14–M21 历史 STEP 不改；ADR-0017 接受状态补记；M22 R3 停滞实现 STEP 计数、M23 双路径、M24 加速证据、M25 发布/项目双结论、M26 发布后 UI 版本边界统一；规划校验器含五个内存负例，不提升 runtime 支持 |
| [STEP-0280](./STEP-0280-w1-e7002-and-zero-indent.md) | implementation and local canaries complete / W1 CI gate subsequently closed by STEP-0292 | M22 compiler self-host — W1 C/D | E7002 从 `loaded`/`model` 字面指纹改为记录字段+形参+guard 块结构；零缩进正文差分发现并修复语义扫描漏检；5 个 SHA 冻结增量用例 Rust oracle/Windows GNU guest 一致，原 215 项分区及 STEP-0221/0245/0261/0262 canary 本机绿；独立 CI 于 STEP-0292 裁决，M22 仍 NO-GO |
| [STEP-0281](./STEP-0281-m22-m26-execution-cards.md) | complete / documentation-only handoff | M22–M26 governance | M22–M26 单步执行卡（每卡固定入口证据、唯一出口、停手规则与六行审查格式）；handoff-m22 重写到 STEP-0280 本机基线（两批未提交修改清单、Windows GNU runner 恢复命令）；M22 计划 §3.2 过时队列修正——`item`..`repeat_indent` 已过 canary 不再列为待办，队列头改为 W1 出口/W2 基线，退役 legacy 三件套标为历史证据；不提升任何 gate，不预留实现 STEP 编号 |
| [STEP-0282](./STEP-0282-m22-w2-measurement-baseline.md) | complete / measurement-only baseline frozen（卡 22-B） | M22 compiler self-host — W2 前置测量 | canary 全量重跑 22/30、前沿 `nearest_match`/`GWPACK-OTHER`、exit 122 与在案记录零漂移；燃料阶梯四档复验区间一致（(1e8,1e9]/(1e9,5e9]、无悬崖）；tracked selfhost 8 源 16,390 行 SHA256 盘点；栈高水位未测得（runner CLI 不暴露）；基线冻结于 [`m22-w2-baseline-2026-09-25.json`](../reports/m22-w2-baseline-2026-09-25.json)；测量 STEP 不计 R3，M22 保持 NO-GO |
| [STEP-0283](./STEP-0283-m22-w1-local-full-ci.md) | complete / local full-CI green；W1 独立 CI 仍未裁决（卡 22-A 本机部分） | M22 compiler self-host — W1 前置裁决 | 修复 STEP-0279 引入的真实潜在 CI 红：validate-step-0124.ps1 中文锚串缺 BOM 在 PS 5.1 ANSI 代码页 ParserError，补 UTF-8 BOM 后 `-File` 直跑绿；登记 windows-gnu 链接 flake（0xc0000409，红→复跑绿，非产品回归）；复跑 run-ci **CI GREEN 11/11** 覆盖全部未提交改动；W1 关闭仍待 owner 授权远端 CI 或显式接受本机裁决，M22 NO-GO |
| [STEP-0284](./STEP-0284-m23-ceremony-census.md) | complete / measurement-only census frozen（卡 23-A） | M23 kickoff inventory | 新增 `tools/census-m23-ceremony.ps1`（纯 ASCII、无时间戳、两次运行字节一致），冻结 [`m23-ceremony-census-2026-09-25.json`](../reports/m23-ceremony-census-2026-09-25.json)：selfhost 8 源 16,390 行中 checked 算术 match 263（单目标 bind 型 248）、比较谓词 if 978（59.7/千行）、嵌套比较 if≥2 位点 79、literal 构造器 2,169（3.199/KB）、chars 调用 153；应用语料 51 文件 3,795 行；AI 语料与运行时计数如实记未测；不构成语法决定，M23 保持 planned，实现仍等双门 |
| [STEP-0285](./STEP-0285-m23-rfc-item1-draft.md) | complete / RFC-0048 registered as **draft**（卡 23-B） | M23 per-item RFC phase | item 1：[`RFC-0048`](./STEP-0285-m23-rfc-item1-draft.md) 草案——五中缀比较+`&&` desugar 到既有 EqualFixed/LessFixed（零新 IR 操作），`||`/`!` 零实测消费被排除，交换形原子限制+E2025；新增实测：`<=` 记号无消费者造成 check 绿/build 拒分裂；item 4 按 §8.4 决策规则收口（组合式为最终形态）；item 3 因 AI 语料外部门 deferred；无实现无 gate 变化 |
| [STEP-0286](./STEP-0286-m23-rfc-item2-draft.md) | complete / RFC-0049 registered as **draft**（卡 23-B item 2） | M23 per-item RFC phase | [`RFC-0049`](./STEP-0286-m23-rfc-item2-draft.md) 草案——仅 `let x = if/else` 与 `let x = match` 两种最小 let 绑定块（单表达式臂，缺 else=E2027、臂非表达式=E2026，类型不匹配走 E2001），desugar 到 D2 已落地机制零新 IR 操作；实测 248+121 个 bind 型 match 模拟位点、两种形态当前 parse 拒 `MismatchedClose`；更大的表达式位置 v0 继续拒绝；无实现无 gate 变化 |
| [STEP-0287](./STEP-0287-m24-kickoff-inventory.md) | complete / inventory frozen（卡 24-A） | M24 kickoff inventory | 五条线盘点冻结于 [`m24-inventory-2026-09-25.json`](../reports/m24-inventory-2026-09-25.json)：加速证据主机未命名（owner 门，gate 2 保持延期）；模型资产 0、RFC-0043 仍 draft；image-vision@1 四纯函数在册而 roster 三包零代码（tetris 链为历史证据）；Sico 原生 UI 库整体不存在（WIT/web renderer 是 M15 面，不算）；codec 依赖 0、拒绝语料零文件；四份前置合同（加速 ADR/provenance RFC/UI RFC+ADR/codec RFC）可起草；无 gate 变化不计 R3 |
| [STEP-0288](./STEP-0288-m24-contracts-codec-provenance.md) | complete / RFC-0050+0051 registered as **draft**（卡 24-A 合同面） | M24 prerequisite contracts | [`RFC-0050`](./STEP-0288-m24-contracts-codec-provenance.md) image-codec@1（JPEG↔PNG 四纯函数、BGRA8 复用 RFC-0041 契约、profile/编码参数按名冻结、字节级稳定、§8.5 拒绝面 typed 收口）；[`RFC-0051`](./STEP-0288-m24-contracts-codec-provenance.md) M17 gate 4 provenance manifest（封闭 schema、SHA-256 复用 M7、预算 typed、fixture 五项证据集、零 live-model 主张）；两份均 draft，无实现无 gate 变化 |
| [STEP-0289](./STEP-0289-m24-ui-contracts.md) | complete / RFC-0052 draft + ADR-0018 proposed（卡 24-A 合同面收口） | M24 §8.6.1 prerequisite contracts | 源侧 [`RFC-0052`](./STEP-0289-m24-ui-contracts.md)：`sico:user/ui@1` 版本化包（八种控件+事件+帧呈现），零语法增长，对话框组合既有 file 能力，封闭 UiError 含 authority-denied 默认拒绝；渲染侧 [`ADR-0018`](./STEP-0289-m24-ui-contracts.md)：Direct2D+DirectWrite 自绘 Fluent token 子集（否决 WinUI3 运行时/web/M5 扩展），隔离 adapter crate，帧=UI 状态纯函数；四份前置合同至此全部起草完毕；无实现无 gate 变化 |
| [STEP-0290](./STEP-0290-m26-inventory-pdf-rfcs.md) | complete / inventory frozen + RFC-0053/0054 draft（卡 26-A） | M26 kickoff inventory and contracts | 五线盘点冻结于 [`m26-inventory-2026-09-25.json`](../reports/m26-inventory-2026-09-25.json)：字节面适配 v0 阅读子集无缺口；PDF oracle 缺失（pikepdf 为 owner 门控候选）；inflate 两候选未实现不可测（filters 决策推迟）；UI 命名缺口 = 滚动/视口（归宿 M24 §8.6.1）；阅读子集钉定 classic xref v0。[`RFC-0053`](./STEP-0290-m26-inventory-pdf-rfcs.md) 结构阅读 + [`RFC-0054`](./STEP-0290-m26-inventory-pdf-rfcs.md) 限额/拒绝/writer 均draft；实现双门 = owner 接受 + M25 GO |
| [STEP-0291](./STEP-0291-m22-nearest-match-design.md) | complete / design + probe evidence frozen（卡 22-C 前置设计，不计 R3） | M22 compiler self-host — W2 前置设计 | 真实 runner 探针钉定 nearest_match 前沿三层形状：while 体 let 的 `u64.to_text` 内建 RHS（GWPACK-OTHER，S1）→ match 主语 `sico.map.get` 不在主语集（CALL-TARGET，S2）→ match-in-while 框架+臂内 if/return 退出（S3）；f22 前缀 exit 0 与 canary 零矛盾；三片降低方案 + R3 前沿移动记账预先声明；W1 GO 后 S1 即首个 22-C 实现 STEP |
| [STEP-0292](./STEP-0292-m22-w1-independent-ci.md) | complete / W1 GO on isolated branch（卡 22-A） | M22 compiler self-host — W1 独立裁决 | owner 授权推送；隔离分支 `codex/m22-w1-ci` 的 `e8495f9` 在 GitHub Windows GNU 独立 CI 运行 36087222284 成功，W1 于该快照裁为 GO；`origin/dev` 的冲突历史未合并，M22 仍 NO-GO |
| [STEP-0293](./STEP-0293-m22-w2-u64-to-text-while-rhs.md) | complete / S1 GO on isolated branch（卡 22-C S1） | M22 compiler self-host — W2 S3/S4 | while 体 `u64.to_text` RHS byte-exact，双参数及错误参数类型 typed 拒绝；formatter 22/30 不变，`nearest_match` 前沿由 GWPACK-OTHER 移至 CALL-TARGET；R3 连续停滞数 0；本机 0261/0262/0245 校验及修复版 `cb371c5` 独立 CI 运行 36090403437 均绿 |
| [STEP-0294](./STEP-0294-m22-w2-map-get-match-subject.md) | complete / S2 GO on isolated branch（卡 22-C S2） | M22 compiler self-host — W2 S3/S4 | `map.get[Text,U64]` match 主语字节差分通过；多余参数/类型与错误 Map/key 类型 typed 拒绝、拒绝后恢复；Map 参数类型 JSON 括号缺口修复；formatter 22/30，`nearest_match` 前沿 CALL-TARGET → STATEMENT，R3 连续停滞数 0；0261/0262/0245 本机全绿，`5a6d0bb` 独立 CI 运行 36147743130 成功 |
| [STEP-0295](./STEP-0295-m22-w2-match-arm-join.md) | complete / S3 GO on isolated branch（卡 22-C S3） | M22 compiler self-host — W2 S3/S4 | match ok 臂内嵌 if 的汇合块接到 end match；formatter 前缀 through `nearest_match` 与 Rust IR 字节一致，return 后多余语句 typed 拒绝并可恢复；canary 23/30，`set_nearest_match` / STATEMENT，R3 连续停滞数 0；`f7288b7` 独立 CI 运行 36166445685 成功，M22 仍 NO-GO |
| [STEP-0296](./STEP-0296-m22-w2-whileless-if-match-length.md) | complete / S4 GO on isolated branch（卡 22-C S4） | M22 compiler self-host — W2 S3/S4 | 无 while 的 if/match 路由与条件中 `sico.text.length(Text)` 字节差分通过；参数数目/类型 typed 拒绝并可恢复；canary 23/30，`set_nearest_match` 前沿 STATEMENT → CALL-SHAPE，R3 连续停滞数 0；`904b86a` 独立 CI 运行 36168754671 成功，M22 仍 NO-GO |
| [STEP-0297](./STEP-0297-m22-w2-nested-slice-subject.md) | local implementation complete / S5 independent CI pending（卡 22-C S5） | M22 compiler self-host — W2 S3/S4 | 嵌套 `bytes.slice` match 主语字节差分通过；多余实参与错误类型 typed 拒绝并可恢复；canary 23/30，`set_nearest_match` 前沿 CALL-SHAPE → EXPRESSION（`map.put` return），R3 连续停滞数 0；M22 仍 NO-GO |
| [STEP-0298](./STEP-0298-m22-w2-map-put-returning-match.md) | local implementation complete / independent CI pending（卡 22-C 切片 6） | M22 compiler self-host — W2 S3/S4 | `map.put` return 与双臂 return match 的字节差分通过；多余实参与错误类型 typed 拒绝并可恢复；canary 24/30，`match_arm_levels` / CALL-SHAPE，R3 连续停滞数 0；M22 仍 NO-GO |
