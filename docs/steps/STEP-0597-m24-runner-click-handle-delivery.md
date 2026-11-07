# STEP-0597: M24 §8.6.1 — runner 侧交付 `click(ui-handle)`（G4 达成片；布局+命中测试已由 STEP-0592 落地）

> - status: **implementation complete locally**（runner 侧把坐标 `Clicked` 经布局+命中测试路由为 `click(ui-handle)` 并跨 guest 边界交付；`select` 明确登记为不可交付；**不翻任何 M24 §3 gate、不改变任何证据等级**）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改
> - evidence: **internal-fixture**（真实 guest 组件经真实 `Runner` + 生产 [`WindowedUiBackend`](`../../crates/sico-ui-renderer-d2d/src/windowed.rs`) 的 `click(handle)` 交付 + 路由正确；Windows x64 复跑；**非** pilot 级、**非** 生产声明）
> - artifacts: [`runner/sico-runner/src/ui.rs`](`../../runner/sico-runner/src/ui.rs`)、[`crates/sico-ui-renderer-d2d/src/windowed.rs`](`../../crates/sico-ui-renderer-d2d/src/windowed.rs`)、[`crates/sico-ui-renderer-d2d/src/backend.rs`](`../../crates/sico-ui-renderer-d2d/src/backend.rs`)、[`runner/sico-runner/tests/ui_click_routing.rs`](`../../runner/sico-runner/tests/ui_click_routing.rs`)

## Objective

STEP-0590 的 C-C 与 STEP-0585 的剩余缺口 #1 指向：`ui@1` **确有** `click(ui-handle)` /
`select(ui-selection)` 变体（`wit/script-ui-v1/ui.wit:59-63`），但点→句柄命中测试缺**渲染器
布局**——坐标点击被宿主侧跳过（`skipped_click_count`，STEP-0585）且「任意点归根」已被明确
否决（无布局 ⇒ 子元素无矩形 ⇒ 错误归属）。STEP-0592 落地了纯计算布局 + 命中测试（
`layout::compute` / `Layout::hit` / `Hit::Node|Miss`，`Node(root)` ⇔ 窗内但无子节点覆盖，
`Miss` ⇔ 窗外；坐标 = client 绝对坐标，命中用半开区间 `[x,x+w)`），但**未接线**：G4 还需
runner 侧把 `WindowEvent::Clicked{x,y}` 经 `compute` + `hit` 映射为 `click(handle)` 并跨
guest 边界交付。本片就是该接线片：**`poll-event` 现在交付 `click(handle)` 且路由正确**。

## 侦察结论（本片逐条复验，只读）

以下为**实际读文件**的结论（非转述 STEP 摘要）。括号内为证据来源。

### `Hit` / `Rect` / `compute` 的确切签名（`crates/sico-ui-renderer-d2d/src/layout.rs`）

- `pub struct Rect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }`（`layout.rs:62-71`）；
  `Rect::contains` 用半开区间 `[x, x+w) × [y, y+h)`（`layout.rs:73-81`）——右/下边**独占**。
- `pub enum Hit { Node(u64), Miss }`（`layout.rs:102-113`）——`Node(root)` ⇔ 窗内但**无任何
  子节点矩形覆盖**（窗口 padding、容器空隙）；`Miss` ⇔ 窗外（或退化子矩形延伸到窗外仍为
  Miss，`layout.rs:200-202`）。
- `pub fn compute(tree: &UiTree, width: u32, height: u32) -> Result<Layout, UiBackendError>`
  （`layout.rs:295`）：无根 → `Err(NoWindow)`；`width==0 || height==0` → `Err(InvalidArgument{client-size})`；
  畸形/过深树 → `Err(UnknownHandle/Limit)`。纯函数、跨机器确定（冻结语料 `tests/layout_corpus.rs::CORPUS`）。
- `pub fn hit(&self, x: i32, y: i32) -> Hit`（`layout.rs:197`）：取**最深**包含点节点；点出界
  （`px<0||py<0||px>=w||py>=h`）→ `Miss`。
