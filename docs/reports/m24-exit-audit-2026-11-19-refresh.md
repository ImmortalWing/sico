# M24 出口审计刷新（2026-11-19，refresh of 2026-11-07-refresh）

> **总裁决：M24 overall = NO-GO（维持）。** 本文件为 **audit-of-current-state refresh**：在
> [`m24-exit-audit-2026-11-07-refresh.md`](./m24-exit-audit-2026-11-07-refresh.md)（v2）基础上
> **逐门重裁**，不沿用 v2 结论，不移动任何 §3 gate。逐门裁决见 §2。
>
> **本刷新的关键新事实（相对 v2）**：
> 1. **E2「#3 JPEG 离线不可得」前提已被实测推翻**——`jpeg-decoder 0.3.2` / `jpeg-encoder 0.7.1`
>    现已在离线 registry cache 中（cached + extracted），`guests/image-codec` 可离线为 wasm32 构建
>    且原生往返测试绿。**E2 门在本 host 上可合法关闭**（v2 判其「决定性外部门」的前提不再成立）。
> 2. **gate-5 UI 子项 G1–G5 已全部有执行证据**（v2 记 G4/G5「在途」已过时）：G4a `click(handle)`
>    （STEP-0597 `route_click_event` 点→句柄命中测试）、G5 客户级帧确定性均落地并跑绿。
> 3. **gate-2 / gate-3 / gate-7 的 runner 套件获得新鲜执行证据**（见 §3 证据表）。
>
> **工具链诚实声明（关键，2026-11-19 复测更新）**：本刷新的**全部新鲜执行证据运行在
> `x86_64-pc-windows-msvc` 工具链**，**非** 仓库钉定的 `x86_64-pc-windows-gnu`。
>
> **EDR 阻塞已解除（复测确认）**：本刷新实跑时，GNU 工具链下 `windows-sys` 构建不再报
> `os error 1006`（奇安信天擎 EDR 使 raw-dylib `kernel32.dll_imports.lib` 句柄失效的错误
> 已消失）⇒ **EDR 不再是阻塞因**。但 GNU 构建**仍失败**，失败点变为
> `error calling dlltool 'dlltool.exe': program not found`——本 host 的 GNU 工具链环境
> **缺 `dlltool.exe`**（MSYS2/UCRT64 bin 未在 PATH），这是**环境缺件、与 EDR 无关**。
> MSVC 目标不生成 raw-dylib 导入库（直接链 Windows SDK `kernel32.lib`），故不受影响。
>
> 因此：**MSVC 证据是真实的本 host 执行证据**（测试真跑、窗口真建），足以支撑子项进展与残余登记，
> 但**权威 GO 判定若严格要求 Windows GNU 独立证据，则须在补齐 GNU `dlltool.exe` 后于 GNU 复跑**。
> 本刷新**不据 MSVC 证据单独翻转任何 gate 到 GO**（与 §1「不以文档单独翻门」一致）。
>
> 证据等级：`internal-fixture`（本机 Windows x64 真实 runner/Wasmtime/窗口执行，语料为仓库内冻结集）；
> **无 `clean-room-consumer` 证据**（gate-5 G6 转换器 pilot 应用**尚未编写**）；
> 无 `external-pilot`/`production` 声称。

## 1. 本审计与 v2 的关系

| 项 | v2（2026-11-07-refresh） | 本刷新（2026-11-19） |
|---|---|---|
| 形态 | audit-of-current-state，overall NO-GO | 同形态，**逐门重裁**，证据基底更新至 2026-11-19 |
| gate 1 加速器 | NO-GO（E1） | **NO-GO（E1 不变，owner 外部门）** |
| gate 2 provenance | NO-GO（C1/C2 + I3） | **NO-GO（C1/C2 owner + I3 依赖 C1/C2 语义，未单独推进）**；运行期五件套获 MSVC 新鲜证据 |
| gate 3 roster | GO | **GO（再确认）**；registry 消费半部 11/11 MSVC 新鲜证据 |
| gate 4 M17 审计 | NO-GO（E1 + gate2 残余） | **NO-GO（E1 + gate2 残余不变）** |
| gate 5 转换器 pilot | NO-GO（E2 决定性 + G4/G5 在途 + G6 未达） | **NO-GO（E2 前提推翻但核心未重冻结；G6 应用未编写；§8.5 corpora 未绿）**；UI G1–G5 已证据 |
| gate 6 证据标签 | GO | **GO（再确认）** |
| gate 7 回归 | NO-GO（E4/E5 + I7） | **NO-GO（E4 CI + 干净候选 SHA + GNU；broad 回归 MSVC 绿）** |

