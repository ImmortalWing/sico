# STEP-0590: M24 §8.6.1 — `ui@1` 合同空白修正案起草（纯合同起草片，不改代码/不实现/不自裁）

> - status: **proposed（draft；未提交未推送；本片不实现、不改行为、不触碰 WIT/源码/测试；等 owner 接受后另开实现 STEP）**
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - drafted: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本片只读消费、不修改
> - deliverables: [`RFC-0052 Amendment A2 (PROPOSED)`](../rfc/RFC-0052-amendment-A2-ui-error-render-failed-and-event-visibility.md)（新文件，PROPOSED 修正案草案）
> - evidence: **只读核验 + 起草**（无任何实现、无任何行为变更、无任何 WIT/源码/测试改动；证据为既有文件的逐行事实核验，见「核验」）
> - **引用注意**：`runner/sico-runner` 与 `crates/sico-ui-renderer-d2d` **正被并发代理编辑**（主线简报：3 个代理在改 runner/渲染器）。本片全部 `文件:行号` 均为 **2026-11-07 在 worktree 实测锚点**，会漂移；实现者须对已提交快照重新锚定后再引用。符号名（`fold_error`、`classify_window_event`、`MAX_QUEUE_EVENTS`、`ShellState::push`）是稳定身份。

## 背景

`ui@1`（`sico:script/ui@1.0.0`）host-provider 面在 STEP-0581（runner 侧接线）与 STEP-0577
（窗口壳/事件泵）落地后被「使用中」暴露了三处合同空白：**C-A** `ui-error` 无渲染失败判别式
（渲染侧失败被折叠为 `no-window`）、**C-B** `event-overflow` 对 guest 不可观测（`poll-event`
返回 `option`，无错误通道）、**C-C** `click(ui-handle)`/`select(ui-selection)` 的点→句柄命中
测试缺失（坐标点击被跳过且对 guest 不可观测）。本片是把这三处整理成 **一份 PROPOSED 修正案
草案**（交付物为 RFC-0052 Amendment A2），逐条以可执行证据核验，供 owner 裁决；**不**在片内
实现任何选项，**不**触碰 `wit/**`、源码、测试、既有 RFC 正文与任何计划/状态文档。

## 核验（本片逐条复验，只读；文档与实现冲突时取下界）

以下为**实际读文件**的结论（非转述 STEP 摘要）。括号内为证据来源。

### C-A：`ui-error` 闭集无渲染失败判别式 —— **事实确认**

- `ui-error` 闭集 6 case，wire 判别序 = 声明序（append-only）：`unknown-handle` / `wrong-parent-kind` / `limit` / `no-window` / `event-overflow` / `authority-denied`（`wit/script-ui-v1/ui.wit:29-36`，`ui.wit:25-26`）。
- runner 折叠 `fold_error`：`InvalidArgument→limit`（`runner/sico-runner/src/ui.rs:256`）；`NoWindow|UnsupportedPlatform|RenderFailed→no-window`（`ui.rs:260-262`）。渲染侧 `RenderFailed` 家族含：shell 线程丢失/协议失配（`crates/sico-ui-renderer-d2d/src/windowed.rs:178-197,262-268`）、窗口类注册/建窗/呈现/读回失败（`shell/win.rs:314-323,427-460,529-534,541-573,598-624,643-646,653-661`）。
- **guest 实际看到什么**：`window()`/`run()` 失败一律得 `no-window`；无法区分「窗口不存在」vs「平台不支持」vs「渲染/呈现/建窗失败」——**误导**为「无窗口」而非「渲染失败」。
- **其它被折叠掉的失败原因（逐一）**：①`InvalidArgument→limit`（`ui.rs:256`）——越界窗口尺寸（`windowed.rs:306-315`）、帧尺寸（`shell/win.rs:561-564,605-608`）被报成 `limit`；②`poll-event` 的 `Err(_)→None`（`ui.rs:369`）——shell 线程丢失被报成「无事件」（见 C-B）。
- 结论：`no-window` 折叠是**有记录的类型化折叠**（非 panic、非静默吞，`ui.rs:166-172,260-262`），但属于**有损**折叠，guest 无法判别，且 `no-window` 语义读法具有误导性。

### C-B：`event-overflow` 对 guest 不可观测 —— **事实确认**

