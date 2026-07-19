# Sico audited roadmap

> - updated: 2026-07-19
> - source of phase definitions: [`DEVELOPMENT.md`](../DEVELOPMENT.md)
> - current phase: M10
> - phase context: Runtime observability/debugging planned; M11 structured-concurrency design unlocks after STEP-0098; M12 secure HTTP follows M11 GO; M7 public rollout and M6 mobile remain externally blocked

## Status vocabulary

| 状态 | 含义 |
|---|---|
| `complete` | 交付物和退出证据齐全 |
| `in-progress` | 已开始且存在未完成工作 |
| `planned` | 已定义但尚未开始 |
| `blocked` | 存在明确外部阻塞 |
| `superseded` | 已被新步骤或决定替代 |

## Cross-cutting repository architecture

| Work package | 状态 | 证据/下一步 |
|---|---|---|
| v0.0.1 开发历史归档 | complete | `codex/archive-v0.0.1-development-history` 已同步两个远端 |
| OpenJDK-style modular monorepo | complete | [`ADR-0007`](./adr/ADR-0007-openjdk-style-modular-monorepo.md)、[`STEP-0073`](./steps/STEP-0073-openjdk-style-modular-monorepo.md) |
| 语言/应用/Host 命令所有权 | complete | `sico`、`sico-app`、`sico-desktop-host`；机器边界 validator |

## M0: 设计与技术基线

状态：`complete`

| Work package | 状态 | 证据/下一步 |
|---|---|---|
| 方向、非目标与技术路线 | complete | [`DIRECTION.md`](../DIRECTION.md) |
| 10 个代表性程序 | complete | [`examples/PLAN.md`](../examples/PLAN.md) |
| 跨样本问题与语义审计 | complete | [`examples/ISSUES.md`](../examples/ISSUES.md)、[`SEMANTICS-AUDIT.md`](../examples/SEMANTICS-AUDIT.md) |
| 核心语义草案 | complete as draft | [`SEMANTICS.md`](../SEMANTICS.md)；正式稳定仍需 RFC/案例 |
| P0 正反例 | complete for 10 groups | [`semantic-cases/`](../semantic-cases/README.md)、[`STEP-0005`](./steps/STEP-0005-remaining-p0-semantic-cases.md) |
| 三套局部语法候选 | complete for 10 groups | [`SYNTAX.md`](../SYNTAX.md)、[`syntax-candidates/`](../syntax-candidates/README.md)；历史对照保留 |
| 静态语法体积指标 | complete | [`METRICS.md`](../syntax-candidates/METRICS.md) |
| 审计体系 | complete | [`STEP-0001`](./steps/STEP-0001-project-state-audit.md) |
| 应用宿主正式命名 | complete | [`ADR-0001`](./adr/ADR-0001-sico-host-terminology.md)、[`STEP-0002`](./steps/STEP-0002-sico-host-terminology.md) |
| 单点错误注入与恢复评测 | complete for design corpus v1 | [`STEP-0012`](./steps/STEP-0012-syntax-evidence-completion.md)、[`report`](./reports/syntax-evidence-v1.md)；36 个 mutation，parser 实测留待 M1 |
| AI 常见错误分类 | complete for designed corpus | [`error taxonomy`](../ai-eval/error-taxonomy.json)、[`STEP-0014`](./steps/STEP-0014-m0-exit-audit.md)；12 类，真实频率 not measured |
| AI 理解与修复基线 | complete for offline protocol v1 | [`STEP-0012`](./steps/STEP-0012-syntax-evidence-completion.md)、[`report`](./reports/syntax-evidence-v1.md)；96 个任务，真实模型数据等待凭据/成本授权 |
| 剩余 P0 语义案例 | complete | [`STEP-0005`](./steps/STEP-0005-remaining-p0-semantic-cases.md)、[`report`](./reports/remaining-p0-semantic-cases.md) |
| 诊断协议 v0 | complete for design contract | [`RFC-0001`](./rfc/RFC-0001-diagnostics-protocol-v0.md)、[`diagnostics/`](../diagnostics/README.md)、[`STEP-0006`](./steps/STEP-0006-diagnostics-protocol-v0.md) |
| 语义索引 JSON v0 | complete for design contract | [`RFC-0002`](./rfc/RFC-0002-semantic-index-query-v0.md)、[`semantic-index/`](../semantic-index/README.md)、[`STEP-0007`](./steps/STEP-0007-semantic-query-json-v0.md) |
| Wasm Component 最小原型 | complete | [`STEP-0010`](./steps/STEP-0010-component-runtime-host-call.md)、[`report`](./reports/component-runtime-host-call-v0.md) |
| Runtime 引擎桌面/Android 对比 | complete for M0 decision | [`ADR-0002`](./adr/ADR-0002-runtime-platform-baseline.md)、[`STEP-0011`](./steps/STEP-0011-runtime-desktop-android-feasibility.md)；Android 真机验证留待 M6 前置探针 |
| 数据驱动语法决定 | complete for M1 baseline | [`RFC-0005`](./rfc/RFC-0005-labeled-block-syntax-baseline.md)、[`STEP-0013`](./steps/STEP-0013-syntax-baseline-decision.md)；选择 B，真实 parser/model 按门槛复审 |
| M0 退出审计 | complete | [`STEP-0014`](./steps/STEP-0014-m0-exit-audit.md)、[`report`](./reports/m0-exit-audit.md)；GO to M1 |