- 坐标 = **client 绝对坐标**（原点客户区左上，96-DPI 逻辑像素），与 `WindowEvent::Clicked{x,y}`
  同一空间（`shell/win.rs:280-291`，`events.rs:109-113` 已过滤负坐标）。

### 生产后端是否暴露「当前客户区尺寸」与「当前树」——**未暴露，本片新增只读 affordance**

- `WindowedUiBackend`（`windowed.rs`）暴露 `window_title()`（转发 `D2dUiBackend::window_title`），
  **不暴露**树与客户区尺寸；`D2dUiBackend` 的 `State { tree: UiTree, window: Option<WindowSpec> }`
  是私有的（`backend.rs:22-33`）。
- shell 侧已有 `WindowShell::client_size() -> Result<(u32,u32), UiBackendError>`（实时
  `GetClientRect`，`shell/win.rs:663-670`），但**不跨 `ShellClient` 通道暴露**。
- **本片新增**（最小、类型化、只读）：
  - `D2dUiBackend::tree_snapshot() -> Result<UiTree, UiBackendError>`（`backend.rs`）——锁下克隆
    当前树（纯数据快照），不触渲染路径。
  - `ShellCommand::ClientSize` + `ShellReply::ClientSize`（`windowed.rs`）+ `ShellClient::client_size()`
    ——经既有 request/reply 通道读 shell 的**实时**客户区尺寸（`GetClientRect`）。
  - `WindowedUiBackend::layout_input() -> Result<(u32, u32, UiTree), UiBackendError>`（`windowed.rs`）
    ——组合上述两者；`Err(NoWindow)` 无窗口，`Err(RenderFailed)` 壳线程丢失/锁中毒。

### `WindowEvent::Clicked` 与 `classify_window_event` 现状（`events.rs` / `ui.rs`）

- `WindowEvent::Clicked { x: i32, y: i32 }`（`events.rs:79-84`）——`shell/win.rs:280-291`
  `WM_LBUTTONDOWN` → `translate(RawMessage::LeftButtonDown{x,y})` → `Clicked`；负坐标被过滤
  （`events.rs:109-113`），故 `x,y ≥ 0`（可等于 `width`/`height`）。
- `classify_window_event`（`ui.rs:277-285`，STEP-0585）把 `Clicked → (None, skipped=true)`，
  `Closed → (Some(closed), false)`，`Resized/Paint → (None, false)`——坐标点击**被跳过**而非
  伪造；跳过计入宿主侧 `skipped_click_count`（`ui.rs:175-180,212-224`）。
- 既有 guest 级 fixture `tests/ui_guest_visible.rs` 的 `ReadbackBackend::poll_event` 仍把
  `Clicked` 跳过（`ui_guest_visible.rs:205-221`）——**只读参考，本片不改**。

### `ui-selection` 的确切形状与可否交付（`wit/script-ui-v1/ui.wit`）

- `record ui-selection { handle: ui-handle, index: u64 }`（`ui.wit:53-56`）——一个**选中项**：
  dropdown 节点句柄 + 选中索引。
- `variant ui-event { click(ui-handle), select(ui-selection), closed }`（`ui.wit:59-63`）。
- **`select` 当前不可交付**：`ui-selection` 的载荷要求「选中了哪个 dropdown 的哪个索引」，
  而本模型的唯一事件源是 `WindowEvent::Clicked`（坐标），**无任何选中状态/下拉打开-选择语义**
  ——shell 不产生 select 事件、树模型无 selection 字段、布局无命中「下拉项」的概念。故本片
  **只交付 `click`，`select` 明确登记为「本片不覆盖，仍未达成」**（见「诚实收窄」）。

## 实现

### 1. renderer 只读 affordance（`backend.rs` / `windowed.rs`）

- `D2dUiBackend::tree_snapshot()`：`Ok(self.lock()?.tree.clone())`——锁下克隆纯数据树。
- `windowed.rs`：`ShellCommand::ClientSize`/`ShellReply::ClientSize` + `shell_thread` 匹配臂 +
  `ShellClient::client_size()`（经 request/reply 读 `shell.client_size()`）。
