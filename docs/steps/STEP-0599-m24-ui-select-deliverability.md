# STEP-0599: M24 §8.6.1 — `select(ui-selection)` 可交付性判定（纯判定 + 修正案草案片；不实现/不改 WIT/不自裁）

> - status: **deliverability judged: NOT deliverable — contract/model gap (B)**（已确证为合同/模型缺口，非实现缺口；产出 PROPOSED 修正案草案 [`RFC-0052 Amendment A3`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md)；**不实现、不改 WIT、不改模型语义、不自裁**）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只读消费、不修改
> - deliverables: [`RFC-0052 Amendment A3 (PROPOSED)`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md)（新文件，修正案草案）
> - evidence: **只读核验 + 判定 + 起草**（无任何实现、无任何行为变更、无任何 WIT/源码/测试改动；证据为既有文件的逐行事实核验，见「可交付性判定与依据」）
> - **引用注意**：`runner/sico-runner` 与 `crates/sico-ui-renderer-d2d` **正被并发代理编辑**（STEP-0590/0597 记录同一约束）。本片全部 `文件:行号` 均为 **2026-11-07 在 worktree 实测锚点**，会漂移；实现者须对已提交快照重新锚定后再引用。符号名（`UiNode`、`UiTree::set_options`、`route_click_event`、`classify_window_event`）是稳定身份。

## 背景

`ui@1`（`sico:script/ui@1.0.0`）host-provider 面的事件词汇表为
`variant ui-event { click(ui-handle), select(ui-selection), closed }`
（`wit/script-ui-v1/ui.wit:59-63`），其中 `select(ui-selection)` 的载荷
`record ui-selection { handle: ui-handle, index: u64 }`（`ui.wit:53-56`）要求
「哪个 dropdown 的哪个索引被选中」。

STEP-0590 的 C-C 决策点是 **C3 现在不交付 click·select 并声明收窄，C1 之后补布局+命中测试**。
`click` 已由 **STEP-0592**（布局+命中测试）与 **STEP-0597**（runner 交付）达成；STEP-0597 的
结论是：**`select` 仍未交付**，模型**无选中状态**、shell **不产生 select 事件** ⇒ 需要新的
**模型能力**。本片就是对该结论的**独立复核与可交付性判定**：`select` 到底可不可交付，
判定为可交付则实现，判定为合同/模型缺口则出修正案草案——**本片判定为 (B)，不出实现**。

## 侦察（本片逐条复验，只读）

以下为**实际读文件**的结论（非转述 STEP 摘要）。括号内为证据来源。

### 1. dropdown **有**项（选项列表）——`index` 的定义域存在

- WIT 建 dropdown 时**携带**选项列表：`dropdown: func(parent: ui-handle, options: list<string>) -> result<ui-handle, ui-error>`（`wit/script-ui-v1/ui.wit:71`）；RFC-0052 §2 同形（`RFC-0052:30`）。
- 树模型存储该列表：`pub options: Vec<String>`（`crates/sico-ui-renderer-d2d/src/tree.rs:79`）；创建时经 `backend.rs:146-150`（`insert(NodeKind::Dropdown)` + `set_options`），可改经 `UiTree::set_options`（`tree.rs:228-249`）。
- **结论**：选项列表（`index` 的定义域）**在模型中存在**——**不是**「dropdown 无子项/无项列表」的情形。

### 2. 模型**无选中状态**——选中索引不可表示

- `UiNode` 全字段集为 `kind` / `parent` / `children` / `text` / `options` / `image` / `progress`（`tree.rs:69-84`）——**无 `selected` 字段**。渲染器核心 + runner 绑定全域搜索 `selected`/`selection`，仅命中**事件载荷类型** `UiSelection`/`UiEvent::Select`（`runner/sico-runner/src/ui.rs:79-86,95-97`）与 WIT/契约声明——**模型态字段处处无**。
- 状态 op 闭集为 `set-text` / `set-progress` / `draw-image`（`wit/script-ui-v1/ui.wit:76-80`；RFC-0052 §2 `set_text`/`set_progress`/`draw_image`——`RFC-0052:35-36`）。三者均**不设置选中**：
  - `set_text` 替换 `node.text`（`tree.rs:207-219`）——dropdown 自身标签文本，非选项索引；
  - `set_progress` 设置 `node.progress`（`tree.rs:258-268`）；
  - `draw_image` 设置 `node.image`。
- **结论**：**无 `set-selected` op、无 `selected` 字段、无选中生命周期**（无默认值、无改选项时清除规则）。

### 3. 无逐选项命中几何——坐标点击无法产出索引