### M0 exit gate

- 关键语义都有正反例、诊断根因和明确状态；
- 语法决定有静态、错误恢复和 AI 评测证据；
- 诊断与语义查询协议存在 v0；
- Component 最小链路在选定桌面 Runtime 真实运行；Android 风险、后端和后续实测门槛已有明确 ADR/报告；
- 所有关键开放项明确进入 RFC、ADR 或后续阶段；
- 编译器实现不会隐式替语言做决定。

## M1: 编译器前端与诊断

状态：`complete`

Entry gate：satisfied by STEP-0014。

主要交付：Rust workspace、源码/跨度、词法器、解析与恢复、无损树、语义 AST、格式化器、`sico check`、文本/JSON 诊断、outline、parser fuzzing。

进入条件：M0 exit gate 已由 [`STEP-0014`](./steps/STEP-0014-m0-exit-audit.md) 通过。

退出证据：合法语法案例稳定解析；非法案例产生预期主要诊断；单点错误不会形成不可控级联。

执行计划：[`M1 compiler frontend`](./plans/M1-compiler-frontend.md)，STEP-0015–0021。

当前证据：STEP-0015–0021 已完成；source/lexer/parser/recovery/E1xxx、canonical formatter、CLI、8,192 property inputs、limits 与 performance 全部通过。结论见 [`M1 exit audit`](./reports/m1-exit-audit.md)。

## M2: 静态语义

状态：`complete`

主要交付：名称解析、类型系统、局部推导、名义类型、Option/Result、完整匹配、不可变、效果/能力、资源基础规则和初步语义索引。

进入条件：M1 exit gate 通过。

退出证据：P0 非法案例全部在后端前拒绝；合法案例通过；诊断快照稳定。

执行计划：[`M2 static semantics`](./plans/M2-static-semantics.md)，STEP-0022–0029 全部完成。退出证据：25/25 valid、29/29 exact invalid、semantic CLI、compiler index/query、property/limits/performance；结论见 [`M2 exit audit`](./reports/m2-exit-audit.md)。

## M3: Sico IR 与 Component

状态：`complete`

主要交付：强类型 IR、验证器、lowering、Core Wasm、Component 封装、WIT 绑定和最小端到端 CLI 程序；真实 codegen/Runtime 链路已定义 raw Component `sico build` 与同步 scalar `sico run`。

进入条件：M2 exit gate 通过，M0 Component 原型的技术风险已有结论。

退出证据：2,048-source/1,000-function deterministic Component；Wasmtime Int/Bool/Unit execution；Result/record/resource host boundary；Future/Stream Runtime contract；property/limits/performance 与 M0–M2 regression。结论见 [`M3 exit audit`](./reports/m3-exit-audit.md)。

执行计划：[`M3 Sico IR and Component`](./plans/M3-sico-ir-component.md)，STEP-0030–0037 全部完成。

## M4: `.sapp` 与 Runtime

状态：`complete`

主要交付：包格式、manifest、资源、哈希、开发签名、加载验证、权限交集、隔离存储、资源限额和 `build/run/inspect`；固定源码直跑的编译缓存、参数透传、stdio 与退出码契约，并为后续 `sico -c` 和交互式 REPL 提供稳定 Runtime 接口。

进入条件：M3 exit gate 通过。

退出证据：canonical/strict `.sapp`、development trust、capability closure、isolated storage、11-dimension limits、host survival、package CLI/cache、3,328 security properties 与 M0–M3 regression 全部通过；结论见 [`M4 exit audit`](./reports/m4-exit-audit.md)。

