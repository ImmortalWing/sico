# STEP-0338: M24 §8.6.1 — Sico 原生 UI 库的 Fluent-subset Direct2D 渲染器（冻结 token 表 + 确定性帧语料）

> - status: implementation complete locally（**渲染器核心切片**；不翻任何 §3 gate）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3）
> - completed: 2026-10-21
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted 2026-09-30 STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）、[`RFC-0052` §3 拒绝面]
> - evidence: **internal-fixture / runtime-verified renderer-frame**（Windows x64 真实 Direct2D + DirectWrite；**非** guest 级、**非**试点级、**非**生产渲染器声明）
> - artifacts: [`crates/sico-ui-renderer-d2d/`](../../crates/sico-ui-renderer-d2d)、[`docs/evidence/m24/ui-frame-corpus.json`](../evidence/m24/ui-frame-corpus.json)、[`examples/frame_digest.rs`](../../crates/sico-ui-renderer-d2d/examples/frame_digest.rs)

## Objective

落地 ADR-0018 指定的第一件实现 STEP：**冻结 Fluent-subset token 表 + 冻结帧确定性语料**
（ADR-0018 §Consequences：「The frame-determinism corpus and the Fluent token table
become frozen evidence with the first implementation STEP」）。

本 STEP 之前，M24 #4 门被记为「具名外部门：本机离线无 Direct2D 绑定」。本 STEP 的实测
证明该外部门**在本机已经可以解除**（见 §3），并据此交付渲染器核心 + 冻结 token 表 +
确定性帧语料。

## Changes

- **新 crate [`crates/sico-ui-renderer-d2d`](../../crates/sico-ui-renderer-d2d)**（workspace
  `host` 模块；`tests/architecture/module-boundaries.json` 已登记）：
  - `tokens.rs` —— **冻结的 Fluent-subset token 表**（`TOKEN_SET_VERSION = "fluent-subset-v0"`）：
    色彩（accent `#0078D4`、background `#F3F3F3`、control fill/border、text primary/secondary、
    progress track、image placeholder）、几何（4px 圆角、1px 描边、32px 控件高、12/6px 内边距、
    8px 间距、16px 窗口内边距、4px 进度条高、64px 最小按钮宽、20px chevron 区）、
    Segoe UI Variable 字阶（12/14/16/20px + 冻结行高）。**表中不含任何环境派生值**
    （主题、DPI 缩放、locale、accent 色）——否则帧哈希会随环境漂移。
  - `tree.rs` —— UI 节点模型 + 类型化拒绝：未知 handle、错误父类型、node-count / tree-depth /
    text-bytes / option-count / image-bytes / title-bytes 限额（**限额在分配前判定**）、
    image 载荷与声明尺寸不匹配。
  - `backend.rs` —— 与 `ui@1` host-provider 面同形的 13 个操作（`window`/`column`/`row`/`text`/
    `button`/`dropdown`/`image-preview`/`progress` + 状态 setter）；**不含授权判断**（授权留在
    runner 与 desktop host，ADR-0018 §4）。
  - `render/win.rs` —— **唯一的 unsafe 模块**（ADR-0018 §1：平台 unsafe 收敛到最小 adapter
    crate；共享核保持 unsafe-free，沿用 `sico-automation-host` 的 lint 豁免模式）。
  - `render/unsupported.rs` —— 非 Windows 主机类型化拒绝（不假装能渲染）。
- **帧语料** [`tests/frame_corpus.rs`](../../crates/sico-ui-renderer-d2d/tests/frame_corpus.rs)
  —— 4 个冻结场景（empty / text / controls / row）钉死 SHA-256；另含同树二次渲染逐字节相同、
  状态变化必改帧、类型化拒绝 fail-closed、无窗口拒绝共 5 个测试。
- **digest 打印工具** [`examples/frame_digest.rs`](../../crates/sico-ui-renderer-d2d/examples/frame_digest.rs)
  —— token 变更后用于重新取哈希。
- **证据行** [`docs/evidence/m24/ui-frame-corpus.json`](../evidence/m24/ui-frame-corpus.json)。

## Executed validation

