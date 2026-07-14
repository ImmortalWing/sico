# Sico project status

> - updated: 2026-07-14
> - phase: M0 设计与技术基线
> - phase status: in-progress
> - current step: none
> - last completed step: STEP-0004
> - next step: STEP-0005

## 1. Current objective

下一目标是补齐效果/能力、affine 资源、Future/Task、Stream、Component 调用和 revision 等 P0 语义案例。

## 2. Current step

[`STEP-0004: 建立 AI 评测协议与离线执行器 v0`](./steps/STEP-0004-ai-evaluation-protocol-v0.md) 已完成。下一步骤编号为 `STEP-0005`，尚未开始。

## 3. Verified repository facts

基线检查时间：2026-07-14。

| 事实 | 结果 | 状态 |
|---|---:|---|
| `.sico` 文件 | 118 | measured |
| 代表性程序 | 10 | verified by `examples/` |
| P0 A0 语义案例 | 30 | verified by `semantic-cases/` |
| 候选 B 案例 | 30 | verified by `syntax-candidates/b/` |
| 候选 C 案例 | 30 | verified by `syntax-candidates/c/` |
| 单点语法错误变体 | 18 | verified by mutation validator |
| 固定 AI 评测任务 | 42 | verified by AI evaluation validator |
| Rust `.rs` 文件 | 0 | measured |
| `Cargo.toml` | 0 | measured |
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
- 30 个首批 P0 正反例：[`semantic-cases/`](../semantic-cases/README.md)；
- A0/B/C 的 90 个一一对应案例与静态指标：[`syntax-candidates/`](../syntax-candidates/README.md)；
- 18 个可复现单点结构错误变体：[`syntax-mutations/`](../syntax-mutations/README.md)；
- 42 项 AI 评测协议与离线执行器：[`ai-eval/`](../ai-eval/README.md)；
- 长期自治执行目标：[`AGENT_GOAL.md`](../AGENT_GOAL.md)。

## 5. Incomplete M0 work

按当前依赖顺序：

1. 剩余 P0 案例：效果/能力、affine 资源、Future/Task、Stream、Component 调用、revision；
2. 诊断协议 v0 和正式错误编号分区；
3. 语义索引与 `outline/describe/slice/impact/flow` JSON v0；
4. `Int`、Decimal、资源、异步和 WIT 映射原型；
5. Rust → Component → Runtime → WIT host call 最小链路；
6. Component Runtime 的桌面与 Android 对比报告；
7. 语法候选的数据驱动选择或合并；
8. M0 退出审计。

## 6. Blockers

当前没有阻塞下一步骤的外部条件。

真实 AI API 批量评测仍需要模型凭据和成本授权；协议和离线工具已经完成，因此该条件不阻塞下一项 P0 设计工作。没有真实调用前不产生模型分数。

## 7. Risks

- A0/B/C 目前没有正式词法器或解析器，结构配对不等于语法有效；
- `Int` 任意精度、contract invariant 和 Result 表层写法仍是草案；
- 语法候选只覆盖四组 P0 语义，尚未覆盖模块、能力、资源和异步；
- 没有 Component 技术原型，WIT 和 Runtime 可行性尚未实测；
- 没有稳定诊断编号，当前 reject key 仅是语义名称。

## 8. Next step

`STEP-0005`：补齐效果/能力、affine 资源、Future/Task、Stream、Component 调用和 revision 的 P0 最小合法/非法案例，并同步 A0/B/C 候选表示。