执行计划：[`M4 .sapp and secure Runtime`](./plans/M4-sapp-runtime.md)，STEP-0038–0045 全部完成。

## M5: Sico Desktop Host

状态：`complete`

主要交付：Sico Desktop Host（Windows/macOS/Linux）、文件关联、权限 UI、生命周期、崩溃隔离、最小 UI WIT/SDK 和真实应用。

进入条件：M4 exit gate 通过。

退出证据：同一 `.sapp` 在声明支持的桌面平台保持一致核心行为。

执行计划：[`M5 Sico Desktop Host`](./plans/M5-desktop-host.md)，STEP-0046–0053 全部完成。退出证据：Windows signed install/open/Wasmtime/uninstall、权限/生命周期/typed UI、10,240 properties、release baseline、M0–M4 regression；结论见 [`M5 exit audit`](./reports/m5-exit-audit.md)。

## M6: Sico Android Host

状态：`blocked-external-runner`（STEP-0054–0061 host evidence complete; resume STEP-0060 device validation）

主要交付：Sico Android Host、文件/链接/分享入口、触摸、输入法、生命周期、Android 权限映射和共享行为测试。

进入条件：M5 的应用与 UI 接口达到可移植基线；M0 Runtime 报告证明 Android 可行。

退出证据：同一 `.sapp` 无需重编译即可在 Android 与桌面运行。

执行计划：[`M6 Sico Android Host`](./plans/M6-android-host.md)，STEP-0054–0061；shared core、Intent、permission、lifecycle、native UI contract、Desktop/Mobile parity 与审计已完成。缺少 licensed SDK/NDK/ADB/emulator/device，尚不能证明 Android Runtime、触摸/IME/TalkBack 与启动性能，M6 不得标记 complete。

## M7: 生态、工具与发布

状态：`local-complete / blocked-external-evidence`（product exit NO-GO）

主要交付：标准库稳定边界、包发布与发现、身份/签名/安全更新、LSP、AI 工具协议、真实应用和第三方 Component 试点。

进入条件：平台无关工作由 M4/M5 的 package/Desktop contracts 支撑；移动发布声明仍要求相应平台证据。

退出证据：外部开发者无需修改编译器或 Runtime 即可完成开发、检查、构建、发布、安装、运行和调试。

执行计划：[`M7 ecosystem, tooling and release`](./plans/M7-ecosystem-release.md)，STEP-0062–0069。仓库本地轨已完成 trust、registry、update、dependency stability、bounded LSP、compiler-backed AI tooling 及两版 pilot/exit audit。真实第三方、生产服务、live model、Android/Harmony/Linux runner 与最终项目退出继续作为独立外部 gate；结论见 [`M7 exit audit`](./reports/m7-exit-audit.md)。

### M7 production deployment continuation

状态：`local-complete / blocked-external-deployment-inputs`

仓库所有者于 2026-07-17 明确要求启动实际生产部署环境并简化使用流程，构成 M7 audit 允许的 new roadmap decision。STEP-0074 已实现不持有签名密钥的只读 registry origin、可移植 operator bundle、Windows release 集成和单命令源码工作流。公开部署仍需真实域名/TLS、hosting access、生产身份、密钥托管及外部可用性证据；输入到位后分配新的未使用 STEP，不占用已进入 M8 的 STEP-0075。

执行步骤：[`STEP-0074`](./steps/STEP-0074-production-deployment-origin-and-ux.md)、[`ADR-0008`](./adr/ADR-0008-production-registry-origin.md)。

## M8: Script Profile v0

状态：`GO / STEP-0075–0084 complete`

主要交付：bounded batch Script WIT、动态标量与通用 control/function codegen、Text/Bytes/List/record/Result Canonical ABI、versioned adapter、manifest v1、结构化 `sico-runner`、统一 `sico run`/`eval`、安全缓存，以及 text/bytes/list/JSON/scoped-file 最小标准库。

进入条件：M3/M4 Component/package/trust/Runtime 合约和 STEP-0074 单命令源码基线已存在；不要求公网部署、生产签名身份或移动 runner。

退出证据：args/binary stdin/separated stdout-stderr/exit 的真实 Wasmtime 链路；代表性 word-count/JSON/file scripts；缓存与 capability fail-closed；恶意 guest 后 Host 存活；Windows Runtime 和可获得的平台证据；性能与安全审计。

执行计划：[`M8 Script Profile`](./plans/M8-script-profile.md)，STEP-0075–0084。契约见 proposed [`RFC-0029`](./rfc/RFC-0029-script-profile-v0.md) 与 [`ADR-0009`](./adr/ADR-0009-script-adapter-runner.md)。

