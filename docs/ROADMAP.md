# Sico audited roadmap

> - updated: 2026-07-14
> - source of phase definitions: [`DEVELOPMENT.md`](../DEVELOPMENT.md)
> - current phase: M0

## Status vocabulary

| 状态 | 含义 |
|---|---|
| `complete` | 交付物和退出证据齐全 |
| `in-progress` | 已开始且存在未完成工作 |
| `planned` | 已定义但尚未开始 |
| `blocked` | 存在明确外部阻塞 |
| `superseded` | 已被新步骤或决定替代 |

## M0: 设计与技术基线

状态：`in-progress`

| Work package | 状态 | 证据/下一步 |
|---|---|---|
| 方向、非目标与技术路线 | complete | [`DIRECTION.md`](../DIRECTION.md) |
| 10 个代表性程序 | complete | [`examples/PLAN.md`](../examples/PLAN.md) |
| 跨样本问题与语义审计 | complete | [`examples/ISSUES.md`](../examples/ISSUES.md)、[`SEMANTICS-AUDIT.md`](../examples/SEMANTICS-AUDIT.md) |
| 核心语义草案 | complete as draft | [`SEMANTICS.md`](../SEMANTICS.md)；正式稳定仍需 RFC/案例 |
| 首批 P0 正反例 | complete for 4 groups | [`semantic-cases/`](../semantic-cases/README.md) |
| 三套局部语法候选 | complete for 4 groups | [`SYNTAX.md`](../SYNTAX.md)、[`syntax-candidates/`](../syntax-candidates/README.md) |
| 静态语法体积指标 | complete | [`METRICS.md`](../syntax-candidates/METRICS.md) |
| 审计体系 | complete | [`STEP-0001`](./steps/STEP-0001-project-state-audit.md) |
| 单点错误注入与恢复评测 | planned | STEP-0002 |
| AI 常见错误分类 | partial | 问题矩阵已有材料，需要独立数据集/分类报告 |
| AI 理解与修复基线 | partial | 需求已定义，尚无可执行评测与实测数据 |
| 剩余 P0 语义案例 | planned | 效果/能力、资源、异步、Stream、Component、revision |
| 诊断协议 v0 | planned | 当前只有 provisional reject key |
| 语义索引 JSON v0 | planned | 当前只有文档需求 |
| Wasm Component 最小原型 | planned | 当前无 Rust/Cargo 代码 |
| Runtime 引擎桌面/Android 对比 | planned | 需要独立 report |
| 数据驱动语法决定 | planned | 依赖错误注入与 AI 评测 |
| M0 退出审计 | planned | 依赖以上所有 M0 项 |

### M0 exit gate

- 关键语义都有正反例、诊断根因和明确状态；
- 语法决定有静态、错误恢复和 AI 评测证据；
- 诊断与语义查询协议存在 v0；
- Component 最小链路和 Runtime 可行性真实运行；
- 所有关键开放项明确进入 RFC、ADR 或后续阶段；
- 编译器实现不会隐式替语言做决定。

## M1: 编译器前端与诊断

状态：`planned`

主要交付：Rust workspace、源码/跨度、词法器、解析与恢复、无损树、语义 AST、格式化器、`sico check`、文本/JSON 诊断、outline、parser fuzzing。

进入条件：M0 exit gate 通过。

退出证据：合法语法案例稳定解析；非法案例产生预期主要诊断；单点错误不会形成不可控级联。

## M2: 静态语义

状态：`planned`

主要交付：名称解析、类型系统、局部推导、名义类型、Option/Result、完整匹配、不可变、效果/能力、资源基础规则和初步语义索引。

进入条件：M1 exit gate 通过。

退出证据：P0 非法案例全部在后端前拒绝；合法案例通过；诊断快照稳定。

## M3: Sico IR 与 Component

状态：`planned`

主要交付：强类型 IR、验证器、lowering、Core Wasm、Component 封装、WIT 绑定和最小端到端 CLI 程序。

进入条件：M2 exit gate 通过，M0 Component 原型的技术风险已有结论。

退出证据：确定性 Component 输出；真实 Runtime 执行；WIT host call 与语义案例一致。

## M4: `.sapp` 与 Runtime

状态：`planned`

主要交付：包格式、manifest、资源、哈希、开发签名、加载验证、权限交集、隔离存储、资源限额和 `build/run/inspect`。

进入条件：M3 exit gate 通过。

退出证据：不可信包不能越权；trap 不导致宿主崩溃；包可重复构建和检查。

## M5: 桌面 Player

状态：`planned`

主要交付：Windows/macOS/Linux Player、文件关联、权限 UI、生命周期、崩溃隔离、最小 UI WIT/SDK 和真实应用。

进入条件：M4 exit gate 通过。

退出证据：同一 `.sapp` 在声明支持的桌面平台保持一致核心行为。

## M6: Android Player

状态：`planned`

主要交付：Android Player、文件/链接/分享入口、触摸、输入法、生命周期、Android 权限映射和共享行为测试。

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
  → STEP-0002 syntax error injection
  → AI evaluation protocol and offline harness
  → remaining P0 semantic cases
  → diagnostics v0 + semantic query JSON v0
  → Component and Runtime technical spikes
  → syntax decision
  → M0 exit audit
  → M1
```
