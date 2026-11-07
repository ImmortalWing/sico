# STEP-0591: M24 §8.6.1 — `ui@1` **guest 级**呈现/事件直观测证据（G2∧G3，纯证据片；不改生产语义）

> - status: implementation complete locally（**guest 级 fixture 新增**；不翻任何 M24 §3 gate；未 commit/push）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改
> - evidence: **runtime-verified window + internal-fixture guest**（Windows x64 真实 `CreateWindowExW` 窗口 + GDI DIB 呈现 + `WM_CLOSE` 事件投递，经 hand-encoded guest Component 经真实 Runner 准备/运行路径；**非**试点级、**非**生产声明）
> - artifacts: [`runner/sico-runner/tests/ui_guest_visible.rs`](../../runner/sico-runner/tests/ui_guest_visible.rs)（新增）、本文件
> - **引用注意**：`runner/sico-runner/src/ui.rs` 与 `crates/sico-ui-renderer-d2d/**` **正被并发代理编辑**。本文件全部 `文件:行号` 均为 **2026-11-07 在 worktree 实测锚点**，会漂移；实现者须对已提交快照重新锚定后再引用。

## 背景

STEP-0590（验收条款）把「guest 可见 UI 能力」定义为 **G1 ∧ G2 ∧ G3 ∧ G4**（G5 确定性伴随、G6
独立 pilot/gate-5 项），并登记当前状态：**仅 G1 达成**（STEP-0581），G2/G3/G4/G5 未达成。STEP-0581
的「后续切片 #2」明确写着：`run()`/`closed` 的 **guest 级**直观测尚未覆盖（当前经宿主句柄
驱动）——本 STEP 就是该切片：把 G2、G3 变成 **guest 级可复核证据**。

## 侦察结论（本片只读复验，非转述 STEP 摘要）

### 1. guest 可用的 `ui@1` 面（13 函数，`wit/script-ui-v1/ui.wit`）

`window` / `column` / `row` / `text` / `button` / `dropdown` / `image-preview` / `progress` /
`set-text` / `set-progress` / `draw-image` / `poll-event` / `run`（`ui.wit:66-85`）。

**关键：`ui@1` 没有 guest 可见的 `close`。** `close()` 只是宿主侧生命周期 affordance
（`runner/sico-runner/src/ui.rs:234-236` `D2dUiRenderer::close`；`crates/sico-ui-renderer-d2d/src/windowed.rs:492-494`
`WindowedUiBackend::close`），**非 guest 契约成员**。因此 guest 无法经 `ui@1` 触发窗口关闭；
窗口关闭是宿主/用户动作（`WM_CLOSE`），guest 通过 `poll-event` **观测**结果事件。

### 2. guest 观测呈现结果的通道 —— **无读回通道（合同缺口 1）**

- `run() -> result<bool, ui-error>`（`ui.wit:85`）：guest 只知 `Ok(true)`（呈现调用成功），
  **读不到帧字节**。
- 帧字节读回仅宿主侧：`WindowedUiBackend::read_presented_frame()`（`windowed.rs:483-485`）。
  `D2dUiRenderer` **不暴露** `read_presented_frame`（只暴露 `window_title`/`skipped_click_count`/
  `close`/`destroy`，`ui.rs:207-245`）。
- **结论**：G2 的「逐字节可核」只能是 **「guest 触发 + 宿主读回核验」**，**不是**「guest 自行
  观测」。`ui@1` 无读回通道登记为合同缺口（WIT 变更属 owner 裁决，见 STEP-0590 C-A 模式）。
- 附带：G2 的字节一致性同时是**跨实例确定性**证据（窗口后端的呈现帧 == 独立 `D2dUiBackend`
  离屏帧），但 G5 的正式声明（同树 N 次 → N 帧逐字节一致）仍需专测，本片**不**声称 G5。

### 3. guest 观测事件的通道 —— **有（`poll-event` 交付 `option<ui-event>`）**

