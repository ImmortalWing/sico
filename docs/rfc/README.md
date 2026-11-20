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
| [`RFC-0048`](./RFC-0048-language-v1-batch3-infix-comparison-logical-and.md) | accepted（owner 指令 2026-09-30，STEP-0323） | 语言 v1 batch 3 item 1：中缀比较 `< <= > >= !=` 与 `&&`（desugar 到既有 EqualFixed/LessFixed，无新 IR 操作）；`||`/`!` 因零实测消费被排除；登记 `<=` 无消费记号的 check 绿/build 拒实测；实现已落地（STEP-0328，M23 并行轨），M23 计划 §2 双门适用于其后继的 self-host re-baseline 决策 |
| [`RFC-0049`](./RFC-0049-language-v1-batch3-let-bound-if-match.md) | accepted（owner 指令 2026-09-30，STEP-0323） | 语言 v1 batch 3 item 2：`let x = if/else` 与 `let x = match` 两种 let 绑定块（单表达式臂；desugar 到 D2 已落地的 cell+分支写+汇合读机制，零新 IR 操作）；E2026/E2027 新码；实测 248+121 个 bind 型 match 模拟位点；实现已落地（STEP-0329，M23 并行轨），M23 计划 §2 双门适用于其后继的 self-host re-baseline 决策 |
| [`RFC-0050`](./RFC-0050-image-codec-jpeg-png-v0.md) | draft（owner acceptance required） | M24 image-codec@1 包：JPEG↔PNG 四纯函数（BGRA8 RFC-0041 契约），接受 profile/编码参数按名冻结（PNG Paeth+zlib9 无附属块；JPEG SOF0 quality90 4:4:4），字节级稳定；§8.5 拒绝面 typed 收口；消费走 M7 零 compiler/Runtime 变更 |
| [`RFC-0051`](./RFC-0051-model-provenance-manifest-v0.md) | **accepted（owner 指令 2026-09-30「完整完成M22-M26」，STEP-0323）** | M17 gate 4 模型/包 provenance manifest v0（`sico.m17.provenance.v0`）：封闭 manifest+SHA-256 复用 M7 单一信任路径+预算字段 typed 强制；fixture 模型证据集五项；不产生任何 live-model 主张 |

| [`RFC-0051-A1`](./RFC-0051-amendment-A1-carriage-and-budget-mapping.md) | 见文件 | provenance 修正案 A1：预算装载与 A1.3 映射 |
| [`RFC-0051-A2`](./RFC-0051-amendment-A2-hostcall-budget-and-script-memory-floor.md) | **accepted（owner 2026-11-19「授权都按你推荐的来」；实现 STEP-0638 已落地）** | provenance 修正案 A2：C1=(a) `max_hostcalls` 重释为 per-hostcall fuel cost（A1.3 映射逐字节保留、不新增第二条上限路径）+ C2=(a) Script 类包 install 期强制 `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES`（≈2.06 GiB）否则 distinct typed `Manifest` 拒绝（镜像 STEP-0497、增强 default-deny）+ I3 运行期限额接线（`effective_memory_limits` 按 `provenance_budgets()` 收窄）+ 冻结样例/3 测试重基线；关闭 gate 2 的 C1/C2/I3 残余 |
| [`RFC-0052`](./RFC-0052-native-ui-source-binding-v0.md) | draft（owner acceptance required；配对 ADR-0018 proposed） | Sico 语言原生 UI 库源侧绑定：`sico:user/ui@1` 版本化包（window/column/row/text/button/dropdown/image_preview/progress + poll_event/run），零语法增长；对话框组合既有 file 能力；封闭 UiError + 节点/深度 limit+1 typed；帧=UI 状态纯函数 |
| [`RFC-0053`](./RFC-0053-pdf-structural-reading-v0.md) | draft（owner acceptance required；实现另等 M25 v1.0 release GO） | M26 PDF 结构阅读子集 v0：`sico:user/pdf@1` open() 结构阅读（classic xref table/trailer/catalog/page tree/MediaBox//Rotate 继承），stream 有 /Filter 即 typed 拒绝，加密拒绝；xref stream/object stream/incremental update 各自扩展语料另列 |
| [`RFC-0054`](./RFC-0054-pdf-limits-refusals-writer-v0.md) | draft（owner acceptance required；实现另等 M25 v1.0 release GO） | M26 PDF 限额/拒绝分类 v0：八项限额先检后分配（+1 typed）、封闭 PdfError 十四码带字节 span、字节级稳定 writer 契约（classic xref、规范重编号、幂等探测） |
| [`RFC-0055`](./RFC-0055-native-ui-file-open-save-picker-v0.md) | accepted（owner 2026-11-04；2026-11-18 ported to `dev` from `codex/m24-vision-ui-v2` `6274ae6` under owner porting authorization；实现另立 STEP-0627，证据另登记）+ [`A1`](./RFC-0055-amendment-A1-root-relative-picker-path.md) **accepted（owner 2026-11-06，contract-only，明确未实现）** | M24 §8.6.1 源侧 UI 文件打开/保存路径选择 v0：新增 host-provider `sico:script/path-picker` 面（`open`/`save` → `result<option<string>, picker-error>`，`filters` 为**扁平 `list<string>`** `"name|pattern"`/条），**只选路径不做 IO**，读写权限仍归既有 `fs-read`/`fs-write`（默认拒绝、两能力独立分解）；封闭 `picker-error` 四码、filters limit+1 typed、零语法增长；A1 改为**根相对路径**返回 + 第 5 个 typed case `outside-grant`（合同，未实现）；由 STEP-0555 收尾 §8.4 对话框残差时量出「既有 file 能力无用户选路径原语」这一缺口后提出 |

