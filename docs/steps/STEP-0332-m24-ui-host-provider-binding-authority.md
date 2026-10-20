# STEP-0332: M24 #5（部分）— `sico:script/ui@1.0.0` host-provider 绑定 / 默认拒绝授权层

> - status: implementation complete locally（绑定 + 默认拒绝授权层 + **runtime-verified end-to-end 授权拒绝**）；**不移动任何 M24 §3 gate**；真实渲染帧证据仍未交付（gated on #4，见 Honest residuals）
> - phase: M24 执行卡 #5（UI 切片 2：`ui@1` 控件面 + 事件 + 授权），按 [STEP-0330 §5 队列](./STEP-0330-m24-24a-contract-alignment-codec-probe.md) 与 [RFC-0052 A1](../rfc/RFC-0052-amendment-A1-host-provider-ui-surface.md) §3「binding/authority-refusal layer」范围
> - completed: 2026-09-30
> - renumber: 主权威线（`codex/m22-w1-ci` + M23）最高为 STEP-0329；M24 轨已占 STEP-0330/0331，本片按「不抢占、不重号」规则取下一空闲号 **STEP-0332**；最终号在 M24 轨并入时再确认
> - evidence: internal-fixture（单元级 + runtime-verified end-to-end）——`runner/sico-runner/src/ui.rs` 4/4 单元测试绿；`sico:script/ui@1.0.0` 绑定经真实 wasmtime `=47.0.2` Linker 注册校验；`runner/sico-runner/tests/ui_authority.rs` **4/4 绿**——`wasm-encoder` 手编真实客户组件导入 `ui@1`（`window` + `poll-event` 两条**授权门控闭包各自不同**的路径）、经 runner 实例化并断言：`window` 无授权 → 类型化 `authority-denied`（`exit=110+5=115`）且渲染器 double 调用计数 **0**，受权 → 返回 fixture handle（`exit=1`）且调用计数 **1**；`poll-event` 无授权 → 类型化 `none`（`exit=100+0=100`）且**事件队列零触达**，受权 → 投递 `Some` 事件（`exit=100+1=101`）且 poll 计数 **1**。完整 runner 套件单线程全绿（含 runner 37/37、http2 6/6、image_codec 5/5，无回归）

## Objective

落地 RFC-0052（经 A1 改首页）§2 冻结的 `ui@1` host-provider 面的**绑定 +
默认拒绝授权层**：把封闭 v0 UI 面注册进组件 `Linker`，并在任何渲染器效果之前
对每次调用做默认拒绝授权门控。本片**不**实现真实窗口/帧/事件泵（那是 ADR-0018
渲染器 adapter = #4，本机被 Direct2D/DirectWrite 离线绑定外部门挡住，
STEP-0330 §5）。授权门控是 M24 UI 线的安全核心（AGENTS.md §4：authority
explicit / least-privilege / versioned / default-deny），可完全离线交付与验证。

## 合同依据（不改合同）

- [RFC-0052 §2](../rfc/RFC-0052-native-ui-source-binding-v0.md)：封闭控件面
  （`window/column/row/text/button/dropdown/image-preview/progress/set-text/
  set-progress/draw-image/poll-event/run`）+ `record UiHandle{id:U64}` +
  `UiEvent = click(handle)|select(handle,index)|closed`。
- [RFC-0052 §3](../rfc/RFC-0052-native-ui-source-binding-v0.md)：`UiError`
  封闭集（声明序即 wire discriminant 序，append-only）：`unknown-handle`(0) /
  `wrong-parent-kind`(1) / `limit`(2) / `no-window`(3) / `event-overflow`(4) /
  `authority-denied`(5)。
- [RFC-0052 A1](../rfc/RFC-0052-amendment-A1-host-provider-ui-surface.md)
  §2（已接受）：`ui@1` 改 host-provider import 面 `sico:script/ui@1.0.0`，
  由 `runner/sico-runner/src/ui.rs::link_ui` 桥到 ADR-0018 渲染器 adapter，
  镜像 `link_http2`；host-provider 路径允许 record/variant（E8017 不放松，仍
  只管真 `sico:user/` package）；授权默认拒绝，无授权时每次调用返回
  `authority-denied`。A1 §3 明记：本层解锁到「binding/authority-refusal
  layer」，**真实渲染帧证据仍 gated on #4**。

## Changes

