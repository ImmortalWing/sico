# STEP-0330: M24 24-A 收口 — codec 实现家探针裁决 + 合同对齐登记

> - status: planning/contract-alignment complete locally；探针 measurement-only，不移动任何 gate；codec/UI 实现切片未开始
> - phase: M24 执行卡 24-A（合同对齐与实现家裁决）
> - completed: 2026-09-30
> - renumber: 原占用 STEP-0325；按 §5「STEP 号冲突改号规则」登记改号为 **STEP-0330**——主权威线（`codex/m22-w1-ci` + M23）已提交 STEP-0325（m22-w2 list-get/match-subjects）与 STEP-0326（m22-w2 host-call-lets），最高号为 STEP-0329，故 M24 轨改用其上的空闲号，不抢占、不重号；最终号在 M24 轨并入时再确认
> - evidence: internal-fixture（`prototypes/image-codec-probe`，Windows x64 真实 wasmtime `=47.0.2`，`1.98.0-x86_64-pc-windows-gnu`，`png =0.18.1`）；独立 oracle = Python `zlib` 手写 PNG 分帧

## Objective

把 M24 的实现家裁决与合同对齐收口为可执行队列：(1) 用有界探针定 codec
实现家（纯 package vs host provider）；(2) 登记探针量出的合同缺陷并提出
修订草案；(3) 对齐 RFC-0043 roster 与 M24 §8.3 的 contour 不一致；
(4) 钉 codec crate 版本与独立 oracle 方案；(5) 补 M24 切片级执行队列。
本步不改任何 §3 gate 文本、能力合同或证据等级，不预留实现切片之外的未来
STEP 号。

## 1. 实现家裁决：纯 package（探针结论）

探针 `prototypes/image-codec-probe`（已排除出根 workspace）结论
`package-route-feasible`，见 `results/probe-report.json` 与 README：

- Rust→`wasm32-unknown-unknown` 纯核心模块经仓库自带
  `package_builder::build_package_component` 包装为
  `sico:user/image-codec-probe@1.0.0`，走 wasmtime 组件 ABI 执行；
- 9/9 解码用例对齐**独立** Python/zlib oracle（ok 2×2 RGBA 载荷逐字节等，
  bad-signature / trailing-data / truncated / corrupt-crc / oversized
  limit+1 / palette / depth-16 / interlaced 全部类型化拒绝）；
- encode 两次调用字节稳定、chunk 集冻结 `{IHDR,IDAT,IEND}`、
  `decode(encode(pixels))` round-trip 载荷一致、3/3 encode 拒绝类型化；
- guest 核心模块 `cargo clean` 后重建字节一致，组件重建字节一致。

⇒ **RFC-0050 §1「zero compiler/Runtime changes」原文保住，codec 实现家 =
纯 package，不转 host provider。** 早先 owner 的「host provider + crates.io
codec」答案被此测量推翻，正如「先探针再定」选项所预期。手写 Canonical ABI
shim（`Bytes`→`(i32 ptr,i32 len)`、`U64`→`i64`、`Result[Bytes,Text]`→单个
i32 指向 12B 返回区：disc byte@0、payload ptr@+4、len@+8）与仓库自带
`emit_csv_error` 一致，升降正确。

环境前置（如实登记，非路线缺陷）：gnu target 的 `windows-sys`
（wasmtime 依赖）用 `raw-dylib`，`cargo build` 需 MinGW binutils
（`as.exe`/`dlltool.exe`）在 PATH——与 STEP-0196/`run-ci.ps1` 记录的 `ring`
前置同源；本机 MSVC host 变体亦不可用（Git `link.exe` 遮蔽 + 无 VS Build
Tools）。探针脚本 `tools/probe.ps1` 自动定位 repo-local msys2-binutils。

## 2. 量出的合同缺陷 + 修订草案（待 owner 复裁决）

RFC-0050 §1 已被 owner directive 2026-09-30 接受（STEP-0323），而 STEP-0323
明记「修订以新草案 + 复裁决登记，不静默改合同」。故本步另立草案并经 owner
**复裁决接受**（2026-09-30）：

- **缺陷**：§1 的 `record Image` 与 `enum CodecError` 落在 v0 user-WIT 值集
  外——`sico-semantics::interface_value_offender`（RFC-0039 §2.4 A7）对
  record/enum 发 **E8017**，只接受标量 + 一层 `List/Option` + 标量 error 的
  `Result`。接受的 §1 面无法经现有纯 package 路径下降。
