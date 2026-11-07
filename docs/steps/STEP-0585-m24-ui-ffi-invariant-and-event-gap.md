# STEP-0585: M24 — 窗口壳 `!Send/!Sync` 不变式固化 + 窗口事件缺口显式化（STEP-0577 独立安全复核收口 + STEP-0581 表述/缺口纠正）

> - status: implementation complete locally（**加固 + 缺口显式化**；不翻任何 M24 §3 gate，不改任何证据等级）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改
> - evidence: **internal-fixture + runtime-verified window**（Windows x64；真实窗口生命周期测试 + 手写编译期负约束断言的类型检查证据；**非**试点级、**非**生产声明）
> - artifacts: [`crates/sico-ui-renderer-d2d/src/shell/win.rs`](../../crates/sico-ui-renderer-d2d/src/shell/win.rs)、[`crates/sico-ui-renderer-d2d/src/shell/mod.rs`](../../crates/sico-ui-renderer-d2d/src/shell/mod.rs)、[`crates/sico-ui-renderer-d2d/src/windowed.rs`](../../crates/sico-ui-renderer-d2d/src/windowed.rs)、[`crates/sico-ui-renderer-d2d/src/events.rs`](../../crates/sico-ui-renderer-d2d/src/events.rs)、[`runner/sico-runner/src/ui.rs`](../../runner/sico-runner/src/ui.rs)

## Objective

STEP-0577 的**独立安全复核**（只读第三方）给出 1 项 medium 级潜伏 soundness 隐患 + 6 项 low
防御性建议；STEP-0581 的**主线核对**又发现其一处**表述与行为缺口**（称「坐标事件无 `ui@1`
变体故跳过」，但 `wit/script-ui-v1/ui.wit:59-63` 确有 `click(ui-handle)` / `select(ui-selection)`
变体；真实缺口是**点→句柄命中测试缺渲染器布局**）。本 STEP 收口复核的 medium 项与两项
high-value low 项，并把「坐标点击被跳过」从**静默跳过**改为**可观测的诚实行为**。

## 复核发现与对应处置

| # | STEP-0577 复核发现 | 等级 | 本 STEP 处置 |
|---|---|---|---|
| 1 | `WindowShell`/`WinShell` 的 `!Send + !Sync` **唯一**由 `Dib.bits: *mut c_void` 维持；`windows` 0.52 的 `HWND`/`HDC`/`HBITMAP`/`HGDIOBJ` 均为 `pub struct(pub isize)`（本身 `Send+Sync`）⇒ 一次重构（如 `Dib` 改存 `Vec<u8>`/`usize`）即让窗口壳**静默**变 `Send+Sync` ⇒ 窗口跨线程 ⇒ `WndProc` 状态 Box 数据竞争（UB）+ 泵错线程队列 | **medium（必修）** | 用 `PhantomData<*mut ()>` **刻意**固化；编译期负约束断言 + 「若去掉标记则 `Send`」镜像证据 |
| 2 | `Drop` 路径忽略 `DestroyWindow` 失败 ⇒ 窗口存活而 `self.state` 已释放 ⇒ `GWLP_USERDATA` 悬垂 | low (a) | 销毁失败时**不得**释放被 `GWLP_USERDATA` 引用的状态：`Drop` 中 `mem::forget(self.state.take())` 刻意泄漏；`destroy()` 已在失败时 `?` 提前返回、保留状态（补注释说明） |
| 3 | `BeginPaint` 失败仍调用 `EndPaint`（非法 `PAINTSTRUCT` 的 Win32 API 误用） | low (b) | 仅在 `BeginPaint` 成功（DC 非空）时调用 `EndPaint`；失败路径不调用、不伪造，注释说明 |

其余 4 项 low 建议（本片**只做上述两项高价值**）**登记不改**，列入「剩余缺口」。

## Changes

### 1. 窗口壳 `!Send + !Sync` 不变式固化（`shell/win.rs` + `shell/mod.rs`）

- **刻意标记**：`WinShell` 与 `WindowShell` 各新增一个零大小字段
  `_not_send_sync: PhantomData<*mut ()>`。`PhantomData<*mut ()>` 是 `!Send + !Sync`，且
  是**刻意、独立**于任何句柄/指针字段的标记——它不依赖 `Dib.bits` 之类的偶然字段，因此
  即使将来 `Dib` 改存 `Vec<u8>`/`usize`，壳仍保持 `!Send + !Sync`。`PhantomData` 字段不会被
  `dead_code` 警告（ZST、刻意置入），亦不影响 `Drop`/`Debug`。
