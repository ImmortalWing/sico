# STEP-0577: M24 §8.6.1 — OS 窗口壳 + 事件泵（STEP-0338 渲染器核心的后续切片）

> - status: implementation complete locally（**窗口壳 + 事件泵切片**；不翻任何 §3 gate，不声称 guest 可见 UI 能力）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3，RFC-0052 §2/§3）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted 2026-09-30 STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改
> - evidence: **internal-fixture / runtime-verified window-shell**（Windows x64 真实 `CreateWindowExW` + `PeekMessageW` 消息泵 + GDI DIB `BitBlt` 呈现路径；**非** guest 级、**非**试点级、**非**生产声明）
> - artifacts: [`crates/sico-ui-renderer-d2d/`](../../crates/sico-ui-renderer-d2d)、[`tests/event_corpus.rs`](../../crates/sico-ui-renderer-d2d/tests/event_corpus.rs)、[`examples/event_digest.rs`](../../crates/sico-ui-renderer-d2d/examples/event_digest.rs)、[`docs/evidence/m24/ui-event-corpus.json`](../evidence/m24/ui-event-corpus.json)

## Objective

STEP-0338 交付了**离屏、headless、可哈希**的渲染器核心；它明确把「OS 窗口外壳与事件泵
未做、`poll-event` 不在 v0 面内」列为声明限制。本 STEP 补齐这第一件后续切片：真实 OS
窗口（创建/销毁）、消息泵（pump-once 与 drain）、窗口事件到既有 UI 事件模型的确定性映射、
以及与渲染器的呈现桥接（离屏帧逐字节拷贝 → GDI DIB → `BitBlt`，渲染核心一行不改）。

## Changes

### 新模块（均属 `crates/sico-ui-renderer-d2d`）

- **`src/events.rs`（事件模型安全核）** —— 平台中立，不引用任何 Win32 类型：
  - `RawMessage`（`Close` / `Resize{width,height}` / `Paint` / `LeftButtonDown{x,y}`）与
    `WindowEvent`（`Closed` / `Resized` / `Paint` / `Clicked`）。**命名对齐既有 `ui@1`
    变体**：`Closed` ↔ `closed`、`Clicked` ↔ `click`（RFC-0052 §2）；`Resized` / `Paint`
    是窗口生命周期通知，无 `ui@1` 变体，只驱动重渲染、**不过 guest 边界**。
  - `translate()`：`RawMessage → Option<WindowEvent>`，**过滤退化消息**（零宽/零高 resize
    如最小化窗口、负 client 坐标点击）。
  - `MAX_QUEUE_EVENTS = 256`：事件队列上限，**分配前判定**。
  - `event_sequence_digest()`：固定版本化 wire 形式（每事件一个 tag 字节 + 小端载荷）的
    SHA-256，作为冻结事件语料身份。
- **`src/shell/mod.rs`（平台分派壳）** —— `WindowShell`
  （`new` / `create_window` / `pump_once` / `drain` / `present_frame` / `read_presented_frame` /
  `close` / `should_quit` / `client_size` / `destroy`），`MAX_TITLE_BYTES = 256`。
- **`src/shell/win.rs`（本 crate 第二个 unsafe 模块）** —— 与既有 `render/win.rs` 相同的
  unsafe 集中模式（ADR-0018 §1：平台 unsafe 收敛到最小 adapter；共享核 unsafe-free）：
  - 窗口类注册（进程内 `OnceLock`）、`CreateWindowExW` / `DestroyWindow`，client 区按请求的
    96-DPI 逻辑尺寸（`AdjustWindowRectEx`），**DPI-unawareness 尽力设置**（进程已钉不同
    awareness 时以进程为准，不作坐标确定性声称）。
  - 消息泵：`PeekMessageW` + `TranslateMessage` + `DispatchMessageW`，**pump-once 与 drain**
    两种；`WM_CLOSE → Closed`、`WM_SIZE → Resized`、`WM_PAINT → Paint`、`WM_LBUTTONDOWN →
    Clicked(client 坐标)`；`WM_QUIT → should_quit()`（close 后窗口被 OS 销毁时先补一次
    peek 捕获已排队的 `WM_QUIT`，循环确定性终止，不再空转）。
  - 事件队列：预分配 256，**按类型无损合并**（Paint 合并且、Resized 取最新、Closed 幂等且
    终态、Clicked 为点事件不合并），**超限报 overflow 标志而非静默丢弃**。
  - 呈现路径：帧字节**逐字拷贝**进 `BITMAPV5HEADER` + `BI_BITFIELDS` + 显式 BGRA mask DIB
    （STEP-0338 实测的必需形态），`BitBlt` 到窗口 DC；**帧绝不在此重算**。
  - `destroy` **幂等**：窗口已被 `WM_CLOSE → DestroyWindow` 销毁时只清理，不二次销毁。
- **`src/shell/unsupported.rs`（非 Windows 类型化拒绝）** —— 不假装能开窗。
- **`src/lib.rs`** —— 模块声明与再导出（`D2dUiBackend` / `UiBackendError` / `Frame`；
  `events` / `shell` / `tokens` / `tree` 公开）。