`poll-event: func() -> option<ui-event>`（`ui.wit:83`），`ui-event` = `click|select|closed`
（`ui.wit:59-63`）。guest 可区分 `none` 与 `Some(closed)`——**G3 可 guest 级观测**。close 触发
为宿主动作（合同缺口 2：`ui@1` 无 close 函数，guest 不能自发起关闭；但关闭本身是宿主/用户
生命周期动作，非 guest 能力缺失）。

### 4. 现有 e2e 的 guest 覆盖缺口（STEP-0581 确认）

`tests/ui_authority.rs` 的 guest fixture 只覆盖 `window` 与 `poll-event` 的 `none` 分支；
`run()`/`closed` 在 e2e 中经 **宿主句柄**（`renderer.run()`、`renderer.close()`、宿主侧
`poll_event()`）跑通，**不是 guest 观测**（`STEP-0581:114-117,136`）。

## 实现（本片唯一生产外新增：`tests/ui_guest_visible.rs`）

纯证据片：**不改任何生产代码**（`ui.rs`/`lib.rs`/`windowed.rs`/`backend.rs`/WIT 全部只读），
仅新增一个 integration test 文件。

### 1. 真实后端适配（`ReadbackBackend`）

`D2dUiRenderer` 不暴露 `read_presented_frame`，但 G2 需要字节读回。`ReadbackBackend` 包装
**同一个生产后端** `WindowedUiBackend`（STEP-0338 渲染器核心 + STEP-0577 窗口壳的组合，
STEP-0581），把全部 13 个 `ui@1` 操作按 `D2dUiRenderer` 相同方式委托，另暴露宿主侧
`window_title`/`read_presented_frame`/`close`/`destroy` 与一个 `calls` 计数器（零触达断言用）。
**不是新渲染器**，不声称任何额外行为；guest 路径不变（guest 的 `run` 仍到达真实
`WindowedUiBackend::run`）。错误折叠 `fold` 按 `ui.rs::fold_error` 的文档映射复刻
（`InvalidArgument→limit`；`NoWindow|UnsupportedPlatform|RenderFailed→no-window`）。

### 2. 两个 hand-encoded guest Component（`wasm-encoder`）

- **Guest A**（导入 `window`/`text`/`run`/`poll-event`）：`window("ui-fixture",320,160)` →
  `text(root,"Sico")` → `run()` → `poll-event()`。成功路径 `exit = 1`（guest 观测到的窗口根
  句柄）；`window` 错误 → `110+disc`；`text` 错误 → `80+disc`；`run` 错误 → `90+disc`；
  `poll-event` 意外 `Some` → `70+disc`。`poll-event` 读 `option` 判别 @返回区+0、`ui-event`
  变体判别 @+8（镜像 `ui.rs` derive 的 case 序 click/select/closed）。
- **Guest B**（导入同四函数，只调 `poll-event`）：轮询 `poll-event`（≤64 轮），
  `option==1 ∧ 变体==2` 即 `closed` → `exit = 105`；64 轮无事件 → `exit = 106`。
- 两组件共享一个后端（`Arc<ReadbackBackend>`）跨两代：Guest A 建窗+呈现+空轮询，宿主
  `close()` 发 `WM_CLOSE`，Guest B 轮询到 `closed`。`PreparedProgram::run` 每次新 Store
  （`lib.rs:2366-2382`），`Ui1State` 只持 renderer Arc 克隆（`ui.rs:405-410`），窗口/壳线程
  在代间存活（无 destroy），故两代共享真实窗口。

### 3. 退出码范围约束

Runner 校验 guest `exit-code` 在 `0..=119`，故全部退出码落在该区间（`105/106/1` 与错误基
`110/90/80/70` 加判别式 ≤ 115）。首版用 `201/202` 触发 `Trap("guest exit value 201 is
outside 0..=119")` 后修正——这是**诚实失败**（runner 拒绝越界退出码），不是放宽断言。

### 4. fail-closed 与环境诚实

headless（无交互桌面）下 `CreateWindowExW` 失败 → guest `window()` 得类型化 `ui-error` →
测试断言 `(111..=115).contains(&exit)` 并打印 `ENV_LIMIT` 标记，**不伪造「已呈现」**；
非 Windows 主机 `WindowedUiBackend::new()` → 类型化 `UnsupportedPlatform`（`cfg(not(windows))`
测试）。本机（Windows 交互桌面）为真实路径：`window_title()` 为 `Some`、读回帧字节一致、
`closed` 交付。

