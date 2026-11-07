# STEP-0603: M24 §8.6.1 — 原生 UI 库「当前可执行能力」用户文档（纯文档片；不改代码/测试/RFC 正文/冻结语料）

> - status: **documentation complete locally**（新增/更新 3 份文档：用户手册 [`NATIVE-UI.md`](../user-guide/NATIVE-UI.md)、STEP-0590 addendum、本文件；**不翻任何 M24 §3 gate、不改任何证据等级、未 commit/push**）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改；[`RFC-0052-A2`](../rfc/RFC-0052-amendment-A2-ui-error-render-failed-and-event-visibility.md)/[`RFC-0052-A3`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md)（PROPOSED，只读参考，不改写）
> - evidence: **只读核验 + 文档化**（无任何实现、无任何行为变更、无任何 WIT/源码/测试/冻结语料改动；证据为既有 STEP 文档 + 源码锚点的逐条复验，见「文档化口径与证据来源」）
> - deliverables: [`docs/user-guide/NATIVE-UI.md`](../user-guide/NATIVE-UI.md)（新，用户文档）、[`STEP-0590`](./STEP-0590-m24-ui-contract-amendments-draft.md)（追加 2026-11-07 addendum）、本文件
> - **引用注意**：`runner/sico-runner` 与 `crates/sico-ui-renderer-d2d` **正被并发代理编辑**（STEP-0590/0597/0599 记录同一约束）。本文件全部 `文件:行号` 均为 **2026-11-07 在 worktree 实测锚点**，会漂移；实现者须对已提交快照重新锚定后再引用。符号名（`route_click_event`、`layout_input`、`classify_window_event`、`UiNode`、`UiTree::set_options`）是稳定身份。

## 背景

M24 §8.6.1 的「guest 可见 UI 能力」经 STEP-0581（接线）、STEP-0591（G2/G3）、STEP-0592
（布局+命中）、STEP-0595（G5）、STEP-0597（G4a `click`）、STEP-0599（G4b `select` 判定）
推进后，已具备可向用户陈述的**当前实际可执行能力**，但仓库**尚无用户可读、且不夸大**的
能力文档——现有相关陈述分散在 STEP 文档（面向开发）与 M24 审计报告（面向 owner）中。
本片把该能力写成**用户可读、只陈述已可执行事实、不夸大**的文档，并把验收条款的 **G4
拆分口径（G4a `click` / G4b `select`）** 同步到相关文档。

## 文档化口径与证据来源（本片逐条复验）

本片**先复核**以下事实（实际读文件 + 既有 STEP 文档），再写文档；文档与实现冲突时取下界
（AGENTS §1）。