- dropdown **不能有子节点**：`accepts_children` = `Root | Column | Row` 仅（`tree.rs:45-47`）——选项是 `Vec<String>`，**非子节点**。
- 布局把 dropdown 当**单一叶子盒**：落在叶子分支 `Text | Button | Dropdown | ImagePreview | Progress => (own_width, own_height)`（`layout.rs:429-433`），`natural_size(Dropdown) = (text_advance + DROPDOWN_CHEVRON_WIDTH + 2*CONTROL_PADDING_H).max(MIN_BUTTON_WIDTH), CONTROL_HEIGHT`（`layout.rs:470-476`）。
- 故**无逐选项矩形**：dropdown 内一点解析到 dropdown 自身（`Hit::Node(dropdown)`），runner 经 `route_click_event`（`ui.rs:304-316`）路由为 `click(dropdown_handle)`（`layout.rs:197` `hit`）。**坐标→选项索引无路径**。

### 4. 无下拉打开/选择 OS 消息

- 窗口事件词汇表闭集为 `Close` / `Resize` / `Paint` / `LeftButtonDown`（`events.rs:38-57` `RawMessage`；`events.rs:65-85` `WindowEvent`）；`translate` 确定映射（`events.rs:98-117`）。**无**「下拉项被选中」或「下拉被打开」消息 ⇒ shell 自身不产生 select 事件。

## 可交付性判定与依据

**判定：(B)——`select(ui-selection)` 属合同/模型缺口，非实现缺口。**

依据（逐条，均实测）：

| # | 事实 | 证据 | 对判定的作用 |
|---|---|---|---|
| 1 | 契约确有 `select(ui-selection)`，载荷 `{handle, index}` | `ui.wit:53-56,59-63`；`RFC-0052:39`；`ui.rs:79-86,95-97` | 交付需要「哪个 dropdown 的哪个索引」 |
| 2 | dropdown **有**项（`options: Vec<String>`） | `ui.wit:71`；`tree.rs:79`；`tree.rs:228-249`；`backend.rs:146-150` | `index` 定义域存在——**非**「无项列表」缺口 |
| 3 | 模型**无选中状态字段** | `tree.rs:69-84`（字段集无 `selected`）；全域 grep | 选中索引**不可表示** |
| 4 | 无设置选中的 op | `ui.wit:76-80`；`tree.rs:207-219,258-268`（set_text/set_progress 均非选中） | guest **无法确定性设置**选中 |
| 5 | 无逐选项命中几何 | `tree.rs:45-47`（dropdown 无子节点）；`layout.rs:429-433,470-476` | 坐标点击**无法产出索引**（只产 `click(dropdown_handle)`，`ui.rs:304-316`） |
| 6 | 无下拉选择 OS 消息 | `events.rs:38-57,65-85,98-117` | shell **不产生** select 事件 |

**结论**：`select` 的载荷要求「选中了哪个 dropdown 的哪个索引」，而当前模型**既无选中态表示
（3）、亦无设置通道（4）、亦无逐选项命中几何（5）、shell 亦不产 select 事件（6）**——虽
dropdown 有项（2），但「哪个索引被选中」在当前模型中**不可表达**。按 STEP-0597 的结论
（`STEP-0597:187-189`「模型无选中状态、shell 不产生 select 事件 ⇒ 需要新的模型能力」），
本片**独立复核确认**该结论，且进一步定位为**模型能力缺口**（非 hit-testing 缺口——hit-testing
STEP-0592 已落地，但它只服务 `click`，不服务 `select`）。

**诚实口径**：契约声明 `select`，实现从不产生——guest 轮询 `poll-event` 永不见 `select`，
无法区分「用户未选择」与「用户已选择但不可交付」。此即 STEP-0590 C-C 对 click 记录过的
「guest 不可观测跳过」同类缺口，但成因更深（缺模型能力，非缺命中测试）。

## 实施或修正案要点（B 分支——修正案草案）

产出 [`RFC-0052 Amendment A3 (PROPOSED)`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md)，
三案对比（均 owner 待裁，**本片不预裁、不实现**）：

