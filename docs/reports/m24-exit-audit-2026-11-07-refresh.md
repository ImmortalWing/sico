# M24 出口审计刷新（2026-11-07，refresh of STEP-0589）

> **总裁决：M24 overall = NO-GO。** 本文件为 **audit-of-current-state refresh**：在
> [`m24-exit-audit-2026-11-07.md`](./m24-exit-audit-2026-11-07.md)（v1，STEP-0589）基础上
> **逐门重裁**，不沿用 v1 结论，不移动任何 §3 gate。
> 逐门裁决：gate 1 **NO-GO**、gate 2 **NO-GO（具名残余 C1/C2 + I3）**、gate 3 **GO（两半部闭合，带 1 项口径备注）**、
> gate 4 **NO-GO（M17 整体 + 具名残余）**、gate 5 **NO-GO（E2 外部门决定性 + G4/G5 在途 + G6 未达）**、
> gate 6 **GO（纪律保持）**、gate 7 **NO-GO（E4/E5 外部门 + I7 未跑）**。
>
> 证据等级：`internal-fixture`（本机 Windows x64 GNU 真实 runner/Wasmtime 执行，语料为仓库内冻结集）；
> `clean-room-consumer` 尚无任何证据可标（pilot 应用未编写）；无 `external-pilot`/`production` 声称。
> `contract-verified` 与 runtime 支持继续分离；**只有真正在已执行 runner 上跑过的才计 runtime 支持**。

## 0. 本审计与 v1（STEP-0589）的关系

| 项 | v1（STEP-0589，2026-11-07） | 本审计（refresh，2026-11-07） |
|---|---|---|
| 形态 | audit-of-current-state，M24 overall NO-GO | 同形态，**逐门重裁，不沿用 v1 结论** |
| gate 1 加速器 | NO-GO / 具名外部门 E1 | **NO-GO（E1 不变，本审计再核）** |
| gate 2 provenance | **在途待刷新**（STEP-0587 在途） | **NO-GO + 具名残余（C1/C2 + I3）**——STEP-0587 已落地（五件逐条 ✓），但 C1/C2 合同缺口未裁、I3 未接线 |
| gate 3 roster | corpora 半部闭合 / registry 半部**在途待刷新**（STEP-0588） | **GO（两半部闭合）**——STEP-0588（fail-closed 4/4）+ STEP-0593（11/11，table 限额修正）⇒ registry 半部闭合；带「零 compiler/Runtime 变更」口径备注 |
| gate 4 M17 显式审计 | NO-GO + 具名残余 | **NO-GO + 具名残余**（残余组成收窄：gate 3 已 GO，剩 E1 + C1/C2 + I3） |
| gate 5 转换器 pilot | NO-GO（E2 #3 JPEG 决定性；UI 子项 STEP-0581 在途） | **NO-GO（E2 决定性；UI 子项改善：G1/G2/G3 已达成〔G2/G3 收窄口径〕，G4/G5 在途 STEP-0597/0595，G6 未达）** |
| gate 6 证据标签 | GO（纪律保持） | **GO（纪律保持，本审计再核）** |
| gate 7 回归 + 分项裁决 | NO-GO（E4/E5 + I7） | **NO-GO（E4/E5 + I7；本地回归证据基底更广但仍为登记值，未复跑）** |

**三条纪律**：
1. **不改历史结论**：v1（STEP-0589）原文保留；本文件是**新的逐门裁决**。凡与本文件冲突处以本文件为准。
2. **在途不预裁**：STEP-0594（校验器族普查数重同步）、STEP-0595（G5 guest 级帧确定性专测）、
   STEP-0596（`image-codec@1` registry 消费）、STEP-0597（runner 侧交付 `click(handle)` ⇒ G4）
   **正在实施**（均无 STEP 文档文件，与任务在途清单一致）⇒ 依赖它们的门/分项（gate 5 的 G4/G5、
   `image-codec@1` registry 消费）本审计**不裁定其成败**，写「在途待刷新」，并说明**若落地则该门剩余项为何**。
3. **本机无并发构建**（`tasklist` 无 cargo/rustc 进程；target 锁为 7 月陈旧文件），但工作树含大量并行代理在途改动、
   无干净候选 SHA、本机无 gcc/binutils ⇒ **本审计未独立复跑任何 cargo 测试**，全部测试证据为
   **STEP 文档登记值**（见 §6 诚实收窄）。

## 1. 审计基线

| 项 | 值 |
|---|---|
| HEAD SHA | `7bcdc35f664a0c5c05be6c49e755139d9d586476`（merge commit：`origin/codex/m22-w2` 并入 `dev`，2026-11-06）——**与 v1 相同，v1 后无新提交** |
| 分支 | `dev` |
| 快照 | 2026-11-07（本地时区 Asia/Shanghai）；对 2026-11-07 worktree 成立 |
| 工作树状态 | **大量既有 dirty**（与 v1 相同基底 + 新增交付物）：staged（`AGENTS.md`/`AGENT_GOAL.md`/`Cargo.*`/`docs/*`/`tests/architecture/module-boundaries.json`）+ unstaged（`crates/*`/`runner/*`/`tools/*`）+ untracked（`crates/sico-ui-renderer-d2d/**`、`crates/sico-package/tests/provenance_*.rs`、`runner/sico-runner/tests/{ui_guest_visible,provider_crash_isolation,package_roster_consumer,package_image_codec_consumer}.rs`、`docs/evidence/m24/ui-*.json`、`docs/rfc/RFC-0051/0052-amendment-A2-*.md`、`docs/steps/STEP-0577…0593-*.md`）。**全部为既有/并发代理改动，本审计未触碰** |
| 本审计新增 | **仅** `docs/reports/m24-exit-audit-2026-11-07-refresh.md`（本文件） |
| 在途项 | STEP-0594（校验器族普查数重同步）、STEP-0595（G5 guest 级帧确定性专测）、STEP-0596（`image-codec@1` registry 消费；测试文件 `package_image_codec_consumer.rs` 已在工作树，8 测试，STEP 文档未出）、STEP-0597（runner 侧交付 `click(handle)` ⇒ G4） |
| 证据等级说明 | `internal-fixture` = M17 corpora/refusal fixture + roster/UI/provenance 仓库内冻结集（真实 runner 执行）；`clean-room-consumer` = pilot 应用（**尚无证据**）；`external-pilot`/`production` = 本里程碑**不产生、不声称**（§6） |

> 本审计**未运行任何 cargo 命令**（本机无 gcc/binutils，冷构建会失败）。证据以既有 STEP 记录与可复核入口文件为准
> （文件:行 / 证据 JSON / ADR/RFC header），与 AGENTS §1.4「以可执行证据为准、文档与实现冲突时取下界」同一口径。

## 2. 逐门裁决表

> 「门文本要点」引用 [`M24 plan`](../plans/M24-vision-closure-gui-application-pilot.md) 原文行号（**worktree 版与 HEAD 的
> §3 gate 文本逐字一致**——工作树仅 §8.6.1 状态块被更新为 2026-11-07 后版本，gate 行号 49–79 不变）。

