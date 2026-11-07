# STEP-0581: M24 §8.6.1 — runner 侧 `ui@1` host-provider 接到真实 Direct2D 窗口后端（STEP-0338 渲染器核心 + STEP-0577 窗口壳的组装切片）

> - status: implementation complete locally（**runner `ui@1` → 真实 D2D 窗口后端接线**；不翻任何 M24 §3 gate）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改
> - evidence: **runtime-verified window + internal-fixture guest**（Windows x64 真实 `CreateWindowExW` 窗口 + GDI DIB `BitBlt` 呈现 + `WM_CLOSE` 事件投递，经 hand-encoded guest Component 经真实 Runner 准备/运行路径；**非**试点级、**非**生产声明）
> - artifacts: [`crates/sico-ui-renderer-d2d/src/windowed.rs`](../../crates/sico-ui-renderer-d2d/src/windowed.rs)、[`runner/sico-runner/src/ui.rs`](../../runner/sico-runner/src/ui.rs)、[`runner/sico-runner/tests/ui_authority.rs`](../../runner/sico-runner/tests/ui_authority.rs)

## Objective

STEP-0338 交付了离屏可哈希渲染器核心，STEP-0577 交付了 OS 窗口壳与事件泵，但两者之间
**没有组装**：runner 侧 `ui.rs` 的 `UiRenderer` trait 仍是「绑定/授权拒绝层」的 seam，生产
实现从未存在 —— 无后端时 `ui@1` 不广告（导入它的 guest 实例化失败），有后端且无授权时
每个调用返回 `authority-denied`。本 STEP 补上这第二件后续切片：把渲染器核心与窗口壳
组装成一个 `Send + Sync` 的生产后端，接到 runner 的 `UiRenderer` seam 上，使受权的
`window` 调用真实创建 OS 窗口，`run` 真实呈现，`poll-event` 真实泵事件。

## Changes

### 1. 渲染器 crate：`src/windowed.rs`（新模块，组装 seam）

`WindowedUiBackend` —— `Send + Sync` 的生产后端，组合 `D2dUiBackend`（离屏树）与
`WindowShell`（OS 窗口 + 泵 + 呈现）：

- **线程模型**：Win32 窗口线程亲和（窗口必须在创建线程上泵消息）。因此 `WindowShell`
  放在一条**专用 shell 线程**上，本对象经请求/应答 channel 与其对话；树留在调用线程
  （`Mutex` 后的纯数据）。`Sender`/`Mutex<Receiver>`/`JoinHandle` 均 `Send + Sync`，
  所以 `WindowedUiBackend` **天然 `Send + Sync`，无任何 `unsafe impl`** —— shell 的
  `!Send` 状态从不离开其线程。`Receiver` 是 `!Sync`，故应答侧用 `Mutex` 包裹（单消费者、
  命令串行、无争用），仅为满足 `Sync`。
- **错误闭集**：每个失败路径都是类型化 `UiBackendError`，从不 panic、从不静默吞错。
  channel 断开 / 协议失配 → `RenderFailed{shell thread lost|protocol}`；shell 线程启动失败
  → `RenderFailed`；非 Windows → `UnsupportedPlatform`。
- **`window` 顺序**：先建 OS 窗口（shell 线程），再插树根。shell 失败绝不留下半建树；
  树步骤万一拒绝（根已存在 —— 实际不可达，因为 shell 会先拒绝）则拆掉刚建的 OS 窗口。
  标题/尺寸在创建前校验（`title-bytes` / `window-size` 类型化拒绝）。
- **`run`**：调用线程离屏渲染一帧（纯函数，ADR-0018 §3），仅字节跨 channel 到 shell
  线程 `present_frame`（逐字拷贝 DIB + `BitBlt`，STEP-0577 既有路径，帧绝不重算）。
- **`poll_event`**：一次泵返回一个 `WindowEvent`（client 坐标）。`click(handle)` /
  `select` 仍不交付 —— 点→句柄命中测试需要渲染器布局（下一片），诚实不伪造句柄。
- **host 侧生命周期 affordance**：`close`（`WM_CLOSE`）/`destroy`（幂等）/`window_title`
  /`read_presented_frame`，供宿主与 e2e 审计用，**非** guest 契约成员。
- 单元测试：`real_window_lifecycle_via_windowed_backend`（**opt-in `#[ignore]`**，镜像
  STEP-0577 冒烟：new → window → 渲染+呈现 → 读回 320×200 → close → 泵到 `Closed` →
  destroy 幂等 → 二次 destroy 类型化 `NoWindow`）。

