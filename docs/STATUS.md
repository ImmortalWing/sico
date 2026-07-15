# Sico project status

> - updated: 2026-07-15
> - phase: M1 编译器前端与诊断
> - phase status: in-progress
> - current step: none
> - last completed step: STEP-0015
> - next step: STEP-0016

## 1. Current objective

下一目标是按 RFC-0006 实现统一 source/span、line index 与无损 lexer，并让 STEP-0015 的 21 个 contract case 成为真实 Rust behavior tests。

## 2. Current step

[`STEP-0015: 建立编译器 workspace 并冻结 lexical/source 契约`](./steps/STEP-0015-compiler-workspace-lexical-source.md) 已完成：七 crate workspace 可构建，RFC-0006 accepted，21 个 contract case 和独立 validator 通过。crate 仍是空实现边界，没有实际 lexer/parser 行为。

## 3. Verified repository facts

基线检查时间：2026-07-15。

| 事实 | 结果 | 状态 |
|---|---:|---|
| `.sico` 文件 | 208 | measured |
| 代表性程序 | 10 | verified by `examples/` |
| P0 A0 语义案例 | 54 | verified by semantic case validator |
| 候选 B 案例 | 54 | verified by semantic case validator |
| 候选 C 案例 | 54 | verified by semantic case validator |
| 单点语法错误变体 | 36 | verified by mutation validator |
| 固定 AI 评测任务 | 96 | verified by AI evaluation validator |
| 稳定诊断 code/key | 24 | verified by diagnostic validator |
| 已映射非法 case | 29 | verified by diagnostic validator |
| Rust `.rs` 文件 | 18 | measured |
| `Cargo.toml` | 13 | measured |
| 数值原型单元测试 | 10 passed | verified |
| resource/async 动态测试 | 10 passed | verified |
| WASI 0.3 WIT parser 测试 | 1 passed | verified |
| 真实 WebAssembly Component | 2 | verified |
| Component sync/async 重跑 | 2 + 2 passed | verified |
| 4096-bit/Decimal ABI roundtrip | 512 bytes + true | verified |
| 正式编译器 workspace | 7 crates, build/test pass | verified |
| source/lexer/parser 实现 | 不存在 | verified |
| 实际 `.sico` 编译结果 | 不存在 | verified |

M0 已完成；仓库已进入 M1 并建立正式 workspace，但可执行编译器和前端行为仍不存在。现有 accept/reject 案例仍是设计判定，直到 M1/M2 产生真实 parser/type-checker 结果。

## 4. Completed assets

- 方向与非目标：[`DIRECTION.md`](../DIRECTION.md)；
- 工程与阶段架构：[`DEVELOPMENT.md`](../DEVELOPMENT.md)；
- 核心语义草案：[`SEMANTICS.md`](../SEMANTICS.md)；
- 候选语法与评测方法：[`SYNTAX.md`](../SYNTAX.md)；
- 10 个代表性程序与问题矩阵：[`examples/`](../examples/README.md)；
- 候选 A 跨样本语义审计：[`examples/SEMANTICS-AUDIT.md`](../examples/SEMANTICS-AUDIT.md)；
- 54 个 P0 正反例：[`semantic-cases/`](../semantic-cases/README.md)；
- A0/B/C 的 162 个一一对应案例与完整静态指标：[`syntax-candidates/`](../syntax-candidates/README.md)；
- 36 个可复现单点结构错误变体：[`syntax-mutations/`](../syntax-mutations/README.md)；
- 96 项 AI v1 评测协议与离线执行器：[`ai-eval/`](../ai-eval/README.md)；
- 24 个稳定诊断、29 个 case 映射、JSON Schema 与离线校验器：[`diagnostics/`](../diagnostics/README.md)；
- 10 模块 Semantic Index fixture、五类查询 JSON v0 与离线校验器：[`semantic-index/`](../semantic-index/README.md)；
- `Int`/Decimal Rust 原型、WIT 候选、RFC 和可复现报告：[`prototypes/numeric/`](../prototypes/numeric/README.md)；
- resource/async Rust 原型、compile-fail、WASI 0.3 WIT、RFC 和报告：[`prototypes/resource-async/`](../prototypes/resource-async/README.md)；
- 真实 Component host call、resource、数值记录和原生 async 往返：[`prototypes/component-host-call/`](../prototypes/component-host-call/README.md)；
- 桌面/Android Runtime 基线、发布政策与 Android 最小探针：[`ADR-0002`](./adr/ADR-0002-runtime-platform-baseline.md)、[`runtime report`](./reports/runtime-desktop-android-v0.md)；
- M1 B labeled-block 语法基线、候选取舍与复审门槛：[`RFC-0005`](./rfc/RFC-0005-labeled-block-syntax-baseline.md)；
- 12 类 AI-oriented 错误 taxonomy（真实频率未测量）：[`error-taxonomy.json`](../ai-eval/error-taxonomy.json)；
- M0 requirement-by-requirement GO 结论与递延登记：[`M0 exit audit`](./reports/m0-exit-audit.md)；
- STEP-0015–0021 前端工程计划：[`M1 compiler frontend`](./plans/M1-compiler-frontend.md)；
- 正式 Rust workspace、lexical/source v0 与 21 个 contract case：[`STEP-0015`](./steps/STEP-0015-compiler-workspace-lexical-source.md)、[`RFC-0006`](./rfc/RFC-0006-lexical-source-contract-v0.md)；
- 长期自治执行目标：[`AGENT_GOAL.md`](../AGENT_GOAL.md)。

## 5. Incomplete M0 work

无。真实 AI、Android、Future/Stream Runtime 与 proposed RFC 接受条件已登记为后续/外部证据，不是 M0 完成声明。

## 6. Blockers

当前没有阻塞 STEP-0015 的外部条件。

真实 AI API 批量评测仍需要模型凭据和成本授权；协议和离线工具已经完成，因此该条件不阻塞 STEP-0015/M1。没有真实调用前不产生模型分数。

## 7. Risks

- B 已选为 M1 baseline，lexical/source contract 已冻结，但目前没有正式 source/lexer/parser 实现；结构配对与具名关闭不等于已测恢复能力；
- `Int`/Decimal 记录已通过 Rust 原型和真实 Component 往返，但 RFC-0003 仍待非 Windows 重现与稳定限额诊断分类；contract invariant 和 Result 表层写法仍是草案；
- 语法候选已覆盖 10 组 P0 判定、完整静态指标和离线 AI 任务，但仍没有真实 parser 或模型实测；
- WIT 0.253 已真实往返 resource、数值记录和 `async func`；`future<T>`/`stream<T>` 仍只有 parser 与 Rust 状态机证据；
- Wasmtime Android aarch64/x86_64 仍是 Tier 3；Pulley/真机/JNI/商店政策只有 M0 选择，尚无仓库实测；
- 诊断协议已有设计目录和 fixtures，但尚无 compiler 生成真实 code、跨度或级联数据；
- 语义查询协议已有设计 fixtures，但只有两个模块详细展开，没有真实 index/accuracy/latency 数据。

## 8. Next step

`STEP-0016`：实现 source/span、line index 与 lossless lexer；执行 token golden、Unicode/invalid-byte diagnostics、line index property tests 和 trivia byte-preservation tests，不开始 B parser 或 M2 语义。
