# Reports

本目录保存可复现的实验、AI 评测、性能、安全、兼容性和 Runtime 对比报告。

现有静态语法指标位于 [`syntax-candidates/METRICS.md`](../../syntax-candidates/METRICS.md)。后续新报告使用 [`report template`](../templates/REPORT.md)。

| Report | Status | Related step | 内容 |
|---|---|---|---|
| [`script-profile-contract-v0`](./script-profile-contract-v0.md) | complete-contract / architecture-gate-complete | STEP-0075 | M8/M9 scope、Script WIT/exit/limit/capability/cache decisions and STEP-0076 gate |
| [`script-profile-prototype-v0`](./script-profile-prototype-v0.md) | verified direct-path evidence | STEP-0076 | hard-coded Program 6/6、Canonical ABI、cold/warm/RSS 与 encoding findings |
| [`script-profile-composition-v0`](./script-profile-composition-v0.md) | verified composition evidence | STEP-0076 | direct/composed 各 20 × 6/6、Adapter digest、latency/RSS 与 `composition-go` 裁决 |
| [`fixed-width-dynamic-scalars-v0`](./fixed-width-dynamic-scalars-v0.md) | verified implementation evidence | STEP-0077 | I64/U64 typed failure、2,048 × 8 Core oracle、11 个 Wasmtime Component 边界用例 |
| [`scoped-http-provider-v0`](./scoped-http-provider-v0.md) | verified implementation evidence | STEP-0089 | 精确端点 grant、GET/POST loopback、默认拒绝、redirect/limit/timeout/cancel 与 `network.connect` closure |
| [`persistent-runner-watch-v0`](./persistent-runner-watch-v0.md) | verified implementation evidence | STEP-0090 | Engine/Component/Linker reuse、fresh Store、watch coalescing/failure/trap isolation 与 5.8–6.1 ms warm median |
| [`bounded-repl-session-v0`](./bounded-repl-session-v0.md) | verified implementation evidence | STEP-0091 | deterministic cells、rollback/replay/reset/export、4 KiB/256/16 KiB bounds 与 7.8 MiB sampled peak RSS |
| [`top-level-script-syntax-decision-v0`](./top-level-script-syntax-decision-v0.md) | verified decision and implementation evidence | STEP-0092 | explicit main accepted；B/C rejected；E1013 closes silent AST/HIR drop across parser/formatter/LSP/AI |
| [`tooling-execution-plan-v0`](./tooling-execution-plan-v0.md) | verified implementation evidence | STEP-0093 | shared LSP/AI direct argv、1 MiB logs、client tree cancellation 与 honest source-map/debug bounds |
| [`ecosystem-release-contract-v0`](./ecosystem-release-contract-v0.md) | complete | STEP-0062 | 32-threat ecosystem trust boundary, 12 compatibility surfaces and split-track evidence |
| [`m6-exit-audit`](./m6-exit-audit.md) | blocked-external-runner | STEP-0061 | host evidence complete; Android Runtime/UI/startup gate NO-GO |
| [`desktop-android-parity-v0`](./desktop-android-parity-v0.md) | partial-runtime-evidence | STEP-0060 | same package Desktop result and Mobile Host metadata parity |
| [`android-native-ui-adapter-v0`](./android-native-ui-adapter-v0.md) | accepted-cross-check | STEP-0059 | native widgets, IME and accessibility mapping |
| [`android-runtime-lifecycle-v0`](./android-runtime-lifecycle-v0.md) | accepted-cross-check | STEP-0058 | Wasmtime Android compile evidence and process-death lifecycle contract |
| [`android-permission-storage-v0`](./android-permission-storage-v0.md) | accepted | STEP-0057 | exact capability to Android permission/storage mapping |
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
| [`publisher-identity-key-lifecycle-v0`](./publisher-identity-key-lifecycle-v0.md) | complete | STEP-0063 | canonical publisher policy、五角色阈值、2,048 mutation 与 rotation/revocation/recovery |
| [`signed-local-registry-v0`](./signed-local-registry-v0.md) | complete | STEP-0064 | signed namespace/release/channel/checkpoint、1,231 mutation 与严格下载复验 |
| [`secure-update-recovery-v0`](./secure-update-recovery-v0.md) | complete | STEP-0065 | monotonic journal、staged activation、1,592 mutation、advisory 与 recovery |
| [`dependency-standard-library-v0`](./dependency-standard-library-v0.md) | complete | STEP-0066 | source-aware resolution、canonical lock、compatibility 与 capability closure |
| [`language-server-editor-workflow-v0`](./language-server-editor-workflow-v0.md) | complete | STEP-0067 | bounded stdio LSP、UTF-16、compiler diagnostics/index、format 与 run/debug boundary |
| [`ai-tooling-measured-evaluation-v0`](./ai-tooling-measured-evaluation-v0.md) | complete-offline | STEP-0068 | compiler-backed inspect/fix、54-source/12-fix 测量与 live-model authorization gate |
| [`m7-pilot-release-drill`](./m7-pilot-release-drill.md) | complete-local | STEP-0069 | 两版洁净室 Component/registry/Host/Wasmtime 发布升级与四类安全拒绝 |
| [`production-deployment-origin-v0`](./production-deployment-origin-v0.md) | complete-local | STEP-0074 | 只读 registry origin、operator bundle、release 集成与单命令源码 Runtime `42` |
| [`m7-exit-audit`](./m7-exit-audit.md) | blocked-external-evidence | STEP-0069 | M7 本地轨完成、产品 NO-GO 与七项外部 gate |
| [`project-completion-audit`](./project-completion-audit.md) | blocked-external-evidence | STEP-0069 | 仓库本地闭环和支持产品未完成边界 |
| [`m14-m26-milestone-audit-2026-09-24`](./m14-m26-milestone-audit-2026-09-24.md) | planning audit | STEP-0279 | M14–M26 gate register、M22/23 双路径、M24 并行与 M25 双结论；不提升 runtime 证据 |
| [`m22-w2-baseline-2026-09-25`](./m22-w2-baseline-2026-09-25.json) | frozen measurement baseline | STEP-0282 | W2 records-surface 基线（schema `sico.m22.w2.baseline.v0`）：canary 22/30 + `nearest_match`/`GWPACK-OTHER` 前沿、四档燃料区间复验一致、8 源 16,390 行 SHA256 盘点、栈高水位未测得；纯测量不计 R3 |
| [`m23-ceremony-census-2026-09-25`](./m23-ceremony-census-2026-09-25.json) | frozen measurement census | STEP-0284 | M23 开工仪式普查（schema `sico.m23.ceremony.v0`，工具两次运行字节一致）：selfhost 比较谓词 978 行、嵌套比较 if≥2 位点 79、checked 算术 match 263/bind 型 248、literal 构造器 2,169、chars 调用 153；AI 语料与运行时计数为登记缺口；纯测量，不作语法决定 |
| [`m24-inventory-2026-09-25`](./m24-inventory-2026-09-25.json) | frozen kickoff inventory | STEP-0287 | M24 五条线盘点（schema `sico.m24.inventory.v0`）：加速主机未命名（gate 2 延期）、模型资产 0、roster 三包缺、原生 UI 库缺、codec 依赖 0；四份前置合同待起草；纯盘点不改 gate |
| [`m26-inventory-2026-09-25`](./m26-inventory-2026-09-25.json) | frozen kickoff inventory | STEP-0290 | M26 五线盘点（schema `sico.m26.inventory.v0`）：字节面适配 v0、oracle 缺失（pikepdf 候选 owner 门控）、inflate 不可测、UI 滚动缺口、classic-xref 子集钉定；纯盘点不改 gate |
| [`m3-exit-audit`](./m3-exit-audit.md) | complete | STEP-0037 | M3 determinism/property/limits/performance、全阶段 regression 与 GO |
| [`sapp-builder-loader-v0`](./sapp-builder-loader-v0.md) | complete | STEP-0039 | canonical `.sapp` builder、strict loader、hash/path/limit verification |
| [`package-cli-cache-v0`](./package-cli-cache-v0.md) | accepted | STEP-0044 | `.sapp` build/run/inspect、explicit trust、source cache 与 args/stdio |
| [`m4-exit-audit`](./m4-exit-audit.md) | complete | STEP-0045 | M4 security/property/performance、M0–M3 regression 与 M5 GO |
| [`m9-exit-audit-v0`](./m9-exit-audit-v0.md) | accepted / GO | STEP-0094 | M8/M9 aggregate regression、security/performance evidence、Windows-only Runtime matrix 与 M10 entry |
| [`runtime-fault-source-frames-v0`](./runtime-fault-source-frames-v0.md) | accepted | STEP-0097 | typed Runtime fault classes、exact verified source frames、bounded CLI 与 fail-closed sidecars |
| [`typed-cancellation-bridge-v0`](./typed-cancellation-bridge-v0.md) | accepted / Windows native | STEP-0098 | real console control、canonical client requests、single terminal winner 与 blocked Host cancellation |
| [`m10-exit-audit-v0`](./m10-exit-audit-v0.md) | accepted / GO | STEP-0102 | M0–M9 aggregate regression, M10 security/performance audit, Windows x64 GNU matrix and external-gate recheck |
| [`m22-exit-audit-2026-09-28`](./m22-exit-audit-2026-09-28.md) | reviewed / NO-GO | STEP-0310 | exact-SHA CI 裁决与 M22 计划 §4 七门复审；S3/S4/S5 partial、S6 未进入；0306–0309 独立 CI 待执行 |
| [`m22-exit-audit-2026-10-20`](./m22-exit-audit-2026-10-20.md) | reviewed / NO-GO | STEP-0485 | 七门复审（门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO）；门 6 当时**未在提交快照上复跑** ⇒ 已被 2026-10-25 报告取代（原文保留 + addendum） |
| [`m22-exit-audit-2026-10-25`](./m22-exit-audit-2026-10-25.md) | reviewed / **NO-GO（门 6 GO）** | STEP-0543 | 门 6 由 NO-GO 升 **GO**（锚点 = 独立 CI **run #52** = `9f0b7a3`，completed/success，57m34s ⇒ 干净提交 SHA 上的完整 M0–M21 回归）；门 1/5 GO、门 2 PARTIAL、门 3 NO-GO、**门 4 与门 7 按 `AGENTS.md` §10.1 不得沿用 GO**；#17/#18/#39/#44/#46 **五次失败如实在册** |
| [`ci-arena-ceiling-attribution-2026-10-21`](./ci-arena-ceiling-attribution-2026-10-21.md) | measured / attributed to M22 | STEP-0489, STEP-0490 | 两次独立 CI 同点红的确定性归因（`MEMORY_PAGES` 33792 页 vs 宿主上限 32768 页）、仓库固定 wasmtime 本机逐字节复现与 5 组反证、三条候选修法 |
| [`ai-eval-item3-live-model-2026-10-21`](./ai-eval-item3-live-model-2026-10-21.md) | measured (live model) / pending owner review | STEP-0490 | M23 item 3 AI 语料腿：100 次计费调用（USD 0.0272）、14 份不同程序、0 仪式调用及其表面语法成因、`Int`/`I64` 类型事实；D3 未决 |
| [`m23-exit-audit-2026-10-21`](./m23-exit-audit-2026-10-21.md) | complete-audit / **GO** | STEP-0500 | M23 出口审计复审：四项逐项状态 + 计划 §3 七门逐门复核 + 三项残余闭合状态（#1 STEP-0498、#3 STEP-0489、#2 待 CI 裁决） |
| [`m22-support-surface-t1-text-concat`](./m22-support-surface-t1-text-concat.md) | registered / family **closed**（STEP-0527 收口片 X） | STEP-0521, STEP-0522, STEP-0523, STEP-0524, STEP-0525, STEP-0526, **STEP-0527** | M22 W2 族 T1（`sico.text.concat`）**D15 支持面登记表**：子集边界六项（名字 / 实参个数 / 实参类型 / 结果槽数 / **语料覆盖仍 0 条** / 未覆盖形状 **0 类缺口**）+ **族级真值表 38 行**（四轴实测，固件钉住 36 行 + 2 行；**逐字节等同 35 行**、前端 typed 拒绝 1 行（BE）、**组件层 typed 拒绝 0 行**）+ **缺口登记全部闭合**（G-b 转义字面量 ⇒ STEP-0522 已闭合；G-e 空字面量 ⇒ STEP-0522 已闭合；**G-d1** 单语句 return 的非形参操作数 ⇒ STEP-0523 已闭合；**G-f** 调用型操作数带多个实参 ⇒ STEP-0524 已闭合；**G-f2** 实参本身是嵌套调用 ⇒ STEP-0525 已闭合；**G-d2** `if` 块里的 concat（发射端分支支持）⇒ **STEP-0526 已闭合**；**G-d3** 发射端缺 `equal_fixed` 算子 ⇒ **STEP-0526 已闭合**；G-a 模块级 data 段偏移 ⇒ STEP-0521 已修；G-c 重复字面量 ⇒ STEP-0521 已修）+ S 内调用点普查（文本计数 2,943）、**G-f 面普查 117 个调用点**与那 10 个调用点的逐个定位（**10 / 10 闭合**）+ **族级出口判据六项全过**（STEP-0527 收口核对）+ **诚实收窄段**（**族闭合 ≠ M22 闭合**）|
| [`m22-support-surface-t2-text-length`](./m22-support-surface-t2-text-length.md) | registered / family **not** closed（**STEP-0533 收口片 X 裁决 = NO-GO**；**独立 CI 绿已达成**：STEP-0532 的 run #40 = success） | **STEP-0528**, **STEP-0529**, **STEP-0530**, **STEP-0531**, **STEP-0532**, **STEP-0533** | M22 W2 族 T2（`sico.text.length`）**D15 支持面登记表**：子集边界六项（名字 / 实参个数 = 恰好 1（**语义层**拒 `stdlib intrinsic arity`）/ 实参类型 = 一个 `Text`（2 槽，**guest 前端查类型**，不符 ⇒ `ERR:E-SH-IR-CALL-TYPE`；oracle 侧 **语义层** `E2001` / `TYPE_MISMATCH`）/ 结果槽数 = 1（`U64`）/ **语料覆盖 0 条** / 未覆盖形状）+ **族级真值表**（四轴实测，固件钉住：**前端逐字节等同 10 行**、**组件层逐字节等同 12 行**（STEP-0530 的 10 + STEP-0532 的 2）、**发射端拒 3 行**（STEP-0532；内层算子 `sico.bytes.utf8_decode` = **T11 族，不进本族账**）、**元数 2 行语义层拒绝**、**类型探针 1 行前端 typed 拒绝**）+ **缺口登记 T2-g1 已闭合**（STEP-0529：纯接线，5 处接线 + 2 个新函数，不新造机器）/ **T2-c1 已闭合**（STEP-0530：纯接线，`cgs_intrinsic_hex` 新分支 + helper body + 类型 / 函数 / 代码三段接线 + **名字排序索引** `模块函数数 + 3 + 名字排序位`）/ **T2-g2 形状面已闭合**（STEP-0532：新函数 `nested_text_call_packed` + 两处纯插入接线，**前端 7/7 形状逐字节等同** 2477 / 2656 / 2369 / 2783 / 2374 / 2656 / 2369 B）⇒ **但两个真实 S 调用点的宿主函数仍不降**（归因 = **T7 族** / **匹配路由**，**都不在本族**）⇒ **形状面闭合 ≠ 族闭合**（**D10** ⇒ **族未闭合**）；**拒绝分层记 + 对照形状证伪**（`while` 循环形状、`U64.bit_or` / `U64.shr` **都与 T2 无关**）；**STEP-0533 收口片 X（验证片，零生产代码改动）**：**口径漂移订正**（**322** → **332**，漂移 +10 全部来自本族自己的实现刀 ⇒ **D10 判定必须钉到一棵树**）+ **族级覆盖细化为 8 个单元格**（前端 **325 / 332** 逐字节等同）+ **新缺口 T2-g3**（length 调用作为**另一个算子的操作数**，**7 个站点**，两个拒绝身份，对照形状证伪）+ **跨族实测**（**`match` 519** / **`U64.less_than` 990** / 算子绑 `let` 501）⇒ **收口裁决 = NO-GO** |
| [`m22-support-surface-match`](./m22-support-surface-match.md) | registered / family **not** closed（**STEP-0534 拍 1 已落地**：前端 **57 / 519（11%）**；**STEP-0535 组件层发射刀已落地**：**4 个合成形状**端到端逐字节等同；**R0.4 逐站复核**：42 个候选真实站点中组件级全等 **8 / 42**） | **STEP-0526**, **STEP-0527**, **STEP-0533**, **STEP-0534**, **STEP-0535** | M22 W2 **`match` 构造族**（**语言构造**，非内建算子族）**D15 支持面登记表**：子集边界六项（**两个 case**（`ok` + `error`）/ 匹配头 = `U64.checked_add` 与 `sico.list.get`（含泛型形式）/ 宿主形参 = `checked_add` 头 **1 或 2 个**、**`list.get` 头无限制** / case 体 = `set`+`return` 或 `return`+`return`（**嵌套 `match` 也可**）/ **语料覆盖 0 条** / **5 类未覆盖形状**）+ **族级真值表**（12 行：前端逐字节等同 **6** 行 / 前端 typed 拒 **6** 行 / **组件层 typed 拒 12 行**）+ **缺口登记 M-g1**（形参计数门，**354**）/ **M-g2**（算子白名单，**46**）/ **M-g3**（无路由认领该头，**21**）/ **M-g4**（组件层零支持，**519**）+ **S 内普查 519 条**（匹配头算子分布 / 宿主形参分布 / case 体分布）+ **诚实收窄**（族未闭合 ⇒ **D10 不得降级**）|