- **修订（已接受）**：[`RFC-0050-amendment-A1-flattened-surface`](../rfc/RFC-0050-amendment-A1-flattened-surface.md)
  （accepted by owner re-adjudication 2026-09-30）：四函数扁平为
  `decode-*(Bytes)->Result[Bytes,Text]` /
  `encode-*(Bytes,U64,U64)->Result[Bytes,Text]`；decode ok 载荷 = 16B LE
  宽高头 + RFC-0041 BGRA8；error = case-name `Text`（沿用 HTTP 先例）；
  §3 封闭集**新增** `encode-buffer-mismatch`（探针量出：encode 侧
  `pixels.len() != width·height·4` 在原集无对应 case，探针暂映射
  `corrupt-stream` 并源码标注）。§2 profiles / §3 limits / 确定性 / §4
  纯 package 实现家均不变。RFC-0050 §1/§3 正文已就地更新为 A1 形式并指向
  本修订。

## 3. RFC-0043 roster 与 M24 §8.3 contour 对齐（owner 复裁决：增补 contour）

- M24 §3 gate 3 与 §8.3 把 roster 命名为「template match / grid /
  **contour**」；但 [`RFC-0043`](../rfc/RFC-0043-deterministic-vision-packages-v0.md)
  v0 roster 原只有 `image-threshold@1 / image-color@1 / image-match@1 /
  image-grid@1`，**无 contour**，且实际落地的是 `image-vision@1`
  （to-grey8/threshold/occupancy/occupancy-mask，A1）。
- RFC-0043 原状态行「draft — accepted-by-session-directive pending completion
  evidence」口径含糊，且 RFC-0043 **不在** STEP-0323 接受的八份合同之列。
- **owner 复裁决（2026-09-30）= 增补 contour 到 RFC-0043**：已以 RFC-0043
  **A4** 增补 `image-contour@1`（`find(bmp, min-area) -> list<region>`，
  外部边界跟随、integer-only、确定性 row-major 扫描、ties by lowest (y,x)、
  degenerate/min-area limit+1 类型化），并把状态行改为与 re-adjudication
  一致的接受口径（completion evidence 仍逐包 pending）。gate 3 保持三包。
  roster 包名与 `image-vision@1` 的命名漂移留待各包实现 STEP 厘清。

### 3b. RFC-0052 `ui@1` 实现家缺陷（量出，草案待复裁决）

把 RFC-0052 §2 面对 `interface_value_offender` 与 runner package/http2 链接
路径实测，量出**两处**缺陷（与 RFC-0050 A1 同类但更深）：

- **值集（E8017）**：§2 的 `record UiHandle`/`UiPoint` 与
  `poll_event()->Option[UiEvent]`（variant `click|select|closed`）落在 v0
  user-WIT 值集外；`interface_value_offender` 对**任何 Sico 源里带版本的
  `interface` 声明**发 E8017（`sico-semantics/src/lib.rs:497-518`），故
  `sico:user/ui@1` 作为源编写 package 无法承载 §2 面。
- **纯度/host-effect**：`window/run/poll_event/draw_image` 是 host 效果；
  runner package 路径要求**纯**（`prepare_package` 拒任何 import，
  `link_packages` 只把 `sico:user/` import 桥到纯 package 实例）。驱动原生
  窗口的面不能是纯 `sico:user/` package，而是 host provider 满足的 guest
  import——与 `sico:script/http@0.2.0` 同形。
- **草案**：[`RFC-0052-amendment-A1-host-provider-ui-surface`](../rfc/RFC-0052-amendment-A1-host-provider-ui-surface.md)
  （proposed，待复裁决）：把 `ui@1` 从纯 user-WIT package 改为
  **host-provider import 面**（工作名 `sico:script/ui@1.0.0`，新增 `link_ui`
  桥到 ADR-0018 渲染 adapter，镜像 `link_http2`）；host-provider 路径允许
  record/variant，故 §2 面**无需扁平**；§3 UiError / §4 确定性 / 默认拒绝
  授权 / 零语法增长均不变；E8017 不放松（仍只管真 `sico:user/` package）。