### gate 1 — M17 gate 2 加速器 provider ADR（§3 行 51–54；§8.1 行 139–158）

| 项 | 内容 |
|---|---|
| 门文本要点 | 加速器 provider ADR **accepted**；在 ≥1 台 Windows host 上加速实现与 reference 在声明容差内一致（冻结语料）；provider crash/资源耗尽隔离且 fail-closed（行 51–54、146–153）。§8.1 记录延宕子句：无授权加速器 ⇒ gate 2 保持 NO-GO/deferred（行 154–158） |
| **裁决** | **NO-GO（具名外部门 E1）** |
| 支撑证据 | **本审计再核：`docs/adr/` 全目录仍无加速器/accelerator ADR**（`ls docs/adr/ | grep -iE 'acceler|accelerator'` = NONE）；无参考-vs-加速容差对比表；无 provider crash/资源耗尽注入证据。`docs/evidence/m24/image-*-support.json` 的「accelerator」仅出现在 `not-claimed` 字段（如 `image-match-support.json`：「M17 §3 gate 1 (accelerator) and #4 Direct2D remain external doors」）。`image-vision@1`（gate-1 前置，STEP-0166，M17 gate 1 GO）已绿，但不覆盖本 gate |
| 具名残余 | E1：①加速器 provider ADR（accepted）；②≥1 台 Windows host 参考-vs-加速容差一致性表（冻结 `image-vision@1` 语料）；③provider crash/资源耗尽注入 + fail-closed + 宿主免重启 |
| 关闭条件 | owner 指定具名加速器硬件 + 接受加速器 ADR + 在该 host 跑出容差一致性表与 crash/耗尽注入证据。**本审计不给「可自行关闭」暗示** |

**与 v1 相比：不变**（仍 NO-GO，E1；本审计独立复核确认 `docs/adr/` 无加速器 ADR）。

### gate 2 — M17 gate 4 model provenance（§3 行 55–58；§8.2 行 160–177）

| 项 | 内容 |
|---|---|
| 门文本要点 | model RFC **accepted**；一个 model-consuming 路径（可用 fixture model）证明五件套：digest-verified load、budget 强制（limit+1 typed）、cancellation mid-inference、malicious-asset 拒绝、provider-crash 隔离免宿主重启（行 55–58、171–175） |
| **裁决** | **NO-GO（具名残余 C1/C2 + I3）**——五件逐条有 internal-fixture 证据（完成度），但 **budget 强制件因 C1/C2 合同缺口不完整、I3 未接线**，不得判 GO |
| 支撑证据（已落地，登记值） | **RFC-0051 + A1 accepted**（STEP-0323）。**digest-verified load**：`crates/sico-package/src/provenance.rs:162` `verify_asset` 三态 distinct（`LengthMismatch`/`Truncated`/`DigestMismatch`，:167/:172/:176；本审计核对行号）+ `tests/provenance_fixture.rs` 6/6（STEP-0334，internal-fixture/unit-level）。**budget 强制（部分）**：`VerifiedPackage::provenance_budgets` + `Runner::run_program_with_budgets` + `RunnerLimits::narrowed_by`（STEP-0578，`crates/sico-package/tests/provenance_budget.rs` 3 绿 + `runner/sico-runner/tests/provenance_runtime.rs` 6 绿，runtime-verified memory/wall/fuel/取消 4 态 + 端到端 `MemoryLimit`；`cargo test -p sico-package` 51/51）。**cancellation mid-inference**：STEP-0578 测试 5（自旋 guest + `CancelToken` → typed `Cancelled`）。**malicious-asset 拒绝**：`provenance_fixture` 未知字段结构拒 + `malformed-unknown-field.provenance.json`。**provider-crash 隔离（第 5 件）**：**STEP-0587**（`runner/sico-runner/src/stream_provider.rs` 新 DI seam `StreamTransport`/`StreamTransportFactory`，**依赖注入而非生产后门**，默认 `Runner::new()` 恒走真实 stdio；`runner/sico-runner/tests/provider_crash_isolation.rs` 4 测试——资源耗尽→typed `MemoryLimit`、provider 错→typed `HostProviderFailure`、provider panic→被拦截且 typed、正控制真实传输；三态均断言**宿主存活 + 同 runner 随后跑正常 guest 成功**（Store 可复用），复跑 3 次；证据等级 internal-fixture） |
| 具名残余 | ① **C1** hostcall 预算运行期不生效 + 单位不匹配（`max_hostcalls` 计数 vs `hostcall_fuel` 每调用费用；`invoke`/`start_debug` 以 `max(hostcall_fuel, stdin*64+64MiB)` 地板覆盖 ⇒ 冻结 fixture `max_hostcalls=100000` 实际生效 64 MiB ⇒ 预算零效果）；② **C2** 真实 Script 组件 `SCRIPT_MEMORY_BYTES`≈2.06 GiB、1 MiB 预算运行期不透明 trap；③ **I3** `provenance_budgets`/`run_program_with_budgets` 未接入 CLI/app 层（`sico-cli`/`sico-app-cli`，STEP-0578 §7 接线点）。①② 见 [`RFC-0051-A2`](../rfc/RFC-0051-amendment-A2-hostcall-budget-and-script-memory-floor.md)，**PROPOSED 未裁决**（本审计核对 header：`status: PROPOSED — draft for owner adjudication`；推荐 C1→(a)「每 hostcall 的 fuel 费用」、C2→(a)「Script 包内存预算 ≥ SCRIPT_MEMORY_BYTES 否则 install 拒绝」） |
| 关闭条件 | ① owner 就 `RFC-0051-A2` 裁决 C1/C2 + 另开实现 STEP；② I3 实现 + 真实验证。**C1/C2 未裁前不得宣称运行期强制完整**（STEP-0578 原文口径），gate 2 不能翻 GO |

**与 v1 相比：由「在途待刷新」变为「NO-GO + 具名残余」**。v1 因 STEP-0587 在途而标「在途待刷新」；
现 STEP-0587 已落地（五件逐条 ✓），但 **C1/C2 合同缺口仍未裁决、I3 未接线**，且无任何在途 STEP 覆盖
C1/C2 实现或 I3 ⇒ 依「证据清单齐备 ≠ 裁决」纪律判 **NO-GO**，不沿用「在途」标签。

### gate 3 — Roster 三包 deterministic corpora（§3 行 59–61；§8.3 行 179–188）——**两半部分别记**