- `poll-event: func() -> option<ui-event>` 无错误通道（`ui.wit:83`）；`ui-event` = `click|select|closed`（`ui.wit:59-63`）。
- 队列上限 `MAX_QUEUE_EVENTS = 256`（`events.rs:30`），预分配、按类型合并、超限置 `overflow` 而非静默丢弃（`shell/win.rs:86-131`；单测 `shell/win.rs:747-860`）。
- **`overflow` 标志从不经 guest 路径上浮**：只被 `take_overflow()`（`shell/win.rs:132-134`）消费，而 `take_overflow()` 只在 `drain()`（`shell/win.rs:514-524`）里读；`drain()` 仅被 shell 包装引用（`shell/mod.rs:134-136`），**无 guest 调用点**。guest 路径是 `WindowedUiBackend::poll_event→ShellClient::pump→shell.pump_once()`（`windowed.rs:453-455,218-223`；`shell/win.rs:465-512`），`pump_once()` 从不读该标志。
- **结论**：`event-overflow` 判别式（`ui.wit:34`）在闭集里声明了，但**当前面无任何 guest 可达的 producer**；guest 无法区分「无事件」与「事件被丢弃」。附带：shell 线程丢失→`None`（`ui.rs:369`），同为「无事件」信号。

### C-C：`click(ui-handle)`/`select(ui-selection)` 点→句柄命中测试缺失 —— **事实确认**

- 契约**确有** `click(ui-handle)`/`select(ui-selection)` 变体（`ui.wit:59-63`；RFC-0052 §2 `poll_event() -> Option[UiEvent]` `docs/rfc/RFC-0052-native-ui-source-binding-v0.md:39-40`）。
- 渲染器**无布局**：指针按下仅捕获为 client 坐标 `WindowEvent::Clicked{x,y}`（`events.rs:65-82,79-81`；`shell/win.rs:280-291`；负坐标过滤 `events.rs:109-113`）；点→句柄映射需渲染器布局（下一片），后端诚实不交付句柄（`windowed.rs:24-26`，`shell/mod.rs:27-30`；`STEP-0577:85-86`）。
- runner 把坐标点击**跳过**而非伪造：`poll_event`（`ui.rs:346-372`）经 `classify_window_event`（`ui.rs:277-285`）把 `Clicked→(None, skipped=true)`，`Closed→closed`，`Resized/Paint→(None,false)`；跳过点击计入宿主侧 `skipped_click_count`（`ui.rs:180,362`，getter `ui.rs:221-224`）——**STEP-0585 已把跳过改为宿主可观测**（「真实点击被忽略」可区分于「无事件」），但该计数**非 guest 契约成员**（`ui.rs:218-219`），故对 guest 仍不可观测。
- **结论**：契约承诺 `click(handle)` 交付，实现从不交付；guest 轮询 `poll-event` 永不见 `click`，无法区分「无点击」与「点击被跳过」——**对契约可见事件的 guest 不可观测跳过**（宿主侧已计数）。STEP-0581 已登记为残余（`STEP-0581:118-119`）；STEP README 记为须显式化的能力缺口（`docs/steps/README.md:519`）。

### 核验结论汇总

三处空白**全部事实确认**，无纠正；两处文档/实现张力记录如下（均取下界）：
1. STEP-0581 自陈「guest 可见 UI = 部分达成」（`README.md:519`）vs M24 出口审计 v2「当前无 guest 可见 UI 能力」（`README.md:521`）——**口径不同非矛盾**，由本片「验收条款」以清单化定义消解（G1 已达成 = STEP-0581 的「部分达成」；G2∧G3∧G4∧G5 未达成 = 审计的「无能力」）。
2. RFC-0052 §3 声明「events coalesce by kind, never drop silently into wrong delivery」（`RFC-0052:53-58`）与实现一致（合并/overflow 标志，`shell/win.rs:86-131`），但该声明**当前不可观测**（§C-B）——取下界：声明成立、可观测性不成立。
3. STEP-0585 的「显式登记缺口 + 可观测化」**已在 worktree 落地宿主侧可观测**（`ui.rs:175-180,212-224,277-285`），但**guest 侧仍不可观测**——本片按此复验，未只依赖主线简报。

## 决策点 A —— `ui-error` 渲染失败判别式