- `WindowedUiBackend::layout_input() -> Result<(u32,u32,UiTree), UiBackendError>`：
  `(shell.client_size()?, tree.tree_snapshot()?)`——**实时**客户区尺寸 + 树快照；只读、类型化、
  不触渲染路径/事件队列。这是本片给 G4 的最小暴露 seam。

### 2. runner 路由（`ui.rs`）

- `classify_window_event(event) -> Option<UiEvent>`（**改签名**）：`Closed → Some(closed)`；
  `Clicked/Resized/Paint → None`。坐标点击**不再**由它分类——它需要布局，交由 `route_click_event`。
- `pub fn route_click_event(x, y, tree: &UiTree, width, height) -> Result<Option<UiEvent>, UiBackendError>`
  （**新增，`pub` 供 e2e 测试适配器复用生产路由**）：
  `layout::compute(tree,width,height)?` 然后 `layout.hit(x,y)`：
  - `Hit::Node(h) → Ok(Some(click(ui-handle{h})))`——最深节点：子/叶子 → 自身；容器空隙 → 容器；
    窗口 padding → `Node(root)`。
  - `Hit::Miss → Ok(None)`——窗外，调用方计为跳过。
  - `Err → 类型化失败`——不伪造事件。
- `D2dUiRenderer::poll_event`（**重写**）：`Clicked` 时取 `self.backend.layout_input()` →
  `route_click_event`：`Ok(Some)` → 交付；`Ok(None)`（Miss）→ `skipped_click_count += 1`（沿用
  STEP-0585 可观测通道）并继续泵；`Err`（布局失败，宿主不变量）→ 返回 `None`（闭集
  `option<ui-event>` 无错误通道，诚实信号）；`layout_input` 自身 `Err`（无窗口/壳线程丢失）→
  `None`。`Closed → closed`；`Resized/Paint → 生命周期通知，不跨界`。

### 3. 归属性语义（三例，与 STEP-0585 否决的「任意点归根」严格区分）

| 命中 | 交付 | 观测（guest 侧） | 说明 |
|---|---|---|---|
| 子节点/叶子（最深） | `click(子句柄)` | `poll-event` 见 `click(handle)` | 点在其矩形内；**不是**根 |
| `Node(root)` | `click(根句柄)` | `poll-event` 见 `click(root)` | 点在窗内但**无子节点覆盖**（padding/容器空隙）。STEP-0585 曾否决「任意点归根」——那时**无布局**，无法区分根与子；现在有布局，`root` 是**确实无子节点覆盖**时的正确归属，二者语义不同（`route_click_event` 注释已写明） |
| `Miss`（窗外） | **不交付** | `poll-event` 见 `none`；宿主 `skipped_click_count` 递增 | 明确无命中；跳过被计数（STEP-0585 通道） |

### 4. 端到端证据（`runner/sico-runner/tests/ui_click_routing.rs`，新建）

真实 hand-encoded guest 组件（`wasm-encoder`，镜像 `ui_guest_visible.rs` 形态）经真实
`Runner` 跑生产 [`WindowedUiBackend`]，经测试本地适配器 [`ClickRoutingBackend`]（包装生产后端，
树操作逐项委托，`poll_event` 用**生产** `route_click_event` 对**真实**树+实时客户区尺寸路由）。

- **Guest A**：`window("ui-click",320,160)` → `column` → `button("OK")`，报告 button 句柄（3）。
- **Host**：`layout_input()` → `layout::compute` → `layout.rect(3).center()`（或根 padding (8,8)，
  或窗外 (400,400)）→ `inject_click(cx, cy)`。
- **Guest B**：`poll-event` 循环，报告观察到的 click 句柄（exit = handle）或 `EXIT_NO_CLICK`（107）。
- **三种归属**：子节点 → `click(3)`（exit 3）；根 padding → `click(1)`（exit 1）；窗外 →
  `none`（exit 107）且 `skipped()==1`。
- **④ authority 回归**：`UiGrants::default()` → `authority-denied`（exit 115）+ 后端零触达。