**三条纪律（沿用 v2）**：① 不改历史结论，v2 原文保留，冲突以本文件为准；② 在途不预裁；
③ 本审计的执行证据为**本机 MSVC 实跑**（见 §3 证据表，含命令与结果），非登记值转抄。

## 2. 逐门裁决表

### gate 1 — M17 gate 2 加速器 provider ADR（§3 行 51–54）——**NO-GO（E1，owner 外部门）**

| 项 | 内容 |
|---|---|
| 门文本要点 | 加速器 provider ADR accepted；≥1 台 Windows host 加速实现 vs reference 在声明容差内一致（冻结语料）；provider crash/资源耗尽隔离且 fail-closed |
| **裁决** | **NO-GO（E1）** |
| 本刷新再核 | `docs/adr/` 全目录仍无加速器/accelerator ADR；无参考-vs-加速容差对比表；无 provider crash/耗尽注入证据。`image-vision@1`（M17 gate 1，STEP-0166）已绿但不覆盖本门 |
| 具名残余 | E1：①加速器 provider ADR（accepted）；②≥1 台 Windows host 容差一致性表；③provider crash/耗尽注入 + fail-closed |
| 关闭条件 | **owner 指定具名加速器硬件 + 接受加速器 ADR + 跑出容差表与注入证据。本审计不给「可自行关闭」暗示** |

### gate 2 — M17 gate 4 model provenance（§3 行 55–58）——**NO-GO（C1/C2 owner + I3 owner-sequenced）**

| 项 | 内容 |
|---|---|
| 门文本要点 | model RFC accepted；model-consuming 路径证明五件套：digest-verified load、budget 强制（limit+1 typed）、cancellation mid-inference、malicious-asset 拒绝、provider-crash 隔离免宿主重启 |
| **裁决** | **NO-GO** |
| 本刷新执行证据（MSVC，internal-fixture） | **digest-verified load**：`sico-package` 51/51（含 `provenance_fixture` 6、`security_properties` 4）。**budget 强制（部分）+ cancellation mid-inference**：`provenance_runtime` **6/6**（memory/wall/fuel/取消 4 态 + 端到端 `MemoryLimit`）。**provider-crash 隔离（第 5 件）**：`provider_crash_isolation` **4/4**（资源耗尽→typed `MemoryLimit`、provider 错→typed `HostProviderFailure`、provider panic→typed、正控制；三态均断言宿主存活 + 同 runner 随后跑正常 guest 成功） |
| 具名残余 | ① **C1** hostcall 预算运行期不生效 + 单位不匹配（`max_hostcalls` 计数 vs `hostcall_fuel` 每调用费用；`invoke`/`start_debug` 以 `max(hostcall_fuel, stdin*64+64MiB)` 地板覆盖）；② **C2** 真实 Script 组件 `SCRIPT_MEMORY_BYTES`≈2.06 GiB、1 MiB 预算运行期不透明 trap（见 [`RFC-0051-A2`](../rfc/RFC-0051-amendment-A2-hostcall-budget-and-script-memory-floor.md)，**PROPOSED 未裁**）；③ **I3** `provenance_budgets`/`run_program_with_budgets` 未接入 CLI/app 层 |
| **I3 为何本刷新未单独推进** | I3（把 `Budgets` 接入 `sico-runtime::effective_limits` → runner 子进程 flags）**依赖 C1/C2 裁定的预算语义**：`Budgets` 携带 `max_hostcalls`，而 C1 正是「hostcall 预算运行期不生效 + 单位不匹配」。在 C1/C2 未裁前接入会把可能被 owner 变更的语义固化 ⇒ 过早、需返工。故 I3 与 C1/C2 **owner-sequenced**，不在本刷新单独实施 |
| 关闭条件 | owner 就 `RFC-0051-A2` 裁决 C1/C2 → 另开实现 STEP 做 I3 + 真实验证。**C1/C2 未裁前不得宣称运行期强制完整，gate 2 不能翻 GO** |