- **编译期负约束断言（无新第三方依赖）**：手写
  `static_assertions`-风格 `assert_not_impl_any!`（歧义 trait 求解技巧）：
  - 对每个断言类型调用 `assert_not_impl_any!(T: Send, Sync)`，展开为
    `const _: fn() = || { ... <T as AmbiguousIfImpl<_>>::some_item ... }`；
  - `AmbiguousIfImpl<A>` 有 blanket impl `impl<T: ?Sized> AmbiguousIfImpl<()> for T`，并为
    每个被断言 trait 加第二个 impl `impl<T: ?Sized + $t> AmbiguousIfImpl<Invalid> for T`；
  - 若 `T` 实现被断言 trait，`<T as AmbiguousIfImpl<_>>::some_item` 有两个候选 impl，
    `_` 推断**歧义** ⇒ 编译错误（E0283）；若 `T` 不实现，仅 blanket 适用 ⇒ 编译通过。
  - 断言点：`shell/mod.rs` 的 `#[cfg(test)] mod ffi_invariants`（`WindowShell`，跨平台）与
    `shell/win.rs` 的 `#[cfg(test)] mod tests`（`WinShell`，Windows）。
- **可复核证据（两条）**：
  1. **负断言为真**：`assert_not_impl_any!(WindowShell: Send, Sync)` 与
     `assert_not_impl_any!(WinShell: Send, Sync)` 使**整个 crate 编译**；若任一壳变
     `Send`/`Sync`，构建即失败（`-D warnings` 下 clippy/test 全绿，证明无歧义）。
  2. **「若去掉标记则 `Send`」为真**：`shell/win.rs` 测试模块中的
     `WinShellMarkerFreeMirror`（`hwnd: Option<HWND>` + `state: Option<Box<ShellState>>` +
     `pixels: Option<Vec<u8>>` + `quit: bool`，即**改存 `Vec<u8>` 且无标记**的字段集）由
     `assert_send_sync::<WinShellMarkerFreeMirror>()` **证明是 `Send + Sync`**（该测试
     编译通过）。两条合起来证明：标记是唯一维持 `!Send` 的因素，一次「DIB 改 owned 存储」
     的重构若没有标记会**静默**让壳变 `Send + Sync` —— 正是 STEP-0577 指出的隐患。
  3. **宏本身正确性**：独立探针（`rustc --emit=metadata`）——`assert_not_impl_any!(String:
     Send, Sync)`（`String` 为 `Send+Sync`）报 **E0283 歧义**；`assert_not_impl_any!(T:
     Send, Sync)`（`T` 含 `PhantomData<*mut ()>`）**类型检查通过**（本机无 gcc ⇒ 仅
     `--emit=metadata` 类型检查，不经链接）。
- **单元测试**：`winshell_is_deliberately_not_send_sync`、
  `marker_free_refactor_mirror_would_be_send_sync`（`win.rs`）、
  `windowshell_is_deliberately_not_send_sync`（`mod.rs`），均断言标记零大小且镜像 `Send+Sync`。

### 2. low (a)：销毁失败不得释放被 `GWLP_USERDATA` 引用的状态（`shell/win.rs`）

- `WinShell::destroy()`：`DestroyWindow` 失败时 `?` 提前返回，`self.hwnd`/`self.state` 保持
  不变 ⇒ 框态仍存活且经 `GWLP_USERDATA` 可达（补注释：**失败绝不释放活窗口引用的内存**）。
- `WinShell::drop()`：`DestroyWindow` **失败**时用 `std::mem::forget(self.state.take())`
  **刻意泄漏**框态——`Drop` 无法返回类型化错误，而窗口仍存活且 `GWLP_USERDATA` 仍指向该
  框；释放它会让后续 `WndProc` 解引用**已释放内存**（UB）。泄漏小框态是唯一内存安全选择
  （后续正确销毁由对窗口的 `DestroyWindow` 完成）；DIB 与句柄是普通 owned 值，正常 drop。

### 3. low (b)：`BeginPaint` 失败不得 `EndPaint`（`shell/win.rs`）

- `WM_PAINT` 处理：`BeginPaint` 返回 `HDC`（windows 0.52 绑定直接返回，非 `Result`），
  **DC 非空**才配对调用 `EndPaint`；**DC 为空（失败）时不调用 `EndPaint`**（无有效 paint
  状态，调用即 Win32 API 误用），不伪造 `Paint` 之外的任何事件，且注释说明「`WM_PAINT`
  可能重发是诚实行为」。`EndPaint` 自身失败结果有注释说明为何可忽略（paint 收尾最后一步、
  无后续解引用、OS 在窗口销毁时回收）。

