# STEP-0001: 建立审计基线与项目差距审计

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

建立 Sico 的长期审计目录、步骤/ADR/RFC/report 模板、当前状态和阶段路线图，并用仓库可验证事实说明 M0-M7 的真实完成度。

## 2. Context and evidence

- 自治执行要求：[`AGENT_GOAL.md`](../../AGENT_GOAL.md)；
- 正式阶段定义：[`DEVELOPMENT.md`](../../DEVELOPMENT.md)；
- 当前设计入口：[`README.md`](../../README.md)；
- 当前最新基线提交：`bced9cb`；
- 2026-07-14 仓库测量：100 个 `.sico`、0 个 `.rs`、0 个 `Cargo.toml`。

现有 Git 历史已完成方向文档、代表性样本、语义草案、首批 P0 案例和 A0/B/C 局部语法候选，但尚无统一步骤编号和状态入口。

## 3. Scope

包含：

- `docs/` 索引；
- `STATUS.md` 与 `ROADMAP.md`；
- step、ADR、RFC 和 report 模板；
- ADR/RFC/report/steps 索引；
- 当前资产和缺口审计；
- 根 README 文档入口。

不包含：

- 修改语言语义或语法；
- 创建 Rust workspace；
- 运行不存在的编译器；
- 生成 AI 评测分数；
- 开始 STEP-0002 的变异案例。

## 4. Options and decision

### Option A: 继续只使用根目录设计文档

优点是文件少；缺点是阶段状态、执行过程、实验和正式设计混在一起，长期无法追踪每一步证据。

### Option B: 移动全部现有文档到 `docs/`

目录统一，但会制造大量无价值路径变更，破坏已有链接，并混淆正式设计与过程记录。

### Decision: 保留正式设计位置，新增审计目录

选择新增 `docs/` 管理过程记录，同时保留根目录正式设计文件。这样既保持现有引用稳定，又能建立步骤、决策和报告历史。

状态：`accepted for audit process`。若未来文档规模需要重组，应单独建立步骤和迁移映射。

## 5. Plan

1. 测量仓库文件与实现状态；
2. 建立目录和模板；
3. 编写 STATUS 与 ROADMAP；
4. 将 STEP-0001 加入步骤索引；
5. 更新根 README；
6. 验证 Markdown 链接、步骤编号、状态一致性和 Git whitespace；
7. 完成步骤文档并提交推送。

## 6. Changes

新增：

- `docs/README.md`：审计入口；
- `docs/STATUS.md`：当前真实状态、缺口、风险和下一步；
- `docs/ROADMAP.md`：M0-M7 状态、依赖和退出门槛；
- `docs/steps/README.md` 与本步骤记录；
- `docs/adr/README.md`、`docs/rfc/README.md`、`docs/reports/README.md`；
- step、ADR、RFC、report 四个模板。

修改根 `README.md`，增加项目状态与审计入口。

没有修改语言语义、候选源码或工具链实现。

## 7. Validation

实际执行结果：

| Check | Result |
|---|---|
| 全仓 Markdown 相对链接 | `ALL_MARKDOWN_LINKS_OK` |
| STEP 文件名和标题 | 1 个记录，`STEP-0001_ID_STATUS_OK` |
| STATUS/ROADMAP 当前阶段 | `PHASE_STATUS_ALIGNED` |
| 审计模板数量 | 4 |
| `docs/` Markdown 文件 | 12 |
| `git diff --cached --check` | exit 0 |

本步骤只修改文档，且仓库不存在编译器，因此没有运行编译、单元测试或 `.sico` 语义测试。没有把文档检查表述为编译器验证。

## 8. Metrics

基线 measured：

| Metric | Value |
|---|---:|
| `.sico` files | 100 |
| Rust `.rs` files | 0 |
| `Cargo.toml` files | 0 |
| existing `docs/` directory | 0 |

本步骤不产生编译器、Runtime、性能或 AI 实测指标。

## 9. Risks and follow-ups

- 文档可能随代码演进失真，因此每个步骤结束必须更新 STATUS；
- 模板过重会制造形式主义，小型机械修改应合并到父步骤；
- M0 历史提交早于步骤体系，保留 Git 历史，不为旧工作伪造步骤编号；
- 下一步应直接进入可验证设计资产，而不是继续扩充管理文档。

## 10. Audit links

- commit subject: `docs(audit): [STEP-0001] establish project audit baseline`
- next: STEP-0002 syntax error injection