### gate 3 — Roster 三包 deterministic corpora（§3 行 59–61）——**GO（两半部闭合，再确认）**

| 半部 | 裁决 | 本刷新执行证据 |
|---|---|---|
| corpora 半部 | **闭合** | `tools/validate-package-core.ps1`（wasm32 纯 Rust 路径，不受 EDR 影响）：**image-match / image-grid / image-contour 全部 rebuilt byte-identical（True/True/True）**——独立复核了 v2 的 registered 值 |
| M7 registry 消费半部 | **闭合** | `package_roster_consumer` **11/11**（MSVC）：7 happy-path 逐字节 vs 冻结语料 + 4 fail-closed |

> 口径备注（沿用 v2，非阻塞）：STEP-0593 触碰 runner `lib.rs` 限额段属**资源记账 bug 修复**（非语义变更），按非语义口径满足「zero compiler/Runtime changes」；若 owner 采严格字面须澄清。

### gate 4 — M17 显式审计（§3 行 62–63）——**NO-GO（M17 overall）+ 具名残余**

| 项 | 内容 |
|---|---|
| **裁决** | **NO-GO（M17 overall）** |
| 支撑 | gate 1（加速器）= NO-GO（E1）；gate 2 = NO-GO（C1/C2 + I3）；gate 3 = GO ⇒ **M17 必需 gate 未全部有执行证据**（gate 1/2 未闭）⇒ M17 overall NO-GO |
| 具名残余 | E1（gate 1）+ gate 2 残余（C1/C2 + I3）。**gate 3 不列残余（已 GO）** |
| 关闭条件 | gate 1 + gate 2 关闭 + 刷新显式 M17 审计。**gate 1 未关前 M17 overall 不能翻 GO** |

### gate 5 — GUI 格式转换器 pilot（§3 行 64–70；§8.4；§8.6.2）——**NO-GO**

| 项 | 内容 |
|---|---|
| 门文本要点 | 转换器应用以 Sico 编写、基于 Sico 原生 UI 库，在 Windows evidence host 运行；第一步增量 JPEG↔PNG 双向在冻结语料上字节稳定；§8.5 fail-closed corpora 全绿；无静默 fallback/无 native escape/无核心 compiler·Runtime 补丁 |
| **裁决** | **NO-GO** |
| 本刷新执行证据（MSVC，internal-fixture） | **UI 库积木 G1–G5 全部有执行证据**：`ui_authority` **7/7**（G1 真实窗口 + 授权边界）；`ui_guest_visible` **3/3**（G2 guest `run()` 呈现帧逐字节 = offscreen oracle、G3 guest 观测 `closed`、G5 客户级帧确定性）；`ui_source` **6/6**（**G4a** `poll=click:5` / `poll=select:6:2` typed 事件归因，STEP-0632）；renderer crate `frame_corpus` **5/5** + `layout_corpus` **6/6** + `event_corpus` **5/5** + lib **21**（G5 offscreen 帧确定性）。**G4b `select`**：按 owner 2026-11-07 裁决 RFC-0052-A3=A1 **收窄为 declared-but-not-produced**（合同缺口，非实现缺口，诚实收窄非翻门）。**E2 JPEG codec**：`jpeg-decoder 0.3.2`/`jpeg-encoder 0.7.1` 离线可得，`image-codec` 原生往返测试 `tiny_420_jpeg_round_trip_is_not_oversized` **1/1 绿** ⇒ **E2「离线不可得」前提推翻，门可合法关闭** |
| 具名残余 | ① **G6 转换器 pilot 应用本体未编写**（clean-room-consumer 级；`sico.ui.*` host 调用 + 消费 `image-codec` M7 包 + JPEG↔PNG 双向）；② **image-codec 核心 `.wasm` 未重冻结**——committed `image-codec-core.wasm`（sha `28917071…`，PNG-only）≠ 含 JPEG 源码重建（sha `cc47a3ea…`），`validate-package-core.ps1` 判 `NOT-reproducible`；须**刻意 re-pin**（更新 `.wasm` + `.sha256`，ADR-0019 digest-pinned，owner 协调，因系冻结工件）；③ **§8.5 转换器级 fail-closed corpora 未整体绿**（cancellation-mid-conversion / save-path-failure / batch 采样）；④ 文件对话框后端已落地（RFC-0055 picker，`picker_source` 测试在树）但与转换器应用的端到端接线未做 |
| 关闭条件 | ② re-freeze（owner 协调）→ ① 编写 G6 应用 + ④ 端到端接线 + ③ §8.5 corpora 全绿 → 在 Windows host 运行 + 字节稳定。**G6 与 re-freeze 未达前 gate 5 不能翻 GO** |