### 4. 事件缺口显式化（契约诚实，`runner/sico-runner/src/ui.rs`）

- **表述纠正**：`ui.rs` 与 `windowed.rs`/`events.rs`/`shell/mod.rs` 的注释改为准确表述——
  `ui@1` **确有** `click(ui-handle)` / `select(ui-selection)` 变体（RFC-0052 §2）；缺的是
  **点→句柄命中测试**（渲染器尚无布局），不是变体本身。
- **静默跳过 → 可观测诚实行为（方案①，见「方案与理由」）**：新增纯函数
  `classify_window_event(WindowEvent) -> (Option<UiEvent>, bool /*skipped_click*/)`：
  - `Closed` → `(Some(UiEvent::Closed), false)`；
  - 坐标 `Clicked` → `(None, true)`（不伪造句柄；跳过可观测）；
  - `Resized`/`Paint` → `(None, false)`（生命周期通知，无 `ui@1` 变体，不过边界）。
- `D2dUiRenderer` 新增 `skipped_click_count: AtomicU64` 与 `pub fn skipped_click_count() -> u64`
  （host 侧审计 affordance，**非** guest 契约成员，镜像既有 `window_title()` 模式）；
  `poll_event` 在跳过坐标点击时 `fetch_add`。调用方可区分「点击被忽略（诚实已知缺口）」
  与「无事件」（计数为 0 且 `None`）；shell 线程丢失仍返回 `None` 且**不**递增计数。
- **补测试**：`a_coordinate_click_is_not_fabricated_into_a_handle_click_and_is_observable`——
  断言坐标 `Clicked` 映射为 `(None, true)`（**不**被伪装成 `click(handle)` 正常事件，且
  跳过路径可观测），`Closed` 映射为 `(Some(closed), false)`，`Resized`/`Paint` 为 `(None,
  false)`。此为 **internal-fixture** 单元测试（不触真实窗口）。

## 方案与理由（B 部分：默认取①）

- **方案①（采用）**：在 `ui@1` 闭集无「点击未映射」判别式的前提下，**不**私自扩 WIT
  （WIT 属合同面，须 owner 裁决），改为在 **runner 侧既有可观测手段**（host 侧
  `skipped_click_count` 计数 + 纯函数 `classify_window_event` 的 `skipped_click` 信号）把
  跳过**显式登记为已知缺口**，并在 STEP 文档「剩余缺口」列入待办（下一片点→句柄命中测试）。
- **方案②（不采用）**：交付 `click(window_handle)`。**判定会产生错误归属**：渲染器**尚无
  布局** ⇒ 子元素无矩形 ⇒ 无法证明点击落在根窗口还是子元素；把「窗口内任意点」一律归到
  根句柄会**把子元素上的点击误归属**到根，违反「宁要类型化拒绝，不要隐藏回退」。故**不采用**。
- 结论：①为诚实、可复核的方案；点击发生但无法映射句柄时，host 可观测到
  `skipped_click_count > 0`，guest 经 `poll-event` 得到 `none` 的**同时** host 明确知道
  「这不是无事件」，且该缺口在契约诚实声明与待办中**显式登记**，不伪装、不静默。

## Executed validation（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线，温热缓存）

- `cargo test -p sico-ui-renderer-d2d --offline` → **10 通过，0 失败，1 ignored**（lib 单元
  含 3 个新增不变式/镜像测试；`frame_corpus`/`event_corpus` 冻结哈希仍绿 ⇒ 渲染/事件行为
  未动）。
- `cargo test -p sico-ui-renderer-d2d --offline --lib -- --ignored` → **真实窗口生命周期 ok
  （0.22s）**：new → window → 渲染+呈现 → 读回 320×200 → close → 泵到 `Closed` → destroy
  幂等 → 二次 destroy 类型化 `NoWindow`（`WindowedUiBackend` 组装路径不受 `!Send` 标记影响）。
- `cargo test -p sico-ui-renderer-d2d --offline --test event_corpus -- --ignored` → **既有
  STEP-0577 冒烟仍 ok（0.24s）**：present 读回逐字节不变。
- `cargo clippy -p sico-ui-renderer-d2d --offline --all-targets -- -D warnings` → **绿**。
- `cargo test --offline --lib -- --test-threads=1`（runner workspace）→ **42 通过，0 失败**
  （含新 `a_coordinate_click_...` 测试；原 41 项不变，均绿）。
