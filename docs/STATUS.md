# Sico project status

> - updated: 2026-07-15
> - phase: M0 设计与技术基线
> - phase status: in-progress
> - current step: none
> - last completed step: STEP-0010
> - next step: STEP-0011

## 1. Current objective

下一目标是比较 Component Runtime 在桌面与 Android 的可行实现、发布约束和验证路线。

## 2. Current step

[`STEP-0010: 验证 Component Runtime WIT host call 实链`](./steps/STEP-0010-component-runtime-host-call.md) 已完成。下一步骤编号为 `STEP-0011`，尚未开始。

## 3. Verified repository facts

基线检查时间：2026-07-14。

| 事实 | 结果 | 状态 |
|---|---:|---|
| `.sico` 文件 | 190 | measured |
| 代表性程序 | 10 | verified by `examples/` |
| P0 A0 语义案例 | 54 | verified by semantic case validator |
| 候选 B 案例 | 54 | verified by semantic case validator |
| 候选 C 案例 | 54 | verified by semantic case validator |
| 单点语法错误变体 | 18 | verified by mutation validator |
| 固定 AI 评测任务 | 42 | verified by AI evaluation validator |
| 稳定诊断 code/key | 24 | verified by diagnostic validator |
| 已映射非法 case | 29 | verified by diagnostic validator |
| Rust `.rs` 文件 | 11 | measured |
| `Cargo.toml` | 5 | measured |
| 数值原型单元测试 | 10 passed | verified |
| resource/async 动态测试 | 10 passed | verified |
| WASI 0.3 WIT parser 测试 | 1 passed | verified |
| 真实 WebAssembly Component | 2 | verified |
| Component sync/async 重跑 | 2 + 2 passed | verified |
| 4096-bit/Decimal ABI roundtrip | 512 bytes + true | verified |
| 正式编译器 | 不存在 | verified |
| 实际 `.sico` 编译结果 | 不存在 | verified |

因此，仓库当前仍是 M0 设计阶段。现有 accept/reject 案例是设计判定，不是编译器实测结果。

## 4. Completed assets

- 方向与非目标：[`DIRECTION.md`](../DIRECTION.md)；
- 工程与阶段架构：[`DEVELOPMENT.md`](../DEVELOPMENT.md)；
- 核心语义草案：[`SEMANTICS.md`](../SEMANTICS.md)；
- 候选语法与评测方法：[`SYNTAX.md`](../SYNTAX.md)；
- 10 个代表性程序与问题矩阵：[`examples/`](../examples/README.md)；
- 候选 A 跨样本语义审计：[`examples/SEMANTICS-AUDIT.md`](../examples/SEMANTICS-AUDIT.md)；
- 54 个 P0 正反例：[`semantic-cases/`](../semantic-cases/README.md)；
- A0/B/C 的 162 个一一对应案例与首轮静态指标：[`syntax-candidates/`](../syntax-candidates/README.md)；
- 18 个可复现单点结构错误变体：[`syntax-mutations/`](../syntax-mutations/README.md)；
- 42 项 AI 评测协议与离线执行器：[`ai-eval/`](../ai-eval/README.md)；
- 24 个稳定诊断、29 个 case 映射、JSON Schema 与离线校验器：[`diagnostics/`](../diagnostics/README.md)；
- 10 模块 Semantic Index fixture、五类查询 JSON v0 与离线校验器：[`semantic-index/`](../semantic-index/README.md)；
- `Int`/Decimal Rust 原型、WIT 候选、RFC 和可复现报告：[`prototypes/numeric/`](../prototypes/numeric/README.md)；
- resource/async Rust 原型、compile-fail、WASI 0.3 WIT、RFC 和报告：[`prototypes/resource-async/`](../prototypes/resource-async/README.md)；
- 真实 Component host call、resource、数值记录和原生 async 往返：[`prototypes/component-host-call/`](../prototypes/component-host-call/README.md)；
- 长期自治执行目标：[`AGENT_GOAL.md`](../AGENT_GOAL.md)。

## 5. Incomplete M0 work

按当前依赖顺序：

1. Component Runtime 的桌面与 Android 对比报告；
2. 第二轮语法指标与真实 AI 证据（需要凭据/成本授权）；
3. 语法候选的数据驱动选择或合并；
4. M0 退出审计。

## 6. Blockers

当前没有阻塞下一步骤的外部条件。

真实 AI API 批量评测仍需要模型凭据和成本授权；协议和离线工具已经完成，因此该条件不阻塞下一项 P0 设计工作。没有真实调用前不产生模型分数。

## 7. Risks

- A0/B/C 目前没有正式词法器或解析器，结构配对不等于语法有效；
- `Int`/Decimal 记录已通过 Rust 原型和真实 Component 往返，但 RFC-0003 仍待非 Windows 重现与稳定限额诊断分类；contract invariant 和 Result 表层写法仍是草案；
- 语法候选已覆盖 10 组 P0 设计判定，但第二轮 24 个 case 尚无静态指标或 AI 实测；
- WIT 0.253 已真实往返 resource、数值记录和 `async func`；`future<T>`/`stream<T>` 仍只有 parser 与 Rust 状态机证据；
- 诊断协议已有设计目录和 fixtures，但尚无 compiler 生成真实 code、跨度或级联数据。
- 语义查询协议已有设计 fixtures，但只有两个模块详细展开，没有真实 index/accuracy/latency 数据。

## 8. Next step

`STEP-0011`：比较桌面与 Android 的 Component Runtime 方案，记录 Wasmtime/JIT/AOT、包体、沙箱、FFI 和发布限制。