### 测试与证据

- **`tests/event_corpus.rs`（冻结窗口事件语料）** —— 3 条合成事件序列（resize_paint /
  click / close），**钉死 SHA-256**（跨进程一致：冻结常量在每次进程重算比对，与既有
  帧语料同机制）；另含 `translate` 全映射与退化过滤、标题超限类型化拒绝（拒绝后不残留
  半成品窗口）、以及**真实窗口冒烟**（opt-in `#[ignore]`）。
- **`examples/event_digest.rs`** —— 事件语料重取哈希工具（镜像 `frame_digest.rs`）。
- **`docs/evidence/m24/ui-event-corpus.json`** —— 事件语料证据行（镜像
  `ui-frame-corpus.json`）。
- **`src/shell/win.rs` 内单元测试** —— 队列上限/合并/overflow/Closed 终态语义。

### 行为无关的整改（渲染核心语义不变）

- `src/render/win.rs` 与 `src/tokens.rs`：仅新增**行为无关的 pedantic-cast `#[allow]`**
  （有界值 ≤16 384 / ≤0xFF 的 `as f32` / `as u32`）；`frame_corpus_digests_are_frozen`
  复跑**仍绿**，冻结帧语料 4 个哈希逐字节不变 ⇒ 渲染行为未动。
- 顺带修复本切片既有但从未编译的 `win.rs` 的 ~13 处编译错误与 11 处
  `unsafe_op_in_unsafe_fn`/未用导入告警（此前 `lib.rs` 全零，整个 crate 无法构建）。

## Executed validation（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线）

- `cargo test -p sico-ui-renderer-d2d --offline` → **17 通过，0 失败，1 ignored**
  （7 lib 单元 + 5 event_corpus + 5 frame_corpus；`frame_corpus_digests_are_frozen` 仍绿，
  证明渲染器行为未变）。
- `cargo test -p sico-ui-renderer-d2d --offline --test event_corpus -- --ignored` →
  **真实窗口冒烟 ok（0.22s）**：`create_window("Smoke",320,200)` → 离屏渲染 320×200 →
  `present_frame` → `read_presented_frame` 与渲染帧 **SHA-256 逐字节相同** →
  `client_size == (320,200)` → `close()` → `pump_once` 交付 `Closed` → `destroy` 幂等。
- `cargo clippy -p sico-ui-renderer-d2d --offline --all-targets -- -D warnings` → **绿**。
- `cargo fmt -p sico-ui-renderer-d2d -- --check` → 净；`git diff --check` → 净。

## Declared limits（诚实声明，未声称的部分）

- **无 guest 可见 UI 能力**：runner 侧 `ui@1` 接线是下一切片；本 STEP 的点击只捕获为
  **client 坐标**，点→句柄命中测试需要渲染器布局 ⇒ **不交付 `click(handle)` / `select`**。
- **不翻任何 M24 §3 gate**，不改任何证据等级。
- **真实窗口冒烟是 opt-in**（`#[ignore]`）：headless/CI 无交互桌面时不开真窗，套件默认
  不跑它；已在证据主机真实跑过（见上）。未伪造成「自动跑通」。
- **帧哈希仍是证据主机局部**（STEP-0338 既有声明）：本 STEP 呈现路径是逐字节拷贝，
  不重算、不改哈希。
- **坐标确定性**：仅当进程最终处于 DPI-unaware 时成立；进程已钉其它 awareness 时以进程
  为准，不作确定性声称。
- **限额**：标题 256 UTF-8 字节（UTF-16 编码/任何窗口分配前判定）；事件队列 256
  （预分配、按类型合并、超限报 overflow）。
- **单平台**：Windows x64；非 Windows 主机 `WindowShell` / `D2dUiBackend` 均为类型化拒绝。
- **unsafe 模式说明**：本 crate 用「crate 级 `unsafe_code = allow`（覆写 workspace forbid）
  + unsafe 收敛到 `render/win.rs` 与 `shell/win.rs` 两个模块」的既有约定；共享核
  **未**加逐模块 `#[deny(unsafe_code)]`（与既有 `render` 模块一致，本 STEP 未改变该模式）。

## 后续切片（未开工，按契约顺序）

1. runner 侧 `ui@1` 接线（`UiRenderer` seam 绑定，**不得**改 guest 合同面；`poll-event`
   经本 STEP 的事件泵落地）。
2. 点→句柄命中测试（`click(handle)` / `select` 交付，需渲染器布局）。
3. 文件打开/保存对话框（M26 PDF 工具所需，ADR-0018 §4）。
4. M24 转换器试点（gate 5）方可评估；M17 gate 1 加速器仍是**具名外部门**，本 STEP 不触碰。

## 关系说明

本 STEP 是 STEP-0338 声明的后续切片 #1；它解除的是「窗口壳与事件泵未做」这一**能力**
缺口，**不**改变 STEP-0338 的帧语料结论（4 个冻结哈希不变），**不**翻转任何 M24 §3 gate。
