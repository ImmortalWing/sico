# STEP-0586: M24 出口审计输入 — §3 逐门证据盘点（只读，无 gate 裁决）

> - 片型: **纯只读证据盘点**（不改任何代码、不改任何权威文档、不改任何 gate 数值、不翻任何 gate）。本文件只做**盘点**，不做 GO/NO-GO 裁决——裁决权在出口审计与 owner（AGENTS §3「每门有证据且有显式 GO/NO-GO 审计才算完成」）。
> - 依据: [`docs/plans/M24-vision-closure-gui-application-pilot.md`](../plans/M24-vision-closure-gui-application-pilot.md) **§3**（逐门 gate 定义，行 49–79）与 **§8.6.1**（Sico 原生 UI 库工作流，行 252–316）。
> - 快照: 2026-11-07 worktree（HEAD `7bcdc35`，`dev` 分支；并行代理正在改 `runner/`、`crates/`，行号对 2026-11-07 快照成立，开工前须重新核对最新快照）。
> - 纪律: 每条断言带 `文件:行` 或 STEP 章节；以可执行证据为准，不凭计划原文推断实现状态（AGENTS §1.4）；文档与实现冲突时取下界。证据等级明确区分 `internal-fixture` / `clean-room-consumer` / `external-pilot` / `production`；**只有真正在已执行 runner 上跑过的才计 runtime 支持**；离线/合成评测不当 live-model。
> - 本片未运行任何 cargo 命令（本机无 gcc/binutils，冷构建会失败，且并发代理在编译）；仅以既有 STEP 记录与可复核入口文件为准。本片**只新增本文件**，未触碰任何被禁文件（STATUS/ROADMAP/steps-README/handoff-m22/AGENTS/AGENT_GOAL/plans/源码）。

## 1. §3 逐门盘点表

> 「门文本要点」列引用 [`M24 plan`](../plans/M24-vision-closure-gui-application-pilot.md) 原文行号；「当前已具备证据」列只列**可复核**的 STEP + 证据等级 + 入口文件/命令；「裁决」列本片**一律不写**（留出口审计与 owner）。

### gate 1 — M17 gate 2 加速器 provider ADR（§3 行 51–54；§8.1 行 139–158）

| 项 | 内容 |
|---|---|
| 门文本要点 | 加速器 provider ADR **accepted**；在 **≥1 台 Windows host** 上加速实现与 reference 实现**在声明容差内一致**（冻结语料）；provider crash/资源耗尽**隔离且 fail-closed**（行 51–54、146–153）。§8.1 记录延宕子句：无授权加速器 ⇒ gate 2 **保持 NO-GO/deferred**，reference-only 证据**不翻 gate 2**（行 154–158）。 |
| 当前已具备证据 | **无加速器 ADR 文件**（`docs/adr/` 无 accelerator/加速器 ADR；唯一相关为 ADR-0019 Rust-compiled-package-core）。`image-vision@1`（gate-1 前置，STEP-0166，M17 gate 1 GO）已绿，但不覆盖本 gate。**无任何加速-vs-reference 容差对比表、无 crash/资源耗尽注入证据**。 |
| 仍缺什么 | ① 加速器 provider ADR（accepted）；② ≥1 台 Windows host 上的参考-vs-加速容差一致性证据（冻结 `image-vision@1` 语料）；③ provider crash/资源耗尽注入 + fail-closed + 宿主免重启证据。 |
| 归属 | **外部门**（具名硬件 + owner 需决 ADR）。 |
| 关闭条件 | owner 授权并指定具名加速器硬件 + 接受加速器 ADR + 在该 host 跑出容差一致性表与 crash/耗尽注入证据。 |

### gate 2 — M17 gate 4 model provenance（§3 行 55–58；§8.2 行 160–177）