## M9: Streaming, async and interactive scripting

状态：`GO / STEP-0085–0094 complete`

主要交付：backpressured InputStream/OutputStream、resource Canonical ABI、Task/Future/Stream source backend、async runner/cancellation、scoped HTTP Component provider、persistent runner/watch、bounded REPL、top-level syntax decision与编辑器/AI执行集成。

进入条件：M8 必须完成自己的 GO，且 Script WIT/manifest/runner identities 可版本化而不是继续重写。M9 不包含 unrestricted process/shell capability。

执行计划：[`M9 streaming, async and interactive scripting`](./plans/M9-streaming-async-interactive.md)，STEP-0085–0094。STEP-0094 已重跑 M8/M9 全部关键门禁并以 Windows x64 Runtime 实证发出 GO；Linux/macOS/mobile M9 execution 不作推断。结论见 [`M9 exit audit`](./reports/m9-exit-audit-v0.md)。

## M10: Runtime observability and debugging

状态：`in-progress / STEP-0095–0098 complete; STEP-0099 next`

主要交付：versioned source/debug identity、compiler debug map、structured Runtime frames、typed OS-signal/client cancellation、bounded execution events/logs、minimal DAP 及 LSP/AI feedback integration。

进入条件：M9 GO，且 `sico.execution-plan.v0` 的 compile-only source coordinates、client-owned cancellation 与 debug refusal 已明确冻结。

执行计划：[`M10 Runtime observability and debugging`](./plans/M10-runtime-observability-debugging.md)，预留 STEP-0095–0102。STEP-0095 已冻结 contracts 与 DAP allowlist；STEP-0096 已实现 deterministic debug triplet；STEP-0097 已实现 exact Runtime source faults；STEP-0098 已实现真实 Windows console control、canonical client cancellation 和单 terminal winner。下一项 STEP-0099 实现 task-aware bounded events/redaction；documentation-only M11 STEP-0103 ADR 已解锁。STEP-0100 必须逐行提供真实 Component 支持证据或 typed refusal。Compiler 只拥有 deterministic debug map；Runtime fault/signal/event 与 DAP 分属 runner/tooling，不允许形成 `compiler → Runtime` 反向依赖。

退出证据：exact source/compiler/Component/debug-map identity、真实 Runtime source frames、typed signal cancellation、task-aware bounded event/log queues、机器清单驱动且在真实 Component 上逐项验证的 DAP 子集、session isolation、M0–M9 regression 与 actual-platform matrix。底层 Runtime 若不能支持真实 breakpoint/pause，必须记录 DAP NO-GO，不得把 post-mortem inspection 改名为 debugger。

## M11: Bounded structured-concurrency Runtime

状态：`planned / STEP-0103 design unlocks after STEP-0098; implementation after M10 GO`

主要交付：Store/arena 架构 ADR、单 Store 协作式 bounded scheduler、structured task scope、await/group/select/race、affine resource 跨 task 所有权、统一 cancellation/terminal winner、bounded channel/stream、persistent-runner/DAP task integration，以及 Windows x64 与 Linux x64 native runner 实证。

进入条件分两段：STEP-0098 接受 typed cancellation 后即可并行启动 documentation-only STEP-0103；任何 scheduler 实现和支持声明必须等待 M10 GO，尤其是 task-aware bounded events、redaction 与 exact DAP claims 稳定。

架构候选：v1 选择“每个 top-level run 一个 fresh Store，Store 内 cooperative scheduler”；多 Store 只作为未来 isolated worker，通过显式 bounded message ABI 通信，禁止共享 borrow、Wasm reference、resource handle 或 authority-bearing state。STEP-0103 ADR 必须先 reconcile M8 canonical ABI scratch arena、`post-return`、resource move/borrow、Store teardown 与 persistent generation 铁律。

执行计划：[`M11 bounded structured-concurrency Runtime`](./plans/M11-structured-concurrency-runtime.md)，STEP-0103–0110。并行仅是 Runtime 内部执行机制：child task 只能继承现有 grant 的相同或更小视图，concurrency quota 是资源限额而不是 capability，M11 不引入新 authority。

退出证据：ADR-before-code、arena borrow 不跨 suspension、affine resource 无 alias/leak、limit+1 与 cancellation/race 单终态、persistent generation 隔离、M0–M10 regression，以及 Windows x64 + Linux x64 原生 runner 同一语料。macOS/mobile 只有真实 native evidence 才能宣称。

