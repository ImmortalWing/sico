# STEP-0607: M24 §8.6.1 — `RFC-0052 Amendment A3` owner 裁决登记（A1 合同收窄；纯登记/文档片，零代码/零 WIT/零测试/零语料）

> - status: **adjudication registered: A1 (narrow the contract) ACCEPTED by owner decision 2026-11-07**（本片把 owner 已作出的裁决**补登进权威登记面**，使登记面与工作树中已正确反映 A1 的 4 个 M24 文件口径一致；**A2/A3 未采纳、挂起**；**不交付 `select`、不翻任何 gate**）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流），`RFC-0052-A3` 三案裁决的权威登记
> - completed: 2026-11-08（裁决作出于 2026-11-07）
> - contracts: [`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md) §2 + [`RFC-0052 Amendment A3`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md)（**ACCEPTED — A1**）+ [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted）——本片只读消费、不修改任何契约正文
> - deliverables: 本 STEP + `docs/steps/README.md` 表格行 + [`m24-exit-audit-2026-11-07-refresh`](../reports/m24-exit-audit-2026-11-07-refresh.md) §7 附录 STEP-0599 行的 A3 状态订正 + [`docs/STATUS.md`](../STATUS.md) 顶部 2026-11-08 dated 登记
> - evidence: **纯登记/文档片，零代码改动**——无 WIT/源码/测试/语料改动、无行为变更、无任何 cargo 编译/测试；证据为既有文件的**口径一致性核验**（见「登记一致性核验」）

## 背景：登记面与工作树的口径不一致

owner 于 **2026-11-07** 就 [`RFC-0052-A3`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md) 的三案（A1 合同收窄 / A2 扩展模型 / A3 宿主侧事件）作出了**真实裁决 = A1（合同收窄）**，并授权补登登记链。

裁决作出后，工作树里 **4 个 M24 文件已正确**反映 ACCEPTED-A1：

| 文件 | 已正确之处 |
|---|---|
| [`RFC-0052-amendment-A3`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md) | 状态头 = **ACCEPTED — A1**；§0/§2.1/§2.2/§2.3/§3.4/§5.1/§6 均标注 A1 accepted、A2/A3 deferred |
| [`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md) §2 | 事件词汇收窄块标注「owner decision 2026-11-07，`RFC-0052-A3` A1」，`select(handle,index)` = **declared but not produced** |
| [`M24 计划 §8.6.1`](../plans/M24-vision-closure-gui-application-pilot.md) | 状态块记「the owner adjudicated `RFC-0052-A3` = A1 (narrow the contract) on 2026-11-07」，并明示 **not a gate flip** |
| [`NATIVE-UI`](../user-guide/NATIVE-UI.md) | G4b `select` 行 + §「已知限制」记「三案已由 owner 于 2026-11-07 裁决为 A1」，A2/A3 未采纳、挂起 |

**缺的是权威登记面的第二条记录**：[`m24-exit-audit-2026-11-07-refresh`](../reports/m24-exit-audit-2026-11-07-refresh.md) §7 附录的 STEP-0599 行仍写「三案见 `RFC-0052-A3` PROPOSED」，与上述 4 文件不一致。本片补齐该记录（**只同步登记面，不改裁决**——裁决权属 owner，本片不自裁）。

## 裁决内容（如实登记，不重述为推荐）