- `cargo test --offline --test ui_authority -- --test-threads=1` → **7 通过，0 失败**（4 既有
  fixture-double + 3 真实后端 e2e，本机真实窗口通过）。
- `cargo clippy --offline --lib -- -D warnings` 与 `cargo clippy --offline --test ui_authority
  -- -D warnings` → **绿**。
- `cargo fmt -p sico-ui-renderer-d2d -- --check` → 净；runner `cargo fmt -- --check` → **仅
  `tests/provider_crash_isolation.rs` 有 diff**（**既有未跟踪文件**，非本片触碰，未改；本片
  变更文件均净）。`git diff --check` → 净。
- `tools/validate-module-boundaries.ps1` → **MODULE_BOUNDARIES_OK model=openjdk-style
  packages=30 exceptions=4**（与基线逐字一致）。
- **宏正确性探针**（`rustc --emit=metadata`，本机无 gcc 不经链接）：`assert_not_impl_any!(
  String: Send, Sync)` → **E0283 歧义**（`Send+Sync` 类型被正确拒绝）；
  `assert_not_impl_any!(T: Send, Sync)`（`T` 含 `PhantomData<*mut ()>`）→ **类型检查通过**
  （`!Send + !Sync` 类型被正确接受）。

## Declared limits（诚实收窄，未声称的部分）

- **M24 §3 gate 无一翻转**；`wit/**`、`docs/steps/README.md`、`docs/STATUS.md`、
  `docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、
  `runner/sico-runner/src/lib.rs`、`crates/sico-package/**`、`guests/**` 与冻结语料**均未触碰**。
- **点击→句柄命中测试仍缺**（本片**显式登记**为已知缺口）：`poll-event` 只交付坐标
  `Clicked`；坐标点击被跳过时 `skipped_click_count` 递增、guest 仍得 `none`——这是
  `ui@1` 闭集下唯一诚实信号，**不是**「无事件」，且缺口已登记待办。
- **`ui-error` 闭合集无渲染失败判别式**：`RenderFailed`/`NoWindow`/`UnsupportedPlatform` →
  `NoWindow`、`InvalidArgument` → `Limit`（STEP-0581 既有折叠，本片未改；专设判别式需 WIT
  追加，属合同面，留 owner）。`event-overflow` 亦不经过 `poll-event` 上浮（`option` 无错误通道）。
- **`!Send + !Sync` 标记是零大小字段**（`size_of::<PhantomData<*mut ()>>() == 0`），不影响
  布局/Drop/Debug；`WindowedUiBackend`（runner 生产后端）保持 **`Send + Sync` 无 `unsafe
  impl`**（窗口壳不离开其线程，标记不传播）。
- **`Drop` 中销毁失败会泄漏小框态**：这是「窗口仍存活 + `GWLP_USERDATA` 仍引用」下唯一
  内存安全选择；泄漏量 = 一个 `ShellState`（队列预分配 256 事件 + 少量字段），仅发生在
  `DestroyWindow` 失败的罕见路径。`destroy()` 显式路径仍返回类型化错误且保留状态。
- **单平台**：Windows x64；非 Windows `D2dUiRenderer::new()` → 类型化 `no-window`（既有）。
- **低价值 4 项 low 建议未处置**（登记不改，见「剩余缺口」）。

## 剩余缺口（登记，不改）

1. **点→句柄命中测试**（`click(handle)` / `select` 交付，需渲染器布局导出）——下一片；本片
   已把「坐标点击跳过」从静默改为可观测 + 显式登记。
2. **STEP-0577 复核其余 low 建议**（4 项，未在本次两项目标内）：如 `SetWindowLongPtrW`
   失败结果注释、GDI 资源清理失败（`DeleteDC`/`DeleteObject`）的显式化等，均属防御性，
   已由现有空指针守卫/资源回收机制兜底，无可达 UB；留后续加固片。
3. **guest 级 `run`/`closed` fixture**（STEP-0581 既有）；`ui-error` render-failure 判别式
   （需 WIT 追加，owner）；`event-overflow` 经 `poll-event` 暴露（闭集无通道）。
4. **呈现假定帧尺寸=客户区**（无 resize 重渲，STEP-0577 既有语义，未改）。

## 关系说明

本 STEP 是 STEP-0577 复核的收口片（medium + 两项 high-value low）与 STEP-0581 主线核对
纠正片。它**不**改变 STEP-0338 帧语料（4 冻结哈希复跑不变）、STEP-0577 事件语料（3 冻结
哈希复跑不变），**不**翻转任何 M24 §3 gate，**不**改变任何证据等级。