### 2. runner：`src/ui.rs`（`D2dUiRenderer` 生产适配 + 错误折叠）

- `D2dUiRenderer { backend: WindowedUiBackend }` —— 实现既有 `UiRenderer` trait（13 函数），
  `new()` 仅构建后端（非 Windows → `UnsupportedPlatform` 折叠为类型化 `no-window`）。
  授权判断**不在此类型**：`guarded()` 的 default-deny 门控不变，无授权绝不触达后端。
- **错误折叠** `fold_error`：渲染器类型化失败折叠进 guest 闭合 `ui-error` 集。
  `UnknownHandle`/`WrongParentKind`/`Limit` 直映；`InvalidArgument` → `Limit`（声明的范围
  约束）；`NoWindow`/`UnsupportedPlatform`/`RenderFailed` → `NoWindow`（**记录型折叠**：
  v0 `ui-error` 没有渲染失败判别式，RFC-0052 §3 闭合集；专设判别式需 WIT 追加，超出本片）。
  折叠是闭合集内的唯一诚实选项 —— 从不是 panic、从不是静默吞错。
- **`poll_event`**：泵到出现可交付事件为止。`Closed` → `closed` 变体；坐标 `Clicked` /
  `Resized` / `Paint` 无 `ui@1` 变体，**跳过而非伪造** `click(handle)`；shell 线程丢失 →
  `None`（`option<ui-event>` 契约下唯一诚实信号，文档记录）。
- **`with_ui_backend` builder 与 `UiHost` 不变**：有后端才 `link_ui`（广告），无后端不广告
  —— 语义未动。

### 3. runner：`tests/ui_authority.rs`（真实后端 e2e）

复用既有 hand-encoded guest Component（`window` + `poll-event` 两个 fixture），经真实
`Runner` 准备/运行路径，`UiRenderer` 换成生产 `D2dUiRenderer`：

- `denied_window_with_real_backend_is_authority_refusal_without_touching_backend`：无授权 →
  `authority-denied`（exit 115），后端零触达（`window_title()` 为 `None`）。
- `granted_window_with_real_backend_creates_a_real_window`：受权 → guest 创建**真实 OS 窗口**
  并观察到根句柄（exit 1）；若 headless 无交互桌面则断言类型化 `ui-error`（exit 111–115）
  并打印 `ENV_LIMIT` 标记 —— 不伪造「已呈现」。
- `granted_window_lifecycle_present_close_event_destroy`：受权 → guest 建窗（exit 1）→ 经
  生产后端句柄 `run()` 呈现 → close → 泵到 `Closed` → destroy 幂等 → 二次 destroy 类型化
  `NoWindow`。
- 非 Windows：`d2d_renderer_is_refused_typed_off_windows`（`D2dUiRenderer::new()` →
  类型化 `no-window`，不 panic）。

### 4. 依赖边

`runner/sico-runner/Cargo.toml` 新增 `sico-ui-renderer-d2d`（路径依赖）与锁文件更新。
**`tests/architecture/module-boundaries.json` 未改**：该验证器只覆盖根 workspace 成员
（30 个包），`sico-runner` 是**独立 workspace**（`runner/sico-runner/[workspace]`），不在
契约内；把它登记进契约会令验证器抛「module contract references missing workspace
package」，故**不登记**（见验证）。

## Executed validation（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线）

- `cargo test -p sico-ui-renderer-d2d --offline` → **17 通过，0 失败，2 ignored**（lib 7 +
  event_corpus 5 + frame_corpus 5；ignored = 本片 `windowed` 真实窗口 + STEP-0577 既有冒烟）。
- `cargo test -p sico-ui-renderer-d2d --offline --lib -- --ignored` → **真实窗口生命周期 ok
  （0.22s）**：new → window("Lifecycle",320,200) 根句柄 1 → 渲染+呈现 → 读回 320×200 →
  close → 泵到 `Closed` → destroy 幂等 → 二次 destroy 类型化 `NoWindow`。
- `cargo test -p sico-ui-renderer-d2d --offline --test event_corpus -- --ignored` → **既有
  STEP-0577 冒烟仍 ok（0.22s）**：present 读回逐字节不变 ⇒ 呈现路径未受组装影响。
- `cargo clippy -p sico-ui-renderer-d2d --offline --all-targets -- -D warnings` → **绿**。
- `cargo test --offline --lib -- --test-threads=1`（runner workspace）→ **41 通过，0 失败**
  （含 `ui::tests` 4 个既有：discriminants / default-deny / 无后端不广告 / 全表面注册）。
