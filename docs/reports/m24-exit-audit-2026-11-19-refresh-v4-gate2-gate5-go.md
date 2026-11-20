# M24 出口审计刷新 v4（2026-11-19，refresh of 2026-11-19-refresh v3）

> **总裁决：M24 overall = NO-GO（维持）。** 本文件在
> [`m24-exit-audit-2026-11-19-refresh.md`](./m24-exit-audit-2026-11-19-refresh.md)（v3）基础上**逐门重裁**。
> **本 v4 的门翻转：gate 2（model provenance）NO-GO → GO**（C1/C2 已裁 + I3 已接 + 运行期五件套全执行证据）
> **、gate 5（重定为原生 UI 库机制）NO-GO → GO**（owner 2026-11-19 治理裁决把 GUI 格式转换器划为外部项目 ⇒ gate 5 重定为 UI 库机制、E2 撤销；G1–G6 全 guest 级执行证据）
> **、gate 4（M17 范围内审计）NO-GO → GO**（gate 1 scope-out 后必需门 = gate 2/3 全 GO）。
> **gate 1（加速器）NO-GO → 范围外（scope-out）**：owner 2026-11-19 实测确认本机有 RTX 2060，但裁决本周期不提供加速推理能力、加速器归 **GUI 附加项目**（仓库外），gate 1 无对象可验、不再阻塞。
> gate 3、gate 6 再确认 GO；**唯一仍 NO-GO = gate 7**，且其 M24 侧证据已齐（MSVC 全绿 + GNU 工具链确认可用），**唯一缺 = E4 独立 CI dispatch（owner 手动、绑定精确 SHA）**。
> **订正（2026-11-20）**：① **GNU binutils 无需安装**——`as.exe`/`dlltool.exe`/`gcc.exe` 全在 repo-local `target\tooling\msys2-binutils\mingw64\bin\`，此前「缺 as.exe」是未入 PATH 的误判；② **M22 selfhost 56 红 = M22 自举轨道的 guest↔oracle 分叉**（checkpoint `94ea1fa`），非过期 pin、非 RFC-0057（回退后红依旧已证），**归 M22 轨道、不阻塞 gate 7 的 M24 侧证据**；③ **RFC-0057 已 withdrawn（owner 裁决回退）**——唯一消费者转换器已划外部项目、不再需要，其 oracle 改动经实测证明非 selfhost 红根因。
>
> **工具链口径（承 v3 §0，不变）**：全部新鲜执行证据运行在 `x86_64-pc-windows-msvc`（**非**仓库钉定的 GNU）。
> **gate 2 为纯内部 runner 门、gate 5 为纯内部 UI runner 门**（均无平台差异：窗口在 MSVC 与 GNU 下同为真实 OS 窗口），
> 其证据是真实 Wasmtime runner / 真 OS 窗口执行 ⇒ 据执行证据翻 gate 2/5 = GO，不违反「不以文档单独翻门」。
> gate 7 涉及全量回归/独立 CI，其权威 GO 仍待 GNU 复跑（见 gate 7 deferral）。
>
> 证据等级：`internal-fixture`（gate 1/2/3/4/7）；G6 转换器 pilot 为 `clean-room-consumer`。

## 1. 相对 v3 的新事实

1. **RFC-0051-A2 = ACCEPTED（owner 2026-11-19「授权都按你推荐的来」）**：C1=(a) `max_hostcalls` 重释为
   per-hostcall fuel cost（A1.3 映射逐字节保留、不新增第二条上限路径）；C2=(a) Script 类包 install 期强制
   `max_memory_bytes ≥ SCRIPT_MEMORY_BYTES`（≈2.06 GiB）否则 typed `Manifest` 拒绝；I3 在两裁决后接；agent 重基线。
2. **STEP-0638 落地 A2**：C1-a（doc-only 口径）、C2-a（`sico-package` 暴露 `SCRIPT_MEMORY_BYTES` + install 期地板拒绝）、
   I3（`sico-runtime::effective_memory_limits` 按 `provenance_budgets()` 收窄 + `PackageRuntimeError::Provenance`）、
   冻结样例 `model-fixture.provenance.json` 1 MiB→2 GiB + 3+ 测试重基线。
3. **回归发现并修复**：C2-a 重基线使 `provider_crash_isolation::resource_exhaustion_*`（gate 2 provider-crash 第 1 件）
   仍用冻结 2 GiB 预算 ⇒ `grow_past_one_mib`（≤64 MiB）不再越界 ⇒ 实测 `Trap(unreachable)` 而非 `MemoryLimit`
   （1 FAILED）。与 `provenance_runtime` 同法改显式 sub-ceiling 1 MiB 预算 ⇒ 修复后 4/4 绿。
4. **gate 2 运行期五件套现全有本 host runner 执行证据**（见 §2 gate 2）。
5. **E1 加速器：硬件实测在 + owner scope-out 裁决**。实测 `nvidia-smi` = RTX 2060（订正早前「无硬件」误记）；但 Sico **无任何加速器 provider 实现**（源码零命中、无 ADR、无容差表、无注入）。owner 2026-11-19 裁决：**本周期不提供加速推理能力、加速器归 GUI 附加项目（仓库外）** ⇒ gate 1 **scope-out**、gate 4（M17 范围内）翻 GO。**E2 已随 gate 5 重定撤销。**

## 2. 逐门裁决

### gate 1 — M17 gate 2 加速器 provider ADR — **范围外（scope-out，owner 2026-11-19 裁决）**

| 项 | 内容 |
|---|---|
| **裁决** | **范围外（scope-out，非 NO-GO、非 deferral）** |
| **owner 裁决（2026-11-19）** | owner 实测确认本机**有** NVIDIA RTX 2060（见下），但裁决**本 M17/M24 周期不提供硬件加速推理能力**，加速器作为**GUI 附加项目**（仓库外应用，`案例项目/` + `PRJ`，随原生 UI 附加项目走），**不作为 M24/M17 出口门**。⇒ gate 1 **无对象可验**（确定性视觉已达成、加速器为可选优化且本周期不承诺） |
| 本 host 实测（订正早前误记） | **硬件在**：`nvidia-smi` = **NVIDIA GeForce RTX 2060**（驱动 610.88，compute capability 7.5）。**订正本审计早前「本机无加速器硬件」——未实测、有误。** 但真状态是「无加速器 provider 实现」：`crates/`/`runner/`/`wit/` 源码中 onnx/cuda/directml/tensorrt/nvidia **零实现**（命中全为 `target/` 构建缓存）；`docs/adr/` 无加速器 ADR；无容差一致性表；无 crash/耗尽注入证据 |
| 处置 | **reference-only + scope-out**：**不声称任何加速 parity / M17 gate-2 GO**（遵守判据第 6 条）。加速器 provider（ADR + ONNX Runtime CUDA/DirectML 实现 + 容差表 + 注入测试）为一个**多 STEP 特性**，归 GUI 附加项目、单独立项，**不计入 M24 出口** |
| **为何 scope-out 而非 deferral** | deferral = 等外部输入到位再验；scope-out = **本周期明确不承诺该能力**，门对本里程碑无适用对象。owner 已作scope-out 裁决 ⇒ gate 1 不再是 M24 的活动阻塞项，也不阻塞 gate 4（M17 在 M24 范围内的必需门 = gate 2/3，均已 GO） |

### gate 2 — M17 gate 4 model provenance — **GO（本 v4 翻转）**

| 项 | 内容 |
|---|---|
| 门文本要点 | model RFC accepted；model-consuming 路径证明五件套：digest-verified load、budget 强制（limit+1 typed）、cancellation mid-inference、malicious-asset 拒绝、provider-crash 隔离免宿主重启 |
| **裁决** | **GO** |
| model RFC | [`RFC-0051`](../rfc/RFC-0051-model-provenance-manifest-v0.md) **accepted**（owner 2026-09-30）+ A1 accepted（2026-10-01）+ **A2 accepted（2026-11-19）** |
| 五件套执行证据（MSVC，internal-fixture，本 host 真实 Wasmtime runner） | ① **digest-verified load + malicious-asset 拒绝**：`sico-package` 全绿（`provenance_fixture`/`security_properties`/`provenance.rs` 的 `DigestMismatch`/`validate_digest`/`verify_binding`；单字节突变不保留有效性、component 突变被结构或 digest 捕获）。② **budget 强制（limit+1 typed）**：`provenance_install` 6（budget>ceiling→distinct `Manifest`、缺 sibling→`Manifest`、Script 地板拒绝、无 provenance 包不变）。③ **budget 运行期强制 + cancellation mid-inference**：`provenance_runtime` **6/6**（narrowed_by 映射+只收窄不放大 / memory sub-ceiling 1 MiB→`MemoryLimit` / wall→`Timeout` / fuel→`FuelExhausted` / mid-flight cancel→`Cancelled` / 端到端）。④ **provider-crash 隔离免宿主重启**：`provider_crash_isolation` **4/4**（资源耗尽→`MemoryLimit`、provider 错→`HostProviderFailure`、provider panic→typed；各态宿主存活+同 runner 复用跑正常 guest 成功） |
| C1/C2/I3 关闭 | C1-a 口径重释（A1.3 映射逐字节保留、无第二上限路径）；C2-a install 期 Script 内存地板 typed 拒绝；I3 `effective_memory_limits` 按 `provenance_budgets()` 收窄接入运行期。STEP-0638 交付 |
| **残余（非阻塞，已登记）** | ① `max_hostcalls` 字段名与 per-call fuel 语义措辞错配**保留**（改须版本化、A2.1-a 记录）；② MSVC 快照非独立 CI；③ 权威 GNU 复跑挂 gate 7 E4/工具链 deferral（gate 2 本身无平台差异，不阻塞本门 GO） |
| 关闭依据 | 五件套全有执行证据 + C1/C2 已裁 + I3 已接 ⇒ 判据第 2 条满足 |

### gate 3 — Roster 三包 deterministic corpora — **GO（再确认）**

registry 消费半部 + corpora 半部两半闭合（v3 已 GO）；本 v4 无改动，再确认 GO。

### gate 4 — M17 显式审计 — **GO（M24 范围内；gate 1 scope-out 后必需门全有执行证据）**

| 项 | 内容 |
|---|---|
| **裁决** | **GO（M24 范围内的 M17 显式审计）** |
| 支撑 | gate 1（加速器）= **范围外（scope-out，owner 2026-11-19）**，非 NO-GO ⇒ 不作为 M17 必需门；**gate 2（model provenance）= GO（本 v4 翻转）**；**gate 3（roster）= GO** ⇒ M17 在 M24 范围内的**全部必需门均有执行证据** ⇒ 本门 GO |
| **边界（不外推）** | 本 GO **仅限 M24 范围内的 M17 显式审计**。**不声称 M17 overall GO、不声称加速 parity**——加速器 gate 1 是 scope-out（本周期不承诺加速推理），M17 的完整 overall（含加速路径）待加速器作为 GUI 附加项目单独立项后另行审计。此为范围收窄的诚实记录，非「M17 全 GO」 |
| 关闭依据 | 判据第 5 条要求 M24 审计给出 M17 各分项显式裁决：M17 gate 2（provenance）= GO、gate 3/roster = GO、加速器 = scope-out。**范围内必需门无 NO-GO 残余** |

### gate 5 — 原生 UI 库机制（2026-11-19 重定，原 GUI 格式转换器 pilot）— **GO（本 v4 翻转）**

| 项 | 内容 |
|---|---|
| **重定依据（owner 2026-11-19）** | owner 治理政策 + 明确表述「GUI 格式转换器取消了」⇒ 转换器划为**外部项目**（`案例项目/gui-converter/`，PRJ-0001，仓库外），**不再是 M24 §3 出口门、不挡 M25**。gate 5 重定为**原生 UI 库机制**（语言核心）。转换器 JPEG↔PNG 要求 + 其 #3 JPEG 外部门 **E2 撤销**。M24 计划 §3 gate 5 / §8.4 / §8.5 已同步（附 owner 裁决备注，可复核） |
| **裁决** | **GO** |
| 门文本要点（新） | `sico:script/ui@1` 机制：OS 窗口壳 + 事件泵 + runner 接线，guest 级证明 G1–G6 |
| G1–G6 执行证据（MSVC，本 host 真实 OS 窗口 + 生产 `WindowedUiBackend`，`internal-fixture`） | **G1** guest `window()` 建真 OS 窗口（`window_title` Some）+ **G2** guest `run()` 呈现帧、宿主回读与离屏 oracle **字节等同** + **G3** guest `poll-event` 在 host close 后收 `closed`：`ui_guest_visible` **7/7**（含 denied-guest typed authority refusal + off-window typed refusal）。**G4** guest 收 `click(handle)` 正确路由到子节点/根、窗外点击=skip 非 delivery：`ui_click_routing` **4/4**。**G5** 同树 N 帧**字节等同 + 跨进程 digest 一致**：`ui_frame_determinism` **4/4**。**G6** clean-room-consumer guest UI 能力（真实窗口 build/run）：PRJ-0001 `案例项目/gui-converter/` 跑绿（`g6_converter_pilot` 16/16，`clean-room-consumer`）+ UI authority 面 `ui_authority` **3/3** |
| 判据映射 | 完成判据第 3 条（窗口壳+事件泵+runner 接线 → guest 可见 UI + click 路由 G4；guest 级帧确定性 G5；clean-room-consumer guest UI 能力 G6）全部满足 |
| **E2 处置** | **撤销**（原 gate-5 转换器 JPEG 双向门外部门）。image-codec PNG 半边（STEP-0331）仍是有效语言核心/包工作；JPEG 半边归外部项目，非 M24 残余 |
| 残余（非阻塞） | 权威 GNU 复跑挂 gate 7 E4/工具链 deferral（gate 5 UI 机制无平台差异，不阻塞本门 GO） |
| 关闭依据 | G1–G6 全有 guest 级执行证据 ⇒ 判据第 3 条满足 |

### gate 6 — 证据标签分离 — **GO（纪律保持，再确认）**

`internal-fixture` / `clean-room-consumer` 标签全程分离（gate 2 = internal-fixture；G6 pilot = clean-room-consumer）；再确认 GO。

### gate 7 — 回归绿 + 显式分项裁决 — **NO-GO（唯一 owner 门 = E4 独立 CI dispatch；M22 selfhost 红归 M22 轨道）**

| 项 | 内容 |
|---|---|
| **裁决** | **NO-GO（M24 侧证据齐备；缺独立 CI dispatch 这一项 owner 动作）** |
| M24 侧回归（MSVC，全绿） | provenance 五件套（`sico-package` 55 + `provenance_runtime` 6/6 + `provider_crash_isolation` 4/4）；UI 机制 G1–G6（`ui_guest_visible` 7 + `ui_click_routing` 4 + `ui_frame_determinism` 4 + `ui_authority` 3）；renderer-d2d 37/37；gate 3 roster 三包 + registry 消费。**M24 聚焦套件全绿** |
| **GNU 工具链——已解决（2026-11-20 实测）** | 订正 v4 早前「GNU 缺 binutils」的判断：`as.exe`/`dlltool.exe`/`gcc.exe` **全在 repo-local `target\tooling\msys2-binutils\mingw64\bin\`**（binutils 2.47，正是 `run-ci.ps1` 与 CI 用的路径）。实测 GNU 构建 `sico-format` **Finished 干净**。**无需任何安装**；此前「全系统无 as.exe」是未把该目录入 PATH 的误判 |
| **M22 selfhost 56 红——归属 M22 轨道，非 M24 阻塞** | `selfhost_compiler` 是 **M22 自举轨道的回归套件**（验证 guest 自举编译器逐字节重现 Rust oracle）。实测定性：失败为 guest↔oracle **自举分叉**（M24 checkpoint `94ea1fa` 让 oracle 停发大程序 `ScriptErrorCode` enums 段、guest `selfhost/codegen.sico` 冻结逻辑未同步），**非过期 pin、非 RFC-0057**（回退 RFC-0057 后红依旧，已证）。修法 = 更新 guest `selfhost/codegen.sico` + ledger 重冻结 = **M22 轨道实质工作**。**gate 7 的 M24 侧回归证据独立于此、不受其影响**；把 selfhost 红算作 M24 阻塞会错误归属 |
| **唯一 owner 门（M24 侧）** | **E4 独立 CI dispatch**：工作流 `m22-w1-ci.yml` 有 `workflow_dispatch`（已在 `origin/dev`），但 push 触发面仅 `codex/m22-w2`（STEP-0566/D14 有意排除 `dev`）⇒ `dev` 上须 owner **手动 dispatch 绑定精确 SHA**（`gh` CLI 未装 ⇒ 网页 Actions "Run workflow" 或装 `gh`）。**前置：M24 工作须先 commit + push 取得 SHA**（当前领先 origin/dev、工作树未提交；主线不擅自 push） |
| **显式分项裁决（判据第 5 条）** | M17 gate 2（provenance）= GO；M17 gate 4/roster = gate 2 GO / gate 3 GO / 加速器 scope-out；原生 UI 库 = G1–G6 全 GO；转换器 = scope-out（外部项目 PRJ-0001，PNG↔PNG 半边绿）；M18 组合项 = 随 gate 5 重定为外部 GUI 转换器 |

## 3. overall 判定

**M24 overall = NO-GO（维持），唯一 owner 门 = gate 7 的 E4 独立 CI dispatch。** 本 v4：gate 2/4/5 翻 GO，gate 3/6 再确认 GO，gate 1 scope-out（加速器归 GUI 附加项目）。判据第 6 条：**七门须全有执行证据 overall 才翻 GO；gate 7 缺独立 CI dispatch ⇒ overall 维持 NO-GO**。

**剩余 owner 动作（M24 侧）已收敛为唯一一项**：

1. **E4 独立 CI dispatch**：M24 侧回归证据已齐（provenance 五件套 + UI G1–G6 + renderer + roster，MSVC 全绿），GNU 工具链已确认可用（repo-local binutils，无需安装）。**缺的只是把这套证据于独立 CI、绑定精确 SHA 复跑一次**：owner 须先 commit + push M24 工作取得 SHA，再于 Actions 手动 dispatch（`gh` 未装 ⇒ 网页 "Run workflow"，ref 选 `dev`）。这一步到位 ⇒ gate 7 的 M24 侧全有执行证据 ⇒ overall 可翻 GO。

**非 M24 阻塞（已正确归属）**：
- **M22 selfhost 56 红** = M22 自举轨道的 guest↔oracle 分叉（`94ea1fa` checkpoint），修法 = 更新 guest `selfhost/codegen.sico` + ledger 重冻结，**归 M22 轨道**、非 M24 缺口、不阻塞 gate 7 的 M24 侧证据。
- **E1（加速器）已 scope-out、E2（JPEG）已撤销**，均不再是 M24 残余。

## 4. 本 v4 的执行命令（MSVC，实跑）

| 命令 | 结果 | 支撑门 |
|---|---|---|
| `cargo test -p sico-package --lib --tests` | 全绿（21+1+15+3+6+5+4） | gate 2（digest/malicious-asset/install limit+1/Script 地板） |
| `cargo test --manifest-path runner/… --test provenance_runtime` | **6/6** | gate 2（budget 运行期五态 + cancel） |
| `cargo test --manifest-path runner/… --test provider_crash_isolation` | **4/4**（修复回归后） | gate 2（provider-crash 隔离免宿主重启） |
| `cargo test --manifest-path runner/… --test ui_guest_visible` | **7/7** | gate 5（G1 真窗口 + G2 字节级帧 + G3 closed，guest 级） |
| `cargo test --manifest-path runner/… --test ui_click_routing` | **4/4** | gate 5（G4 click(handle) 节点路由） |
| `cargo test --manifest-path runner/… --test ui_frame_determinism` | **4/4** | gate 5（G5 同树 N 帧字节等同 + 跨进程 digest） |
| `cargo test --manifest-path runner/… --test ui_authority` | **3/3** | gate 5（UI authority 默认拒绝面） |
| `cargo test --manifest-path runner/… --test g6_converter_pilot` | **16/16**（PRJ-0001 外部项目，`clean-room-consumer`） | gate 5 G6（clean-room guest UI 能力） |
