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
| [STEP-0109](./STEP-0109-cross-platform-runner-parity.md) | blocked-external-evidence | M11 | Linux x64 原生 runner 缺失（无 WSL/主机）；parity corpus 与 `validate-step-0109.sh` 已就绪待执行 |
| [STEP-0110](./STEP-0110-m11-exit-audit.md) | complete / NO-GO pending Linux | M11 | M0–M10 聚合回归 + M11 链全绿；gate 9（Linux 原生实证）未满足，按规则判 NO-GO；M12 保持锁定 |
| [STEP-0119](./STEP-0119-ai-generation-quality-baseline.md) | complete | M13 | generation 失败归因、prompt/guide canonical 修正、fixture 修复与重测基线 0.8846→0.9744 |
| [STEP-0120](./STEP-0120-ai-quality-budget-adr.md) | complete | M13 | ADR-0011 AI 质量预算：floor 非回归层（≤ 实测基线）与 target 完成门（proof 待 live-model，blocked-external-evidence） |

STEP-0062–0069 的仓库本地顺序已闭环；STEP-0070–0074 为后续支持工作。STEP-0075–0084 完成 M8（GO）；STEP-0085–0094 完成 M9 并由 exit audit 发出 GO。STEP-0095–0102 完成 M10 并由 exit audit 发出 GO；M11 STEP-0103 ADR（accepted-design）、STEP-0104 semantic/IR contract、STEP-0105 scheduler core、STEP-0106 cancellation/race/select、STEP-0107 bounded channels 与 STEP-0108 persistent/watch/REPL/DAP 集成均已完成；STEP-0109 Linux 原生实证因环境缺失 blocked-external-evidence；STEP-0110 出口审计完成，判定 NO-GO（仅缺 Linux 原生证据，其余九项 gate 全 GO），M12 锁定至 Linux parity 补齐。STEP-0119–0123 为 M13 AI tooling closure 并行支持轨，STEP-0119 已完成，权威 live-model 评测等待所有者提供 DeepSeek 凭据。