| # | 事实 | 证据来源（复验） | 文档化口径 |
|---|---|---|---|
| 1 | guest `window()` 创建真实 OS 窗口并观测到句柄（G1） | STEP-0581（runtime-verified window + internal-fixture guest）；STEP-0591/0595 再验证（`window_title()`=`Some`，guest exit=1）；`runner/sico-runner/tests/ui_guest_visible.rs`、`ui_authority.rs` | G1 **已达成** |
| 2 | guest `run()` 触发呈现，宿主读回帧与离屏帧逐字节相等；`ui@1` 无 guest 可见帧读回通道（G2，收窄口径） | STEP-0591（合同缺口 1）；`run() -> result<bool, ui-error>`（`ui.wit:85`），读回仅宿主侧 `read_presented_frame` | G2 **已达成（收窄口径）**：guest 触发 + 宿主读回核验，**非**「guest 自行观测到画面」 |
| 3 | 宿主触发关闭后 guest 自身 `poll-event` 交付 `closed`；`ui@1` 无 guest 可见 `close`（G3，收窄口径） | STEP-0591（合同缺口 2）；`poll-event: func() -> option<ui-event>`（`ui.wit:83`），`ui-event` = `click|select|closed`（`ui.wit:59-63`） | G3 **已达成（收窄口径）**：guest 观测 + 宿主/用户触发 close |
| 4a | `click(handle)` 已交付：生产路由 seam `route_click_event`（`ui.rs:304`）+ 只读 affordance `layout_input()`；最深命中 ⇒ 该句柄；`Node(root)` ⇒ 根句柄**且仅在确实无子节点覆盖时**；窗外 `Miss` ⇒ 不交付并计入宿主 `skipped_click_count`（`ui.rs:183,224`）；布局失败 ⇒ 类型化 `None`（闭集无错误通道的诚实信号） | STEP-0597；`layout.rs:197` `hit`、`layout.rs:295` `compute`（纯函数、跨机器确定）；`events.rs` `RawMessage`/`WindowEvent`（`:38-57,65-85`），`translate` 确定映射（`:98-117`）；`runner/sico-runner/tests/ui_click_routing.rs` 4/4 绿 | G4a **已达成（收窄口径）**：坐标由宿主注入，被旁路的仅是 OS `WM_LBUTTONDOWN`+`translate`（纯确定映射，已被冻结事件语料覆盖）⇒ **不得**写成「真实鼠标点击已验证」 |
| 4b | `select` 不可交付（合同/模型缺口，非实现缺口）：契约确有 `select(ui-selection{handle,index})`（`ui.wit:53-56,59-63`），dropdown 有 `options`（`ui.wit:71`，`tree.rs:79`，`UiTree::set_options` `tree.rs:228-249`），但模型无选中态字段（`UiNode` 字段集 `tree.rs:69-84` 无 `selected`）、无设置选中的 op（`ui.wit:76-80` 闭集）、无逐选项命中几何（`tree.rs:45-47` dropdown 无子节点；`layout.rs:429-433,470-476` 单一叶子盒）、shell 不产生 select 事件（`events.rs:38-57`） | STEP-0599（判定 (B)）；`RFC-0052-A3`（PROPOSED 三案未裁决） | G4b **未达成（合同/模型缺口）**：guest 永不见 `select`，无法区分「未选择」与「已选择但不可交付」；**文档只陈述现状，不替 owner 选案** |
| 5 | guest 级帧确定性：N=6 个完全独立 (Store, 后端, OS 窗口) 代渲染同一树，帧逐字节+digest 全等且锚定冻结 `text` 场景；限缩：单平台（Windows x64 交互桌面）、单树、单 token 集 | STEP-0595；`runner/sico-runner/tests/ui_frame_determinism.rs` 4/4 绿（含 `frame_digest` 独立进程复算） | G5 **已达成（限缩口径）**，不推广到任意树/平台/跨主机光栅化 |
| 6 | G6 未达成（clean-room pilot 应用属 gate 5；#3 JPEG 外部门未解） | STEP-0595:165；STEP-0597:179 | **未提供** |
| 7 | 两套布局并存：命中判定用冻结 token 派生的 extent 布局（度量只取 token 表，跨机器确定），渲染绘制仍用旧路径（DirectWrite 度量 + 容器 0×0 cell）⇒ 命中网格与视觉网格可能不一致；统一需切换渲染路径 ⇒ 帧字节改变 ⇒ 需重新冻结帧语料（owner 授权项，RFC-0052-A2 C-C） | STEP-0592（「两套布局并存」）、STEP-0597 残余 ② | 如实告知用户，不作隐藏 |
| 8 | 默认拒绝与限额：`ui@1` 授权默认拒绝（无授权 ⇒ 类型化 `authority-denied` 且渲染器零触达）；无后端 ⇒ `ui@1` 不广告；标题 256 UTF-8 字节限额、事件队列 256（预分配/按类型合并/超限类型化 overflow） | STEP-0577（限额）、STEP-0581（默认拒绝语义）、STEP-0591（零触达断言） | 写入使用前提与限制 |
| 9 | 窗口创建为隐藏、呈现假定帧尺寸=客户区尺寸（无 resize 重渲） | STEP-0581:123-124（呈现假设）；STEP-0597 残余 ③（resize 后命中/显示瞬时错位） | **必须写清，不得省略** |
| 10 | `ui-error` 闭集无渲染失败判别式（有损折叠为 `no-window`）、`event-overflow` 无 guest 可达 producer、`skipped_click_count` 非 guest 契约成员 | STEP-0590（C-A/C-B）、STEP-0585 | 写入已知限制 |

**结论**：10 项事实全部复验一致，**未发现**与任务书登记不符之处；文档化口径与任务书
1–10 逐条对应。**无纠正项**。