### 点击制造方式（诚实说明）

`ui@1` 无 guest 侧注入点击的途径；shell 仅从真实 `WM_LBUTTONDOWN` 捕获（`HWND` 私密），真实
`SendInput` 需窗口前台/焦点且 CI 不确定——**本片不驱动真实鼠标**。证据为「**宿主投递坐标 →
guest 观测 `click(handle)`**」：宿主把坐标直接送入路由 seam（`ClickRoutingBackend::inject_click`），
路由用**同一生产函数** `route_click_event`（`D2dUiRenderer::poll_event` 用的就是它）对**真实**
树 + **实时**客户区尺寸执行。与真实用户点击的唯一差别：OS `WM_LBUTTONDOWN` 捕获 + `translate`
（纯确定消息→事件映射，已由冻结事件语料覆盖）被旁路；**本片新增的路由部分**端到端实证。

## 执行验证（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线，温热缓存）

基线（改动前）：本机已有 STEP-0592 的 renderer 布局/语料与 STEP-0591 的 `ui_guest_visible` 绿。

改动后（`-- --test-threads=1`，runner 为独立 workspace）：

- `cargo test -p sico-runner --test ui_click_routing --offline -- --test-threads=1` →
  **4 passed**（`guest_observes_click_on_the_button_child`、`guest_observes_click_on_the_root_when_no_child_covers_the_point`、
  `a_click_outside_the_window_is_a_skip_not_a_delivery`、`denied_guest_is_typed_authority_refusal_with_zero_backend_touch`）。
  （首跑进程退出报 `0xc0000005`，与 STEP-0592 记录的「刚重链完成的 GNU/rust-lld 二进制首跑
  退出瞬时访问违规」一致；4 个测试全部 `ok`，复跑 exit 0。）
- `cargo test -p sico-runner --lib --offline -- --test-threads=1` → **43 passed**（含本片新增
  `ui::tests::classify_window_event_covers_the_non_click_events`、
  `ui::tests::route_click_event_delivers_the_deepest_node_and_misses_explicitly`）。
- `cargo test -p sico-runner --test ui_guest_visible --offline -- --test-threads=1` → **3 passed**。
- `cargo test -p sico-runner --test ui_authority --offline -- --test-threads=1` → **7 passed**。
- `cargo test -p sico-runner --test package_roster_consumer --offline -- --test-threads=1` → **6 passed**。
- `cargo test -p sico-runner --test provider_crash_isolation --offline -- --test-threads=1` → **4 passed**。
- `cargo test -p sico-runner --test provenance_runtime --offline -- --test-threads=1` → **6 passed**。
- `cargo test -p sico-ui-renderer-d2d --offline`（主 workspace）→ **lib 5 passed + 1 ignored +
  frame_corpus 5 passed + layout_corpus 6 passed**，0 失败；**冻结帧/事件/布局哈希全部通过**
  （`frame_corpus_digests_are_frozen` / `event_corpus_digests_are_frozen` /
  `layout_corpus_digests_are_frozen` 绿）。
- `cargo test -p sico-ui-renderer-d2d --offline --lib -- --ignored --test-threads=1` → **1 passed**
  （`real_window_lifecycle_via_windowed_backend`——含新增 `ClientSize` 命令的真实窗口生命周期）。
- `cargo test -p sico-ui-renderer-d2d --offline --test event_corpus -- --ignored --test-threads=1` →
  **1 passed**（`real_window_smoke_close_present_quit`）。
- `cargo clippy -p sico-runner --offline --lib -- -D warnings` → **绿**；
  `cargo clippy -p sico-runner --test ui_click_routing --offline -- -D warnings` → **绿**；
  `cargo clippy -p sico-ui-renderer-d2d --offline --all-targets -- -D warnings` → **绿**。
- `cargo fmt -p sico-runner -- --check` → **净**（本片先 `cargo fmt -p sico-runner -- src/ui.rs
  tests/ui_click_routing.rs`；见「诚实收窄」关于未跟踪 `ui_frame_determinism.rs` 的说明）。
  `cargo fmt -p sico-ui-renderer-d2d -- --check` → **净**。
