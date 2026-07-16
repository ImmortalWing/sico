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

下一可用编号：`RFC-0026`。

创建时使用 [`RFC template`](../templates/RFC.md)，并在本页登记状态和替代关系。