## 用户文档要点（docs/user-guide/NATIVE-UI.md，新建）

新增 `docs/user-guide/NATIVE-UI.md`（**非重复篇**：该目录无既有 UI 文档，grep 确认
`docs/user-guide/` 与 `docs/README.md` 无 `ui@1` 相关条目），结构：

1. **状态横幅**：当前能力、非生产声明；只陈述可执行事实；证据等级如实分开
   （`runtime-verified` / `internal-fixture`），均非 pilot/生产；平台范围 Windows x64 交互桌面。
2. **概述表**（能力 / 现状 / 口径）：G1 可用；G2 可用（收窄）；G3 可用（收窄）；
   G4a 可用（收窄）；G4b 不可交付；G5 可用（限缩）；G6 未提供。
3. **整体判定**：guest 可见 UI 能力 = G1∧G2∧G3∧G4（STEP-0590 §6）⇒ G4 未整体达成 ⇒
   **整体仍未达成**；不声称任何 §3 gate 翻转。
4. **使用前提**：Windows x64 交互桌面；默认拒绝语义（无授权 ⇒ `authority-denied` 且零触达；
   无后端 ⇒ 不广告）；13 函数面。
5. **可复核入口**（测试文件 / 函数名）：G1/G2/G3（`ui_guest_visible.rs`）、G4a
   （`ui_click_routing.rs`）、G4b（STEP-0599，无交付测试）、G5（`ui_frame_determinism.rs`）、
   渲染器语料（`frame_corpus.rs`/`event_corpus.rs`/`layout_corpus.rs`）。
6. **已知限制**（诚实收窄）：`select` 不可交付（合同/模型缺口 + 三案未裁决）；点击是
   「宿主投递坐标 → guest 观测」，非真实鼠标点击；两套布局并存（命中网格 vs 视觉网格，
   统一需重新冻结帧语料，owner 授权）；窗口创建为隐藏；呈现假定帧尺寸=客户区尺寸（无
   resize 重渲）；`ui-error` 无渲染失败判别式 + `event-overflow` 不可观测；
   `skipped_click_count` 非 guest 契约成员；限额（标题 256 字节、队列 256）。
7. **平台与证据范围**：单平台 Windows x64；G5 限缩；行号漂移注意（以 STEP 文档为主要引用）。
8. **相关文档**：STEP-0590（含 addendum）/0581/0591/0592/0595/0597/0599；RFC-0052-A2/A3
   （PROPOSED，只读）；RFC-0052 / ADR-0018（accepted）。

**注册**：`docs/user-guide/README.md` 推荐阅读顺序新增第 11 项「原生 UI 库当前能力」。

## 验收条款同步（STEP-0590 addendum，追加）

在 `docs/steps/STEP-0590-m24-ui-contract-amendments-draft.md` 末尾追加**日期化 addendum**
（2026-11-07）：把 §6 验收条款的 **G4 拆为 G4a（`click`，已达成，收窄口径）/ G4b
（`select`，合同·模型缺口，不可交付）**，并注明：

- G4a 口径 = 宿主投递坐标 → guest 经真实 runner 观测 `click(handle)`，路由经生产
  `route_click_event` 对真实树 + 实时客户区尺寸；旁路的仅是 OS `WM_LBUTTONDOWN`+`translate`
  （纯确定映射，冻结事件语料已覆盖）；证据等级 `internal-fixture`；可复核入口 4 个测试。
- G4b 判定 = 合同/模型缺口（非实现缺口），依据 STEP-0599；修正案 `RFC-0052-A3`（PROPOSED）
  三案 **A1（合同收窄）/ A2（模型扩展）/ A3（宿主侧事件）均未裁决** ⇒ **本 addendum 只陈述
  现状，不预裁 owner 选择**；G4b 保持未达成。
- **不**改变 §6 的 G4 整体定义（G4 = click 且 select）⇒ **G4 未整体达成**，guest 可见 UI
  能力未整体达成，无任何 §3 gate 翻转。

## 执行验证（纯文档片）

本片无代码/WIT/测试/冻结语料改动，验证为**文档卫生 + 索引登记**：

