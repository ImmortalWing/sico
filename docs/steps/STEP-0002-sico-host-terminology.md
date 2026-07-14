# STEP-0002: 将 Sico Player 统一更名为 Sico Host

> - status: complete
> - phase: M0
> - started: 2026-07-14
> - completed: 2026-07-14
> - owners: autonomous-agent

## 1. Objective

将面向用户、负责打开和运行 `.sapp` 的原生宿主产品从 `Sico Player` 统一更名为 `Sico Host`，消除“影音播放器”的错误联想，同时保持 `Sico Runtime` 的底层执行职责不变。

## 2. Context and evidence

- 仓库所有者在 2026-07-14 明确接受 `Sico Host` 命名；
- 旧称分布于 README、方向、开发、自治目标、路线图和 Notes UI 规格；
- 正式架构已经区分底层 Runtime 与面向用户的产品，只需统一术语，不改变执行模型；
- 决策记录：[`ADR-0001`](../adr/ADR-0001-sico-host-terminology.md)。

## 3. Scope

包含：

- `Sico Player` → `Sico Host`；
- `桌面 Player` → `Sico Desktop Host` 或“桌面 Host”；
- `Android Player` → `Sico Android Host`；
- `player-desktop/player-android` 建议 crate 名 → `host-desktop/host-android`；
- Runtime/Player 组合指标和版本名称同步修改；
- 文档标题、图示、路线图、示例和自治目标同步修改。

不包含：

- 修改 Sico Runtime 的职责；
- 创建 Host 实现；
- 改变 `.sapp`、Component、WIT 或能力模型；
- 确定面向终端用户的商店展示名称和品牌视觉。

## 4. Options and decision

考虑 `Launcher`、`Viewer`、`Container`、`Environment` 和 `Host`。

- Launcher 只表达启动，不能覆盖安装、权限、生命周期与宿主能力；
- Viewer/Player 容易被理解为内容查看或影音播放；
- Container 容易与 OS/OCI 容器混淆；
- Environment 过于抽象；
- Host 准确表达“承载应用并提供宿主能力”。

决定使用 `Sico Host`。底层继续使用 `Sico Runtime`。平台构建称为 `Sico Desktop Host` 和 `Sico Android Host`。

## 5. Plan

1. 定位全部旧称；
2. 记录 ADR；
3. 更新正式设计、阶段路线图和自治提示词；
4. 更新 crate 结构提案和示例规格；
5. 检查旧称残留、Markdown 链接、状态与 whitespace；
6. 完成步骤记录并提交推送。

## 6. Changes

正式术语更新：

- 根 README：技术路线使用 Sico Host；
- `DIRECTION.md`：运行平台、职责、图示和打开 `.sapp` 流程；
- `DEVELOPMENT.md`：架构边界、crate 提案、Host 章节、测试、版本、阶段和体积指标；
- `AGENT_GOAL.md`：M5/M6、完成标准和质量指标；
- `docs/ROADMAP.md`：M5/M6 及依赖链；
- Notes UI 规格：跨平台目标改为 Desktop Host 与 Android Host。

审计更新：

- 新增并接受 `ADR-0001`；
- 更新 ADR、步骤、STATUS 和 ROADMAP 索引；
- 原计划的语法错误注入顺延为 STEP-0003。

代码结构提案中的 `player-desktop`、`player-android` 已改为 `host-desktop`、`host-android`。没有实现代码需要迁移。

## 7. Validation

实际执行结果：

| Check | Result |
|---|---|
| 活跃文档中的 `player|播放器` | 0 个文件 |
| 审计历史中的旧称 | 仅本 STEP 与 ADR-0001，作为迁移证据保留 |
| 正式文件中的 Host 新称 | README、方向、开发、自治目标、路线图和 Notes UI 均存在 |
| Markdown 相对链接 | `ALL_MARKDOWN_LINKS_OK` |
| STEP/ADR 状态与索引 | aligned |
| `git diff --cached --check` | exit 0 |

本步骤只改变术语和文档，没有运行代码测试。Runtime、Component 与 `.sapp` 行为未改变。

## 8. Metrics

基线：7 个文件包含旧称，共 41 处匹配。该计数来自 2026-07-14 的大小写不敏感文本搜索。

本步骤不生成运行时、性能或 AI 指标。

## 9. Risks and follow-ups

- `Host` 在技术语境中可能指宿主进程、宿主接口或产品，应通过完整名称和上下文区分；
- 面向普通用户的产品展示名称可以仍为 `Sico`，不需要暴露技术后缀；
- 后续代码、包名、日志字段和文档必须使用新术语；
- STEP-0003 继续原计划的语法错误注入工作。

## 10. Audit links

- ADR: [`ADR-0001`](../adr/ADR-0001-sico-host-terminology.md)
- commit subject: `docs(naming): [STEP-0002] rename Player to Host`
- next: STEP-0003 syntax error injection