| 项 | 内容 |
|---|---|
| 门文本要点 | model RFC **accepted**；一个 model-consuming 路径（可用 fixture model）证明五件套：**digest-verified load**、**budget 强制（limit+1 typed）**、**cancellation mid-inference**、**malicious-asset 拒绝**、**provider-crash 隔离免宿主重启**（行 55–58、171–175）。 |
| 当前已具备证据 | **RFC-0051 accepted + A1 accepted**（STEP-0323/2026-09-30 owner 裁决；`docs/rfc/RFC-0051-*`）。**digest-verified load**：`crates/sico-package/src/provenance.rs`（`verify_asset` 三态 distinct：`LengthMismatch`/`Truncated`/`DigestMismatch`）+ `tests/provenance_fixture.rs` 6/6 绿（STEP-0334，internal-fixture/**unit-level**）。**budget 强制**：`VerifiedPackage::provenance_budgets` + `Runner::run_program_with_budgets` + `RunnerLimits::narrowed_by`（STEP-0578，runner 6 绿 / sico-package 51 绿，internal-fixture，**runtime-verified**——memory/wall/fuel/取消 4 态 + 端到端 `MemoryLimit`）。**cancellation mid-inference**：STEP-0578 测试 5（自旋 guest + `CancelToken` → typed `Cancelled`，runtime-verified）。**malicious-asset 拒绝**：`provenance_fixture` 未知字段结构拒 + `malformed-unknown-field.provenance.json`（internal-fixture/unit-level）。 |
| 仍缺什么 | ① **C1** hostcall 预算运行期不生效 + 单位不匹配（`max_hostcalls` 计数 vs `hostcall_fuel` 每调用费用；`invoke`/`start_debug` 64 MiB 地板覆盖）；② **C2** 真实 Script 组件 `SCRIPT_MEMORY_BYTES`≈2.06 GiB、1 MiB 预算运行期不透明 trap（STEP-0578 §6，草案见 STEP-0583 / `RFC-0051-amendment-A2`，**PROPOSED 未裁决**）；③ **provider-crash 隔离免宿主重启**注入测试（**未实现**）；④ `provenance_budgets`/`run_program_with_budgets` **未接入 CLI/app 层**（`sico-cli`/`sico-app-cli`，STEP-0578 §7 后续接线点）。 |
| 归属 | **内部门为主**（可离线建）：③ provider-crash 注入 + ④ CLI/app 接线均可离线推进；① ② 需 **owner 裁决**（C1/C2 修正案 PROPOSED）。 |
| 关闭条件 | ① ② owner 就 `RFC-0051-amendment-A2` 裁决（推荐 C1-a/C2-a）并另开实现 STEP；③ ④ 实现 + 真实验证。**C1/C2 未裁前不得宣称运行期强制完整**。 |

### gate 3 — Roster 三包 deterministic corpora（§3 行 59–61；§8.3 行 179–188）

| 项 | 内容 |
|---|---|
| 门文本要点 | 三包（template match/grid/contour）**deterministic corpora 字节精确 vs reference**；**consumers 经 M7 registry 路径安装**、零 compiler/Runtime 改动（行 59–61、180–185）。 |
| 当前已具备证据 | **三包 3/3 全绿**（STEP-0335 `image-match@1`、STEP-0336 `image-grid@1`、STEP-0337 `image-contour@1`，全部 **internal-fixture / runtime-verified package-Component 级**）：独立 oracle（纯 stdlib Python，与 Rust core 无共享代码）内容语料逐元素字节精确 + RFC-0041 可达 `image-error` 闭集逐 case 类型化拒绝 + fresh-store 字节稳定；`tools/validate-package-core.ps1` → **PACKAGE-CORE VERDICT: reproducible**（clean 重建字节全等）；**零 compiler/Runtime 改动**（`build_package_component` 路线）。契约：RFC-0043 + A1（region→16B packed Bytes）+ A2（bitmap 输入 header），owner 2026-10-01 复裁决接受。 |
| 仍缺什么 | **consumers 经 M7 registry 路径安装**：机制本身已在 runner 边界证明（`tests/package_consumer_authority.rs` 3/3 + `package_consumer_forward.rs` 2/2，`link_packages`/`prepare_package` 路径，但用 `csv@1.0.0`）；**roster 三包自身的 M7 安装/消费路径未单独演示**——`sico-lock.json`（M7 lock）条目随首个消费该包的试点（#6，转换器 pilot）落地，本片以 artifact 形式交付（STEP-0335:96、0337:95）。 |
| 归属 | corpora 半部**内部门（已完成）**；M7 registry 消费半部**内部门（可离线推进，但实际与 gate 5 试点联动）**。 |
| 关闭条件 | 三包 corpora 字节精确（✓）；补充 roster 三包经 M7 registry 路径被真实 consumer 安装 + 消费的演示（可随 gate 5 试点一并交付）。 |

### gate 4 — M17 显式审计（§3 行 62–63）

| 项 | 内容 |
|---|---|
| 门文本要点 | 显式 M17 审计：仅当所有必需 gate 有执行证据才 GO；否则保持 overall NO-GO + 具名残余登记（行 62–63）。 |
| 当前已具备证据 | STEP-0333（M24 §3 出口审计，2026-09-30）已按「本机当前状态」逐门裁决并出具具名残余登记；gate 4 当时裁决 = **整体 NO-GO + 具名残余**。此属**审计动作本身**，不是 gate 翻转。 |
| 仍缺什么 | M17 gate 2（= §3 gate 1，外部门）未达 ⇒ M17 overall 保持 NO-GO；gate 4 的「显式审计」在 gate 1 关闭前只能再出 NO-GO + 残余。 |
| 归属 | **外部门**（随 gate 1 加速器）。 |
| 关闭条件 | gate 1（加速器）关闭 + 刷新显式 M17 审计（GO 或再登记 NO-GO + 残余）。 |

### gate 5 — GUI 格式转换器 pilot（§3 行 64–70；§8.4 行 190–222；§8.6.2 行 318–338）

| 项 | 内容 |
|---|---|
| 门文本要点 | 转换器应用**以 Sico 编写、基于 Sico 原生 UI 库**，在 Windows evidence host 运行；第一步增量 **JPEG↔PNG 双向**在冻结语料上**字节稳定**；§8.5 fail-closed corpora 全绿；无静默 fallback、无 native escape、无核心 compiler/Runtime 补丁（行 64–70、204–222、318–330）。 |
| 当前已具备证据 | **PNG 半部**：`sico:user/image-codec@1` PNG 双向纯 package，digest-pinned Rust core，runner 内容测试 5/5，clean 重建 byte-identical（STEP-0331，internal-fixture/runtime-verified）。**UI 库积木**：STEP-0338（Fluent-subset Direct2D 渲染器核心 + 冻结 token 表 + 4 场景冻结帧语料，5/5，internal-fixture/runtime-verified renderer-frame）、STEP-0577（OS 窗口壳 + 事件泵 + 冻结事件语料，17 通过/0 失败/1 ignored，真实窗口冒烟 0.22s read-back SHA-256 与离屏帧相同，internal-fixture/runtime-verified window-shell）。**consumer 边界 typed-outcomes 半部**：`package_consumer_authority`/`package_consumer_forward`（§8.5「typed outcomes end to end」happy-path 半部 + fail-closed 半部）。 |
| 仍缺什么 | ① **JPEG 半部 = 外部门**（#3：离线无 JPEG 编解码 crate + 独立 oracle；225-crate cache 仅 `png`，无 zune-jpeg/jpeg-encoder/image/mozjpeg/turbojpeg）；② **UI 库不完整**：runner 侧 `ui@1` 接线（⇒ guest 可见 UI）**未做**（STEP-0577 后续切片 #1，**由并行代理 STEP-0581 在做**）；点→句柄命中测试（`click(handle)`/`select`）未做（需渲染器布局）；文件打开/保存对话框未做；③ **转换器应用本体未编写**（clean-room-consumer 级）；④ §8.5 转换器级 fail-closed corpora（cancellation-mid-conversion/save-path-failure/batch 等）未整体绿。 |
| 归属 | ② ③ ④ **内部门**（可离线推进，② 为 STEP-0581 在途）；① **外部门**（#3 JPEG）。 |
| 关闭条件 | ① owner 授权/引入 JPEG crate（或改口径）；② ③ ④ 内部实现；随后整个应用在 Windows host 运行 + §8.5 corpora 全绿 + 字节稳定。**PNG→PNG 非有意义增量，JPEG 关闭前 gate 5 不能翻**（plan §8.4）。 |

### gate 6 — 证据标签分离（§3 行 71–74；§6 行 116–121）

| 项 | 内容 |
|---|---|
| 门文本要点 | 证据标签保持 distinct：M17 corpora/refusal fixture = `internal-fixture`；pilot 应用 = `clean-room-consumer`；任何外部观察 owner-gated、不隐含于本里程碑（行 71–74）。 |
| 当前已具备证据 | **纪律保持**：所有 M17/provenance/roster/UI 证据均标 `internal-fixture`（unit-level 或 runtime-verified）；pilot 未产生证据故**无** clean-room-consumer 标签可标；无 external-pilot/production 声称（STEP-0333、各 STEP header）。 |
| 仍缺什么 | 无（纪律已保持；待 pilot 落地时须以 `clean-room-consumer` 标 pilot 应用，不得与 internal-fixture 混标）。 |
| 归属 | **内部门（纪律项）**。 |
| 关闭条件 | 审计时逐项核对标签与证据类一致；pilot 证据出现后标 `clean-room-consumer`。 |

### gate 7 — 回归绿 + 显式分项裁决（§3 行 75–79）

| 项 | 内容 |
|---|---|
| 门文本要点 | M0–M22 + 已落地 M23 变更在 M24 审计 revision 回归绿；对 M17 gate 2、M17 gate 4/roster、原生 UI 库、转换器 pilot、M18 组合项增补给出**显式、分项**裁决（行 75–79）。 |
| 当前已具备证据 | **局部回归绿**：runner 工作区全量单线程全绿（STEP-0333 Addendum C：41 lib + 30 套件 0 failed；STEP-0338 工作区 `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0）；`sico-package` 51/51（STEP-0578）。**M23 变更**：RFC-0048/0049（STEP-0328/0329）已落地。 |
| 仍缺什么 | ① **root 全量回归**（M0–M22 编译器/语言核心 + M23）在**干净、唯一、已提交候选 SHA** 上复核（本机无 gcc/binutils，冷构建失败；且并行代理在改 `runner/`、`crates/`，回归状态易变）；② **独立 CI 绿**：`dev` **刻意不在 push 集**（`.github/workflows/m22-w1-ci.yml` 注释，STEP-0566 触发重定向），仅 `workflow_dispatch` 手动 ⇒ **owner 手动 dispatch**（外部门）；③ 显式分项裁决（审计交付物，本片不代作）。 |
| 归属 | ① **内部门**（可离线推进，但需干净快照 + 无并发代理干扰）；② **外部门**（CI 触发面 = owner 手动 dispatch）；③ 内部门（审计动作）。 |
| 关闭条件 | ① 干净候选 SHA 上 root 全量回归绿；② owner 手动 dispatch 取得 `m22-w1-ci.yml` 独立 CI success（绑定精确 SHA）；③ 审计输出显式分项裁决。 |

## 2. §8.6.1 Sico 原生 UI 库工作流（行 252–316）— 能力盘点

| 能力 | 状态 | 证据（STEP + 证据等级 + 入口） |
|---|---|---|
| 渲染器核心（Fluent-subset Direct2D + 冻结 token 表 + 冻结帧语料） | **已落地** | STEP-0338，internal-fixture/**runtime-verified renderer-frame**（`crates/sico-ui-renderer-d2d`，`tests/frame_corpus.rs` 5/5，4 场景 SHA-256 跨两进程一致；`docs/evidence/m24/ui-frame-corpus.json`） |
| OS 窗口壳 + 事件泵（`poll-event` 语义落地） | **已落地** | STEP-0577，internal-fixture/**runtime-verified window-shell**（`shell/win.rs` + `events.rs`，`tests/event_corpus.rs` 17 通过/0 失败/1 ignored；真实窗口冒烟 read-back SHA-256 == 离屏帧；`docs/evidence/m24/ui-event-corpus.json`） |
| **runner 侧 `ui@1` 接真实 D2D 窗口后端 ⇒ guest 可见 UI** | **未做（在途）** | **并行代理 STEP-0581 在做**（STEP-0577 后续切片 #1；无 STEP-0581 文件已落地）。`ui@1` host-provider 绑定 + 默认拒绝授权层已落地（STEP-0332，`runner/sico-runner/src/ui.rs` + `tests/ui_authority.rs` 4/4），但无后端时**不广告** ⇒ 无 guest 可见 UI。 |
| 点→句柄命中测试（`click(handle)`/`select`） | **未做** | 需渲染器布局（STEP-0577 后续切片 #2）。 |
| 文件打开/保存对话框 | **未做** | 后续切片 #3（M26 PDF 工具所需，ADR-0018 §4）。 |
| 转换器 pilot 应用（clean-room-consumer） | **未做** | §8.6.2 行 326–330 bounded exit test；依赖上面 UI 线 + #3 JPEG。 |

> §8.6.1 诚实边界（STEP-0338/0577 声明）：本工作流**不翻任何 §3 gate**；「无 guest 可见 UI 能力」是当前真实状态（渲染器 + 窗口壳 + 事件泵三件套齐备 ≠ 可见 UI；缺 runner 接线）。帧哈希**证据主机局部**（DirectWrite 字形栅格化随 Windows 构建而异，换主机=重新冻结非回归）。**ADR-0018 状态两线登记不一致**：dev 线按 STEP-0323 owner 指令记 **accepted**，`codex/m22-w2` 线记 **proposed** ⇒ 按 AGENTS §1 **取下界**（不当已接受），差异留 owner 裁决（外部门项）。

## 3. 内部门清单（按可推进性排序，每项附 bounded exit test）

| 序 | 内部门 | 属 gate | bounded exit test | 可推进性 |
|---|---|---|---|---|
| I1 | runner 侧 `ui@1` 接真实 D2D 窗口后端（STEP-0581，在途） | §8.6.1 / gate 5 前置 | `ui@1` 面各操作经真实 runner + 真实窗口后端实例化成功；`poll-event` 经事件泵交付 `Closed/click`；受权渲染产生与离屏帧相同 SHA-256；无后端时仍类型化拒绝不广告 | **最高**（并行代理在做） |
| I2 | gate 2 剩余：provider-crash 隔离免宿主重启注入测试 | gate 2 | fixture-model 路径注入 provider crash/资源耗尽 → typed outcome + 宿主进程存活、Store 可复用 | **高**（纯 internal-fixture，无需裁决） |
| I3 | gate 2 剩余：CLI/app 层接线（`run_program_with_budgets`/`provenance_budgets`） | gate 2 | `sico run`/app 路径真实执行携带预算的 `.sapp`；limit+1 typed | **高**（依赖 C1/C2 口径或并行；STEP-0578 §7） |
| I4 | gate 3 剩余：roster 三包 M7 registry 安装 + 消费演示 | gate 3 | 三包经 `link_packages`/`prepare_package` 被真实 consumer 导入 + 调用取回字节 | **中**（可离线，但与 pilot 联动；机制已用 csv 证明） |
| I5 | §8.6.1 点→句柄命中测试（`click(handle)`/`select`） | §8.6.1 / gate 5 | 真实渲染布局上点击坐标 → 正确 handle；越界 → typed | **中**（需渲染器布局） |
| I6 | gate 5 转换器 pilot 应用（Sico 编写） | gate 5 | 应用在 Windows host 运行、Fluent 渲染、冻结语料字节稳定、§8.5 corpora 全绿 | **低**（依赖 #3 JPEG + I1/I5） |
| I7 | gate 7 root 全量回归 + 显式分项裁决 | gate 7 | 干净候选 SHA 上 root 全量绿；审计输出分项裁决 | **低**（需无并发代理干扰；CI 为外部门） |

## 4. 外部门清单与关闭条件

| 序 | 外部门 | 属 gate | 现状证据 | 关闭条件 |
|---|---|---|---|---|
| E1 | **M17 gate 2 加速器**（具名硬件 + 加速器 ADR） | gate 1 / gate 4 | `docs/adr/` 无加速器 ADR；无容差对比表；STEP-0333 gate 1 NO-GO | owner 指定具名加速器 host + 接受加速器 ADR + 容差一致性表 + crash/耗尽注入 |
| E2 | **#3 JPEG 编解码**（离线无 crate + 独立 oracle） | gate 5 | STEP-0330 §4 / STEP-0333 Addendum A：225-crate cache 仅 `png-0.18.1` | owner 授权引入 JPEG crate（离线可得）或改口径；否则 gate 5 step-1 不可达（PNG→PNG 非有意义增量） |
| E3 | **ADR-0018 状态两线登记不一致**（dev=accepted / m22-w2=proposed） | §8.6.1 合同 | `docs/adr/ADR-0018-*.md:3`（dev 线 accepted）；STATUS 2026-11-07 登记差异 | owner 裁决；未裁前**按下界**（不当已接受），不得作 accepted 使用 |
| E4 | **CI 触发面**（`dev` 不在 push 集，仅 `workflow_dispatch`） | gate 7 | `.github/workflows/m22-w1-ci.yml` 注释（STEP-0566）；本机无 `gh` CLI、不代用凭据 | owner 手动 dispatch 或调整触发集，取得绑定精确 SHA 的独立 CI success |
| E5 | **本机无 gcc/binutils**（影响 root 冷构建与独立 CI 证据） | gate 7 | STEP-0338 §3（`dlltool`/`as` 缺失） | owner 授权安装工具链或改走预编译路径（Windows GNU 已用预编译 import lib 规避） |

## 5. 若要申请 M24 出口审计，还差哪几项

**结论：M24 出口审计可作为「审计-of-current-state（NO-GO + 具名残余）」随时申请（STEP-0333 即此形态）；但要申请「full GO 审计」则仍差外部 + 内部两类，且外部项决定审计只能是 NO-GO。**

### 还差的内部项（可在本机离线推进）
1. **I1  runner `ui@1` 接线 → guest 可见 UI**（STEP-0581 在途；§8.6.1 诚实边界）。
2. **I2  gate 2 provider-crash 隔离注入测试**（五件套最后一件）。
3. **I3  gate 2 CLI/app 层接线**（`run_program_with_budgets` 未接入 CLI）。
4. **I4  gate 3 roster M7 registry 安装/消费演示**（或随 pilot 一并交付）。
5. **I5  §8.6.1 点→句柄命中测试**（gate 5 的 UI 交互所需）。
6. **I6  gate 5 转换器 pilot 应用本体**（clean-room-consumer，依赖 E2）。
7. **I7  gate 7 root 全量回归 + 显式分项裁决**（干净 SHA 复核）。

### 还差的外部项（决定审计等级；全部需 owner）
1. **E1  M17 gate 2 加速器**（gate 1 / gate 4）——无硬件 + 无 ADR。
2. **E2  #3 JPEG**（gate 5 step-1）——无离线 crate。
3. **E3  ADR-0018 状态裁决**（§8.6.1 合同口径）。
4. **E4  CI 触发面**（gate 7 独立 CI 绿）——owner 手动 dispatch。
5. **E5  gcc/binutils 工具链**（gate 7 root 冷构建）。

### 底线判定
- **gate 2（provenance）是当前唯一「主要剩内部、仅一处待 owner 裁决」的门**：关闭它需 owner 就 `RFC-0051-amendment-A2`（C1/C2）裁决 + I2 + I3；C1/C2 未裁前不得宣称运行期强制完整（STEP-0578/0583）。
- **gate 3 corpora 半部已闭环**（3/3 runtime-verified）；仅剩 M7 registry 消费演示（I4）。
- **gate 5 与 gate 1/4 在外部项（E2 / E1）关闭前无法翻 GO**；§8.6.1 的「guest 可见 UI」是 gate 5 的必要非充分条件。
- **不存在「本机无需任何 owner 裁决即可完成」的 M24 门**（与 STEP-0333 Addendum B 结论一致，且现况因 STEP-0334/0578/0577 而更接近：provenance 已到 C1/C2 待裁 + 两项内部门；UI 已到 runner 接线 + 命中测试）。

### 对出口审计的建议输入
- 审计应按 §3 gate 1–7 逐门给出 **GO/NO-GO + 具名残余**，不得以「证据清单齐备」代替裁决；本片对任何门**未给裁决**。
- 审计需把 **§8.6.1「guest 可见 UI」** 与 **gate 5 转换器 pilot** 作为**两个独立分项**（§3 gate 7 明确要求对「原生 UI 库」与「转换器 pilot」分别裁决）。
- 审计需把 **gate 3 corpora（✓）** 与 **gate 3 M7 registry 消费（I4）** 分开记；不得以 corpora 全绿代 registry 消费证据。