- **A1**：向 `ui-error` **追加** `render-failed`（及同批 `invalid-argument`，收编 `InvalidArgument→limit` 折叠 `ui.rs:256`）。追加在**末尾**保持 0–5 判别序（append-only `ui.wit:25-26`）；wire 兼容（既有 guest 若穷尽 match 需补 case，否则不变）。变更集：runner `UiError`+`fold_error`（`ui.rs:40-59,249-264`）+ 编译器手编码实例类型（两者都不解析 WIT `ui.wit:9-11`）+ 穷尽 match 的 guest fixture；需版本化说明。
- **A2**：维持折叠，在 RFC/ADR 与用户文档**明确声明**「`no-window` 覆盖渲染侧失败」；**无** guest 可判别替代手段（唯一宿主侧 `window_title()` `ui.rs:208-210` 非 guest 契约成员）。只修文档、不除歧义。
- **影响**：A1 让 guest 区分「窗口不存在 / 渲染失败 / 平台不支持」与「非法参数 / limit」；A2 不除歧义。
- **推荐**：**A1**（同批追加 `render-failed` + `invalid-argument`）；A2 是诚实的过渡姿态但永久保留歧义。

## 决策点 B —— `event-overflow` 可观测化

- **B1a**：`poll-event` 返回 `result<option<ui-event>, ui-error>`（cap 命中→`err(event-overflow)`；线程丢失→`err(render-failed)` 替代 `None` `ui.rs:369`）。**破坏性**：改变 wire 签名，既有 guest 二进制约需重编译；需版本提升或新 `ui@1.1`。
- **B1b**：**增量**伴生查询（如 `poll-event-overflow: func() -> result<bool, ui-error>`，复用 shell 一次性 `overflow` 标志 `shell/win.rs:132-134`）。**非破坏**：既有 `poll-event` 不动；`event-overflow` 变为可产生。
- **B2**：维持现状，在 RFC/用户文档登记「事件队列满可能丢事件，cap 256，overflow 经 `poll-event` 不可观测」，要求使用者知悉。不除歧义。
- **影响**：B1a/B1b 均使 `event-overflow` 可达、泵失败类型化；B2 维持「无事件」与「丢弃」的歧义。实际暴露面窄（队列按类型合并，仅非合并点事件可溢出 `shell/win.rs:106-127`；`closed` 终态 `shell/win.rs:89-98`）但非零。
- **推荐**：**B1b**（增量查询），并与 A1 **合并进同一次 WIT 追加**避免二次 bump；B1a 简单但破坏 ABI；B2 是诚实现状。

## 决策点 C —— `click(ui-handle)`/`select` 点→句柄命中测试

- **C1**：为渲染器补**布局 + 命中测试**（新能力、实现量大，需单独 ADR/合同评估——渲染器当前无布局 `windowed.rs:24-26`）。闭合 C-C；guest 收到真实 `click(handle)`。
- **C2**：布局落地前把**每个**坐标点击交付为 `click(root-handle)` 并声明收窄。**比跳过更糟**：子元素点击被误归属到根（错误句柄），违反「诚实不伪造句柄」立场（`ui.rs:347-356`，`STEP-0581:58-60`）。
- **C3**：布局落地前**不交付** click/select，把收窄写入契约 + 用户文档（default-deny 型诚实收窄；跳过即现状 `ui.rs:346-372`，宿主侧已计数 `ui.rs:221-224`）。除「guest 不可观测跳过」的诚实缺口；不增能力（能力仍属 C1）。
- **影响**：C3 需文档 + 在途 STEP-0585 显式化；C1 需新能力片 + 新 ADR；C2 被否。
- **推荐**：**C3 现在、C1 之后**。C2 因伪造错误句柄被否。

## 推荐（汇总）

- **A1**：追加 `render-failed`（+同批 `invalid-argument`）——修 `no-window` 有损折叠与误导。
- **B1b**：增量 overflow 查询，与 A1 合并为一次 WIT 追加——让 `event-overflow` 判别式可产生，且非破坏。
- **C3 现在、C1 之后**：把 click/select 的 guest 不可观测跳过改为声明式诚实收窄；布局+命中测试作为后续新能力片（需新 ADR）。
- 三项均为**契约/文档决定**，属 owner 裁决；本片**不实现**、不改行为、不触碰 WIT/源码/测试。

## 影响面

