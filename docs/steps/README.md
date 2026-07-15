# Step records

每个非平凡执行步骤使用稳定 `STEP-xxxx` 编号，并链接实现、验证、RFC、ADR 和报告。

| Step | 状态 | 阶段 | 标题 |
|---|---|---|---|
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

下一可用编号：`STEP-0046`。