- `git diff --check` → **净**（仅覆盖已跟踪文件）；`grep -nP ' +$'` 本片新/改文件 → 无尾随空白。

## G1–G6 状态更新表

| # | 证据 | 本片前（STEP-0591/0592） | 本片后 | 口径 |
|---|---|---|---|---|
| G1 | guest `window()` 真实窗口 | 已达成（STEP-0581，runtime-verified） | **已达成（不变）** | 不变 |
| G2 | guest `run()` 呈现帧证据 | 已达成（STEP-0591，宿主读回逐字节） | **已达成（不变）** | 不变 |
| G3 | guest `poll-event` 交付 `closed` | 已达成（STEP-0591） | **已达成（不变）** | 不变 |
| **G4** | guest `poll-event` 交付 `click(handle)`/`select` 且路由正确 | 未达成（命中测试缺失） | **`click(handle)` 达成；`select` 未达成** | **本片 `click` 在「宿主投递坐标 → guest 经真实 runner 观测 `click(handle)`，路由经生产 `route_click_event` 对真实树+实时客户区尺寸」口径下达成；`select` 因模型无选中状态不可交付（见「诚实收窄」）** |
| G5 | guest 级帧确定性 | 已达成（STEP-0591） | **已达成（不变）** | 不变 |
| G6 | clean-room pilot（gate 5） | 未达成 | **未达成（不变）** | 不变 |

**判定**：按 STEP-0590 §6 定义（G4 = click **且** select），**G4 未整体达成**（`select` 未交付）；
但**G4 的 `click(handle)` 分项达成**。G1∧G2∧G3∧G5 不变，G6 不变。**无任何 M24 §3 gate 翻转**
（本片不产生 guest 可见完整 UI 能力——`select` 与 G6 仍缺口）。

## 诚实收窄（本片未声称的部分）

- **`select` 未交付**：`ui-selection = { handle, index }` 需「哪个 dropdown 的哪个索引被选中」，
  本模型唯一事件源是坐标 `Clicked`，无选中状态/下拉打开-选择语义；shell 不产生 select 事件。
  **`select` 仍未达成**，须在模型引入下拉选择语义（或 shell 产生 select 事件）后另片实现。
- **点击制造方式**：**非真实鼠标点击**。宿主投递坐标 → guest 观测 `click(handle)`；OS
  `WM_LBUTTONDOWN`+`translate` 被旁路（纯确定映射，事件语料已覆盖）。未声称「真实用户点击」。
- **两套布局并存**（STEP-0592 已登记）：命中网格（token 派生，本片路由用它）与渲染器视觉网格
  （DirectWrite 度量 + 容器 0×0 cell，`render/win.rs::place`）**不一致**；本片命中网格的正确性
  属「归属性正确」，不属「视觉网格一致」。统一布局会改帧字节 ⇒ 需重新冻结帧语料（后续片）。
- **客户区尺寸来源**：`layout_input` 用 shell **实时** `GetClientRect`（`client_size()`），而
  渲染器帧用树声明的初始尺寸（`State.window`，无 resize 跟踪——既存限制）。窗口未 resize 时
  二者一致；resize 后未重渲染的间隙内，命中网格（实时）与显示帧（初始尺寸）可能瞬时错位——
  记录为既存行为，非本片引入。
- **`route_click_event` 为 `pub`**：e2e 测试适配器复用**生产**路由（避免在测试里复制路由逻辑）；
  属宿主侧路由 seam 的最小暴露。
- **`classify_window_event` 签名变更**（`(Option,bool)` → `Option`）：STEP-0585 的「Clicked
  被无条件跳过」前提已被本片取代（`Clicked` 改由 `route_click_event` 路由）；`ui.rs` 内既有
  单测已同步更新（`classify_window_event_covers_the_non_click_events` 等）。**guest 可见语义**
  的变更仅是「坐标点击不再被无理由跳过」——即本片目标。
- **`skipped_click_count` 语义**：保持「被跳过」（`Hit::Miss` 计），**非**「全部点击」——命中
  的点击交付、不计数；布局失败计为 `None`（非 Miss，不计数）。语义未回归。