- **契约（WIT）**：A1+B1b 追加 `ui-error` case + 新查询函数——需 owner 接受 + 版本化说明 + RFC-0052 §2/§3 与用户文档同批更新；B1a 则另需签名变更（破坏）。C3 只改文档措辞，不改 WIT。
- **runner**：A1 改 `UiError`+`fold_error`（`ui.rs:40-59,249-264`）；B1b 加 `link_ui` 一个 `func_wrap`（`ui.rs:463-589`）；两者均需配套手编码 guest fixture（若有穷尽 match）。B1a 则改 `poll-event` 签名（`ui.rs:346-372,569-581`）。
- **编译器**：A1/B1a 需同步 hand-encoded `ui@1` 实例类型（不解析 WIT `ui.wit:9-11`）——具体位置在实现 STEP 定位，本片不预判。
- **渲染器**：C1（布局+命中测试）是新能力，属未来片；A1/B1/C3 **不需要**渲染器改动。
- **文档**：RFC-0052 §2/§3、用户文档、STEP README/STATUS（在实现 STEP 中同步，本片不动）。
- **兼容性**：A1+B1b wire 兼容（append-only `ui.wit:25-26`）；B1a 破坏。既有 guest 除非穷尽 match `ui-error`，否则不变。

## 验收条款 ——「什么算达成 guest 可见 UI 能力」（可判定）

**定义**：guest 可见 UI 能力达成 ⇔ **G1 ∧ G2 ∧ G3 ∧ G4**（guest 能建真实窗口、呈现并观测帧、收到 `closed` 事件、收到按节点路由的点击）；**G5** 为确定性伴随、**G6** 为独立 pilot/gate-5 项。

| # | 证据 | 可执行核验 | 当前 |
|---|---|---|---|
| G1 | guest `window()` 真实窗口 | guest fixture 调 `window()` 得 `Ok(handle)`；宿主证明真实 OS 窗口存在（`window_title()` 为 `Some` `ui.rs:208-210`） | **已达成**（STEP-0581:114-117，runtime-verified） |
| G2 | guest `run()` 呈现帧证据 | guest fixture 调 `run()`；宿主 `read_presented_frame()` 返回预期 client 尺寸且帧逐字节一致（`windowed.rs:483-485`） | **未达成（仅宿主侧）**（STEP-0581:115-117） |
| G3 | guest `poll-event` 交付 `closed` | guest fixture close 后泵 `poll-event` 观察到 `closed` | **未达成（仅宿主侧）** |
| G4 | guest `poll-event` 交付 `click(handle)`/`select` 且节点路由正确 | guest fixture 在已知子节点边界点击，收到 `click(该句柄)`；需命中测试 | **未达成**（命中测试缺失，§C-C） |
| G5 | guest 级帧确定性 | guest 同树渲染 N 次→N 帧逐字节一致（ADR-0018 §3 `ADR-0018:40-44`） | **未达成（仅渲染器核心语料，STEP-0338）** |
| G6 | clean-room pilot 应用（gate 5） | Sico 消费者应用经 `ui@1` 构建/运行真实窗口 | **未达成**（STEP-0581:138） |

**判定**：按此定义**当前未达成**（仅 G1）；STEP-0581 的「部分达成」= G1 达成；M24 审计 v2 的「无能力」= G2∧G3∧G4∧G5 未达成。G1–G5 为能力最小证据集，G6 为 pilot/gate-5 独立分项（`README.md:521` 已要求与能力分项分开记）。

## 未决问题（owner 裁决项，本片不预裁）

1. A1 vs A2（`no-window` 有损是否 v0 可接受）。
2. B1b vs B1a vs B2（增量查询 / 破坏性签名 / 声明式限制）。
3. C1 vs C3（先建布局+命中测试新能力，还是先声明收窄）。
4. A1+B1b 合并一次 WIT 追加 vs 两次。
5. 版本化形式（`@1.0.0` append 记录 vs `@1.1.0` wire 注记）。
6. 「guest 可见 UI 能力」定义是否采纳本片 §6 清单（或放宽为 G1 即部分能力）。
7. STEP-0585 在途范围是否覆盖 C-C 的声明式收窄（C3），或需另开一片（宿主侧 `skipped_click_count` 已落地 `ui.rs:175-180,212-224,277-285`；guest 侧声明收窄仍开放）。

## 诚实收窄（本片未声称的部分）