### gate 6 — 证据标签分离（§3 行 71–74）——**GO（纪律保持，再确认）**

| 项 | 内容 |
|---|---|
| **裁决** | **GO（纪律保持）** |
| 支撑 | 全部 M17/provenance/roster/UI 证据均标 `internal-fixture`（各 STEP header + `docs/evidence/m24/*.json` 的 `class`/`evidence-class` 字段）；**无 clean-room-consumer 证据可标**（G6 pilot 未编写，无混标风险）；无 external-pilot/production 声称 |
| 关闭条件 | G6 pilot 落地时须以 `clean-room-consumer` 标 pilot 应用，不得与 internal-fixture 混标 |

### gate 7 — 回归绿 + 显式分项裁决（§3 行 75–79）——**NO-GO（E4 + 干净候选 SHA + GNU）**

| 项 | 内容 |
|---|---|
| 门文本要点 | M0–M22 + 已落地 M23 变更在 M24 审计 revision 回归绿；对 M17 gate 2、gate 4/roster、原生 UI 库、转换器 pilot、M18 组合项增补给出显式分项裁决 |
| **裁决** | **NO-GO** |
| 本刷新执行证据（MSVC） | **broad 回归绿**：runner `--lib` **48/48**；`sico-package` **51/51**；M24 聚焦套件（§3）。**selfhost_compiler（M22 自举回归）= 红**：本刷新实跑完成，**多个测试 FAILED**（含 `every_match_site_is_measured_on_the_component_axis`、`sico_codegen_guest_*_byte_exactly` 系、`sico_codegen_guest_pins_the_t1_family_truth_table`、`sico_codegen_guest_refuses_*`、`oracle_host_call_in_a_straight_line_body_truth_is_pinned`）——**非本审计引入**（本审计零源码改动，未触碰 `selfhost/*.sico`/`codegen-wasm`；红源于并发 M22 codegen/emitter-vocabulary 改动 + STEP-0627 已登记的 HEAD 预存红 `every_match_site...`）⇒ **M22 自举回归在本 host 当前为红，gate 7「M0–M22 回归绿」不满足**（reinforce NO-GO）。**显式分项裁决（本审计交付）**：M17 gate 2 = NO-GO（E1）；M17 gate 4/roster = gate 2 NO-GO / gate 3 GO；原生 UI 库 = G1–G5 证据（G6 未达）；转换器 pilot = NO-GO（G6 未编写）；M18 组合项 = 按 STEP-0258 重定为 GUI 格式转换器，随 gate 5 |
| 具名残余 | ① **E4** 独立 CI 未取得（`dev`/`m24` 触发面 + owner 手动 dispatch 绑定精确 SHA）；② **干净候选 SHA**：本刷新证据基于 `m24` 分支 checkpoint commit `94ea1fa`（已 push origin），但全量回归未在该 SHA 上于 GNU 复跑；③ **GNU 工具链**：见 §0 工具链诚实声明，EDR 阻塞 GNU，权威 GO 需 GNU 复跑 |
| 关闭条件 | 补齐 GNU `dlltool.exe`（EDR 已解除）→ 在干净候选 SHA 上于 GNU 跑 M0–M22 全量 + selfhost（并先收口 M22 match-site ledger `sites.len()==406` 断言与 `corpus-parity-v0.json` 的计数不一致，见下）→ owner dispatch 独立 CI success → 新七门复审 |

