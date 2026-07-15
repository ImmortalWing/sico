# Sico audited roadmap

> - updated: 2026-07-15
> - source of phase definitions: [`DEVELOPMENT.md`](../DEVELOPMENT.md)
> - current phase: M3

## Status vocabulary

| 状态 | 含义 |
|---|---|
| `complete` | 交付物和退出证据齐全 |
| `in-progress` | 已开始且存在未完成工作 |
| `planned` | 已定义但尚未开始 |
| `blocked` | 存在明确外部阻塞 |
| `superseded` | 已被新步骤或决定替代 |

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

状态：`in-progress`（STEP-0030 complete）

主要交付：强类型 IR、验证器、lowering、Core Wasm、Component 封装、WIT 绑定和最小端到端 CLI 程序；在真实 codegen/Runtime 链路上定义 `sico run app.sico`，并以 `sico app.sico` 作为候选便捷形式。

进入条件：M2 exit gate 通过，M0 Component 原型的技术风险已有结论。

退出证据：确定性 Component 输出；真实 Runtime 执行；WIT host call 与语义案例一致。

执行计划：[`M3 Sico IR and Component`](./plans/M3-sico-ir-component.md)，STEP-0030–0037；typed IR contract/verifier 已完成，下一步 STEP-0031 core lowering。

## M4: `.sapp` 与 Runtime

状态：`planned`

主要交付：包格式、manifest、资源、哈希、开发签名、加载验证、权限交集、隔离存储、资源限额和 `build/run/inspect`；固定源码直跑的编译缓存、参数透传、stdio 与退出码契约，并为后续 `sico -c` 和交互式 REPL 提供稳定 Runtime 接口。

进入条件：M3 exit gate 通过。

退出证据：不可信包不能越权；trap 不导致宿主崩溃；包可重复构建和检查。

## M5: Sico Desktop Host

状态：`planned`

主要交付：Sico Desktop Host（Windows/macOS/Linux）、文件关联、权限 UI、生命周期、崩溃隔离、最小 UI WIT/SDK 和真实应用。

进入条件：M4 exit gate 通过。

退出证据：同一 `.sapp` 在声明支持的桌面平台保持一致核心行为。

## M6: Sico Android Host

状态：`planned`

主要交付：Sico Android Host、文件/链接/分享入口、触摸、输入法、生命周期、Android 权限映射和共享行为测试。

进入条件：M5 的应用与 UI 接口达到可移植基线；M0 Runtime 报告证明 Android 可行。

退出证据：同一 `.sapp` 无需重编译即可在 Android 与桌面运行。

## M7: 生态、工具与发布

状态：`planned`

主要交付：标准库稳定边界、包发布与发现、身份/签名/安全更新、LSP、AI 工具协议、真实应用和第三方 Component 试点。

进入条件：M4-M6 的发布、运行和平台能力稳定。

退出证据：外部开发者无需修改编译器或 Runtime 即可完成开发、检查、构建、发布、安装、运行和调试。

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