| [`RFC-0056`](./RFC-0056-language-v1-batch4-enum-payload-match-option.md) | **accepted（owner 2026-11-19）** | 语言 v1 batch 4：已声明 payload enum 的单层 payload 绑定 match 臂泛化（`Named` enum + `Option` subject、多绑定、`Module.enums` 声明表 + project `"<case>"/"<case>.<index>"` 约定）+ `Option[T]`/`some`/`none` + 宿主面 enum 约定（程序按钉死形状声明 `ui-event`，不符 `E2028`）；依 STEP-0629 八探针实测（p3/p4 lower 哨兵、p1/p2 non-scalar 边界拒、p7 瞬态形状已通）；使 `sico.ui.poll_event` 源签名回 `Option[ui-event]` 并回收 STEP-0628 Text 面；嵌套绑定/函数边界 first-class/RFC-0039 值集为 v0 非目标 |
| [`RFC-0057`](./RFC-0057-language-v1-batch5-bytes-to-u64-and-i64-cast.md) | **withdrawn（owner 2026-11-20 裁决「3」：回退）** | 语言 v1 batch 5：`sico.bytes.le_u64` + `sico.conv.u64_from_i64`（原为解锁 M24 G6 转换器 PNG 重编码）。**回退理由**：唯一消费者 GUI 转换器已随 gate 5 重定划为**外部项目**（`案例项目/gui-converter/` PRJ-0001）、不再需要该 intrinsic；而其 Rust-oracle 改动一度被误判为 `selfhost_compiler` 56 红根因。**实测更正**：回退后那批红依旧 ⇒ 真根因是已提交的 M24 checkpoint `94ea1fa` 造成的 **guest↔oracle 自举分叉**（guest `selfhost/codegen.sico` 仍冻结 `ScriptErrorCode` enums 发射逻辑、oracle 已停发），属 **M22 自举轨道**、非 M24 缺口。实现已回退（5 个 oracle crate 复原 HEAD、RFC-0057 测试文件删除）；gate 5 GO 证据（UI 机制 G1–G6）不依赖本 RFC |
| [`RFC-0058`](./RFC-0058-m22-split-scalar-ir-component-axis-v0.md) | proposed（owner 2026-11-20 授权立项；**2026-11-20 规模侦察订正机制**，实现另立 STEP，未开工） | M22 option-C：把 `selfhost/parser.sico:scalar_ir` 体内 **25 个 `match U64.checked_add` 站点物理迁移**进小包裹 helper（改其包围函数），使每站点包围函数闭包脱离 1 MiB closure fixture bound ⇒ 25 站（MS-375..399）组件轴从 `closure-too-large` 转可逐站点实测。**侦察否决原「抽内联逻辑成顶层函数以缩小 `scalar_ir` 闭包」机制**（传递闭包守恒 ⇒ 闭包只增不减；且全模块只有 `scalar_ir` 闭包超界、燃料 311 站全在另外 51 个函数与拆 `scalar_ir` 脱钩）。**不抬任何限额、不收窄语义、不改 `A==B==C` 合同/Rust oracle/出口门判据**；逐字节恒等守卫（whole-file IR pin + 全部 `*_byte_exactly` 组件 pin + formatter 28,459 B）为验收硬条件；备选「紧凑 IR 传输」「上调限额」「按机器族抽函数」均否决 |

下一可用编号：`RFC-0059`。

创建时使用 [`RFC template`](../templates/RFC.md)，并在本页登记状态和替代关系。