## 验证（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线，温热 target 缓存）

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& $cargo test --offline --test ui_guest_visible -- --test-threads=1   # 新测试
& $cargo test --offline --test ui_authority -- --test-threads=1       # 回归
& $cargo fmt -- --check tests/ui_guest_visible.rs                     # 格式（我动范围）
& $cargo clippy --offline --test ui_guest_visible -- -D warnings      # clippy（我动范围）
git diff --check
```

结果：

| 检查 | 结果 |
|---|---|
| `ui_guest_visible` | **3 passed, 0 failed**：`guest_run_presents_a_real_frame_byte_identical_to_offscreen_oracle`、`guest_poll_event_delivers_closed_after_host_close`、`denied_guest_is_typed_authority_refusal_with_zero_backend_touch` |
| `ui_authority`（回归） | **7 passed, 0 failed**（4 既有 fixture-double + 3 真实后端 e2e 不变） |
| `cargo fmt -- --check tests/ui_guest_visible.rs` | 净 |
| `cargo clippy --test ui_guest_visible -- -D warnings` | 绿 |
| `git diff --check` | 净（无尾随空白；本片新增文件未引入） |

真实路径在本机跑通：`window_title()` 为 `Some("ui-fixture")`；guest A `exit=1`（观测句柄 1）；
`read_presented_frame()` 为 320×160 且 `pixels()` **逐字节**等于独立 `D2dUiBackend` 离屏帧
（SHA-256 摘要相等）；guest B `exit=105`（自身 `poll-event` 交付 `closed`）。

## G1–G6 达成状态更新表

| # | 证据 | 可执行核验 | 本片前 | 本片后 |
|---|---|---|---|---|
| G1 | guest `window()` 真实窗口 | guest fixture 调 `window()` 得 `Ok(handle)`；宿主 `window_title()` 为 `Some` | **已达成**（STEP-0581，runtime-verified） | **已达成**（本片再验证：`window_title()`=`Some("ui-fixture")`，guest exit=1） |
| G2 | guest `run()` 呈现帧证据 | guest 调 `run()`；宿主 `read_presented_frame()` 返回预期 client 尺寸且帧逐字节一致 | 未达成（仅宿主侧） | **达成（guest 触发 + 宿主读回核验）**：guest 自身 `run()` 后，宿主读回 DIB 320×160 且 `pixels()` 逐字节等于独立离屏帧。**guest 无读回通道 ⇒ 非「guest 自行观测」**，合同缺口 1 |
| G3 | guest `poll-event` 交付 `closed` | guest 的 `poll-event` 在关闭后观察到 `closed` | 未达成（仅宿主侧） | **达成（guest 观测；close 为宿主动作）**：宿主 `close()` 发 `WM_CLOSE` 后，guest B 自身 `poll-event` 交付 `closed`（exit 105）。guest 无 `close` 函数 ⇒ 触发为宿主侧，合同缺口 2 |
| G4 | guest `poll-event` 交付 `click(handle)`/`select` 且节点路由正确 | guest fixture 在已知子节点边界点击，收到 `click(该句柄)` | 未达成（命中测试缺失） | **未达成**（点→句柄命中测试仍缺，STEP-0590 C-C；`click`/`select` 变体存在但不交付句柄） |
| G5 | guest 级帧确定性 | guest 同树渲染 N 次→N 帧逐字节一致 | 未达成（仅渲染器核心语料，STEP-0338） | **未达成（正式声明）**——本片 G2 的跨实例字节一致是附带确定性证据（窗口后端呈现帧 == 独立离屏帧），不构成 G5 的正式声明（需同树 N 次专测） |
| G6 | clean-room pilot 应用（gate 5） | Sico 消费者应用经 `ui@1` 构建/运行真实窗口 | 未达成 | **未达成**（STEP-0581:138；gate 5 范围，本片不触碰） |

**判定**：按 STEP-0590 定义（G1∧G2∧G3∧G4），当前从「仅 G1」推进到 **G1∧G2∧G3**；G4 仍缺
（命中测试），故 **「guest 可见 UI 能力」仍未整体达成**——本片不声称达成，只推进两个分项。
G2 的「达成」是**收窄口径**（guest 触发 + 宿主读回核验），非「guest 自行观测」；G3 的
「达成」是 **guest 观测 + 宿主触发 close**。

## 诚实收窄（本片未声称的部分）

- **未翻任何 M24 §3 gate**；不改任何证据等级（`internal-fixture` guest + `runtime-verified`
  真实窗口，Windows x64——与 STEP-0581 同档，**非**试点级、**非**生产声明）。
- **未声称 G4/G5/G6**；G2 的达成**降级表述**为「guest 触发 + 宿主读回核验」，非「guest 自行
  观测」；G3 的 close 触发为宿主/用户动作。
- **未改 WIT/源码/既有测试/既有 STEP**；仅新增 `tests/ui_guest_visible.rs` 与本文件。
- **未 commit/push**；保留全部既有 dirty/untracked 改动（含并发代理的 `ui.rs`/`lib.rs`/
  `windowed.rs` 等，本片未触碰）。
- **未新增依赖**：`sico-ui-renderer-d2d`/`wasm-encoder` 均为既有（依赖）项，`Cargo.toml`/
  `Cargo.lock` 未因本片改动（`Cargo.lock` 的既有 `M` 来自并发代理，非本片）。
- **单平台**：Windows x64 真实路径；非 Windows 为类型化拒绝断言（`UnsupportedPlatform`）。
- **环境依赖**：真实窗口路径需交互桌面；headless 时测试走类型化拒绝断言 + `ENV_LIMIT`，
  不伪造「已呈现」。本机为真实路径（已跑通）。

## 合同缺口（owner 裁决项，本片只登记不裁决）

1. **G2 无 guest 读回通道**：`ui@1` 的 `run()` 返回 `result<bool, ui-error>`，guest 无法读帧
   字节；读回仅宿主侧 `WindowedUiBackend::read_presented_frame`。⇒ G2 只能是「guest 触发 +
   宿主读回核验」。WIT 追加（如 `read-frame` 或帧哈希查询）属 owner 裁决（STEP-0590 C-A 模式）。
2. **G3 无 guest close**：`ui@1` 无 `close` 函数（宿主侧仅 `D2dUiRenderer::close`/`WindowedUiBackend::close`）；
   guest 不能自发起关闭。关闭为宿主/用户动作，guest 只观测 `closed` 事件。是否追加 guest
   `close` 属 owner 裁决（STEP-0590 C-A 模式）。
3. **G4 点→句柄命中测试**：`click(ui-handle)`/`select` 变体**存在**（RFC-0052 §2）但**不交付**；
   坐标点击被跳过（宿主侧 `skipped_click_count` 可观测，`ui.rs:221-224`）。需渲染器布局
   （STEP-0590 C-C；能力仍属未来片）。

## 残余缺口与建议主线审核点

- **残余**：G4（命中测试，需布局+新 ADR）、G5 正式专测（同树 N 次）、G6（clean-room pilot，
  gate 5）；`ui-error` 无渲染失败判别式与 `event-overflow` 不可观测（STEP-0590 C-A/C-B，均
  owner 裁决项）；`no-window` 折叠（STEP-0581:120-122）。
- **建议审核点**：①本片是否把 G2/G3 记为「分项达成（收窄口径）」，还是要求 G2 升格为
  「guest 自行观测」（需 WIT 读回通道，owner 裁决）；②G3 的「宿主触发 close」是否可接受为
  guest 观测前提（建议可接受——关闭本质是宿主/用户动作，`poll-event` 交付 `closed` 才是
  guest 契约）；③`ReadbackBackend` 作为测试侧真实后端适配（非 `D2dUiRenderer`）是否被接受；
  ④G2 跨实例字节一致是否可预记 G5 附带证据（不构成正式 G5）。