1. **`runner/sico-runner/src/ui.rs`（新增）**：host-provider 绑定模块。
   - `UI_INTERFACE = "sico:script/ui@1.0.0"`。
   - `UiError`（6 case，`#[component(enum)] #[repr(u8)]`，derive
     `ComponentType+Lower`，discriminant 序 = §3）。
   - `UiHandle{id:u64}`（record，`Lift+Lower`，双向跨界）；`UiSelection`
     （record，承载 select 的 handle+index）；`UiEvent`（variant：
     `click(UiHandle)|select(UiSelection)|closed`）。**实现说明**：wasmtime
     `variant` derive 每 case 至多一个未命名字段，故 `select(handle,index)`
     的两载荷打包进 `UiSelection` record——纯 ABI 实现细节，事件仍精确承载
     handle+index，无语义变更、不改合同（WIT 同步登记）。
   - `UiGrants{window:bool}`（默认 false = 拒绝）+ `allows()`。
   - `trait UiRenderer`（ADR-0018 #4 渲染器后端 seam：§2 受权路径委托的
     window/column/.../poll-event/run）；生产实现 = Direct2D/DirectWrite
     adapter（本机外部门缺失）；测试用确定性 double 验证绑定/授权接线，
     **不**声称任何真实帧。
   - `UiHost`（Runner 级：grants + `Option<Arc<dyn UiRenderer>>`，默认无后端）
     + `Ui1State`（per-Store，随 Store 消亡）。
   - `link_ui(linker)`：注册 §2 全封闭面；每闭包经 `guarded()` 先查授权——
     无授权 → 类型化 `authority-denied`（`poll-event` → `none`，不泄事件），
     渲染器**零触达**；有授权才委托后端。
2. **`wit/script-ui-v1/ui.wit`（新增）**：`package sico:script@1.0.0;
   interface ui` 规格制品（runner 手注册、compiler 手编码实例类型，均不解析
   此文件；镜像 `wit/script-http2-v0/http2.wit` 地位）。
3. **`runner/sico-runner/src/lib.rs`（改）**：`pub mod ui;`；`Runner` 加
   `ui: UiHost`（默认无后端）+ `with_ui_backend(grants, renderer)` builder；
   `RunState` 加 `ui: Ui1State`（3 处 Store 构造点：run / debug-run /
   debug-probe）；`prepare_program_inner` 在 `self.ui.has_backend()` 时调
   `link_ui`——**无后端则不广告 `ui@1`**，导入它的客户实例化失败（诚实的
   「UI 后端不可用」launch failure，绝不伪造帧）。
4. **`runner/sico-runner/tests/ui_authority.rs`（新增，runtime-verified）**：
   镜像 `tests/http2.rs` 的 `wasm-encoder` 手编客户组件范式，导入 `ui@1` 的
   `window` + `poll-event` 子集；`ui_instance_type()` 索引记账按 http2 适配器
   的既有规则（`ty()` 占一、`Eq` 别名导出占一、`Func` 导出占零）。
   - **`window`**：`result<ui-handle,ui-error>` 结果扁平化为两核类型
     （discriminant + u64 payload），故走 return-area 形式：tag@0、payload@8
     （union 对齐 8）；`run` 体读 `window` 结果 tag，拒绝分支发
     `exit=110+discriminant`（`authority-denied`=5→115），受权分支发
     `exit=handle id`（fixture 返回 1）。
   - **`poll-event`**：`ui@1` 面上**唯一不经共享 `guarded` 错误路径**的闭包
     （`ui.rs` 独立闭包：无授权 → `Ok((None,))` 而非 `authority-denied`）。
     `ui_instance_type()` 据此追加 `ui-selection`（record）、`ui-event`
     （variant `click|select|closed`，case 序严格对齐 `ui.rs` derive）、
     `option<ui-event>` 三型与 `poll-event` 函数（`Func(11)`，空 params 需显式
     编码——`wasm-encoder` 的 `result()` 断言 `params` 先编）。`option<ui-event>`
     结果扁平化为 >1 核类型，同走 return-area 形式：降形为 `func(retptr:i32)`。
     客户体**仅读 discriminant 字节 @0**（`none`=0 / `some`=1），从不解码事件
     payload——要证的是授权门与事件投递路径，不是帧。两测试经真实 runner
     实例化 + 断言 `FixtureRenderer` 调用计数（`window` 拒绝 0 / 受权 1；
     `poll-event` 拒绝 0（事件队列零触达）/ 受权 1）。

## Executed validation

- `cargo build --lib`（runner workspace，`--locked --offline`，
  `1.98.0-x86_64-pc-windows-gnu`）：绿。