| 半部 | 门文本要点 | 裁决 | 支撑证据 | 具名残余 / 备注 |
|---|---|---|---|---|
| **corpora 半部** | 三包（template match/grid/contour）deterministic corpora 字节精确 vs reference（行 59–61、180–185） | **闭合（已达成）** | **3/3 全绿**：STEP-0335 `image-match@1`、STEP-0336 `image-grid@1`、STEP-0337 `image-contour@1`，全部 internal-fixture/**runtime-verified package-Component 级**——独立 oracle（纯 stdlib Python，与 Rust core 无共享代码）内容语料逐元素字节精确 + RFC-0041 可达 `image-error` 闭集逐 case 类型化拒绝 + fresh-store 字节稳定；`tools/validate-package-core.ps1:123-136` → **PACKAGE-CORE VERDICT: reproducible**（clean 重建字节全等）；**零 compiler/Runtime 语义改动**（`build_package_component` 路线）。契约：RFC-0043 + A1（region→16B packed Bytes）+ A2（bitmap 输入 header），owner 2026-10-01 复裁决接受 | 无 |
| **M7 registry 消费半部** | consumers 经 M7 registry 路径安装、零 compiler/Runtime 改动（行 59–61、184–185） | **闭合（已达成）** | **STEP-0588**（`runner/sico-runner/tests/package_roster_consumer.rs`：6 逐包消费 + 1 可重复 + 4 fail-closed，`#![test]` 共 11；**fail-closed 4/4 绿**，happy-path 当时被 `tables()=0` 阻塞）+ **STEP-0593**（table 限额修正：`lib.rs:1441` `tables() = package_instances_capacity`、:1412 `table_growing` 允许 `desired<=maximum` 且 `maximum==None` 拒绝、拒绝保持 `denied=true`；`--test package_roster_consumer` **11/11 全绿**：7 happy-path **逐字节 vs 冻结语料** + 4 fail-closed；回归 `package_consumer_forward` 2/2、`package_consumer_authority` 3/3、`sico-package` 51/51、runner `--lib` 42/42；证据 = registry 消费 internal-fixture + runtime-verified，包核 table 计数 = internal-fixture（四份 core `.wasm` 二进制解析：roster 三份各 1 个 funcref table `min==max==18`））。测试名覆盖三包：`image_match_template_*`/`image_grid_detect_lines_*`/`image_contour_find_*` | **口径备注（非阻塞）**：STEP-0593 触碰 runner `lib.rs` 限额段（Runtime 侧源码）。就「zero compiler/Runtime changes」条款，这是**资源记账 bug 修复**（恢复既有 M7 路径的预期资源模型，非语义变更；包仍是已编译 Component、消费仍即实例化）⇒ 按**非语义变更**口径满足；若 owner 按**字面**「任何 Runtime 源码触碰」读，须 owner 澄清（建议：本备注即为澄清请求，不影响 GO 判定） |
| | | | | **若 STEP-0596 落地**：`image-codec@1` registry 消费属 gate 5 侧（codec 包），**不属 gate 3**（gate 3 仅 roster 三包）——见 §4 与 §2 gate 5 |

**与 v1 相比：由「corpora 闭合 / registry 在途待刷新」变为「两半部闭合 ⇒ gate 3 聚合 = GO」**。
STEP-0588 + STEP-0593 把 registry 半部证据补齐（11/11 runtime-verified 逐字节），STEP-0593 自身声明
「gate 3 聚合裁决仍属出口审计，不翻转」，本审计据此作聚合裁决。这是**本次刷新最大的门级变化**。

### gate 4 — M17 显式审计（§3 行 62–63）

| 项 | 内容 |
|---|---|
| 门文本要点 | 显式 M17 审计：仅当所有必需 gate 有执行证据才 GO；否则保持 overall NO-GO + 具名残余登记（行 62–63） |
| **裁决** | **NO-GO（M17 overall）+ 具名残余登记**（本片执行显式审计动作本身） |
| 支撑证据 | gate 1（加速器）= NO-GO（E1，外部门）；gate 2（provenance）= NO-GO（C1/C2 + I3）；gate 3（roster）= **GO** ⇒ **M17 必需 gate 未全部有执行证据**（gate 1/2 未闭）⇒ M17 overall NO-GO。`image-vision@1`（STEP-0166）已绿但不覆盖 M17 其余门 |
| 具名残余 | E1（gate 1 加速器）+ gate 2 残余（C1/C2 owner 裁决 + I3 接线）——**gate 3 不再列残余**（已 GO） |
| 关闭条件 | gate 1 关闭 + gate 2 关闭 + 刷新显式 M17 审计（GO 或再登记 NO-GO + 残余）。**gate 1 未关前 M17 overall 不能翻 GO** |

**与 v1 相比：裁决不变（NO-GO），残余组成收窄**（gate 3 已 GO，从残余清单移除）。

### gate 5 — GUI 格式转换器 pilot（§3 行 64–70；§8.4 行 190–222；§8.6.2）

| 项 | 内容 |
|---|---|
| 门文本要点 | 转换器应用以 Sico 编写、基于 Sico 原生 UI 库，在 Windows evidence host 运行；第一步增量 **JPEG↔PNG 双向**在冻结语料上字节稳定；§8.5 fail-closed corpora 全绿；无静默 fallback、无 native escape、无核心 compiler/Runtime 补丁（行 64–70、204–222） |
| **裁决** | **NO-GO**（决定性残余 = E2 #3 JPEG 外部门；UI 子项：G1/G2/G3 已达成〔G2/G3 收窄口径〕、G4/G5 在途 STEP-0597/0595、G6 未达、文件对话框未做） |
| 支撑证据（已落地，登记值） | **PNG 半部**：`sico:user/image-codec@1` PNG 双向纯 package，digest-pinned Rust core，runner 内容测试 5/5（STEP-0331；`docs/evidence/m24/image-codec-support.json` class=rust-compiled-core）。**UI 库积木**：STEP-0338（Fluent-subset Direct2D 渲染器核心 + 冻结 token 表 + 4 场景冻结帧语料 5/5，`docs/evidence/m24/ui-frame-corpus.json` 4 个 `sha256` 帧）、STEP-0577（OS 窗口壳 + 事件泵，17/0/1）、STEP-0585（FFI 不变式固化：`PhantomData<*mut()>` + 编译期负约束 + Win32 失败路径；渲染器 20/0/2）、STEP-0592（布局 + 点→句柄命中测试：`crates/sico-ui-renderer-d2d/src/layout.rs:295` `compute`、:197 `hit`、:80 `Rect::contains` 半开区间；`layout_corpus.rs` 6 测试；**冻结帧/事件哈希逐字节不变**）、STEP-0581（runner `ui@1` 接真实 D2D 窗口后端：`D2dUiRenderer` 13 函数，**G1 达成**）、STEP-0591（guest 级 G2/G3：`runner/sico-runner/tests/ui_guest_visible.rs` `guest_run_presents_a_real_frame_byte_identical_to_offscreen_oracle`（G2）、`guest_poll_event_delivers_closed_after_host_close`（G3），3/3 绿）。**consumer 边界 typed-outcomes 半部**：`package_consumer_authority`/`package_consumer_forward`（§8.5「typed outcomes end to end」happy-path + fail-closed 半部） |
| 具名残余 | ① **E2 #3 JPEG 半部 = 外部门（决定性）**（225-crate 离线 cache 无 zune-jpeg/jpeg-encoder/image/mozjpeg/turbojpeg——本审计再核 `ls ~/.cargo/registry/cache/*/ | grep -iE 'jpe?g|zune'` = 空；**PNG→PNG 非有意义增量**，plan §8.4）；② **G4 在途**（STEP-0597：`Clicked{x,y}`→`compute`+`hit`→交付 `click(h)`；`Hit::Miss`⇒不交付并计入 `skipped_click_count`；`select` 语义不可确定则不交付）；③ **G5 在途**（STEP-0595：guest 级帧确定性专测）；④ **G6/I6 转换器 pilot 应用本体未编写**（clean-room-consumer 级）；⑤ **文件打开/保存对话框未做**（后续切片 #3）；⑥ §8.5 转换器级 fail-closed corpora（cancellation-mid-conversion / save-path-failure / 长程 batch 采样等）未整体绿；⑦ **C-A/C-B/C-C 合同缺口未裁**（[`RFC-0052-A2`](../rfc/RFC-0052-amendment-A2-ui-error-render-failed-and-event-visibility.md) PROPOSED：ui-error 无 render-failure 判别式 / `event-overflow` 对 guest 不可观测 / 点击·选择命中测试与**两套布局并存**的绘制统一需**重新冻结帧**） |
| 关闭条件 | ① owner 授权/引入 JPEG crate（或改口径——owner 门，**不得自造关闭路径**）；② G4 落地（STEP-0597）；③ G5 落地（STEP-0595）；④⑤⑥⑦ 内部实现（⑦ 需 owner 先裁 RFC-0052-A2 再实现）；随后整个应用在 Windows host 运行 + §8.5 corpora 全绿 + 字节稳定。**E2 未关前 gate 5 不能翻** |

**与 v1 相比：裁决不变（NO-GO），残余组成变化**——v1 标「UI 子项 STEP-0581 在途」；
现 STEP-0581 已落地（G1 达成）、G2/G3 已达成（收窄口径）、布局+命中测试已落地（STEP-0592，G4 铺路），
G4/G5 转在途（STEP-0597/0595），G6 仍未达、文件对话框仍未做；**E2 决定性残余不变**。

### gate 6 — 证据标签分离（§3 行 71–74；§6 行 116–121）

| 项 | 内容 |
|---|---|
| 门文本要点 | 证据标签保持 distinct：M17 corpora/refusal fixture = `internal-fixture`；pilot 应用 = `clean-room-consumer`；任何外部观察 owner-gated、不隐含于本里程碑（行 71–74） |
| **裁决** | **GO（纪律保持）** |
| 支撑证据 | 所有 M17/provenance/roster/UI 证据均标 `internal-fixture`（unit-level 或 runtime-verified，各 STEP header + `docs/evidence/m24/*.json` 的 `class`/`evidence-class` 字段逐一核对：image-match/grid/contour/codec 均 `"class": "rust-compiled-core"`）；pilot 未产生证据故**无** clean-room-consumer 标签可标（无混标风险）；无 external-pilot/production 声称（STEP-0333、各 STEP header、`not-claimed` 字段） |
| 具名残余 | 无（纪律已保持；待 pilot 落地时须以 `clean-room-consumer` 标 pilot 应用，不得与 internal-fixture 混标） |
| 关闭条件 | 审计时逐项核对标签与证据类一致（本片已核）；pilot 证据出现后标 `clean-room-consumer` |

**与 v1 相比：不变**（GO，纪律保持；本审计再核证据 JSON 标签）。

### gate 7 — 回归绿 + 显式分项裁决（§3 行 75–79）

| 项 | 内容 |
|---|---|
| 门文本要点 | M0–M22 + 已落地 M23 变更在 M24 审计 revision 回归绿；对 M17 gate 2、M17 gate 4/roster、原生 UI 库、转换器 pilot、M18 组合项增补给出显式、分项裁决（行 75–79） |
| **裁决** | **NO-GO**（外部门 E4/E5 + 内部 I7 未达；分项裁决由本审计交付） |
| 支撑证据（局部，登记值） | **局部回归绿**：STEP-0587（根工作区 `cargo check --offline --locked --workspace --all-targets` ⇒ 24 crate 全 Checking 无错）；STEP-0593（`package_roster_consumer` 11/11 + `package_consumer_forward` 2/2 + `package_consumer_authority` 3/3 + `sico-package` 51/51 + runner `--lib` 42/42 + `--no-run` exit 0 + clippy/fmt/diff-check 绿）；STEP-0592（renderer 22 lib〔21/1 ignored〕+ 5 event_corpus + 5 frame_corpus + 6 layout_corpus）；STEP-0591（`ui_guest_visible` 3/3 + `ui_authority` 7/7）；STEP-0585（renderer 20/0/2）。**M23 变更**：RFC-0048/0049（STEP-0328/0329）已落地 |
| 具名残余 | ① **I7 root 全量回归**（M0–M22 编译器/语言核心 + M23）在**干净、唯一、已提交候选 SHA** 上复核——无 gcc/binutils（E5）冷构建失败，且工作树含大量并行代理在途改动、无干净候选 SHA；② **E4 独立 CI 绿**：`dev` 刻意不在 push 集（`.github/workflows/m22-w1-ci.yml:24-27` `on.push.branches: [codex/m22-w2]` + `workflow_dispatch`，本审计再核）⇒ **owner 手动 dispatch**（外部门，本机无 `gh` CLI、不代用凭据）；③ 显式分项裁决（审计交付物，本片已作：见 §2 gate 1–5 + §3 §8.6.1） |
| 关闭条件 | ① 干净候选 SHA 上 root 全量回归绿；② owner 手动 dispatch 取得 `m22-w1-ci.yml` 独立 CI success（绑定精确 SHA）；③ 本审计分项裁决（已交付，不重复） |

**与 v1 相比：裁决不变（NO-GO）**；本地回归证据基底变广（新增 STEP-0585/0591/0592/0593 的各套绿值），
但**全部为登记值，本审计未复跑**（§6 诚实收窄），E4/E5/I7 不变。

## 3. §8.6.1 Sico 原生 UI 库工作流（行 252–316）— 能力分项裁决（独立于 gate 5）

| 能力 | 裁决 | 证据（STEP + 证据等级 + 入口） | 备注 |
|---|---|---|---|
| 渲染器核心（Fluent-subset Direct2D + 冻结 token 表 + 冻结帧语料） | **已落地** | STEP-0338，internal-fixture/**runtime-verified renderer-frame**（`crates/sico-ui-renderer-d2d`，`tests/frame_corpus.rs` 5/5，4 场景 SHA-256 跨两进程一致；`docs/evidence/m24/ui-frame-corpus.json` 4 个 `sha256`） | 帧哈希**证据主机局部**（DirectWrite 字形栅格化随 Windows 构建而异，换主机=重新冻结非回归） |
| OS 窗口壳 + 事件泵（`poll-event` 语义落地） | **已落地** | STEP-0577，internal-fixture/**runtime-verified window-shell**（`shell/win.rs` + `events.rs`，`tests/event_corpus.rs` 17 通过/0 失败/1 ignored；真实窗口冒烟 read-back SHA-256 == 离屏帧；`docs/evidence/m24/ui-event-corpus.json`） | **STEP-0585 已修复 v1 登记的 medium soundness 隐患**（见下） |
| FFI 不变式（`!Send`/`!Sync` 显式固化 + Win32 失败路径） | **已落地（v1 的 I8 已完成）** | STEP-0585，internal-fixture/**runtime-verified renderer**（`PhantomData<*mut()>` 零大小字段 + 手写 `assert_not_impl_any!` 编译期负约束 + `WinShellMarkerFreeMirror` 反证「标记是唯一维持 `!Send` 的因素」 + 独立 `rustc --emit=metadata` 探针；`destroy()`/`BeginPaint` 失败路径加固；渲染器 20/0/2） | **v1 §8.6.1「1 项 medium 潜伏 soundness」已消除**；不再依赖 `Dib.bits` 偶然字段 |
| 布局 + 点→句柄命中测试 | **已落地（G4 铺路）** | STEP-0592，internal-fixture（纯计算、平台中立；`layout.rs:295` `compute` 纯函数、:197 `hit`、:80 `Rect::contains` 半开区间；`layout_corpus.rs` 6 测试；**冻结帧/事件哈希逐字节不变**） | **暴露架构分叉**：渲染器绘制仍用旧布局（DirectWrite 度量 + 容器 own box 0×0），命中测试用新 token-proxy extent 布局 ⇒ **两套布局并存**；统一需**重新冻结帧**（受控审计动作，**需 owner 授权**——见 C-C，RFC-0052-A2） |
| runner 侧 `ui@1` 接真实 D2D 窗口后端（⇒ guest 可见 UI） | **已落地（G1 达成）** | STEP-0581（`crates/sico-ui-renderer-d2d/src/windowed.rs` `WindowedUiBackend` + `runner/sico-runner/src/ui.rs` `D2dUiRenderer` 13 函数；窗口壳置专用工作线程 + 请求/应答通道，**零 `unsafe impl`**；默认拒绝语义不变：无授权→typed `authority-denied`、无后端→不广告；渲染器 crate 17/0/2 + 真实窗口生命周期 0.22s + runner `--lib` 41 + `ui_authority` 7） | 本项是 §8.6.1「guest 可见 UI」的必要条件，**现已满足**（v1 时在途） |
| 文件打开/保存对话框 | **未做** | 后续切片 #3（M26 PDF 工具所需，ADR-0018 §4） | 复用既有文件能力 + 用户选定路径 |

### §8.6.1 G1–G6 逐项达成状态（STEP-0590 验收条款，**独立于 gate 5**）

> 判定条款来源：[`STEP-0590`](../steps/STEP-0590-m24-ui-contract-amendments-draft.md) ④
> （G1∧G2∧G3∧G4∧G5∧G6）。**收窄口径标注**（STEP-0591 主线裁决）：G2 = **guest 触发 + 宿主读回核验**
> （`ui@1` 无帧读回通道，合同缺口）；G3 = **guest 观测 + 宿主触发 close**（`ui@1` 无 guest 可见 `close`，合同缺口）。

| 条款 | 状态 | 证据 | 备注 |
|---|---|---|---|
| **G1** guest `window()` 创建真实 OS 窗口并观测 handle | **已达成** | STEP-0581（guest `window()` 经生产后端句柄观测到真实窗口） | 无收窄 |
| **G2** guest `run()` 呈现帧逐字节一致 | **已达成（收窄口径）** | STEP-0591（`ui_guest_visible.rs:1017` `guest_run_presents_a_real_frame_byte_identical_to_offscreen_oracle`：guest 触发 `run()` 后宿主读回 DIB 320×160 与独立离屏帧**逐字节相等**） | **收窄**：guest 读不到帧字节（`ui@1` 无读回通道）；「guest 自行观测」需 WIT 追加 ⇒ **owner** |
| **G3** guest `poll-event` 交付 `closed` | **已达成（收窄口径）** | STEP-0591（`ui_guest_visible.rs:1067` `guest_poll_event_delivers_closed_after_host_close`：宿主 `WM_CLOSE` 后 guest 自身 `poll-event` 交付 `closed`） | **收窄**：关闭是宿主/用户动作；「guest 触发 close」需 WIT 追加 ⇒ **owner** |
| **G4** guest `poll-event` 交付 `click(handle)`/`select` 且路由正确 | **在途待刷新** | STEP-0592 铺路（布局 + 命中测试就绪）+ **STEP-0597 在途**（runner 侧交付 `click(handle)`；`Hit::Miss`⇒不交付并计入 `skipped_click_count`；`Node(root)` 交付根句柄；`select` 若语义不可确定则不交付） | 本审计**不预裁**其成败 |
| **G5** guest 级帧确定性（同树 N 次 → N 帧逐字节一致） | **在途待刷新** | **STEP-0595 在途**（guest 级帧确定性专测） | 本审计**不预裁**其成败；STEP-0591 的跨实例字节一致只作**附带**证据，**不构成正式 G5** |
| **G6** clean-room pilot 应用 | **未达成** | 依赖 G4/G5 + 对话框 + E2 #3 JPEG + I6 应用本体 | clean-room-consumer 级，无证据 |
| **guest 可见 UI 能力（G1∧G2∧G3∧G4∧G5）** | **未达成** | 见上：G1∧G2∧G3 已达成（G2/G3 收窄），**G4/G5 在途** | 与 [`M24 plan §8.6.1`](../plans/M24-vision-closure-gui-application-pilot.md) 状态块一致（plan 已明示「Guest-visible UI is NOT yet achieved」） |

> **结论：guest 可见 UI 能力当前未达成**（G4/G5 在途 STEP-0597/0595，G6 未达）。v1 时「runner 接线在途」，
> 现 runner 接线已落地（G1），且 G2/G3 已按收窄口径达成——能力**显著推进但未整体达成**。
> **不翻任何 §3 gate**（§8.6.1 是 gate 5 的前置工作流，其达成 ≠ gate 5 GO）。
> **ADR-0018 状态两线登记不一致仍为外部门 E3**：dev 线按 STEP-0323 owner 指令记 **accepted**
> （`docs/adr/ADR-0018-*.md:3`「accepted by owner directive 2026-09-30 (STEP-0323)」，本审计再核），
> `codex/m22-w2` 线记 **proposed** ⇒ 按 AGENTS §1 **取下界**（不当已接受），差异留 owner 裁决。

## 4. gate 3 两半部分别记 + `image-codec@1` registry 消费状态

- **gate 3 corpora 半部**：**闭合**（STEP-0335/0336/0337，3/3，runtime-verified，独立 Python oracle，
  `PACKAGE-CORE reproducible`）。见 §2 gate 3 表。
- **gate 3 M7 registry 消费半部**：**闭合**（STEP-0588 fail-closed 4/4 + STEP-0593 11/11，runtime-verified
  逐字节 vs 冻结语料）。见 §2 gate 3 表。⇒ **gate 3 聚合 = GO**（带 §2 的「零 compiler/Runtime 变更」口径备注）。
- **`image-codec@1` registry 消费**（STEP-0596，**在途**，**不属 gate 3**）：STEP-0593 已把 `image-codec@1`
  登记为「修正的预期受益方，未在本片补断言」——其 core 带 1 个 funcref table（`min==max==71`），同被
  `tables()=0` 阻塞，修正后应可沿 M7 路径实例化，但导出形参（`decode-png` 1 参 / `encode-png` 3 参）与
  roster 测试辅助硬编码的 2 参不同、输入为 PNG 文件（非 A2 packed header）⇒ 补断言需扩展测试基础设施。
  现工作树已有 `runner/sico-runner/tests/package_image_codec_consumer.rs`（8 测试，文件头声明为「STEP-0593
  记录的『未在本片补断言』缺口之闭合」），STEP 文档未出 ⇒ **在途**。**若 STEP-0596 落地**：`image-codec@1`
  沿 M7 registry 路径被真实 consumer 消费（起点：冻结用例 `decode-ok-2x2-rgba`，期望
  `payload_hex = 020000000000000002000000000000001e140aff3c3228805a504600786e6440`），
  属 **gate 5**（转换器 codec 前置）而非 gate 3（gate 3 仅 roster 三包）。

## 5. 剩余工作总表

### 5.1 内部项（按可推进性排序，每项附 bounded exit test）

| 序 | 内部项 | 属 gate | bounded exit test | 可推进性 | 状态 |
|---|---|---|---|---|---|
| I1 | gate 5 UI：runner 侧交付 `click(handle)`（G4） | gate 5 / §8.6.1 | `Clicked{x,y}`→`compute`+`hit`→交付 `click(h)` 且路由正确；`Miss`⇒不交付并计入 `skipped_click_count`；`select` 语义不可确定则不交付并登记 | **最高**（布局+命中已就绪 STEP-0592） | **在途（STEP-0597）** |
| I2 | gate 5 UI：guest 级帧确定性专测（G5） | §8.6.1 / gate 5 | 同树 N 次 guest 运行 → N 帧逐字节一致（guest 级） | **高**（G2 框架已立） | **在途（STEP-0595）** |
| I3 | gate 5 codec：`image-codec@1` registry 消费 | gate 5 前置 | `image-codec@1` 经 M7 registry 路径被真实 consumer 消费；`decode-ok-2x2-rgba` 逐字节 | **高**（STEP-0593 已修限额，测试文件已在工作树） | **在途（STEP-0596）** |
| I4 | gate 2：C1/C2 修正案实现（待 owner 裁决后另开 STEP） | gate 2 | C1-a 语义修订（每 hostcall fuel 费用）/ C2-a install 拒绝；真实组件复测 | **中**（需 owner 裁决 RFC-0051-A2） | 未做（**待 C1/C2 裁决**） |
| I5 | gate 2：CLI/app 层接线（`run_program_with_budgets`/`provenance_budgets`） | gate 2 | `sico run`/app 路径真实执行携带预算的 `.sapp`；limit+1 typed | **中**（依赖 C1/C2 口径或并行；STEP-0578 §7） | 未做 |
| I6 | gate 5：§8.5 转换器级 fail-closed corpora | gate 5 | cancellation-mid-conversion / save-path-failure / 长程 batch 采样全部 typed，无 trap/hang/部分输出/静默 fallback | **中**（需 codec + UI 线） | 未做 |
| I7 | gate 5：文件打开/保存对话框 | gate 5 | 对话框复用既有文件能力 + 用户选定路径；typed save 失败 | **中**（后续切片 #3） | 未做 |
| I8 | gate 5：转换器 pilot 应用（clean-room-consumer） | gate 5 | 应用在 Windows host 运行、Fluent 渲染、冻结语料字节稳定、§8.5 corpora 全绿 | **低**（依赖 E2 #3 JPEG + I1/I2/I6/I7） | 未做 |
| I9 | gate 7：root 全量回归 + 显式分项裁决 | gate 7 | 干净候选 SHA 上 root 全量绿；审计输出分项裁决 | **低**（需无并发代理干扰 + 干净 SHA；CI 为外部门） | 回归未跑；**分项裁决本审计已交付** |
| I10 | §8.6.1 合同：C-A/C-B/C-C 修正案实现（待 owner 裁决 RFC-0052-A2） | §8.6.1 / gate 5 | `render-failed` 判别式 / `event-overflow` 可观测 / 两套布局统一（**重新冻结帧**，需 owner 授权） | **中**（需 owner 裁决） | 未做（STEP-0590 纯起草） |

### 5.2 外部项（owner 门 + 关闭条件；**不得自造关闭路径**）

| 序 | 外部项 | 属 gate | 现状证据（本审计再核） | 关闭条件 |
|---|---|---|---|---|
| E1 | **M17 gate 2 加速器**（具名硬件 + 加速器 ADR） | gate 1 / gate 4 | `docs/adr/` 无加速器 ADR（`grep -iE 'acceler'` = NONE）；无容差对比表；v1 gate 1 NO-GO | owner 指定具名加速器 host + 接受加速器 ADR + 容差一致性表 + crash/耗尽注入 |
| E2 | **#3 JPEG 编解码**（离线无 crate + 独立 oracle） | gate 5 | 225-crate 离线 cache 无 zune-jpeg/jpeg-encoder/image/mozjpeg/turbojpeg（`ls ~/.cargo/registry/cache/*/ \| grep -iE 'jpe?g\|zune'` = 空） | owner 授权引入 JPEG crate（离线可得）或改口径；否则 gate 5 step-1 不可达（PNG→PNG 非有意义增量） |
| E3 | **ADR-0018 状态两线登记不一致**（dev=accepted / m22-w2=proposed） | §8.6.1 合同 | `docs/adr/ADR-0018-*.md:3`（dev 线 accepted）；STATUS 2026-11-07 登记差异 | owner 裁决；未裁前**按下界**（不当已接受），不得作 accepted 使用 |
| E4 | **CI 触发面**（`dev` 不在 push 集，仅 `workflow_dispatch`） | gate 7 | `.github/workflows/m22-w1-ci.yml:24-27` `on.push.branches: [codex/m22-w2]` + `workflow_dispatch`（STEP-0566）；本机无 `gh` CLI、不代用凭据 | owner 手动 dispatch 或调整触发集，取得绑定精确 SHA 的独立 CI success |
| E5 | **本机无 gcc/binutils**（影响 root 冷构建与独立 CI 证据） | gate 7 | `which gcc`/`as`/`dlltool` 均无；已用 `windows` 0.52 预编译 import lib 规避 D2D 侧 | owner 授权安装工具链或改走预编译路径 |
| E6 | **C1/C2（RFC-0051-A2）owner 裁决** | gate 2 | `docs/rfc/RFC-0051-amendment-A2-*.md` status=PROPOSED；推荐 C1→(a)/C2→(a) | owner 裁决后另开实现 STEP（I4）+ I5 |
| E7 | **C-A/C-B/C-C（RFC-0052-A2）owner 裁决** | §8.6.1 / gate 5 | `docs/rfc/RFC-0052-amendment-A2-*.md` status=PROPOSED；C-C 统一布局需**重新冻结帧**（owner 授权） | owner 裁决后另开实现 STEP（I10） |

## 6. 诚实收窄（本审计不主张什么）

- 本审计**未运行任何 cargo 命令**：本机无 gcc/binutils（冷构建失败）+ 工作树含大量并行代理在途改动 ⇒
  证据以**既有 STEP 记录 + 文件/证据 JSON 只读核验**为准，**未独立重跑**任何测试
  （含 STEP-0587 的 4/4、STEP-0591 的 3/3、STEP-0592 的 6/6、STEP-0593 的 11/11、STEP-0577 的 17/0/1、
  STEP-0338 的 5/5、STEP-0578 的 51/51、STEP-0585 的 20/0/2——均引用登记值，非本审计现场复跑）。
- **未取得独立 CI**：本机无 `gh` CLI；`dev` 不在 push 集 ⇒ gate 7 的独立 CI 证据缺失（E4），如实登记为 owner 手动 dispatch 项。
- **未跑 root 全量回归**（E5 + 无干净候选 SHA + 在途工作树）⇒ gate 7 的「回归绿」仅以局部（runner 工作区 + 包侧）登记值呈现，不得读作 M0–M22 全量绿。
- **并行在途项未纳入**：STEP-0594/0595/0596/0597 未落地（STEP 文档未出；0596 的测试文件已在工作树但未验证绿）；
  gate 5 的 G4/G5 分项与 `image-codec@1` registry 消费**不预判其成败**，标「在途待刷新」。
- **M24 overall 不翻 full GO**：gate 1/5/7 存在外部门/owner 门（E1/E2/E4/E5），gate 2 存在未决内部门
  （C1/C2 + I5）与 owner 待裁（E6），gate 5 存在内部门（I1/I2/I6/I7/I8）与 owner 待裁（E7），gate 7 存在
  未跑回归（I9）⇒ **NO-GO**（不是 blocked-external-evidence——因为存在未达的内部门与 owner 待裁项，
  不纯是外部证据阻塞；且 gate 1 的 E1 与 gate 5 的 E2 是外部阻塞，但 gate 2/5/7 还有内部与待裁残余）。

## 7. 复核发现：与 v1（STEP-0589）/ STEP-0586 / STEP-0590 / STATUS 不一致或需修正之处

> 这是本审计最重要的交付物之一。下列每一项均为**审计者对既有登记的核对结果**，按重要性排序。

1. **v1「gate 3 registry 半部在途待刷新（STEP-0588）」已过时**：STEP-0588 已交付（fail-closed 4/4，happy-path 被
   `tables()=0` 阻塞）+ STEP-0593 已交付（table 限额修正，`package_roster_consumer` 11/11）。⇒ **gate 3 聚合 =
   GO**（两半部闭合）。STATUS/plan 相应行已更新（STATUS 2026-11-07 line 4/7；plan §8.6.1 ⑦），**v1 的「在途」标签
   需在下次 STATUS 刷新时更正为「已闭合」**。
2. **v1「gate 2 在途待刷新（STEP-0587）」已过时**：STEP-0587 已交付（五件逐条 ✓，`provider_crash_isolation` 4/4），
   **但** C1/C2（RFC-0051-A2）仍 PROPOSED、I3（CLI/app 接线）未做 ⇒ 本审计判 **NO-GO + 具名残余**，非「在途」。
   这是**从「在途」到「NO-GO」的判定变化**（不是门变好/变坏，而是 v1 因在途而未裁，现可裁且裁为 NO-GO）。
3. **v1「§8.6.1 runner 接线在途（STEP-0581）」已过时**：STEP-0581 已落地（G1 达成）。v1 的 I1 状态「在途」需更正为「已落地」。
4. **v1「I8 = `!Send+!Sync` 显式标记（soundness 修复）未做」已过时**：STEP-0585 已落地（`PhantomData<*mut()>` +
   编译期负约束 + Win32 失败路径；渲染器 20/0/2）。**v1 的 §8.6.1「1 项 medium 潜伏 soundness 隐患」已消除**。
5. **v1「§8.6.1 点→句柄命中测试 未做」已过时**：STEP-0592 已落地（`layout.rs` + `layout_corpus.rs` 6 测试），
   **但**交付 `click(handle)`（G4）仍由 STEP-0597 在途实现；且 STEP-0592 暴露**两套布局并存**架构分叉
   （渲染绘制用旧 DirectWrite 度量布局、命中测试用新 token-proxy extent 布局）——v1 不知此分叉，**新增**
   为需 owner 裁决项（C-C，RFC-0052-A2，统一需重新冻结帧）。
6. **G2/G3 收窄口径（v1 未条款化）**：STEP-0590 建立 G1–G6 验收条款，STEP-0591 依主线裁决把 G2 定为
   「guest 触发 + 宿主读回核验」（`ui@1` 无帧读回通道）、G3 定为「guest 观测 + 宿主触发 close」（`ui@1`
   无 guest 可见 `close`）。**v1 没有这套 G1–G6 框架**；v1 的「guest 可见 UI = 无能力」与 STEP-0581 的
   「部分达成」之分歧，现已由条款化消解。**这是 STEP-0590/0591 对 v1 口径的正式化，非矛盾**。
7. **STEP-0596 属 gate 5 而非 gate 3**：STEP-0593 把 `image-codec@1` 登记为「预期受益方、未补断言」，现
   `package_image_codec_consumer.rs`（8 测试）在途闭合。**该缺口不在 gate 3**（gate 3 仅 roster 三包，
   已闭合）；它在 **gate 5**（转换器 codec 前置）。若 STATUS/盘点把它列在 gate 3 侧须修正归属。
8. **「零 compiler/Runtime 变更」口径备注（新增）**：STEP-0593 触碰 runner `lib.rs` 限额段（Runtime 侧源码）。
   就 gate 3「without compiler/Runtime changes」条款，这是**资源记账 bug 修复**（非语义变更）——按非语义口径
   满足；若 owner 采严格字面（任何 Runtime 源码触碰），须 owner 澄清。**本审计不把它作阻塞残余**，但如实登记。
9. **v1「STEP-0584 oracle 补登记与 M24 门正交」仍成立**：STEP-0584 为 M22 侧数据项，本审计不涉及。
10. **无新提交**：HEAD 仍 `7bcdc35`（与 v1 相同），v1 后所有 M24 推进（STEP-0581…0593）均为**未提交工作树改动**。
    ⇒ 本审计是对**同一 dirty 快照**的刷新；**gate 7 的「干净候选 SHA」仍不存在**，I9 保持未跑。
11. **STATUS 2026-11-07 各条目与 plan §8.6.1 状态块与本次裁决一致**（二者均明示「guest 可见 UI NOT achieved」、
    G1 达成、G2/G3 收窄、G4/G5 在途、G6 未达、M24 overall NO-GO）——**无需修正**；仅 v1 审计自身的
    「在途」标签（见 1/2/3）需要后续刷新同步。

## 8. 结论

**M24 overall（本机）= NO-GO。** 逐门：gate 1 NO-GO（E1 加速器，外部门）、gate 2 NO-GO（C1/C2 owner 待裁 + I3 未接线）、
gate 3 **GO**（两半部闭合；带「零 compiler/Runtime 变更」口径备注）、gate 4 NO-GO（M17 整体 + 具名残余 E1 + C1/C2 + I3）、
gate 5 NO-GO（E2 #3 JPEG 外部门决定性；G1/G2/G3 已达成〔收窄〕、G4/G5 在途、G6 未达）、gate 6 GO（纪律保持）、
gate 7 NO-GO（E4/E5 外部门 + I9 未跑）。

**与 v1 相比的净变化**：gate 3 由「corpora 闭合 / registry 在途」翻为 **GO**；gate 2 由「在途」落为
**NO-GO + 具名残余**（STEP-0587 落地但 C1/C2/I3 未决）；gate 5 残余收窄（G1/G2/G3 已达成、G4/G5 转在途）；
§8.6.1 由「runner 接线在途 + 1 项 soundness 隐患」变为「runner 接线已落地 + soundness 已修复 + 布局/命中已落地」。

**不存在「本机无需任何 owner 裁决即可完成」的 M24 门**（与 v1、STEP-0586 底线判定一致）。本机唯一
「主要剩内部、仅一处待 owner 裁决」的门是 **gate 2（provenance）**——关闭需 owner 就 `RFC-0051-A2`（C1/C2）裁决 +
I5 接线；**gate 5 与 gate 1/4 在 E2/E1 关闭前无法翻 GO**；gate 3 为唯一新翻 GO 的门（带口径备注）；
gate 6 为唯一可直接判 GO 的门（纪律项）。

> 证据等级：`internal-fixture`；本机 Windows x64 GNU 真实 runner（Wasmtime）+（对既往登记值）仓库内冻结语料。
> `clean-room-consumer`、`external-pilot`、`production` 本审计均**不产生、不声称**。本报告**只新增本文件**，
> 未触碰任何被禁文件（STATUS/ROADMAP/steps-README/handoff-m22/AGENTS/AGENT_GOAL/plans/selfhost/源码/测试/v1 报告），
> 未 commit/push。

---

## 7. 附录：晚于本审计快照的变更（2026-11-07 晚，主线登记）

> 本审计快照之后又落地若干片；**本节只更新「事实与裁决依据」，不新增/撤销任何门的裁决**（裁决权仍属出口审计与 owner）。原 §1–§6 正文保留不动。

| 变更 | 对门的影响 |
|---|---|
| STEP-0599（`select` 可交付性判定 = **合同/模型缺口**：`ui-selection{handle,index}` 需「哪个 dropdown 的哪个索引」，但 dropdown 虽有 `options`（索引定义域存在），**模型无选中态字段、无设置选中的 op、无逐选项命中几何、shell 不产 select 事件** ⇒ guest 永不见 `select`；三案 [`RFC-0052-A3`](../rfc/RFC-0052-amendment-A3-ui-selection-model.md) **已于 2026-11-07 由 owner 裁决为 A1（合同收窄）**——A1 是 documentation-only 诚实收窄（零 WIT/零代码/零测试、wire 兼容），**`select` 仍不交付**） | **gate 5 前置**：§8.6.1 的 G4 由「待刷新」细化为 **G4a `click` ✓ / G4b `select` = 模型缺口**；A1 把该缺口由「guest 不可观测跳过」变为**声明式收窄**（`select` 已声明但不产出），**不是交付、不是 gate flip** ⇒ **G4b `select` 仍不可交付/未交付**；`select` 缺口**不是实现缺口**，不得以「再写一版实现」消解 |
| STEP-0597（G4a `click(handle)` 交付：生产路由 seam `route_click_event` + 只读 affordance `layout_input()`；最深命中⇒该句柄、`Node(root)`⇒根句柄且**仅在确实无子节点覆盖时**、`Miss`⇒跳过并计入 `skipped_click_count`、布局失败⇒类型化 `None`） | gate 5 前置推进；**证据口径**：「宿主投递坐标 → guest 经真实 runner 观测」，被旁路的仅是 OS `WM_LBUTTONDOWN`+`translate`（纯确定映射，已被冻结事件语料覆盖） |
| STEP-0591（G2/G3 guest 级证据，**收窄口径**：G2 = guest 触发 + 宿主读回核验；G3 = 宿主触发 close + guest 观测 `closed`）+ STEP-0595（G5 正式专测：N=6 独立代逐字节一致 + 冻结 `text` 场景锚定 + 三重跨进程一致） | §8.6.1 的 G2/G3/G5 由「在途」变为**已达成（收窄）**；**guest 可见 UI 能力仍未整体达成**（G4b + G6 缺） |
| STEP-0585（FFI 不变式**刻意固化**：`PhantomData<*mut ()>` + 编译期负约束 + 「无标记即 `Send+Sync`」的编译期反证；Win32 失败路径加固） | **消除 v1 登记的 medium soundness 隐患**（§2 能力分项的「带 1 项 medium 隐患」现已闭合） |
| STEP-0593（registry `tables()` 预算修正）+ STEP-0596（`image-codec@1` registry 消费 8/8） | **gate 3 维持 GO**（两半部闭合，registry 半部 11/11 + `image-codec` 8/8）；`image-codec@1` 属 gate 5 前置，其 registry 消费已**闭合** |
| STEP-0600（M22 燃料墙敏感度实测） | **不影响 M24 门**；为 M22 的 D8 提供可判定证据（2× 仅解 5/331 = 1.51% ⇒ 重基线不足） |
| STEP-0601（`selfhost_formatter` 断言口径修复，**由红转绿**；4 个同族失败经复跑确认同属 **IND-003 裁决面**，只登记不修） | **不影响 M24 门**；属 M22 语料/契约面（owner 待裁） |
| 主线实测：根工作区 `--no-fail-fast` 全量 **19 目标绿、4 个真实失败全为已登记的 IND-003 家族**；runner 全量真实失败 3 个目标（console-control 两例＝**本机缺 Python**，用仓库自带 `tools/ensure-python.ps1` 引导后 36/36 全绿，CI 侧无此问题；`selfhost_formatter` 已修；`selfhost_compiler` 182/5 ＝ m22-w2 线上**既知红**，随合并进入 `dev`，属 M22 出口阻塞项） | **gate 7**：本地回归分项由「登记值」升级为**实测值**；**本批次未引入任何新红测**；`tools/run-ci.ps1` 三步测试已加 `--no-fail-fast`（修掉「teardown flake 掩盖后续目标」的证据完整性缺陷） |

**门裁决不变**：gate 1 NO-GO、gate 2 NO-GO（+C1/C2 待裁 + I3 未接线）、**gate 3 GO**、gate 4 NO-GO、gate 5 NO-GO（E2 决定性 + G4b/G6 缺）、gate 6 GO、gate 7 NO-GO（E4/E5 + 无干净候选 SHA）⇒ **M24 overall 仍 NO-GO**。
