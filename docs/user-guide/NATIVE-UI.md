# 原生 UI 库当前能力（M24 §8.6.1）

> 状态：**当前能力，非生产声明**。本页只陈述**已经可执行**的现状——你现在能做什么、能
> 观察到什么、什么**尚未**提供。未写任何「计划支持」「即将支持」；每条结论都有对应
> STEP 文档与可复现测试入口（见「可复核入口」）。
>
> 证据等级如实分开：`runtime-verified`（真实 OS 窗口路径）与 `internal-fixture`
> （hand-encoded guest 组件经真实 Runner）。两者都**不是** pilot 级、**不是**生产声明。
> 平台范围：Windows x64 交互桌面（详见「平台与证据范围」）。

## 概述：能做什么 / 不能做什么

| 能力 | 现状 | 口径（诚实收窄） |
|---|---|---|
| 创建真实 OS 窗口（`window()`） | **可用**（G1 达成） | guest 调 `window()` 得到 `Ok(handle)`，宿主 `window_title()` 为 `Some` |
| 触发呈现（`run()`） | **可用，收窄口径**（G2 达成） | **guest 触发 + 宿主读回核验**；`ui@1` 无 guest 可见的帧读回通道（guest 读不到帧字节） |
| 关闭事件（`poll-event` 交付 `closed`） | **可用，收窄口径**（G3 达成） | **guest 自身 `poll-event` 观测 `closed`，但关闭由宿主/用户触发**；`ui@1` 无 guest 可见的 `close` |
| 点击事件（`poll-event` 交付 `click(handle)`） | **可用，收窄口径**（G4a 达成） | **坐标由宿主注入**，旁路的仅是 OS `WM_LBUTTONDOWN` + `translate`（纯确定映射，已被冻结事件语料覆盖）。**不是「真实鼠标点击已验证」** |
| 选择事件（`poll-event` 交付 `select`） | **不可交付**（G4b 缺口） | 合同/模型缺口：契约确有 `select(ui-selection{handle,index})`，dropdown 也有 `options`，但模型无选中态字段、无设置选中的 op、无逐选项命中几何、shell 不产生 select 事件 |
| 帧确定性（同树 N 次渲染） | **可用，限缩**（G5 达成） | N=6 个完全独立 (Store, 后端, OS 窗口) 代渲染同一树，帧逐字节 + digest 全等，锚定冻结 `text` 场景。**限缩**：单平台、单树、单 token 集，不推广 |
| clean-room 试点应用（gate 5） | **未提供**（G6 未达成） | 属 M24 gate 5；`#3 JPEG` 为外部门，未解 |

**整体判定**：guest 可见 UI 能力 = **G1 ∧ G2 ∧ G3 ∧ G4**（STEP-0590 §6 定义）。当前
G1/G2/G3 达成、G4 **未整体达成**（G4a `click` 达成、G4b `select` 缺口）、G5 达成（伴随
分项）、G6 未达成 ⇒ **「guest 可见 UI 能力」整体仍未达成**。本页不声称任何 M24 §3 gate
翻转，也不声称该能力整体达成。

## 使用前提

- 运行在 **Windows x64** 交互桌面（有真实 `CreateWindowExW` 窗口路径）。非 Windows 主机
  `D2dUiRenderer::new()` → 类型化 `no-window`；headless/无交互桌面时窗口创建失败 → guest
  得类型化 `ui-error`（测试打印 `ENV_LIMIT`，不伪造「已呈现」）。
- 授权默认拒绝：`ui@1` 无授权 → 每个调用得类型化 `authority-denied` 且渲染器**零触达**；
  无后端时 `ui@1` **不广告**（导入它的 guest 实例化失败）。
- 你（guest）可用的 `ui@1` 面是 13 个函数：`window` / `column` / `row` / `text` / `button` /
  `dropdown` / `image-preview` / `progress` / `set-text` / `set-progress` / `draw-image` /
  `poll-event` / `run`（`wit/script-ui-v1/ui.wit:66-85`）。

## 可复核入口

| 事实 | 测试文件 | 可复现函数/入口 |
|---|---|---|
| G1 真实窗口 | `runner/sico-runner/tests/ui_guest_visible.rs` | `guest_run_presents_a_real_frame_byte_identical_to_offscreen_oracle`（guest 观测句柄 1）；`runner/sico-runner/tests/ui_authority.rs`（真实后端 e2e） |
| G2 呈现帧（收窄口径） | `runner/sico-runner/tests/ui_guest_visible.rs` | `guest_run_presents_a_real_frame_byte_identical_to_offscreen_oracle` |
| G3 `closed`（收窄口径） | `runner/sico-runner/tests/ui_guest_visible.rs` | `guest_poll_event_delivers_closed_after_host_close` |
| G4a `click(handle)` | `runner/sico-runner/tests/ui_click_routing.rs` | `guest_observes_click_on_the_button_child`；`guest_observes_click_on_the_root_when_no_child_covers_the_point`；`a_click_outside_the_window_is_a_skip_not_a_delivery`；`denied_guest_is_typed_authority_refusal_with_zero_backend_touch` |
| G4b `select` 缺口 | （无交付测试）`docs/steps/STEP-0599-m24-ui-select-deliverability.md` | 判定为合同/模型缺口；修正案 `docs/rfc/RFC-0052-amendment-A3-ui-selection-model.md`（PROPOSED） |
| G5 帧确定性 | `runner/sico-runner/tests/ui_frame_determinism.rs` | `same_guest_tree_renders_n_byte_identical_frames`；`guest_frame_digest_matches_across_processes`；`same_tree_presented_repeatedly_on_one_backend_is_byte_identical` |
| 渲染器语料 | `crates/sico-ui-renderer-d2d/tests/` | `frame_corpus.rs`（4 冻结帧）、`event_corpus.rs`（3 冻结事件）、`layout_corpus.rs`（4 场景布局/命中） |