- **影响**：#5（ui@1 绑定）原阻塞于 A1 复裁决——**A1 已于 2026-09-30 接受**，
  合同前置解除（绑定/授权拒绝层可进，真实渲染帧待 #4）；**#4（host 侧渲染
  adapter crate）不受 E8017/合同阻**——它不是 Sico 源 user-WIT interface、不经
  E8017，按 ADR-0018（已接受）本可独立推进；但 #4 另有一道**外部门**（Direct2D
  /DirectWrite 离线绑定缺失，见 §5 队列与 Honest residuals），非合同缺陷。

## 4. codec crate 选型与独立 oracle

- **PNG**：`png =0.18.1`（探针已验证：编解码、`DeflateCompression::Level(9)`
  精确 zlib level、`Filter::Paeth`、`Limits{bytes}`、字节可复现）。独立
  oracle = Python `zlib` 手写 PNG 分帧（`tools/make-fixtures.py`），与 `png`
  crate 无共享代码。
- **JPEG（候选，切片前需第二支探针）**：decode `zune-jpeg`（baseline SOF0）
  / encode `jpeg-encoder =0.7.1`（quality 90、4:4:4）。JPEG 字节稳定性比 PNG
  脆（量化表/Huffman 表实现相关），故 #3 切片**前置**一支同形探针：用第二
  实现或 Pillow 作独立 oracle，钉死 §2 的 SOF0/quality-90/4:4:4 输出字节，
  未通过则不进实现切片。
  - **离线可得性实测（2026-09-30，本机 cargo cache 全量 225 crate 清点）**：
    cache 内**唯一**的图像编解码 crate 是 `png-0.18.1`；`zune-jpeg` /
    `jpeg-encoder` / `image` / `mozjpeg` / `turbojpeg` **均无**（`registry/cache`
    与 `registry/src` 双查皆空）。`--offline` +「未授权不外联下载」⇒ 探针与
    实现切片都取不到 JPEG crate；按 ADR-0019 §6 / RFC-0050 §4，JPEG 的 DCT/
    Huffman 不可手发（correctness risk unacceptable，且要求 vetted crate）。
    ⇒ **#3 JPEG 在本机为具名外部延期**（缺失输入 = 离线 JPEG 编解码 crate），
    与 #4 Direct2D、gate 1 硬件门同类；详见 §5 队列 #3 行与 Honest residuals。
    探针本身（§4 的 SOF0/quality-90/4:4:4 oracle 钉死）在 JPEG crate 可得前
    无法执行，故 #3 连前置探针都被外部门挡住。
- 版本一律 `=` 钉死；任何 crate 升级触发 chunk-set/字节稳定性复验（探针
  encode 后自扫 chunk 集即为此设）。

## 5. M24 切片级执行队列（每片唯一出口测试 + 允许改动范围）

| 切片 | 唯一出口测试 | 允许改动范围 | 前置 |
|---|---|---|---|
| #2 PNG 双向 | `sico:user/image-codec@1` 的 PNG 双向在真实 runner 上对冻结语料字节稳定 + §8.5 拒绝语料逐行类型化 | codec package 实现 + 语料 + runner 测试 + 支持矩阵行 | A1 ✓接受；ADR-0019 ✓接受（2026-09-30 复裁决） |
| #3 JPEG 双向 | 同上，JPEG SOF0/quality-90/4:4:4 字节钉死 | 同 #2 + JPEG 探针 | #2 ✓完成；**JPEG 独立 oracle 探针外部门未开**（离线无 JPEG 编解码 crate，见 §4 实测 + Honest residuals）|
| #4 渲染器 adapter | Direct2D/DirectWrite 隔离 adapter crate 在固定 client size 下帧哈希对冻结语料确定 | 新 adapter crate（最小 unsafe FFI）+ Fluent token + 帧语料 | ADR-0018（已接受）；**Direct2D/DirectWrite 离线绑定外部门未开（见 Honest residuals）** |
| #5 `sico:user/ui@1` | 控件/事件/授权面经 RFC-0052 源绑定编译，default-deny UI 授权拒绝类型化 | UI 包 + 源绑定 + 拒绝语料 | RFC-0052 A1 ✓接受（2026-09-30 复裁决）；绑定/授权拒绝层可进，**真实渲染帧证据待 #4 Direct2D 外部门** |
| #6 GUI 转换器试点 | Sico 编写的转换器在 Windows evidence host 运行，JPEG↔PNG 双向字节稳定 + §8.5 fail-closed | 试点 app（clean-room-consumer）+ M18 增补 | #3 + #5 |
| #7 M17 gate 4 + roster | fixture model 五件套（digest/budget limit+1/cancel/malicious/crash-isolation）+ roster 三包语料 byte-exact + M7 registry 零改安装 | RFC-0051 实现 + roster 包 + 语料 | A1/contour 裁决；**roster 三包（match/grid/contour）返回签名经 §7 实测探针为 E8017 合同门阻挡，须 owner 裁决 region 编码/包 ABI 后方可实现** |
| #8 出口审计 | 逐门裁决 + gate 2 具名延期登记 + 文档同步 | STATUS/ROADMAP/plan/STEP | #6 + #7 |

