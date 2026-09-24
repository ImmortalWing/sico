# Requests for comments

RFC 记录语言语义、表层语法、标准库、WIT 接口、诊断协议和版本兼容决定。

当前根目录语义和语法文件仍是 M0 草案，尚未通过正式 RFC 接受为稳定规范。诊断协议 v0 已作为 M0 工具契约接受。

| RFC | 状态 | 内容 |
|---|---|---|
| [`RFC-0001`](./RFC-0001-diagnostics-protocol-v0.md) | accepted | 诊断身份、编号、文本/JSON、跨度和兼容规则 |
| [`RFC-0002`](./RFC-0002-semantic-index-query-v0.md) | accepted | Semantic Index、五类查询、事实依据、完整性和预算 |
| [`RFC-0003`](./RFC-0003-numeric-representation-v0.md) | proposed | `Int`、Decimal、资源限额和 WIT 数值记录 |
| [`RFC-0004`](./RFC-0004-resource-async-mapping-v0.md) | accepted | affine resource、结构化 Task 与 WASI 0.3 async/future/stream 映射 |
| [`RFC-0005`](./RFC-0005-labeled-block-syntax-baseline.md) | accepted | 选择 B labeled-block 作为 M1 parser/formatter 唯一语法基线 |
| [`RFC-0006`](./RFC-0006-lexical-source-contract-v0.md) | accepted | 固定 M1 UTF-8、Unicode identifier、newline、trivia、token、span 与输入限额 |
| [`RFC-0007`](./RFC-0007-m2-hir-name-prelude-contract-v0.md) | accepted | 固定 M2 deterministic HIR/source maps 与 P0-only name/prelude contract |
| [`RFC-0008`](./RFC-0008-typed-sico-ir-contract-v0.md) | accepted | 固定 typed Sico IR、canonical JSON、source maps、限额与独立 verifier contract |
| [`RFC-0009`](./RFC-0009-core-lowering-evaluation-order-v0.md) | accepted | 固定 core lowering 支持边界与 left-to-right single-evaluation/error order |
| [`RFC-0010`](./RFC-0010-effect-resource-revision-ir-flow-v0.md) | accepted | 固定 effect/capability、affine resource 与 revision guard 的显式 IR/verifier flow |
| [`RFC-0011`](./RFC-0011-deterministic-core-wasm-backend-v0.md) | accepted | 固定 verified IR 到 deterministic Core Wasm 的 scalar/control 子集与 representation refusal |
| [`RFC-0012`](./RFC-0012-component-wit-boundary-v0.md) | accepted | 固定 compiler Component lift 与 WIT Result/record/resource 结构边界 |
| [`RFC-0013`](./RFC-0013-async-backend-support-v0.md) | accepted | 固定 Future/Stream Runtime 证据与 compiler Task/Future/Stream typed refusal 边界 |
| [`RFC-0014`](./RFC-0014-minimal-build-run-cli-v0.md) | accepted | 固定 M3 raw Component build/run、入口、Runtime 查找、退出码与无产物失败契约 |
| [`RFC-0015`](./RFC-0015-sapp-package-format-v0.md) | accepted | 固定 `.sapp` v0 framing、canonical manifest、hash/signature domain、路径与 parser limits |
| [`RFC-0016`](./RFC-0016-development-signing-trust-v0.md) | accepted | 固定 Ed25519 development signature、strict verification 与显式 local trust policy |
| [`RFC-0017`](./RFC-0017-capability-closure-permission-v0.md) | accepted | 固定 source/manifest/import closure、host grant intersection 与 unknown default-deny |
| [`RFC-0018`](./RFC-0018-runtime-limits-fault-taxonomy-v0.md) | accepted | 固定 manifest/host effective limits、七类 fault 与 malicious guest 后 host survival |
| [`RFC-0019`](./RFC-0019-package-cli-cache-stdio-v0.md) | accepted | 固定 `.sapp` build/run/inspect、source cache、args/stdio 与 exit contract |
| [`RFC-0020`](./RFC-0020-desktop-ui-permission-contract-v0.md) | accepted | 固定 Desktop permission record 与 typed bounded UI model |
| [`RFC-0021`](./RFC-0021-android-intent-jni-contract-v0.md) | accepted | 固定 Android Intent/URI 与 typed bounded JNI bridge contract |
| [`RFC-0022`](./RFC-0022-ecosystem-compatibility-contract-v0.md) | accepted | 固定 production release identity、canonical metadata、依赖锁与独立兼容面 |
| [`RFC-0023`](./RFC-0023-production-publisher-policy-v0.md) | accepted | 固定 production publisher policy、角色阈值、精确身份与 rotation/revocation/recovery |
| [`RFC-0024`](./RFC-0024-signed-registry-metadata-v0.md) | accepted | 固定 signed namespace/release/channel/checkpoint 与不可信 transport 复验 |
| [`RFC-0025`](./RFC-0025-secure-update-recovery-v0.md) | accepted | 固定 consistent update、monotonic state、advisory 与显式 recovery |
| [`RFC-0026`](./RFC-0026-dependency-lock-compatibility-v0.md) | accepted | 固定 source-aware dependency、canonical lock、compatibility 与 capability closure |
| [`RFC-0027`](./RFC-0027-language-server-editor-protocol-v0.md) | accepted | 固定 bounded stdio LSP、UTF-16、compiler-owned editor features 与 run/debug boundary |
| [`RFC-0028`](./RFC-0028-ai-tooling-inspect-fix-v0.md) | accepted | 固定 compiler-backed inspect/fix、source/diagnostic binding 与 offline/live evidence boundary |
| [`RFC-0029`](./RFC-0029-script-profile-v0.md) | proposed | 定义 bounded batch Script entry、args/stdin/stdout/stderr、WIT/adapter、缓存、权限与兼容边界 |
| [`RFC-0030`](./RFC-0030-script-streaming-v0.md) | proposed | 定义 M9 streaming channel：versioned input/output-stream resources、ownership、EOF/backpressure/close/cancellation 语义 |
| [`RFC-0031`](./RFC-0031-scoped-http-v0.md) | proposed | 定义 scoped HTTP provider：精确端点授权、无 redirect、body/header 上限、无凭证与默认无网络 |
| [`RFC-0032`](./RFC-0032-bounded-repl-session-v0.md) | proposed | 定义 bounded expression REPL：deterministic cell ID、replay/reset/export、cell/history 上限与无 top-level syntax 预判 |
| [`RFC-0033`](./RFC-0033-top-level-script-syntax-decision.md) | accepted | 保留 explicit typed `main`，拒绝 unrestricted top-level statements 与 `script:` block，并以 E1013 fail closed |
| [`RFC-0034`](./RFC-0034-tooling-execution-plan-v0.md) | accepted | 统一 LSP/AI direct-argv execution plan、1 MiB logs、client cancellation 与 compile-only source-map/debug 边界 |
| [`RFC-0035`](./RFC-0035-runtime-observability-debug-v0.md) | accepted | 冻结 digest-bound debug identity/map、Runtime fault/event、typed cancellation race 与机器 DAP claimed subset |
| [`RFC-0036`](./RFC-0036-structured-concurrency-semantic-ir-v0.md) | accepted | 冻结 task scope 静态身份、eager-start spawn、affine 消费规则、E5003/E5103–E5105 与 IR task-scope 表/操作 |
| [`RFC-0037`](./RFC-0037-secure-http-provider-v0.md) | accepted | 冻结 secure HTTP provider authority/TLS/framing/redirect/secret 合同与 `sico:script/http@0.2.0` 结构化错误分类 |
| [`RFC-0038`](./RFC-0038-application-profile-v0.md) | accepted | 冻结 M14 应用 profile 支持矩阵（控制流/集合/位运算/checked 算术/源级 PRNG/http emission）与出口语料 |
| [`RFC-0039`](./RFC-0039-source-modules-package-resolution-user-wit-v0.md) | accepted（部分实现） | 显式源码模块与 `use` 导入、M7 lock 包解析、user WIT 导入 v0 值域与 typed 拒绝类；模块切片已随 STEP-0143 落地，包解析/user WIT 为后续切片 |