## M12: Secure HTTP Provider and Automation SDK

状态：`planned after M11 GO / STEP-0111–0118 reserved`

主要交付：成熟 TLS 实现上的 HTTPS/SNI/certificate validation、exact scheme/host/port authority、IPv6/IDNA 与 DNS pinning、bounded streaming upload/download、redirect reauthorization、opaque Host secret injection、per-Store connection lifecycle、automation SDK/tooling 和出口审计。

进入条件：M11 GO 已冻结 Store/task/arena/cancellation/in-flight Host-operation accounting；M10 structured events、mandatory redaction 和 typed cancellation 保持稳定；M9 `http@0.1.0` compatibility profile 保留。若没有成熟 TLS stack，M12 在 contract/prototype gate 暂停，绝不自研 TLS。

模块边界：语言 semantics/IR 只拥有 HTTP 类型/effect/capability；codegen 只生成 versioned Component import；TLS/DNS/redirect/credentials 位于独立 Host provider；runner/manifest 负责默认拒绝的 endpoint/secret grants；标准库只提供不扩权的 helpers。Provider 必须复用 M11 scheduler，不创建第二套并行模型。

执行计划：[`M12 Secure HTTP Provider and Automation SDK`](./plans/M12-secure-http-automation-sdk.md)，STEP-0111–0118。M12 不包含 general sockets、ambient proxy/credentials、browser state、public deployment 或 mobile Runtime completion。

退出证据：真实 HTTPS 正反证书矩阵、IDNA/IPv4/IPv6/DNS rebinding 拒绝语料、1/16/256 MiB bounded-RSS streaming、redirect/secret non-leak、one-Store/task connection isolation、M0–M11 regression 与 actual-platform matrix。

M10–M12 之所以在此时推进内部基建，是因为 production domain/identity/credentials、third-party pilots、live-model credentials 和 mobile runners 仍是缺失的外部输入，不能靠仓库代码伪造。每个 exit audit 都必须重新检查这些输入；内部 GO 不得替代任何外部 gate。

## Immediate dependency chain

```text
STEP-0001 audit baseline
  → STEP-0002 application-host terminology migration
  → STEP-0003 syntax error injection
  → STEP-0004 AI evaluation protocol and offline harness
  → STEP-0005 remaining P0 semantic cases
  → STEP-0006 diagnostics v0
  → STEP-0007 semantic query JSON v0
  → STEP-0008 Int and Decimal representation prototypes
  → STEP-0009 resource, async, and WIT mapping prototypes
  → STEP-0010 Rust → Component → Runtime → WIT host-call chain
  → STEP-0011 desktop/Android Runtime feasibility report
  → STEP-0012 syntax evidence completion
  → STEP-0013 syntax decision
  → STEP-0014 M0 exit audit
  → STEP-0015–0021 M1 compiler frontend and exit audit
  → STEP-0022–0029 M2 static semantics and exit audit
  → STEP-0030 M3 typed Sico IR contract/validator
  → STEP-0031 core expression/control/data lowering
  → STEP-0032 effect/resource/revision IR lowering
  → STEP-0033 deterministic Core Wasm backend
  → STEP-0034 Component/WIT boundary codegen
  → STEP-0035 async/task/stream backend decision and implementation
  → STEP-0036 minimal end-to-end CLI build/run chain
  → STEP-0037 M3 determinism/quality/exit audit
  → STEP-0038 .sapp threat model and package contract
```

### Historical M0 execution sequence

| Step | 目标 | 关键退出证据 |
|---|---|---|
| STEP-0008 | `Int`/Decimal 表示原型 | Rust 实测运算、序列化、限额与 WIT 边界报告 |
| STEP-0009 | resource/async/WIT 映射原型 | affine move/drop、Future/Task/Stream 与 Component 映射证据 |
| STEP-0010 | 最小 Component host-call 链路 | 真实构建、加载、WIT 调用和确定性重跑 |
| STEP-0011 | Runtime 桌面/Android 对比 | 桌面实测、官方资料和可审计选择/保留项 |
| STEP-0012 | 补齐语法证据 | 54-case 静态指标、36 mutation、96-task 离线协议；真实 AI runs 等待外部授权且不得虚构 |
| STEP-0013 | 语法决定 | 接受、合并或淘汰 A0/B/C，并用 RFC 固定理由 |
| STEP-0014 | M0 退出审计 | 全部门槛、残余风险和进入 M1 的明确结论 |