- **STEP 号冲突改号规则**：本 M24 轨与 M22 W2 并行；若某切片落地时其候选
  STEP 号已被 M22 线占用（如 STEP-0330 之后的号），该切片改用下一个未占用
  号并在文首登记改号，不抢占、不重号。
- 本 24-A 收口经改号占用 **STEP-0330**（原 STEP-0325 与主权威线已提交的
  m22-w2 STEP-0325 重号；主线现最高为 STEP-0329，见文首 renumber 行）。#2
  codec PNG 切片占用 **STEP-0331**。

## Executed validation

- `tools/image-codec-probe` 探针：`PROBE VERDICT: package-route-feasible`，
  core reproducible = true（见 `results/probe-report.json`）。
- `validate-step-0124.ps1 -SelfTest`（规划合同 + 负例）绿；`git diff --check` 绿。

## Honest residuals

- 本步是 24-A 收口（探针 + 合同对齐登记 + owner 复裁决），**不产生任何 M24
  实现进展**：codec/UI/roster/model 切片（#2–#8）均未开始。
- A1（RFC-0050 §1/§3 扁平化 + `encode-buffer-mismatch`）与 contour（RFC-0043
  A4）均已 owner 复裁决接受 ⇒ #2/#3/#7 的**合同前置已解除**，可按 §5 队列
  进入实现切片；但各切片仍需自己的 STEP + 真实 runner 证据，本步不替代。
- M24 §3 gate 1（= M17 gate 2，具名加速器硬件上的 accelerated-vs-reference
  真实比对）是 owner-gated 外部门：本机无授权加速器，按 §8.1 只能记
  reference-only + 具名 gate-2 延期，**M17 overall 不能据此翻 GO**。
- M24 plan §8.4 仍把 RFC-0050 标为「draft, owner acceptance required」，与
  STEP-0323 的接受状态不一致——属文档陈旧，留待 #8 出口审计统一同步（本步
  不改 plan 以免触碰 validate-step-0124 的 M24 不变式）。
