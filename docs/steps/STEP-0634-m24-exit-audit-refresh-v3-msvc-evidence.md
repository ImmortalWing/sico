# STEP-0634 — M24 出口审计刷新 v3（2026-11-19；MSVC 证据基底）

状态：audit-of-current-state complete locally（2026-11-19）。**M24 overall = NO-GO（维持）**；
逐门刷新裁决交付于
[`m24-exit-audit-2026-11-19-refresh.md`](../reports/m24-exit-audit-2026-11-19-refresh.md)。
**未翻任何 §3 gate 数值、未改证据等级、未据 MSVC 单独翻门**；纯审计/登记片，零源码/零 WIT/零 fixture 改动。

## 1. 与前两版审计的关系

第三版 audit-of-current-state，逐门重裁，不沿用 v1（STEP-0589）/v2（STEP-0598）结论。
v1/v2 原文保留。冲突以本版为准。

## 2. 关键新事实（相对 v2，2026-11-07-refresh）

1. **E2「#3 JPEG 离线不可得」前提被实测推翻**：`jpeg-decoder 0.3.2` / `jpeg-encoder 0.7.1`
   已在离线 registry cache（cached + extracted），`guests/image-codec` 可离线 wasm32 构建，
   原生往返测试 `tiny_420_jpeg_round_trip_is_not_oversized` 1/1 绿。**E2 门在本 host 可合法关闭**
   （v2 判其 gate 5「决定性外部门」的前提不再成立）。**但** committed `image-codec-core.wasm`
   仍为 PNG-only（sha `28917071…`）≠ 含 JPEG 重建（sha `cc47a3ea…`），`validate-package-core.ps1`
   判 `NOT-reproducible` ⇒ 须**刻意 re-pin**（ADR-0019 digest-pinned 冻结工件，owner 协调）。
2. **gate-5 UI 子项 G1–G5 全部有执行证据**（v2 记 G4/G5「在途」已过时）：G4a `click(handle)`
   （STEP-0597 `route_click_event` 点→句柄命中测试，`ui_source` `poll=click:5`）、
   G5 客户级帧确定性（`ui_guest_visible` byte-identical-to-offscreen-oracle + renderer
   `frame_corpus`/`layout_corpus`/`event_corpus`）均跑绿。G4b `select` 仍按 RFC-0052-A3=A1
   收窄为 declared-but-not-produced（合同缺口，非翻门）。**G6 转换器 pilot 应用仍未编写**。
3. **gate-2 / gate-3 / gate-7 runner 套件获新鲜执行证据**（见 §4 证据表）——独立复跑了 v2 仅以
   registered 值记录的项。

## 3. 工具链诚实声明（本版特有，关键）

本版**全部新鲜执行证据运行在 `x86_64-pc-windows-msvc`**，**非**仓库钉定的
`x86_64-pc-windows-gnu`。原因：本 host GNU 构建被**奇安信天擎 EDR** 阻塞——EDR 实时扫描使
rustc 为 `windows-sys` raw-dylib 生成的 `kernel32.dll_imports.lib` 句柄失效（`os error 1006`）；
**系统级、跨卷（C: 亦失败）、改 target 目录不可绕过**。**MSVC 目标不生成 raw-dylib 导入库**
（直接链 Windows SDK `kernel32.lib`），故绕过。

⇒ **MSVC 证据是真实的本 host 执行证据**（测试真跑、`CreateWindowExW` 窗口真建），足以支撑
子项进展与残余登记；但**权威 GO 若严格要求 Windows GNU 独立证据，须解除 EDR 后于 GNU 复跑**。
本版**不据 MSVC 证据单独翻转任何 gate 到 GO**。

## 4. 逐门裁决（详见报告 §2）

| 门 | 裁决 | 本版执行证据（MSVC）/ 残余 |
|---|---|---|
| gate 1 | **NO-GO（E1，owner）** | `docs/adr/` 仍无加速器 ADR/容差表/注入证据 |
| gate 2 | **NO-GO（C1/C2 owner + I3 owner-sequenced）** | `sico-package` 51/51、`provenance_runtime` 6/6、`provider_crash_isolation` 4/4；**I3 未单独推进——依赖 C1/C2 预算语义**（`Budgets.max_hostcalls` vs C1 hostcall 单位不匹配），先接会固化可能被变更的语义 |
| gate 3 | **GO（再确认）** | `validate-package-core.ps1` match/grid/contour 全 byte-identical；`package_roster_consumer` 11/11 |
| gate 4 | **NO-GO（M17 overall）** | gate 1/2 未闭 ⇒ M17 NO-GO；gate 3 已 GO 移出残余 |
| gate 5 | **NO-GO** | UI G1–G5 全证据（见 §2.2）；**G6 应用未编写 + codec 核心未重冻结 + §8.5 corpora 未绿**；E2 门可关 |
| gate 6 | **GO（再确认）** | 全 internal-fixture；无 clean-room-consumer（G6 未写，无混标风险） |
| gate 7 | **NO-GO（E4 + 干净候选 SHA + GNU + M22 自举回归红）** | runner `--lib` 48/48、`sico-package` 51/51 broad 绿；**但 `selfhost_compiler`（M22 自举回归）实跑红（多个 FAILED：match-site ledger + codegen byte-exactness pins）——非本审计引入（并发 M22 codegen/emitter-vocab 改动 + STEP-0627 HEAD 预存红 `every_match_site...`）**；显式分项裁决已交付；缺独立 CI + GNU 全量 + 干净候选 SHA |

## 5. 执行证据表（2026-11-19，本机 MSVC 实跑，`--locked --offline`）

| 套件 | 结果 |
|---|---|
| `cargo test -p sico-package` | 51/51 |
| runner `--test provenance_runtime` | 6/6 |
| runner `--test provider_crash_isolation` | 4/4 |
| runner `--test package_roster_consumer` | 11/11 |
| runner `--test ui_authority` | 7/7（真实窗口） |
| runner `--test ui_guest_visible` | 3/3（真实窗口） |
| runner `--test ui_source` | 6/6 |
| `sico-ui-renderer-d2d`（lib+frame+layout+event） | 21+5+6+5 全绿 |
| runner `--lib` | 48/48 |
| `tools/validate-package-core.ps1` | match/grid/contour byte-identical；codec NOT-reproducible |
| `guests/image-codec` 原生测试 | 1/1（JPEG 往返） |

## 6. M24 overall = NO-GO；残余寄存器

- **owner 外部门**：E1（gate 1 加速器硬件 + ADR + 容差表 + 注入证据）。
- **owner 裁决/协调**：C1/C2（`RFC-0051-A2`）→ I3；image-codec 核心重冻结（gate 5）。
- **内部实现（可行，未做）**：G6 转换器 pilot 应用 + §8.5 corpora（gate 5）。
- **基础设施**：解除 EDR → GNU 全量 + selfhost + 独立 CI + 干净候选 SHA（gate 7）。

## 7. 诚实收窄

- 全部执行证据为 **MSVC**，非钉定 GNU（见 §3）；未据其单独翻门。
- selfhost_compiler（M22 自举回归）长跑在本版登记时仍在后台构建，最终计数以该 run 落地记录为准。
- 单机快照、非独立 CI；无 clean-room-consumer 证据（G6 未编写）。
- **未翻任何门，M24 overall 仍 NO-GO**；仅 internal-fixture 证据。