| 案 | 内容 | 后果 | 不变量影响 | 兼容性 | 推荐片清单 |
|---|---|---|---|---|---|
| **A1** 合同收窄 | 在 RFC-0052 §2 + 用户文档写明「本版不交付 `select`，只交付 `click`/`closed`」；验收条款 G4 拆 click/select 分项 | 把 guest 不可观测跳过变为**声明式**收窄（同 STEP-0590 C3）；**不**增能力 | 无——`select` 判别式保持不可产生（同 A2 前的 `event-overflow` 形状） | wire 兼容，无 WIT 变更 | 文档 + 验收条款措辞，无代码 STEP |
| **A2** 扩展模型 | 引入 dropdown 项+选中态：`selected: Option<u64>` 字段 + 加法 `set-selected` op 和/或逐选项命中几何；`poll_event` 路由选中变化为 `select` | `select` 变**可交付**，索引由模型确定导出；从根闭合缺口 | 需新 ADR/合同定选中生命周期、与 `click` 关系、初始值；`layout.rs` 若加逐选项几何 ⇒ 改布局语料（`tests/layout_corpus.rs` 冻结）⇒ 需 owner 授权重冻结 | WIT 追加 op 为 wire 兼容（append-only `ui.wit:25-26`） | (a) 选中模型 ADR；(b) WIT 追加 `set-selected`；(c) 模型字段+op（tree.rs/backend.rs）；(d) 逐选项几何（layout.rs，涉语料重冻结）；(e) runner 路由+测试；(f) 编译器手编码实例类型同步 |
| **A3** 宿主侧事件 | `select` 由宿主从 OS 原生下拉控件产出，Sico UI 库只观测；宿主维护 handle↔原生控件映射 | 模型**保持无选中态**（无新字段/新 op）；`select` 成为宿主事件（同 `closed`/`click`） | 树模型 selection-free；`select.handle` 必须命名现有 dropdown 节点 ⇒ 宿主侧映射 | 无 WIT 变更（判别式+载荷已存在） | (a) 宿主原生控件 ADR；(b) 原生控件接线（renderer）；(c) 宿主侧映射；(d) runner 路由；(e) 测试 |

**推荐（咨询性，非约束）**：**A1 现在、A2 之后**。理由：镜像已接受的 C3-now-C1-later 模式
（STEP-0590 §4.4 → STEP-0592/0597 交付 `click`）；A1 是唯一与代码库诚实立场一致（声明收窄而非
静默跳过）且**零新能力、零 WIT 变更、零语料重冻结**的选项，立即消除 guest 不可观测跳过；
A2 是最终正确解（唯一让 `select` 成为**可交付**事件的选项），但是**新模型能力**，需独立
ADR/合同 + WIT 追加 + 布局语料决策 + 编译器手编码同步，不得并入本片；A3 是可行替代但把
选中所有权移出 Sico 模型（guest 只能收事件，不能确定性设置/观测选中），且需 renderer 原生
控件（更重），属不同架构姿态。**owner 待裁**。

## G1–G6 状态更新表

| # | 证据 | 本片前（STEP-0597） | 本片后 | 口径 |
|---|---|---|---|---|
| G1 | guest `window()` 真实窗口 | 已达成（STEP-0581，runtime-verified） | **已达成（不变）** | 不变 |
| G2 | guest `run()` 呈现帧证据 | 已达成（STEP-0591，宿主读回逐字节） | **已达成（不变）** | 不变 |
| G3 | guest `poll-event` 交付 `closed` | 已达成（STEP-0591） | **已达成（不变）** | 不变 |
| **G4** | guest `poll-event` 交付 `click(handle)`/`select` 且路由正确 | **`click(handle)` 达成；`select` 未达成** | **`click(handle)` 达成（不变）；`select` 未达成（不变）——判定为合同/模型缺口** | **G4a `click`：已达成（STEP-0597 口径）；G4b `select`：不可交付——模型无选中态/无设置通道/无逐选项几何/shell 不产 select 事件（见「可交付性判定」），属合同/模型缺口，须 A2 或 A3 后另片实现** |
| G5 | guest 级帧确定性 | 已达成（STEP-0591） | **已达成（不变）** | 不变 |
| G6 | clean-room pilot（gate 5） | 未达成 | **未达成（不变）** | 不变 |

**判定**：按 STEP-0590 §6 定义（G4 = click **且** select），**G4 未整体达成**（`select` 未交付
且为合同/模型缺口）；**G4 的 `click(handle)` 分项达成**。G1∧G2∧G3∧G5 不变，G6 不变。
**无任何 M24 §3 gate 翻转**（本片不实现、不产生任何新 UI 能力）。

## 验证（B 分支：只读核验 + 起草）

本片为 (B) 分支——无实现、无行为变更、无 WIT/源码/测试改动，故验证为**证据核验 + 文档卫生**：