- `cargo test -p sico-ui-renderer-d2d` → **5/5 绿**（含 4 个冻结哈希全等）。
- 跨进程确定性：同一场景在**两个独立进程**中产出**完全相同**的 4 个哈希。
- `cargo clippy -p sico-ui-renderer-d2d --all-targets --all-features -- -D warnings` → **绿**。
- `cargo fmt --all -- --check` → 绿；`git diff --check` → 净。
- `tools/validate-module-boundaries.ps1` → `MODULE_BOUNDARIES_OK packages=30`（新增 1 包）。
- 工作区 `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings`
  → **exit 0**。

## §3 本机实测的两条承重事实（与旧记录不同，按实测为准）

1. **`windows-sys` 里根本没有 Direct2D/DirectWrite**（旧记录只说「未开 feature」）：实测
   `windows-sys 0.52.0` 与 `0.61.2` 的 `Win32/Graphics/` 下只有
   `Dwm/Gdi/GdiPlus/Hlsl/OpenGL/Printing` —— **Direct2D、DirectWrite 目录不存在**
   （`windows-sys` 刻意排除 COM-heavy API），开 feature 也变不出来。载体只能是完整的
   `windows` crate。
2. **必须用 `windows` 0.52 而不是 0.62**：0.62 的 `windows-link` 走
   `kind = "raw-dylib"`，windows-gnu 下 rustc 需调用 `dlltool` 生成 import lib，而本机
   **无任何 binutils**（`gcc`/`as`/`dlltool` 全缺；rust 工具链自带的 `dlltool.exe` 缺
   `as` 汇编器而失败）。`windows` 0.52 走 `windows-targets 0.52` 的**预编译合并 import lib**
   （`libwindows.0.52.0.a`，12.11 MB，已在本地缓存，内含 `D2D1CreateFactory` /
   `DWriteCreateFactory` / `D3D11CreateDevice`）⇒ **零 dlltool、零新增工具链**。
3. **DIB 形态与像素格式是被拒的两个参数**：`BI_RGB` 32bpp DIB 与
   `UNKNOWN` 像素格式都会让 `CreateDCRenderTarget` 失败（`0x88982F80`
   `D2D_ERR_UNSUPPORTED_PIXEL_FORMAT`）；可用的组合是
   **`BITMAPV5HEADER` + `BI_BITFIELDS` + 显式 BGRA mask + `LCS_sRGB`** DIB，配
   **`DXGI_FORMAT_B8G8R8A8_UNORM` + `D2D1_ALPHA_MODE_PREMULTIPLIED`**。实测中心像素
   `[d4, 78, 00, ff]` = BGRA(212,120,0,255) = accent `#0078D4` 精确命中。

## Declared limits（诚实声明，未声称的部分）

- **不是生产渲染器**：本 STEP 只交付**渲染器核心**（离屏、headless、可哈希）。
  **OS 窗口外壳与事件泵未做**，故 `poll-event` 不在 v0 面内；**runner 侧 `ui@1` 接线未做**。
  ⇒ 此刻**没有** guest 可见的 UI 能力，**没有**任何 M24 §3 gate 翻转。
- **帧哈希是证据主机局部的**：DirectWrite 字形栅格化随 Windows 构建而异，跨主机不保证同哈希
  （ADR-0018 §3 本身把语料范围限定为「on the evidence host」）。换主机 = 重新冻结，不是回归。
- **不声称 WinUI 3 像素级对等**：合同是 "Fluent-like subset"（ADR-0018 §2）。
- **不新增任何 authority**：渲染器不携带 capture/input/clipboard/凭据/进程授权。
- **单平台**：Windows x64；非 Windows 主机类型化拒绝。
- **未接 HTTP/网络/存储**：与 M12 授权面无关。

## 后续切片（未开工，按契约顺序）

1. OS 窗口外壳 + 事件泵（`poll-event` 语义落地，RFC-0052 §3 的 `ui-event` 闭集）。
2. runner 侧 `ui@1` 接线（`UiRenderer` seam 绑定，**不得**改 guest 合同面）。
3. 文件打开/保存对话框（M26 PDF 工具所需，ADR-0018 §4 沿用既有文件能力 + 用户选定路径）。
4. M24 转换器试点（gate 5）方可评估；M17 gate 1 加速器仍是**具名外部门**，本 STEP 不触碰。

## 关系说明

本 STEP **不**消费 M24 轨其它在途条目，也不改变它们的结论；它解除的是「M24 #4 具名外部门」
的**可执行性**前提（实测可解），并把 ADR-0018 的第一件实现 STEP 变成冻结证据。