| [`RFC-0040`](./RFC-0040-native-automation-capabilities-v0.md) | accepted | Native Automation Host 能力面 v0（observe/capture/pointer/keyboard/audit/stop 六接口、preview/commit 令牌、审计流与紧急停止的 guest 不可见设计、E9xxx 拒绝语料规划）；2026-09-10 owner 接受（指令“完成M15-17”），威胁模型 F-1 判为声明式敏感区策略、F-2 判为 Host 本地按运行追加 JSONL |
| [`RFC-0041`](./RFC-0041-image-data-contract-v0.md) | accepted | 可移植图像数据合同 v0（bitmap/region、步距/格式/方向、四项决策 D1–D4、构造 limit+1 错误分类与出口语料）；2026-09-10 owner 接受（D1–D4 如案通过，D4 初始上限 2^28 字节） |
| [`RFC-0042`](./RFC-0042-web-ui-controls-v0.md) | accepted | Web UI 控件与事件合同 v0：封闭控件集+稳定 id、确定性 stack/flow/grid 布局、类型化 FIFO 事件、显式属性赋值、ARIA/焦点为合同字段、敌意内容语料规则；2026-09-10 owner 接受 |
| [`RFC-0043`](./RFC-0043-deterministic-vision-packages-v0.md) | draft | 确定性视觉包（template match/grid/contour）WIT 面、确定性参考实现与 limit+1 语料；M17/M24 roster 的合同基础 |
| [`RFC-0044`](./RFC-0044-language-v1-batch1.md) | accepted | 语言 v1 batch 1：中缀 `==` lowering、保留字与冻结快照 append-only；2026-09-13 owner 会话指令接受 |
| [`RFC-0045`](./RFC-0045-stdlib-batch2-byte-text-collections-v0.md) | accepted | stdlib batch 2：byte/text 访问、排序/格式化与集合扩展；2026-09-14 随 STEP-0174 落地 |
| [`RFC-0046`](./RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md) | accepted | 语言 v1 batch 2：for-loops、`expr?` 错误传播、裸字面量决策记录（不改语法）、`List[I64]`/`[U64]`；2026-09-14 owner 会话指令接受 |
| [`RFC-0047`](./RFC-0047-record-types-v0.md) | accepted | record 类型最小封闭集（声明/具名字面量/字段访问/`List[record]`）：M22 R0 产出（ADR-0017 Option A），服务自举 re-baseline、M14 应用 profile 与 M23 人机工学；2026-09-24 owner 会话指令接受，冻结 gate 见 STEP-0270 |

下一可用编号：`RFC-0048`。

创建时使用 [`RFC template`](../templates/RFC.md)，并在本页登记状态和替代关系。