- 逐条断言均以 `文件:行号` 锚定（见「可交付性判定与依据」表与修正案草案正文），无未锚定事实声明。
- 修正案草案的每条断言对应既有文件行号：`ui.wit:53-56,59-63,71,76-80`、`tree.rs:45-47,69-84,79,207-219,228-249,258-268`、`layout.rs:429-433,470-476`、`events.rs:38-57,65-85,98-117`、`backend.rs:146-150`、`ui.rs:79-86,95-97,304-316`、`RFC-0052:30,35-36,39`、`STEP-0597:187-189`。
- `git diff --check` → **净**（仅覆盖已跟踪文件；`docs/rfc/RFC-0052-amendment-A3-*.md` 与 `docs/steps/STEP-0599-*.md` 为**新未跟踪文件**，已另用 `grep -nP ' +$'` 检查无尾随空白）。
- 未跑任何 cargo 编译/测试：本片无代码改动，编译/测试结果不受影响（不声称任何测试已跑；既有 STEP-0597 的测试结果保持为上一片证据，未复跑）。
- 未跑 `validate-module-boundaries.ps1`：本片未改任何 crate 边界（仅新增文档），与 STEP-0592 基线（`MODULE_BOUNDARIES_OK model=openjdk-style packages=30 exceptions=4`）无涉。

## 诚实收窄（本片未声称的部分）

- **`select` 未交付、未实现**：本片判定为合同/模型缺口，**不出实现**、**不改 WIT**、**不改模型语义**、**不自裁**。`select` 仍为登记缺口（`STEP-0597:187-189`，本片确认并升格为模型能力缺口）。
- **无任何 M24 §3 gate 翻转**；`wit/**`、`crates/sico-ui-renderer-d2d/src/layout.rs` 与 `tests/layout_corpus.rs`、`tests/frame_corpus.rs`、`tests/event_corpus.rs`、`src/render/**`、`src/shell/**`、`src/tokens.rs`、`crates/sico-package/**`、`guests/**`、`selfhost/`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、既有 RFC 正文均未触碰。
- **修正案三案均标 PROPOSED**，未把任一写成已裁决；推荐为咨询性、非约束。
- **未独立复验**：编译器 hand-encoded `ui@1` 实例类型的精确位置（在实现 STEP 定位）；STEP-0338 帧语料/STEP-0577 事件语料/STEP-0592 布局语料哈希复跑（本片未跑，属既有结论，不下结论）。**已独立复验**：dropdown 有项（`tree.rs:79`）、模型无选中态（`tree.rs:69-84` 字段集）、无设置选中 op（`ui.wit:76-80`）、无逐选项几何（`layout.rs:429-433,470-476`）、shell 无 select 消息（`events.rs:38-57`）——非仅转述 STEP-0597 摘要。
- **未 commit/push**；未改任何既有文件；新增文件仅本片两个交付物（修正案草案 + 本 STEP）。

## 后续片建议

1. **owner 裁决**：A1（合同收窄）/ A2（模型扩展）/ A3（宿主侧事件）三选一（见修正案草案 §2/§5）。
2. **若 A2**：新 ADR（dropdown 选中模型）+ WIT 追加 `set-selected` + 模型字段/op + 逐选项几何（涉及 `layout_corpus.rs` 冻结哈希重冻结，**owner 授权**）+ runner 路由 `select` + 编译器手编码实例类型同步。
3. **若 A3**：宿主原生下拉控件（renderer）+ 宿主侧 handle↔控件映射 + runner 路由 `select`。
4. **验收条款**：把 G4 拆为 G4a `click` / G4b `select`（STEP-0590 §6），使两分项独立可判、独立跟踪。

## 建议主线审核点

- **判定正确性**：`select` 是合同/模型缺口（非实现缺口）——确认可接受，或要求先扩展模型再判 G4 整体。
- **dropdown 有项但无选中态**：确认「项列表存在」不隐含「选中可表达」——本片按「选中不可表达」判定。
- **G4 口径**：G4a `click` 达成 / G4b `select` 未达成（合同/模型缺口）；G4 整体未达成。确认此拆分口径可接受。
- **修正案三案**：A1 现在 + A2 之后（镜像 C3-now-C1-later）是否采纳；A3 的「宿主侧事件、模型 selection-free」姿态是否值得单独评估。
- **`crates/sico-ui-renderer-d2d/` 整目录未跟踪**：在途 M24 工作；提交/推送须待 owner 授权。

## 关系说明

本片是 STEP-0590 C-C 的 **`select` 子片**（`click` 子片已由 STEP-0592/0597 闭合）。它**独立复核**
并确认 STEP-0597 的「`select` 未交付」结论，定位为**合同/模型缺口**，产出修正案草案 A3；
**不**改变 STEP-0338 帧语料、STEP-0577 事件语料、STEP-0592 布局语料，**不**翻转任何 M24 §3
gate，**不**改变任何证据等级，**不**改变任何渲染字节或行为。