## 3. 本刷新执行证据表（2026-11-19，本机 MSVC 实跑）

| 套件 / 命令 | 结果 | 支撑门 |
|---|---|---|
| `cargo test --locked --offline -p sico-package` | 各套件合计 **51/51 绿** | gate 2（digest/budget 包级） |
| `cargo test --manifest-path runner/… --test provenance_runtime` | **6/6 绿** | gate 2（budget 运行期 + 取消） |
| `cargo test --manifest-path runner/… --test provider_crash_isolation` | **4/4 绿** | gate 2（provider-crash 隔离） |
| `cargo test --manifest-path runner/… --test package_roster_consumer` | **11/11 绿** | gate 3（registry 消费） |
| `cargo test --manifest-path runner/… --test ui_authority` | **7/7 绿**（真实窗口） | gate 5（G1 + 授权边界） |
| `cargo test --manifest-path runner/… --test ui_guest_visible` | **3/3 绿**（真实窗口） | gate 5（G2/G3/G5） |
| `cargo test --manifest-path runner/… --test ui_source` | **6/6 绿** | gate 5（G4a typed click/select） |
| `cargo test --locked --offline -p sico-ui-renderer-d2d` | lib 21 + frame 5 + layout 6 + event 5 **全绿** | gate 5（G5 offscreen 确定性） |
| `cargo test --manifest-path runner/… --lib` | **48/48 绿** | gate 7（broad 回归） |
| `tools/validate-package-core.ps1` | match/grid/contour **byte-identical**；**codec NOT-reproducible**（core 未重冻结） | gate 3 corpora GO；gate 5 残余② |
| `cargo test --manifest-path guests/image-codec/` | **1/1 绿**（JPEG 往返） | gate 5（E2 门可关） |
| `cargo test --manifest-path runner/… --test selfhost_compiler` | **完成、多个 FAILED（红）**——非本审计引入（并发 M22 codegen/emitter-vocab + STEP-0627 HEAD 预存红）；M22 自举回归当前红 ⇒ gate 7 不满足 | gate 7（M22 自举回归） |

> 全部命令在 `RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-msvc'` 下 `--locked --offline` 实跑；
> UI 套件创建真实 `CreateWindowExW` 窗口。证据等级 `internal-fixture`（runtime-verified）。

## 4. M24 overall 裁决

**M24 overall = NO-GO。** 七门中：gate 3、gate 6 = GO；gate 1/2/4/5/7 = NO-GO。
**不能翻 overall GO**：gate 5 的 G6 转换器应用未编写 + image-codec 核心未重冻结；gate 2 的 C1/C2 未裁 + I3 未接；gate 1/4 的 E1 加速器 owner 外部门未关；gate 7 的独立 CI + 干净候选 SHA + GNU 全量回归未达。

**残余寄存器（具名）**：
- **owner 外部门**：E1（gate 1 加速器硬件 + ADR + 容差表 + 注入证据）。
- **owner 裁决/协调**：C1/C2（`RFC-0051-A2`，gate 2 预算语义）→ I3 接线；image-codec 核心重冻结（gate 5）。
- **内部实现（可行，未做）**：G6 转换器 pilot 应用 + §8.5 corpora（gate 5）。
- **基础设施**：解除 EDR → GNU 全量回归 + selfhost + 独立 CI + 干净候选 SHA（gate 7）。

> 本刷新为 audit-of-current-state：**未翻任何 §3 gate 数值、未改证据等级、未据 MSVC 单独翻门**。
> 逐门证据与残余已更新至 2026-11-19；v2（2026-11-07-refresh）原文保留以维持可审计性。