- **未翻任何 M24 §3 gate**；`wit/**`、`layout.rs`（语义）、`render/**`、`shell/**`、`tokens.rs`、
  `tree.rs`、`tests/layout_corpus.rs`、`guests/**`、`selfhost/`、`docs/steps/README.md`、
  `docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、
  `docs/plans/**`、`crates/sico-package/**` 与任何冻结语料（`frame_corpus.rs`/`event_corpus.rs`/
  `layout_corpus.rs`/`docs/evidence/m24/*.json`）均未触碰。
- **副作用披露**：`cargo fmt -p sico-runner`（为满足 workspace fmt 检查）在格式化本片文件时
  也把**未跟踪**既存文件 `runner/sico-runner/tests/ui_frame_determinism.rs`（非本片、非提交版）
  规范化为 rustfmt 形式（仅 `eprintln!` 折叠）。该文件无提交基线、无法回退；属在途 M24 工作，
  非本片引入，未作为本片改动申报。
- **未 commit/push**；未改既有跟踪文件（`ui.rs` 既有改动为 STEP-0585 在途 + 本片，未覆盖）。

## 后续片建议

1. **`select` 交付片**：在树/事件模型引入下拉选择状态（或 shell 产生 select 事件），再补
   `ui-selection` 的路由与交付——本片已把 `click` 路由 seam 就绪，`select` 只需同类 seam。
2. **绘制片统一布局**（STEP-0592 建议 #1）：渲染器切换到本片命中网格的 extent 布局（容器真实
   extent + token 文本推进），替换 `render/win.rs::place`（容器 0×0 cell + DirectWrite 度量）。
   **会改帧字节** ⇒ 须重新冻结 STEP-0338 的 4 个帧 SHA-256（`frame_corpus.rs` +
   `docs/evidence/m24/ui-frame-corpus.json`）+ bump `TOKEN_SET_VERSION`——**owner 授权的受控审计
   动作**，本片未执行。此片同时消除「两套布局并存」与「resize 后命中/显示瞬时错位」。
3. **真实鼠标点击 E2E**（可选）：暴露 `HWND`（或 shell 注入 affordance）后可用 `SendInput`
   驱动真实 `WM_LBUTTONDOWN`，把证据从「宿主投递坐标」升级为「真实用户点击」；需 owner 授权
   shell 改动（本片禁动 `shell/**`）。

## 建议主线审核点

- **归属性语义**：子/叶子 → 自身；容器空隙 → 容器；窗口 padding → `Node(root)`；窗外 → `Miss`
  （跳过+计数）。确认 `Node(root)` 只在**无子节点覆盖**时交付（非「任意点归根」）。
- **`select` 未交付**：G4 整体判定为「`click` 达成、`select` 未达成」——确认此口径可接受，
  或要求模型引入下拉选择语义后再判 G4 整体。
- **点击制造真实度**：宿主投递坐标 vs 真实用户点击的差别已声明；确认可接受为
  `internal-fixture` 级证据（非 pilot/生产）。
- **`route_click_event` 为 `pub`**：宿主侧路由 seam 的最小暴露，供 e2e 复用；确认无 API 洁癖。
- **客户区尺寸**：`layout_input` 用 shell 实时尺寸（非树声明尺寸）；确认 hit 网格随窗口而非
  随帧（既存 resize 无跟踪）。
- **`crates/sico-ui-renderer-d2d/` 整目录未跟踪**：在途 M24 工作；提交/推送须待 owner 授权。

## 关系说明

本片是 STEP-0590 C-C / STEP-0585 剩余缺口 #1 的**接线片**（STEP-0592 的布局+命中测试 + 本片
runner 交付 = `click(handle)` 达成）。它**不**改变 STEP-0338 帧语料、STEP-0577 事件语料、
STEP-0592 布局语料（3 组冻结哈希复跑不变），**不**翻转任何 M24 §3 gate，**不**改变任何证据
等级，**不**改变任何渲染字节。