- `cargo test --offline --test ui_authority -- --test-threads=1` → **7 通过，0 失败**（4 既有
  fixture-double + 3 新真实后端 e2e）。真实窗口 e2e 在本机通过（`window_title()` 为 `Some`，
  即真实 OS 窗口创建成功，guest 观察到句柄 1）。
- `cargo clippy --offline --lib -- -D warnings` 与 `cargo clippy --offline --test ui_authority
  -- -D warnings` → **绿**。
- `cargo fmt -p sico-ui-renderer-d2d -- --check` → 净；`cargo fmt -- --check`（runner 包，
  只检我动范围）→ 净。`git diff --check` → 净。
- `tools/validate-module-boundaries.ps1` → **MODULE_BOUNDARIES_OK model=openjdk-style
  packages=30 exceptions=4**（与基线逐字一致）。说明：runner 是独立 workspace，该验证器不
  覆盖它，故新依赖边不落入契约；未登记（登记会令验证器红）。

## Declared limits（诚实收窄，未声称的部分）

- **guest 可见 UI 能力（window 操作）达成**：受权 guest 的 `window()` 调用经真实
  `CreateWindowExW` 创建 OS 窗口并返回根句柄 1 —— guest 确实观测到真实 UI。**`run()` 呈现
  与 `poll-event` 事件投递在同一生产后端上实测跑通**（e2e 经后端句柄驱动），但 guest 级
  fixture 只覆盖 `window` 与 `poll-event` 的 `none` 分支；`run()`/`closed` 的 **guest 级**
  直观测尚未覆盖（需要 guest 调 `run`/`poll` 的 fixture，后续片）。
- **点击→句柄命中测试仍缺**：`poll_event` 只交付坐标 `Clicked`，不交付 `click(handle)` /
  `select`；需要渲染器布局（下一片）。坐标点击在适配层被**跳过**而非伪造句柄。
- **`ui-error` 闭合集无渲染失败判别式**：窗口创建/呈现失败折叠为类型化 `no-window`
  （`RenderFailed` 无 guest 判别式；专设需 WIT 追加，超出本片）。`event-overflow` 同样不
  经 `poll-event` 上浮（契约返回 `option`，无错误通道），属闭合集的预留判别式。
- **呈现假设帧尺寸 = 窗口 client 尺寸**：本片不重渲染 resize；窗口被拉大时 BitBlt 以帧
  尺寸绘制（STEP-0577 既有语义，未改）。坐标确定性仅当进程最终 DPI-unaware 时成立。
- **真实窗口测试默认跑**（非 `#[ignore]`，本片 e2e）：headless/CI 无交互桌面时 degrades
  到「类型化 `ui-error` 断言 + `ENV_LIMIT` 标记」，不伪造「已呈现」；渲染器 crate 内
  生命周期测试保持 opt-in `#[ignore]`（镜像 STEP-0577）。
- **单平台**：Windows x64；非 Windows `D2dUiRenderer::new()` → 类型化 `no-window`。
- **unsafe 说明**：`windowed.rs` 无 `unsafe`；平台 unsafe 仍收敛在 `render/win.rs` 与
  `shell/win.rs`（STEP-0338/0577 既有），本片未新增任何平台 unsafe。
- **不翻任何 M24 §3 gate**，不改任何证据等级。

## 后续切片（未开工，按契约顺序）

1. 点→句柄命中测试（`click(handle)` / `select` 交付，需渲染器布局导出）。
2. guest 级 `run`/`poll` 的直观测 fixture（当前经后端句柄驱动）。
3. 文件打开/保存对话框（M26 PDF 工具所需，ADR-0018 §4）。
4. M24 转换器试点（gate 5）方可评估；M17 gate 1 加速器仍是**具名外部门**，本片不触碰。

## 关系说明

本 STEP 是 STEP-0577 声明的后续切片 #2（runner 侧 `ui@1` 接线）。它把渲染器核心
（STEP-0338）与窗口壳（STEP-0577）组装为生产后端，使受权 guest 的 `window` 调用真实创建
OS 窗口 —— 这是 M24 §8.6.1 「guest 可见 UI 能力前置条件」的落地；但**不**翻转任何 M24
§3 gate（由出口审计裁决），**不**改变 STEP-0338 帧语料（4 冻结哈希不变，本片复跑绿）与
STEP-0577 事件语料（3 冻结哈希不变，本片复跑绿）。