- `git diff --check` → **净**（本片改动的 3 个 `.md` 均无尾随空白，另用 `grep -nP ' +$'`
  检查新增文件）。
- 索引登记：`docs/user-guide/README.md` 新增第 11 项（指向新 NATIVE-UI.md）；新 STEP 已在
  `docs/steps/README.md` 登记一行（镜像同批 M24 STEP 的登记形态）。
- 未跑任何 cargo 编译/测试：本片无代码改动，编译/测试结果不受影响（不声称任何测试已跑；
  既有 STEP-0581/0591/0592/0595/0597/0599 的测试结果保持为既有证据，未复跑）。

## 诚实收窄（本片未声称的部分）

- **只写可执行事实**：未写「计划支持」「即将支持」；所有「未提供」项（G4b `select`、G6
  pilot、guest 帧读回通道、guest `close`、真实鼠标点击、统一布局、resize 重渲、非 Windows
  路径）均如实列为限制，**未因文档「更好看」而放宽任何限制性陈述**——每条限制都有实测依据。
- **未翻任何 M24 §3 gate**；未声称 guest 可见 UI 能力整体达成（它未达成，G4b + G6 缺口）。
- **未修改已接受 RFC 正文**（RFC-0052、RFC-0051 等）；`RFC-0052-A2`/`A3`（PROPOSED）只读
  参考，未改写。
- **未触碰** `wit/**`、`selfhost/**`、`crates/**`、`runner/**`、`diagnostics/**`、
  `semantic-cases/**`、`syntax-candidates/**`、任何冻结语料（`frame_corpus.rs`/`event_corpus.rs`/
  `layout_corpus.rs`/`docs/evidence/m24/*.json`）。
- **未独立复验**：编译/测试（本片无代码改动，未跑）；STEP-0338/0577/0592 冻结语料哈希复跑
  （属既有结论，不下结论）。**已独立复验**：事实 1–10 的源码锚点（`ui.wit`/`tree.rs`/
  `layout.rs`/`events.rs`/`ui.rs` 的符号与关键行），非仅转述 STEP 摘要。
- **未 commit/push**；未改既有文件（除 STEP-0590 的追加 addendum 与 user-guide README 的
  索引行）；新增文件仅 `docs/user-guide/NATIVE-UI.md` 与本文件。

## 待 owner 裁决项（本片不预裁，只登记）

1. **`select` 三案**（RFC-0052-A3 PROPOSED）：A1 合同收窄 / A2 模型扩展 / A3 宿主侧事件——
   本片只陈述现状，不替 owner 选案；若 A2，需新 ADR + WIT 追加 + 逐选项几何（涉布局语料
   重冻结，owner 授权）+ 编译器 hand-encoded 同步。
2. **统一两套布局**：切换到 extent 布局会改帧字节 ⇒ 须重新冻结 STEP-0338 的 4 个帧
   SHA-256 + bump `TOKEN_SET_VERSION`——**owner 授权的受控审计动作**，本片未执行。
3. **真实鼠标点击 E2E**：需暴露 `HWND` 或 shell 注入 affordance 后经 `SendInput` 驱动真实
   `WM_LBUTTONDOWN`（把证据从「宿主投递坐标」升级为「真实用户点击」）；需 owner 授权
   shell 改动（本片禁动 `shell/**`）。
4. **guest 帧读回通道 / guest `close`**：WIT 追加属 owner 裁决（STEP-0590 C-A 模式）。
5. **`ui-error` 渲染失败判别式与 `event-overflow` 可观测化**：STEP-0590 决策点 A/B，owner
   裁决；`RFC-0052-A2` PROPOSED。
6. **`crates/sico-ui-renderer-d2d/` 整目录未跟踪**：在途 M24 工作；提交/推送须待 owner 授权
   并整体纳入。

## 关系说明

本片是 STEP-0590 §6 验收条款与 STEP-0597/0599 G4 拆分结论的**用户文档化**。它**不**改变
STEP-0338 帧语料、STEP-0577 事件语料、STEP-0592 布局语料，**不**翻转任何 M24 §3 gate，
**不**改变任何证据等级，**不**改变任何渲染字节或行为；只新增/更新 3 份文档并登记索引。