- **A1（合同收窄）= ACCEPTED（owner，2026-11-07）**：在 [`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md) §2 与用户文档写明「本版 `ui@1` 只交付 `click(handle)` / `closed`，`select(ui-selection)` **已声明但不产出**」，并把验收条款 G4 拆为 G4a `click` / G4b `select` 分项。**A1 是 documentation-only 诚实收窄**：
  - **零 WIT 变更**（`select` 判别式与 `ui-selection` 载荷保留在 `wit/script-ui-v1/ui.wit` 中，append-only、wire 兼容）；
  - **零代码、零测试、零语料改动**；无行为变更；
  - 把 guest 的「不可观测跳过」变为**声明式收窄**（同 STEP-0590 对 click 的 C3 姿态），guest 读契约即知 `select` 不会到达。
- **A2（扩展模型）/ A3（宿主侧事件）= NOT TAKEN / deferred**：保留在修正案正文作为**历史选项分析**，非计划；A2 仍是 `select` 的最终正确解（需新 ADR + WIT 追加 `set-selected` + 模型字段/op + 逐选项几何 + 语料重冻结授权 + runner 路由 + 编译器 hand-encoded 同步），A3 需宿主原生下拉控件 + handle↔控件映射，两者均**未排期、未开工**。

## 诚实口径：A1 不交付 `select`、不翻任何门（本片核心不变量）

- **G4b `select` 仍「不可交付/未交付」**：A1 收窄的是**契约声明**，不是交付能力。`select(ui-selection{handle,index})` 需「哪个 dropdown 的哪个索引被选中」，而当前模型**无选中态字段**（`UiNode` 字段集无 `selected`）、**无设置选中的 op**（状态 op 闭集 = set-text/set-progress/draw-image）、**无逐选项命中几何**（dropdown 无子节点、布局为单一叶子盒）、**shell 不产 select 事件**（窗口事件闭集仅 Close/Resize/Paint/LeftButtonDown）⇒ guest 轮询 `poll-event` **永不见 `select`**（逐条实测见 [`STEP-0599`](./STEP-0599-m24-ui-select-deliverability.md)）。**A1 不改变其中任何一条。**
- **G4 整体仍未达成**：按 STEP-0590 §6（G4 = click **且** select），G4a `click(handle)` 达成（STEP-0597）、G4b `select` 未达成 ⇒ **G4 未整体达成**。
- **G6 仍未提供**（gate 5，clean-room pilot；`#3 JPEG` 外部门）。
- **未翻任何 M24 §3 gate**：A1 与本片**不产生任何新 UI 能力**，无 gate 文本移动。
- **M24 overall 仍 NO-GO**（见 [`m24-exit-audit-2026-11-07-refresh`](../reports/m24-exit-audit-2026-11-07-refresh.md)）。

## 登记一致性核验（本片唯一「证据」）

本片为登记/文档片，证据 = 逐处核验「登记面表述」与「工作树已正确文件」是否一致：

| 处 | 改前 | 改后 | 与工作树一致 |
|---|---|---|---|
| [`m24-exit-audit-2026-11-07-refresh`](../reports/m24-exit-audit-2026-11-07-refresh.md) §7 附录 STEP-0599 行 | 「三案见 `RFC-0052-A3` PROPOSED」 | 三案 **已于 2026-11-07 由 owner 裁决为 A1（合同收窄）**，A1 = documentation-only 诚实收窄、`select` 仍不交付；G4b `select` 仍不可交付/未交付、非 gate flip | ✓ 对齐修正案状态头 |
| [`docs/STATUS.md`](../STATUS.md) 顶部 | 无 M24 裁决登记 | 新增 2026-11-08 dated 登记（owner 确认 A1 真实裁决 + 授权补登 + A2 未动 + 未翻门） | ✓ |
| [`docs/steps/README.md`](./README.md) | 无本片行 | 追加 STEP-0607 行 | ✓ |
| [`AGENTS.md`](../../AGENTS.md) §2 M24 段 | **无** `RFC-0052-A2/A3`「PROPOSED 未裁决」类表述（该段仅记 `#5 ui@1` 绑定授权与 ADR-0018 两线登记差异） | **无需订正**（无 A3 PROPOSED 待裁表述可改；A2 状态亦未在此出现） | ✓ 无需改动 |

## 未做的事（诚实收窄）

- **未动 4 个已正确文件**：修正案 A3、`RFC-0052` 正文、M24 计划、`NATIVE-UI` 均**保留不动**（它们已正确反映 A1）。
- **未动 A2**：[`RFC-0052-amendment-A2`](../rfc/RFC-0052-amendment-A2-ui-error-render-failed-and-event-visibility.md) **仍 PROPOSED 未裁决**，状态未触碰。
- **未实现 `select`、未改 WIT/源码/测试/语料**：本片零代码，无任何 gate 翻转。
- **未跑 cargo**：无代码改动，编译/测试结果不受影响（不声称任何测试已跑）。
- **未改历史 STEP 正文**：[`STEP-0599`](./STEP-0599-m24-ui-select-deliverability.md) 保留原文（其「PROPOSED，owner 待裁」是裁决前的历史事实），登记订正落在出口审计 §7 附录、STATUS 与本片。
- **未 commit/push**。

## 后续片（均未排期，需 owner 授权）

1. **若 owner 未来采纳 A2**：新 dropdown 选中模型 ADR + WIT 追加 `set-selected` + 模型字段/op（`tree.rs`/`backend.rs`）+ 逐选项命中几何（涉 `layout_corpus.rs` 冻结哈希重冻结，须 owner 授权）+ runner `select` 路由 + 编译器 hand-encoded 实例类型同步。
2. **若 owner 未来采纳 A3**：宿主原生下拉控件（renderer）+ 宿主侧 handle↔原生控件映射 + runner `select` 路由。
3. **G4b/G6 收口**：无论 A2/A3，`select` 交付与 gate 5 pilot 均需另片实现，且 gate 5 的 `#3 JPEG` 加速器仍为具名外部门。