- **#2 的架构前置已起草 ADR（待 owner 接受）**：仓库现有 package（`csv` /
  `table-stats` / `image-vision`）全是 `package_builder.rs` 内**手工发射**的
  wasm core；探针走的是**另一条**生产路径——把 Rust→`wasm32-unknown-unknown`
  编出的 core 喂给 `build_package_component`，且探针对空 linker 实例化成功
  ⇒ core **无 import**，满足 runner `prepare_package` 纯度检查，零
  compiler/Runtime 改动即可上现有 package 路径。生产路径（核心如何生产/钉版/
  vendored/在 `--locked --offline` CI 复现）属跨组件架构，已按 AGENTS.md §3
  起草 [`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)
  （proposed）：双类 core 单一 wrapper、guest crate 在 workspace 外、提交
  digest-pinned core `.wasm` 走现有 M7 lock、`validate-package-core.ps1`
  字节复现校验、§4/§7 边界不破。**ADR-0019 已于 2026-09-30 经 owner 复裁决接受
  （本 STEP 内 AskUserQuestion 登记）⇒ #2/#3 合同前置全部解除，可进实现切片。**
  同批复裁决亦接受 RFC-0052 A1（ui@1 改 host-provider 面）⇒ #5 绑定/授权层
  合同前置解除（真实渲染帧仍待 #4 Direct2D 外部门）。
- **#4 的外部门（本机实测，2026-09-30）**：ADR-0018 §2/§5 把 M24 渲染器钉死为
  **真实 Direct2D + DirectWrite** 自绘，并显式排除替代品（WinUI3/XAML、
  web/webview、GDI 等）与「不静默改合同」。本机离线 cargo cache 只有
  `windows-sys 0.52.0` 与 `0.61.2`；二者发布的源码 `Win32/Graphics/` 模块树
  **仅含** Dwm/Gdi/GdiPlus/Hlsl/OpenGL/Printing，**不含** `Win32_Graphics_Direct2D`
  /`DirectWrite`/`Dxgi`（217 个 Win32 feature 行里 Direct* 只有 Wdk Direct3D，
  非运行时 Direct2D）。高层 `windows` crate（含 Direct2D）未缓存，且 `--offline`
  + 「未授权不外联下载」⇒ 取不到。
  - 仓库既有 Win32 FFI 家（ADR-0013 §5，`sico-automation-host/src/win32.rs`）
    正是 `windows-sys 0.52` + feature flag 模式；该模式下 Direct2D 绑定不存在。
  - 凭记忆手写整套 Direct2D/DirectWrite COM vtable FFI（`ID2D1Factory`/
    `ID2D1DCRenderTarget`/`IDWriteFactory` 等约 50+ 方法的精确槽位）属高 UB
    风险，且无权威绑定可对照校验，无法支撑 ADR-0018 要求的「在真实 Windows
    native renderer 上字节稳定」证据等级——违反 AGENTS.md §5「不得发射不可验证
    或语义不同的实现」。
  - **裁决**：#4 的 Direct2D 后端在本机为**具名外部延期**（缺失输入 =
    离线 Direct2D/DirectWrite 绑定），与 gate 1 硬件门同类；**不以 GDI/GDI+
    替代、不静默改 ADR-0018**。可在本机无外部门推进的 #4 子件仅为渲染器无关的
    纯件（Fluent token 冻结表、帧哈希/确定性合同机制），其后端绑定待 Direct2D
    绑定可得时按 ADR-0018 落地。M24 overall 据此**不能**翻 full GO。
- **#3 的外部门（本机实测，2026-09-30）**：与 #4 同形但更早——#3 连前置探针
  都被挡住。本机 cargo cache 全量 225 crate 清点中，图像编解码 crate **只有**
  `png-0.18.1`；`zune-jpeg`/`jpeg-encoder`/`image`/`mozjpeg`/`turbojpeg` 双查
  （`registry/cache` + `registry/src`）皆空。`--offline` +「未授权不外联下载」
  ⇒ 取不到任何 JPEG 编解码 crate；§4 要求的「第二实现或 Pillow 作独立 oracle」
  的 Rust 侧候选也不可得（Python Pillow 同样未授权外联安装）。按 ADR-0019 §6
  与 RFC-0050 §4，JPEG 的 DCT/量化/Huffman 不可手发（correctness risk
  unacceptable + 要求 vetted crate + 要求字节钉死语料）。
  - **裁决**：#3 JPEG 双向在本机为**具名外部延期**（缺失输入 = 离线 JPEG
    编解码 crate + 独立 oracle），与 gate 1 硬件门、#4 Direct2D 同类；**不手写
    DCT 替代、不静默改 RFC-0050/ADR-0019**。JPEG crate 可得时按 §4 探针 →
    §5 队列 #3 出口测试落地。⇒ RFC-0050 在本机**只能交付 PNG 半边**（#2 已
    完成），JPEG 半边为登记在案的具名延期。
  - 连带影响 #6：GUI 转换器试点的第一增量（M24 plan §8.4「step 1 = JPEG↔PNG
    双向」）的 JPEG 半边随 #3 延期；#6 在本机最多只能跑 PNG→PNG（无意义），
    故 #6 同样落在 #3 外部门之后，本机不可完整交付。
- **M24 本机可完整交付的内部线（外部门盘点后）**：#2 PNG codec（✓完成，
  STEP-0331）→ #5 `ui@1` 绑定/授权拒绝层（RFC-0052 A1 已接受，不需 Direct2D，
  本机可建，✓STEP-0332 含 end-to-end 授权拒绝 runtime-verified）→ #7 roster 包
  **（2026-09-30 实测探针：合同门阻挡，非本机可径建——见 §8）**→ #8 出口审计 +
  gate 2 具名延期登记 + 文档同步。**外部门 / 合同门（本机不可翻）**：gate 1 加速器
  硬件、#4 Direct2D/DirectWrite 离线绑定、#3 JPEG 离线编解码 crate，及随 #3/#4
  延期的 #6；**#7 roster 三包另受 E8017 合同门阻挡**（region 返回类型不可表达，
  须 owner 裁决）。⇒ **M24 overall 在本机不能翻 full GO**；可达成的是内部线落地 +
  gate 2「reference-only ready + 具名延期登记」，外部门/合同门如实登记不替代。

## 8. #7 roster region 编码合同门探针（2026-09-30，probe-only，无实现）

- **动机**：按 §5 队列推进 #7 的 roster 三包（`image-match@1` / `image-grid@1` /
  `image-contour@1`）前，先验证其返回签名能否作为**纯 `sico:user/` package** 落地
  （AGENTS.md §1「先核实真实 compiler/Runtime/Host 行为，勿从设计文档推断实现支持」）。
- **实测证据**：`crates/sico-semantics/src/lib.rs:745` `interface_value_offender`
  定义的 E8017 接受 v0 包接口值集**只含**：标量 `Bool|I64|U64|Text|Bytes`、
  level-1 `List<Text>`、`Unit`、`Result[level1|Unit|Bool, scalar]`。
  `package_builder.rs` 的 `Type` 枚举同样**无 record/region 变体**（仅
  Bool/整数/String/Bytes/List/Result）。
  ⇒ RFC-0043 的 `find(bmp,min-area)->list<region>`、
  `detect-lines(...)->list<region>`、`template(...)->result<region,error>`
  中 `region`（record）既非标量、`List` 也只接受 `Text`，故 **`list<region>` /
  `result<region,_>` 全部被 E8017 判为 offender 拒绝**（`crates/sico-cli/src/
  packages.rs:353`、`tests/packages_command.rs:194` 钉死该拒绝为类型化错误）。
- **结论（合同门，非代码门）**：#7 roster 三包在本机**不能径建为纯 package**——
  受阻于已接受的 E8017 包接口值集，这是**合同前置未决**，不是缺实现。三选一须
  **owner 裁决**（我不自裁、不静默改合同）：
  1. 增补 RFC-0043：把 `region` 定义为 `Bytes`/`list<u8>` 扁平 packed 编码
     （如 `(x,y,w,h)` 定宽小端串），改可见签名；
  2. 放宽 v0 包接口值集 / 增 record 支持（包 ABI + 安全面变更，需新 ADR + E8017
     修订）；
  3. 把这三包改 home 为 host-provider（`sico:script/`，允许 record/variant）——
     但它们按 RFC-0043 定位为可分享纯 package，语义不符。
- **纪律**：本探针**不移动任何 gate**、**不分配 STEP 去伪造** Bytes 编码（那是
  静默改合同）。roster 三包保持 pending，待 owner 对 region 编码裁决后按 §5 落地。
  `image-vision@1` 之所以能落地，正因它的四个原语全是 `Bytes->Bytes` 扁平形，
  不触 `region`——不可据此推断 `region` 返回可表达。
- **选项 ① 已具体化（2026-09-30 追加，仍为 PROPOSED 未裁）**：读全 RFC-0043 确认
  **3/3 命名 gate-3 包（match/grid/contour）皆返回 `region`/`list<region>`，无一标量/
  Bytes 入口可先落**（`threshold`/`color`/`grey8` 已作为 `image-vision@1` 交付，非本门
  欠账）。据 RFC-0041 实测 `record region { x,y,width,height: u32 }` = 16 B，遂起草
  [`RFC-0043 Amendment A1 (PROPOSED)`](../rfc/RFC-0043-amendment-A1-region-packed-bytes.md)：
  region → 16 B 定宽小端 packed `Bytes`、`list<region>` → `count u32 LE` + `count×16`、
  错误 `image-error` → 案例名 `Text`（复用已接受的 RFC-0050-A1 载体先例），三包签名全落入
  E8017 接受集。A2（放宽值集，需新 ADR+语言面改动）/A3（改 host-provider，语义不符）列为
  备择。**未自裁、未据此实现；gate 3 仍 未达**，与 provenance 的 RFC-0051-A1 同构呈 owner。