- 本片**只起草**：不实现任何选项、不改任何行为、不触碰 `wit/**`、源码、测试、既有 RFC 正文、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、`docs/plans/**`。
- 所有选项均标 **PROPOSED**，未把任一写成已裁决；推荐为咨询性、非约束。
- **未独立复验**：编译器 hand-encoded 实例类型的精确位置（在实现 STEP 定位）；STEP-0338 帧语料与 STEP-0577 事件语料的哈希复跑（本片未跑，属既有结论，不下结论）。**已独立复验**：STEP-0585 的宿主侧 `skipped_click_count` 可观测（`ui.rs:175-180,212-224,277-285`）——非仅转述简报。
- **引用注意**：全部 `文件:行号` 为 2026-11-07 worktree 实测锚点；runner/渲染器正被并发代理编辑，实现时须对已提交快照重新锚定。
- 未翻任何 M24 §3 gate；本片不改任何证据等级、不声称任何新 UI 能力（布局/命中测试/文件对话框/pilot 均仍为登记残余，STEP-0581:133-138）。
- 未 commit/push；未改既有文件；新增文件仅本片两个交付物。

## Addendum 2026-11-07 — G4 拆分口径（G4a `click` / G4b `select`）

> - date: 2026-11-07
> - 状态: **登记（只陈述现状，不预裁 owner 选择）**。本 addendum 把 §6 验收条款的 **G4**
>   拆为 **G4a（`click`）/ G4b（`select`）** 两个独立可判分项，并登记各自当前状态；**不**
>   改变 §6 的 G4 整体定义、**不**裁决任何选项、**不**修改任何既有正文（本节为追加）。
> - 依据: [`STEP-0597`](./STEP-0597-m24-runner-click-handle-delivery.md)（G4a 分项达成）、
>   [`STEP-0599`](./STEP-0599-m24-ui-select-deliverability.md)（G4b 判定为合同/模型缺口）、
>   [`RFC-0052 Amendment A3 (PROPOSED)`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md)
>   （三案未裁决）。本 STEP 只消费、不修改上述任一。

### G4a `click(handle)` —— 已达成（收窄口径）

- **现状**：`ui@1` 的 `poll-event` 现在交付 `click(ui-handle)`，且节点路由正确（最深命中 ⇒
  该句柄；`Node(root)` ⇒ 根句柄**且仅在确实无子节点覆盖时**；窗外 `Miss` ⇒ 不交付并计入宿主
  侧 `skipped_click_count`；布局失败 ⇒ 类型化 `None`，闭集无错误通道的诚实信号）。生产路由
  seam 为 `route_click_event`（`runner/sico-runner/src/ui.rs`），配合 `layout_input()`
  （实时客户区尺寸 + 树快照）。
- **口径（诚实收窄）**：坐标由**宿主注入**，被旁路的仅是 OS `WM_LBUTTONDOWN` + `translate`
  （纯确定映射，已被冻结事件语料覆盖）⇒ **不得**写成「真实鼠标点击已验证」。证据等级
  `internal-fixture`（真实 guest 组件经真实 `Runner` + 生产 `WindowedUiBackend`）。
- **可复核入口**：`runner/sico-runner/tests/ui_click_routing.rs` 的
  `guest_observes_click_on_the_button_child` /
  `guest_observes_click_on_the_root_when_no_child_covers_the_point` /
  `a_click_outside_the_window_is_a_skip_not_a_delivery` /
  `denied_guest_is_typed_authority_refusal_with_zero_backend_touch`（4/4 绿）。

### G4b `select(ui-selection)` —— 不可交付（合同/模型缺口，非实现缺口）

- **现状**：契约确有 `select(ui-selection{handle,index})`（`wit/script-ui-v1/ui.wit:53-56,59-63`），
  dropdown 也有 `options`（索引定义域存在），但当前**模型无选中态字段**（`UiNode` 字段集无
  `selected`）、**无设置选中的 op**（状态 op 闭集为 `set-text`/`set-progress`/`draw-image`）、
  **无逐选项命中几何**（dropdown 无子节点、布局为单一叶子盒）、**shell 不产生 select 事件**
  （窗口事件闭集为 `Close/Resize/Paint/LeftButtonDown`）⇒ guest 轮询 `poll-event` **永不见
  `select`**，无法区分「用户未选择」与「用户已选择但不可交付」。
- **判定**：`select` 属 **合同/模型缺口**（STEP-0599），非实现缺口。修正案
  `RFC-0052-A3`（PROPOSED）给出**三案**（A1 合同收窄 / A2 模型扩展 / A3 宿主侧事件），
  **均未裁决** ⇒ 本 addendum 只陈述现状，**不预裁 owner 选择**。G4b 保持**未达成**。

### G4 整体

按 §6 定义（G4 = click **且** select），**G4 未整体达成**（G4a 达成、G4b 未达成）。本
addendum 不改变「guest 可见 UI 能力未整体达成」的判定，不翻任何 M24 §3 gate。