这些入口都是 `cargo test --offline --test <name> -- --test-threads=1`（runner 独立 workspace，
工具链 `1.98.0-x86_64-pc-windows-gnu`）。渲染器 crate 的真实窗口生命周期测试是 opt-in
`#[ignore]`（`real_window_lifecycle_via_windowed_backend`），headless 不开真窗。

## 已知限制（诚实收窄）

- **`select` 不可交付**：契约声明 `select(ui-selection{handle,index})`（`ui.wit:53-56,59-63`），
  dropdown 有 `options` 列表（索引定义域存在），但当前模型**无选中态字段**（`UiNode` 字段集
  `kind/parent/children/text/options/image/progress`，无 `selected`）、**无设置选中的 op**
  （状态 op 闭集为 `set-text`/`set-progress`/`draw-image`）、**无逐选项命中几何**（dropdown
  无子节点、布局为单一叶子盒）、**shell 不产生 select 事件**（窗口事件闭集为
  `Close/Resize/Paint/LeftButtonDown`）。⇒ guest 轮询 `poll-event` **永不见 `select`**，无法
  区分「用户未选择」与「用户已选择但不可交付」。修正案三案（A1 合同收窄 / A2 模型扩展 /
  A3 宿主侧事件）**均未裁决**——本页只陈述现状，不替 owner 选案。
- **点击是「宿主投递坐标 → guest 观测 `click(handle)`」，不是真实鼠标点击**：`ui@1` 无
  guest 注入点击途径；shell 仅从真实 `WM_LBUTTONDOWN` 捕获（`HWND` 私密），`SendInput` 需
  窗口前台/焦点且 CI 不确定。与真实用户点击的唯一差别是 OS 捕获 + `translate`（纯确定
  映射）被旁路；本页不声称「真实用户点击已验证」。
- **两套布局并存**：命中判定用**冻结 token 派生的 extent 布局**（度量只取 token 表，跨机器
  确定）；而**渲染绘制**仍用旧路径（DirectWrite 度量 + 容器 0×0 cell）。⇒ **命中网格与
  视觉网格可能不一致**（文本宽度是 `utf16 码元数 × BODY_SIZE` 的 token 派生代理，非字形
  精确 advance）。统一需要切换渲染路径 ⇒ **帧字节改变 ⇒ 需重新冻结帧语料**（owner 授权项）。
- **窗口创建为隐藏**：`window()` 创建真实窗口但**不显示**（show 属宿主选择）。
- **呈现假定帧尺寸 = 客户区尺寸，无 resize 重渲染**：窗口被拉大时以帧尺寸绘制；resize 后
  未重渲染的间隙内，命中网格（shell 实时 `GetClientRect`）与显示帧（树声明初始尺寸）可能
  瞬时错位。这是**既存行为**，非本页引入。
- **`ui-error` 闭集无渲染失败判别式**：渲染/呈现/建窗失败被有损折叠为类型化 `no-window`，
  guest 无法区分「窗口不存在 / 平台不支持 / 渲染·建窗·呈现失败」；`event-overflow` 判别式
  已声明但**无 guest 可达 producer**（`poll-event` 返回 `option<ui-event>`，无错误通道）。
- **`skipped_click_count` 是宿主侧观测**（窗外点击计数），**非 guest 契约成员**；guest 经
  闭集仍得 `none`。
- **限额**：窗口标题 256 UTF-8 字节；事件队列 256（预分配、按类型合并、超限报类型化
  overflow）。

## 平台与证据范围

- **平台**：Windows x64（交互桌面）是唯一 runtime-verified 的路径；非 Windows 为类型化
  拒绝（`UnsupportedPlatform` → `no-window`）。
- **证据等级**：G1/G2/G3/G5 为 `runtime-verified window + internal-fixture guest`；G4a 为
  `internal-fixture`（真实 guest 组件经真实 `Runner` + 生产 `WindowedUiBackend`）。
  均**非** pilot 级、**非**生产声明。
- **G5 限缩**：单平台（Windows x64）、单树（冻结 `text` 场景）、N=6、单 token 集
  （`fluent-subset-v0`）；**不**推广到任意树/任意平台/跨主机光栅化（语料本身注明
  evidence-host local）。
- **行号漂移注意**：本页引用的源码符号（`route_click_event`、`layout_input`、
  `classify_window_event`、`UiNode`、`UiTree::set_options`）为稳定身份；`runner` 与
  `crates/sico-ui-renderer-d2d` 正处于并发编辑，源码 `文件:行号` 会漂移，实现者须对已提交
  快照重新锚定。本页以 STEP 文档为主要引用，避免依赖会漂移的行号。

## 相关文档

- 验收条款与合同空白：`docs/steps/STEP-0590-m24-ui-contract-amendments-draft.md`（G4 已拆为
  G4a/G4b，见该文件 2026-11-07 addendum）。
- 分项证据：`STEP-0581`（接线）、`STEP-0591`（G2/G3）、`STEP-0592`（布局+命中）、
  `STEP-0595`（G5）、`STEP-0597`（G4a `click`）、`STEP-0599`（G4b `select` 判定）。
- 修正案（PROPOSED，只读）：`docs/rfc/RFC-0052-amendment-A2-*.md`、
  `docs/rfc/RFC-0052-amendment-A3-ui-selection-model.md`。
- 合同与架构：`docs/rfc/RFC-0052-native-ui-source-binding-v0.md`（accepted）、
  `docs/adr/ADR-0018-native-renderer-fluent-subset-v0.md`（accepted）。
