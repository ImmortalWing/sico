# Sico project status

> - updated: 2026-07-15
> - phase: M2 静态语义
> - phase status: in-progress
> - current step: none
> - last completed step: STEP-0024
> - next step: STEP-0025

## 1. Current objective

下一目标是执行 STEP-0025：实现 effects/capabilities 与 Component boundary semantics，使 capability + component 的 4 valid/4 invalid 产生真实结果。

## 2. Current step

当前没有进行中的 STEP。[`STEP-0024`](./steps/STEP-0024-match-result-control-flow.md) 已完成；下一项只扩展 capability + component oracle 所需规则。

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
| 稳定诊断 code/key | 36（其中 12 个由 parser 产生） | verified by diagnostic validator |
| 已映射非法 case | 29 | verified by diagnostic validator |
| Rust `.rs` 文件 | 22 | measured |
| `Cargo.toml` | 13 | measured |
| 数值原型单元测试 | 10 passed | verified |
| resource/async 动态测试 | 10 passed | verified |
| WASI 0.3 WIT parser 测试 | 1 passed | verified |
| 真实 WebAssembly Component | 2 | verified |
| Component sync/async 重跑 | 2 + 2 passed | verified |
| 4096-bit/Decimal ABI roundtrip | 512 bytes + true | verified |
| 正式编译器 workspace | 8 crates, build/test pass | verified |
| source/line index | 6 tests passed | verified |
| lossless lexer | 8 tests; B 54/54 | verified |
| B happy-path parser | 54/54 + 54 snapshots | verified |
| parser recovery/E1xxx | 12/12 mutation；E1001–E1012 | verified |
| canonical formatter | 54/54 AST stable + idempotent | verified |
| `sico` CLI | 3 commands；4 integration groups | verified |
| deterministic frontend properties | 8,192 inputs | verified |
| parser limits | depth 256；diagnostics 100 | verified |
| M1 performance | median 1061 ms / 4.190 MiB/s；no SLA | measured |
| 实际 `.sico` core/nominal type-check | 7 valid pass；9 invalid exact primary | verified |
| 实际 `.sico` match/Result check | 6 valid pass；8 invalid exact primary | verified |

M0/M1 已完成；仓库已有 numbers/nominal 与 match/Result 的真实 semantic checker。其余 12 valid/12 invalid 仍是 M2 oracle，不能解释为已产生语义编译结果。

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
- 36 个稳定诊断（12 个真实 syntax、24 个 M2 设计）、12 个 syntax mutation 与 29 个 semantic case 映射、JSON Schema 和校验器：[`diagnostics/`](../diagnostics/README.md)；
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
- strict source/span、line index 与 54-file lossless lexer：[`STEP-0016`](./steps/STEP-0016-source-span-lossless-lexer.md)；
- B happy-path lossless parser 与 AST shape：[`STEP-0017`](./steps/STEP-0017-b-grammar-lossless-parser.md)；
- parser recovery、E1001–E1012 与 text/JSON span：[`STEP-0018`](./steps/STEP-0018-parser-recovery-syntax-diagnostics.md)；
- canonical formatter、comment/trivia policy 与 error-tree refusal：[`STEP-0019`](./steps/STEP-0019-canonical-formatter.md)；
- `sico check`/`format`/`outline`、退出码与 M2 capability boundary：[`STEP-0020`](./steps/STEP-0020-cli-check-format-outline.md)；
- frontend fuzz/limits/performance 与 M1 GO：[`STEP-0021`](./steps/STEP-0021-fuzz-performance-m1-exit.md)、[`M1 exit audit`](./reports/m1-exit-audit.md)；
- STEP-0022–0029 静态语义执行计划：[`M2 static semantics`](./plans/M2-static-semantics.md)；
- full B HIR/name/prelude contract：[`STEP-0022`](./steps/STEP-0022-full-hir-name-prelude-contract.md)、[`review report`](./reports/full-hir-name-prelude-v0.md)；
- core/nominal type checker：[`STEP-0023`](./steps/STEP-0023-core-nominal-types.md)、[`review report`](./reports/core-nominal-types-v0.md)；
- match/Result control flow：[`STEP-0024`](./steps/STEP-0024-match-result-control-flow.md)、[`review report`](./reports/match-result-control-flow-v0.md)；
- 长期自治执行目标：[`AGENT_GOAL.md`](../AGENT_GOAL.md)。

## 5. Incomplete M0 work

无。真实 AI、Android、Future/Stream Runtime 与 proposed RFC 接受条件已登记为后续/外部证据，不是 M0 完成声明。

## 6. Blockers

当前没有阻塞 STEP-0025 的外部条件。

真实 AI API 批量评测仍需要模型凭据和成本授权；协议和离线工具已经完成，因此该条件不阻塞 STEP-0015/M1。没有真实调用前不产生模型分数。

## 7. Risks

- B frontend 与确定性 fuzz/limit 已完成，但 coverage-guided 长期 fuzz 和跨平台性能仍是后续质量工作；
- `Int`/Decimal 记录已通过 Rust 原型和真实 Component 往返，但 RFC-0003 仍待非 Windows 重现与稳定限额诊断分类；contract invariant 和 Result 表层写法仍是草案；
- numbers/nominal 与 match/result 已有真实 checker；effect/resource/revision 仍待实现，且仍无真实模型实测；
- WIT 0.253 已真实往返 resource、数值记录和 `async func`；`future<T>`/`stream<T>` 仍只有 parser 与 Rust 状态机证据；
- Wasmtime Android aarch64/x86_64 仍是 Tier 3；Pulley/真机/JNI/商店政策只有 M0 选择，尚无仓库实测；
- E1xxx、STEP-0023 E2xxx 与 STEP-0024 E3xxx 已有真实 compiler code、跨度和 bounded cascade；E4xxx 及以后仍待实现；
- 语义查询协议已有设计 fixtures，但只有两个模块详细展开，没有真实 index/accuracy/latency 数据。

## 8. Next step

`STEP-0025`：实现 effects/capabilities 与 Component boundary semantics；只验收对应 4 valid/4 invalid oracle，不提前实现 resource/async/revision。