- `cargo test --lib ui::`：**4/4 绿** —— `ui_grants_default_deny`（默认拒绝）、
  `ui_error_discriminants_match_the_contract_order`（§3 discriminant 0..5）、
  `host_without_backend_does_not_advertise_ui`（无后端不广告）、
  `link_ui_registers_the_full_closed_surface`（§2 全 13 函数经真实
  wasmtime `Linker<RunState>` 注册无误）。
- `cargo test --test ui_authority`：**4/4 绿（runtime-verified end-to-end）**
  ——`denied_window_is_typed_authority_refusal_without_touching_renderer`
  （无授权 `exit=115`、`FixtureRenderer.calls==0`）、
  `granted_window_reaches_the_renderer_once`（受权 `exit=1`、`calls==1`）、
  `denied_poll_event_is_none_without_touching_renderer`（无授权 `exit=100`
  即类型化 `none`、事件队列 `calls==0`）、
  `granted_poll_event_delivers_an_event_once`（受权 `exit=101` 即 `Some`、
  `calls==1`）。后两测覆盖 `ui@1` 面上唯一不经 `guarded` 的事件闭包。
- 完整 runner 套件 `cargo test -- --test-threads=1`：**全绿**（含 runner
  37/37、http2 6/6、image_codec 5/5、ui 单测 4/4、ui_authority 4/4；RunState/
  Runner/prepare_program_inner 改动无回归）。注：并行跑时
  `dap_hundred_sequential_sessions_have_bounded_rss_handles_and_no_poisoning`
  与 `debug_pause_terminate_leaves_no_stranded_tasks_or_workers` 两个
  RSS/计时敏感测试曾各失败一次，**单独跑与单线程全跑均绿**——为本机并行负载
  下的既有 flakiness，非本片回归。
- `cargo clippy --all-targets -- -D warnings`：绿（let-chain 折叠 1 处修复）。
- `cargo fmt -- --check`：FMT_CLEAN。

## Honest residuals

- **本片交付 #5 的绑定 + 默认拒绝授权层 + runtime-verified end-to-end 授权拒绝**
  （A1 §3 明许的范围）。**已交付**：
  - **end-to-end 客户组件级授权拒绝测试**（`tests/ui_authority.rs` 4/4）：真实
    `wasm-encoder` 客户组件经 runner 实例化，覆盖 `ui@1` 面上**两条不同的授权
    门控闭包**——`window`（共享 `guarded`：拒绝→类型化 `authority-denied`）与
    `poll-event`（独立闭包：拒绝→类型化 `none`、事件队列零触达）；受权正控各
    断言一次触达。安全属性由单元级升到 runtime-verified，且 `ui.rs` 中每条
    授权门控路径都在组件边界被运行期证明。
  **仍未交付**（本 #5 切片范围之外）：
  2. **受权渲染/帧路径**：`run()`/`draw-image()`/真实窗口与帧哈希证据
     gated on **#4**（ADR-0018 Direct2D/DirectWrite adapter），本机外部门未开
     （STEP-0330 §5）；`UiRenderer` 生产实现缺席，仅测试 double 存在。**poll-event
     的事件授权门（拒绝→`none`、受权→投递测试 double 事件）已端到端锁死，但真实
     OS 事件泵与真帧不在本机可证范围**——绑定层已把「无后端不广告、有后端无授权
     则类型化拒绝/`none`、渲染器零触达」不变量端到端锁死。
  3. **compiler 源侧发射**：Sico 源 import `ui@1` 的零语法绑定（intrinsic
     签名 + codegen 实例类型 + 源到 host-name 映射）属 M14 应用语言基线 /
     后续子步，本片未触 compiler crate。
  4. **UI 授权 CLI flag**：`with_ui_backend` 为编程入口；runner CLI 尚无
     `--grant-ui` 类开关（生产默认无后端 + 无授权 = `ui@1` 不链接）。
- 新增一类 host-provider 面（继 `http@0.1.0`/`http@0.2.0`/`fs`/`streams` 后），
  但**未**移动任何 M24 §3 gate；gate 2 仍 reference-only-with-deferral，
  M24 overall 仍 **NO-GO**。
- 本机外部门盘点不变（STEP-0330 Honest residuals）：gate 1 加速器硬件、
  #4 Direct2D 离线绑定、#3 JPEG 离线编解码 crate（及随 #3/#4 延期的 #6）。
  M24 本机可完整交付的内部线 = #2 PNG（✓STEP-0331）→ #5 绑定/授权层 +
  end-to-end 授权拒绝（✓本片）→ #7 roster/contour → #8 出口审计 + gate 2
  具名延期登记。
