# Sico repository agent guide

This file applies to the entire repository. It is the concise operating guide for coding agents; [`AGENT_GOAL.md`](./AGENT_GOAL.md) remains the long-form product goal.

## 1. Read before changing code

1. Run `git status --short` and preserve every pre-existing modification or untracked file.
2. Read [`docs/STATUS.md`](./docs/STATUS.md), the relevant section of [`docs/ROADMAP.md`](./docs/ROADMAP.md), and the matching file under [`docs/plans/`](./docs/plans/README.md).
3. Read the accepted RFC/ADR and the latest STEP for the subsystem being changed. A plan is not an implementation contract when a later accepted RFC/ADR says otherwise.
4. Inspect the actual compiler/Runtime/Host behavior and tests. Do not infer implementation support from design documents or successful `sico check` alone.

If documentation disagrees with executable evidence, keep the lower support claim, record the discrepancy, and repair the authoritative documents in the same scoped change.

## 2. Current roadmap boundary

- M0–M13 全部 complete：M12 于 STEP-0127（2026-09-05）获得完整 GO（10/10 gates，Windows x64 与 Linux x64 原生运行时证据）；M13 已收口（STEP-0123 出口审计；STEP-0128 权威 live-model 评测 0.9095，DeepSeek deepseek-chat，2880 次真实调用、$0.78，达标 ADR-0012；既定缺口 B-repair 0.833 已登记跟踪）。
- M14 GO（STEP-0141 出口审计）：应用 profile 无未声明 check/build/run 缺口；gate 7 记 blocked-external-evidence。
- M15 GO（7/7，STEP-0158/0162）。
- M16 GO（7/7，STEP-0159，Windows 真实路径 GDI PrintWindow + SendInput 闭环实证）。
- M17 NO-GO：gate 1 GO（STEP-0166），gate 2/4 待 M24。
- M18 GO 4/5（STEP-0164/0165）：组合项 4 按 STEP-0258 重定为 GUI 格式转换器，待 M24。
- M19 GO（部分）；M20 NO-GO（部分）；M21 GO。
- M22 编译器自举轨 in progress / 出口 NO-GO：主动线 `codex/m22-w2` **已于 2026-11-06 按 owner「拉取 gitcode 远程」指令并入 `dev`（本 worktree，merge commit `7bcdc35`）**。S1/S2 complete，S3/S4/S5 partial，S6 not entered，S7 已按 STEP-0310 复审并 NO-GO（[出口审计](./docs/reports/m22-exit-audit-2026-09-28.md)）；R0 已拍板 Option A（ADR-0017 accepted），records v0 已封（STEP-0271..0275）；DIVERGENCE_WAIVER 为 0（235 条语料全 strict）。最新执行状态（STEP-0571–0573、B3/B4 负结果、fuel 墙登记）**只以 [`docs/STATUS.md`](./docs/STATUS.md) 与 [`M22 计划`](./docs/plans/M22-compiler-self-host.md) 为准**，本文件不复制这些每步易变的数字。
- M23 language v1 batch 3：项 1/2 已落地（RFC-0048/RFC-0049，STEP-0328/0329），出口审计 item 1 GO / item 2 GO / item 3 deferred-external / item 4 closed（[M23 审计](./docs/reports/m23-exit-audit-2026-09-30.md)）；item 3 的 owner 凭据语料复测未关闭，不得读作 M23 整体 GO。
- M24 vision closure + GUI 格式转换器（含 Sico 原生 UI 库工作流）：开工盘点与合同已登记（STEP-0287..0290）；实现切片 STEP-0330~0338 已落（合同复裁、#2 PNG、`#5 ui@1` 绑定+默认拒绝授权、出口审计、gate 4 provenance schema、gate 3 roster 三包全绿、**§8.6.1 Fluent-subset Direct2D 渲染器 + 冻结 token 表 + 确定性帧语料（STEP-0338）**）。#4 Direct2D 的「本机不可用」前提已实测推翻（须用完整 `windows` crate 钉 0.52 走预编译 import lib；DIB 须 `BITMAPV5HEADER`+`BI_BITFIELDS`+BGRA mask，像素格式须 `B8G8R8A8_UNORM`+`PREMULTIPLIED`）。**但仍只是渲染器核心：OS 窗口壳/事件泵/runner 接线未做 ⇒ 无 guest 可见 UI 能力，无任何 §3 gate 翻转**；M24 overall 本机仍 NO-GO（#3 JPEG 与 gate 1 加速器仍为具名外部门，gate 4 余「预算运行期强制+取消」）。**ADR-0018 状态两线登记不一致**（dev 线 STEP-0323 按 owner 指令裁决 accepted；m22-w2 线登记「仍为 proposed，待 owner 接受」）——按 AGENTS §1 取下界声称（不当已接受），差异留 owner 裁决。
- M25 release v1.0 completion audit、M26 PDF 文档处理（排在 M25 后）：仅有计划与 owner 指令登记（2026-09-17/22/24；RFC-0053/0054 已按 STEP-0323 指令裁决 accepted），无实现 STEP 号。

任何阶段的实现开工与完成声明，都不得早于其入口门槛被证据满足之时；合同或语料准备可以先期提出，但不得包装为特性支持。M12 的 "Automation SDK" 仅指 secure HTTP/API automation，不含屏幕、窗口或输入自动化。

## 3. Milestone and evidence discipline

- Non-trivial implementation work needs one unique STEP, a reproducible validator, evidence, and synchronized STATUS/ROADMAP/plan updates.
- Freeze language or WIT contracts through an RFC before implementation. Freeze cross-component/platform architecture through an ADR before implementation.
- Never allocate a future STEP merely to make the roadmap look scheduled. Allocate it when the entry gate is satisfied and the work has a bounded exit test.
- A milestone is complete only when every exit gate has evidence and an explicit GO/NO-GO audit.
- Keep evidence labels distinct: `internal-fixture`, `clean-room-consumer`, `external-pilot`, and `production` are not interchangeable.
- Only claim runtime/platform support on an actually executed native or browser runner. Contract tests are `contract-verified`, not runtime support.
- Offline or synthetic AI evaluation is not a live-model result. Record model, version, prompts, corpus, token/cost budget and raw run evidence for every live-model claim.

## 4. Architecture boundaries

- `sico` owns source checking, formatting, outlines and compilation. It must not own application trust, Host policy or platform implementations.
- `sico-app` owns package/development execution orchestration and must not silently reinterpret source semantics.
- Compiler crates may emit typed IR, Components, bindings and debug identity; they must not depend on runner, TLS, DNS, credentials, DAP or platform adapters.
- Runtime/runner owns Store lifetime, limits, scheduling, cancellation and typed execution outcomes.
- Host providers own external effects such as storage, HTTP, UI, capture, input and model acceleration. Authority is explicit, least-privilege, versioned and default-deny.
- Standard-library helpers may narrow or compose authority but may never widen or bypass the underlying provider.
- Native FFI belongs in the smallest isolated adapter/provider crate. Keep compiler and shared security cores free of platform-specific unsafe code.

M16 已交付（GO），但能力边界仍然有效：capture 与 input 是分离授权，均不隐含全桌面访问、剪贴板、凭据、进程控制或键盘访问；必需闭环保持 `observe -> plan -> preview -> execute one action -> verify or stop`。

For M17, image/CV/model features belong in versioned packages or providers, not core language syntax. Deterministic vision is the default when it solves the task; model inference must be explicit, budgeted and provenance-bound.

## 5. Source/backend honesty

Sico's frontend accepts more constructs than every executable profile supports. For changes involving language features:

- verify `check`, `build` and `run` separately;
- add or update the machine support matrix and backend-refusal corpus;
- preserve stable diagnostics and source spans;
- do not implement a partial lowering that emits unverifiable or semantically different code;
- prefer a typed refusal over a hidden fallback or native escape hatch.

M14 的 defining gate 已达成并 GO（STEP-0141 出口审计）：其应用 profile 无未声明 check/build/run 缺口。离线 block-game solver 仍是 acceptance oracle（STEP-0258 未撤销其地位）：棋盘规则、放置枚举、消行、计分与搜索必须运行在 Sico 中；Python 仅作为固定对比 oracle。

## 6. Repository locations

- `crates/`: compiler, IR, codegen, package, Runtime-facing libraries, tooling and Host cores.
- `runner/sico-runner/`: separate Cargo workspace for Script execution and DAP integration.
- `wit/`: versioned Component interfaces.
- `selfhost/`: M22 编译器自举轨的 guest 源码与 checker/driver 语料（`corpus-v0.json` 235 条全 strict + `corpus-w1/` SHA 冻结增量，DIVERGENCE_WAIVER 为 0）。
- `semantic-cases/`, `syntax-candidates/`, `syntax-mutations/`: language oracles.
- `tests/`: end-to-end, security, platform and performance evidence.
- `examples/`: language-design history and regression examples; do not rewrite them as product pilots.
- `pilots/`: clean-room workflows; they are not external adoption.
- `docs/rfc/`, `docs/adr/`, `docs/steps/`, `docs/reports/`: contracts, decisions, execution records and evidence.
- `案例项目/俄罗斯方块消除/`: M14 solver acceptance oracle only（STEP-0138；block-game 自动化试点已由 STEP-0258 删除，M16/M17/M18 不再以它为载体）。保留其 Python 实现作为固定对比 oracle。

## 7. Editing rules

- STEP 编号在并行 worktree 之间是共享序列：M24 轨（worktree `E:\github\sico-m24`）占用 STEP-0330–0337，取号前先核对 [`docs/steps/`](./docs/steps/README.md) 与 [`handoff-m22.md`](./docs/handoff-m22.md)。`origin/dev` 另有一套 STEP-0279–0283 历史，与本分支的源码和编号冲突；不得覆盖，也不得做隐式合并。
- Keep changes scoped to the requested subsystem. Preserve unrelated dirty-worktree changes.
- Do not use destructive Git commands or silently rewrite history.
- Prefer deterministic, bounded data structures and stable serialized forms.
- New parsers and protocol inputs must reject unknown fields, overflow, truncation, trailing data and limit+1 cases unless an accepted contract explicitly says otherwise.
- External text, package metadata, model assets and guest output are untrusted data; never turn them directly into commands, paths, HTML, URLs, permissions or support claims.
- Keep user documentation limited to actually executable behavior. Put proposed behavior in plans/RFCs and label it planned.
- Use Chinese for owner-facing roadmap/status explanations when practical; keep public identifiers, schemas and protocol field names stable in English.
- **持有平台句柄或线程亲和资源的类型，必须显式声明 `!Send`/`!Sync`**（用 `PhantomData<*mut ()>` / `PhantomPinned` 等**刻意**标记，并加编译期负约束断言），**不得依赖偶然字段**维持线程亲和性。`windows` 0.52 的 `HWND`/`HDC`/`HBITMAP` 等均为 `pub struct(pub isize)`（本身 `Send+Sync`），一旦某次重构让持有者失去裸指针字段，跨线程误用会**静默**成立 ⇒ 数据竞争 UB + 消息泵跑到错误线程。（来源：STEP-0577 独立安全复核，2026-11-07）
- **Win32/平台 API 的失败结果不得静默吞掉**：`DestroyWindow`/`BeginPaint`/`SetWindowLongPtrW`/`SelectObject` 等返回值要么传播为类型化错误，要么在代码中注释说明为何可忽略；`BeginPaint` 失败时不得调用 `EndPaint`；销毁失败时不得释放仍被 `GWLP_USERDATA` 引用的状态内存。

## 8. Validation

Start with the narrowest relevant tests, then expand in proportion to risk. On this Windows host, `cargo` may need the explicit path `$env:USERPROFILE\.cargo\bin\cargo.exe`.

Typical root-workspace checks:

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$env:SICO_TEST_WASMTIME = & .\tools\ensure-wasmtime.ps1
& $cargo fmt --all -- --check
& $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
& $cargo test --locked --offline --workspace --all-targets --all-features
.\tools\validate-module-boundaries.ps1
```

The runner is a separate workspace:

```powershell
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml
```

For documentation-only roadmap changes, at minimum run the relevant validator and `git diff --check`. The M14–M24 planning contract is checked by:

```powershell
.\tools\validate-step-0124.ps1
git diff --check
```

Do not run the full workspace mechanically when a narrow documentation or fixture change cannot affect compiled code; record what was and was not run.

## 9. Completion checklist

Before reporting completion:

- confirm the requested outcome, not only the attempted action;
- rerun the relevant validator from a clean command invocation;
- inspect `git status --short` and distinguish your files from pre-existing changes;
- update docs, schemas, fixtures and tests together when a contract changes;
- report exact tests, platforms and evidence level;
- list residual blockers without converting them into implied success;
- do not commit, push, publish, install, grant permissions or use external credentials unless the user explicitly requests that action.

## 10. M22 整改与完整收尾 Implementation Plan（2026-10-24 审计）

> **For agentic workers:** REQUIRED SUB-SKILL: use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox syntax for tracking. 本节只登记计划，不代表任何剩余项已经完成。

**Goal:** 先修复 STEP-0535 的证据与覆盖口径，再按既有 S5-W2 → W7 阶梯完成 M22 的冻结语料、自举固定点、`.sapp` 信任链和七门出口审计。

**Architecture:** 保留 Rust 前端、IR verifier 和后端为永久 oracle；guest 只通过生产输出的逐字节差分扩大声明子集。先建立真实 S 站点和畸形 IR 的防伪测试，再扩实现；不把合成 fixture 的成功外推为整族或整文件成功。

**Tech Stack:** Sico guest、Rust、`sico_ir` canonical JSON、`sico-codegen-wasm`、Wasmtime runner、`wasmparser::Validator`、PowerShell 验证器、GitHub Actions Windows GNU。

**Spec:** [`M22 计划 §3.3/§4`](./docs/plans/M22-compiler-self-host.md)、[ADR-0015](./docs/adr/ADR-0015-compiler-bootstrap-closure-v0.md)、[ADR-0016](./docs/adr/ADR-0016-selfhost-soa-compilation-unit-v0.md) 和 [RFC-0011](./docs/rfc/RFC-0011-deterministic-core-wasm-backend-v0.md)。

### 10.1 审计基线与判定

- 已提交 HEAD `54b6d9758a4709f5e6fb9d6982854704fbcbd285` 的独立 CI run `40053245939` 为 success；它只证明 STEP-0534 快照，不证明未提交的 STEP-0535。
- 当前 STEP-0535 仍未提交、未推送；其 173 条 `selfhost_compiler` 测试和 runner clippy 已在本机复跑通过，格式问题后被并发修改修复。`target/tmp/step0535/battery.log` 仍含 integrity/fmt 两类 exit 1，故 `BATTERY_GREEN 23/23` 尚无可用证据。
- 4 个合成 `match` fixture 的组件逐字节等同是有效局部证据；“42/519 形状面已闭合”未获逐站点证明，且 `selfhost/parser.sico` 的 `is_ident_byte`、`type_json` 含未测复杂 case 体。当前只能宣称 **4 个形状通过**。
- 出口口径：门 1/5 GO，门 2 PARTIAL，门 3/6 NO-GO；门 4 在 `B(S)` 实测前不得沿用 GO，门 7 在下列证据问题修复前不得给完成信用。M22 overall 保持 **NO-GO**。
- 畸形容错差距：Result 投影只比较槽数而未比较完整 Wasm 类型序列；variant 模式未拒绝非 variant 值。它们尚未证明可由合法 Sico 源触发，但必须作为 canonical-IR 边界的 fail-closed 缺口整改。

#### 10.1.1 Owner 工作量看板（先看这里）

**当前位置：R0「STEP-0535 证据整改」，尚未进入 R1 实现收口。下一个可见完成点是 R0 exit：可信电池全绿 + 42 个候选真实站点全部有 verdict + 畸形 IR 测试绿 + 文档口径一致。** R0 完成也不代表 M22 GO，只代表当前片可以进入 `match` 全量收口。

| 阶段 | 已经有的证据 | 还必须交付的工作量 | 当前状态 |
|---|---|---|---|
| **R0 证据整改** | 4 个合成 Component 形状；173/173 固件；fmt/clippy | 1 套版本化电池、42 个候选站点 verdict、3 类畸形 IR 测试、布局/边界补测、文档与候选卫生 | **进行中，未达 exit** |
| **R1 `match` 与共享算子** | 前端登记 57/519；4 个合成 Component 形状 | 42 个候选先复核，再完成 M-g3' 15、M-g1 354、M-g2 46、M-g3 21 个具名站点及共享算子；最终 519/519 账本和独立 CI | **未闭合** |
| **R2 剩余宿主族与整源** | T1 闭合；T2 前端登记 325/332；parser/formatter IR 有强证据 | T2 余 7 个站点及跨族依赖；T3–T20 共 18 个未闭合族；其余冻结 S 的整源 IR 与对应 Component 差分 | **未闭合** |
| **R3/R4 语料与 bundle** | 37 accepted / 178 refused 的 Rust 冻结基线已存在 | 1 个 formatter Component、37 条 accepted、178 条拒绝三元组、5 文件 bundle、9 文件全集 | **未开始完整验收** |
| **R5 自举与预算** | IR-only 预算观察；Rust 可生成约 1 MiB 的 A | `A == B == C`、R/B 双跑语料、`.sapp`、hostile/limit+1、完整 B(S) 预算和 M19 对照 | **未进入** |
| **R6 最终出口** | STEP-0534 精确 SHA 的独立 CI 绿 | 干净候选、M0–M21 全量回归、当前 SHA 独立 CI、支持面汇总、七门新审计 | **未进入** |

**粗略剩余估算：约 65–120 个执行片，不是时间承诺。** 分配口径为 R0 4–6、R1 8–14、R2 20–35、R3/R4 24–48、R5 6–8、R6 4–6；实际片数由真实拒绝前沿决定。不能用 STEP-0514 后的 STEP 数简单相减，因为测量片、验证片、返工和新增整改并不等量，也不都推进出口门。

**工作量判读规则：** 每片必须包含实现或测量、真实 runner 差分、回归和证据同步；热点文件要求串行，不能靠并行开族压缩关键路径。看板只按 exit 更新，不按“正在做”或测试数量虚增进度；暂不换算天数。owner 每次只需先看本表，再看 §10.10 已完成清单和 §10.11 未完成清单。

#### 10.1.2 Owner 决策（2026-10-26）：电池分层 + 下一片优先级

**决策一：电池改为分层运行。** 此前"每片改动都跑 2.7 小时整轮电池"**不是成文规则**——成文的整轮闸门只有 R0 收口与 R6 最终，族级判据里的"固件电池绿"落在第 4 拍收口片 X；逐片跑整轮是 STEP-0499 起的实践演化。owner 决定改为分层：

| 层 | 工具 | 何时跑 | 证据效力 |
|---|---|---|---|
| 开发期子集 | `tools/validate-m22-s5-dev.ps1`（`-Set fast` ≈ 1 min / `probes` ≈ 12 min / `firmware` ≈ 28 min） | 每次 guest 改动 | **不是出口证据**：只覆盖该子集，不能关闭任何门 |
| 出口整轮 | `tools/validate-m22-s5.ps1`（15 拍 ≈ 2.7 h） | 族收口片 X、候选 SHA、门 6 / 门 7 出口 | 出口证据，缺一不算 |

子集 runner 的续跑用 `tools/m22-s5-fingerprint.ps1` 的指纹（`target`、`__pycache__`、`*.pyc` 三项排除是承重件，见 §10.11 A）；指纹变了就拒绝复用旧绿。**分层只改节奏，不降判据**：族级六项判据、退出门、证据等级、独立 CI 全部不变。

**决策二：下一步做 B2。** STEP-0545 实测证明 B1（游标推进）只拿回约 0.6× 的二次项，二次项仍占 76%–97%，140 个 `FuelExhausted` 站点一个未回绿 ⇒ 下一片 = **B2：为 value id → 槽号建索引（前缀或哈希），把查槽降到 O(1)、总量降到 O(n)**。B2 动工前 `selfhost/codegen.sico` 必须无并发写入方。

#### 10.1.3 复审（2026-10-26，STEP-0547）：装配策略优先于形状覆盖

全量复核见 [`docs/reports/m22-review-2026-10-26.md`](./docs/reports/m22-review-2026-10-26.md)。结论：门 2/3 的瓶颈是**装配策略**，不是形状覆盖——`selfhost/codegen.sico` 194 个 `function` / 512 处 `sico.text.concat`（461 处自左折叠） / 0 处可变 byte，输出 hex 文本 ⇒ 输出 N 字节需 O(N²) 拷贝；STEP-0516 的「1 MiB 可行」只对「一次 concat 两个既有大操作数」成立（S1/S2 实测上限 128 KiB / 256 KiB），而发射端只能逐碎片拼装。故 0544–0546 三片即便把查表修到 O(n log n)，站点回绿仍为 0。

执行顺序改为：

| 顺序 | 内容 | exit / 决策门 | 片数 |
|---|---|---|---|
| P0 | 尺度预检：同一 payload 比较「左折叠 concat」与「分片 `list.append` + 一次 `text.join`」在 64 KiB / 256 KiB / 1 MiB 的 fuel、wall、trap | 两条曲线；**D1** 绿 ⇒ P1，红 ⇒ RFC 或按 R3 止损 | 1（**STEP-0548 完成，D1 绿**） |
| P1 | 装配重写：hex 段落改分片 + 一次 join，字节输出不变 | 现有 32 个全等站点仍全等 + 新增 ≥256 KiB 站点与 Rust 逐字节全等 | 1–2（**STEP-0549 完成**） |
| P2 | 批量复算：夹具上限 64 fn/256 KiB → **256 fn/1 MiB**，一次重跑 396 站点 | 新直方图 + 每个残差具名原因；**D2** 回绿 <25% ⇒ 判结构性，三选一 | 1（**STEP-0550 完成：回绿 0/364，D2 = 结构性**） |
| P3–P6 | 具名残差 → 语料/bundle → 自举与预算 → 出口（内容同 §10.1.1，仅改顺序） | 同 §10.10 各族判据 | 12–22 |

约束（与 §10.2/§10.3 一致，且更严）：

- **§10.1.1 看板降为残差台账**，不再是执行顺序；「一片一形状」在 P2 之前不算推进主路径。
- **不新增 per-shape packer**（发射词表缺口已于 STEP-0542 归零）。
- **不上调 fuel / 内存限额追认**；P0/P1 读数只用于判断策略，不作为放宽闸门的依据。
- §10.1.2 决策二（下一步 B2）在 P2 之前**不是主路径**；**P2 已完成（STEP-0550，D2 = 结构性）⇒ B2 升级为主路径**：为函数级「值 id → 槽号」建一次索引，exit 必须单独报「195 个新开测站点回绿数」且既有 32 个全等站点逐字节不变。**B2 已完成（STEP-0551，2026-10-28）**：`cgs_value_index` 落地，回绿 **4/364 = 1.10%**（全部来自既有 140，**195 个新开测站点 0 回绿**），既有 32 个全等站点 `component_sha256` 全同；组件轴 **36/396** ⇒ **主路径续为 B3**：剩余 **45 处循环内 `cgj_array_item`** 改游标 + `cgs_module_has_intrinsic`/`cgs_function_has_intrinsic`/`cgs_block_has_intrinsic`/`cgs_function_position`/`cgs_run_index` 改每函数一次索引，exit 仍单列 195 新开测站点的回绿数且既有 36 个全等站点逐字节不变。
- **`parser.sico:scalar_ir` 的 24 个站点**受 runner 8 MiB stdout 结果上限（`runner/sico-runner/src/lib.rs:3599`）阻挡，与 B2 正交；先量 IR 文本真实大小再单独裁决，**不得直接上调结果上限**。
- **ADR-0017 Option A（记录化重基线）挂起到 M25 之后**：现在重写冻结 S 的源文本会作废 149/149 census 与逐站 `source_sha256`/`ir_sha256`/`component_sha256`。若 owner 决定不挂起，须先给出「重基线后如何重建逐站证据」的方案。

#### 10.1.4 复审（2026-11-07，主线评审）：B3 已被实证否决；燃料墙改判「发射器层 · 常数主导」；主路径改判

> 依据：STEP-0555 §3.1/§4/§10.6、STEP-0556 §3/§10/§11、STEP-0562 §3.1–3.2/§4.2、STEP-0567、STEP-0572 §1/§5、STEP-0575 §3.2–3.3/§4.4、2026-11-05/06 STATUS 条目。**本节为 addendum 订正，§10.1.1–§10.1.3 原文保留**。

- **B3 = 已否决，不应重开**（STEP-0555 §3.1/§4/§10.6）：B3 实施后 FuelExhausted **331→344（+13）**、component-equal **36→23（−13）**，**0 站点回绿、13 站点回退**；游标化在 fuel 计量下是**净负**（`cgj_array_item` = 1 个 hostcall，游标形 = 2 个/元素，fuel 按 wasm 指令操作计）；模块索引部分幅度 ≈ **0.01%**（~81K ops vs 单函数固定成本 ~128M），且 331/335 FuelExhausted 站点全在 `parser.sico:scalar_ir`（**从不调 codegen helper**）⇒ **触达不到墙**。⇒ **§10.1.2 决策二与 §10.1.3「B3 升级为主路径」的前提失效**（其隐含假设「O(n²)→O(n) 省 fuel」已被实测推翻）；STATUS 2026-11-05 的「恢复点 = B3」登记同步作废。
- **燃料墙现状判定**：墙在**发射器层**（emit-only 注入绕过前端仍耗尽，STEP-0555 §10.2/§10.4c）；**fuel ~ IR²**（STEP-0555 §10.4c、STEP-0562 §3.2④ 三臂 `fuel/IR²` 近似恒定）；**常数主导**（per-op overhead × op count；STEP-0556 §11 每函数固定成本 ~1.5e8–6e8，静态可读路径重扫合计 ≈0.01%，无法解释 ~1.2e6 ops/输出字节）；**具体 wasm-op 源仍未定位**。STEP-0575 §3.2 钉死平方项随 match 数，但 §3.3 证明两个具名源相对总 fuel 均 <0.1%。
- **口径订正**：组件轴不是 36/396，而是 **335 exhausted / 32 equal / 24 trap(8 MiB) / 5 refused-guest**（STEP-0562/0567：MS-081..084 实为 FuelExhausted，旧「36」偏乐观）；W4 parity 剩余不是 4/150，而是 **3/150 且经 STEP-0600 实测**并非燃料墙**（800e9/1.6T/3.2T 三档均**毫秒级**被 `E-SH-IR-FUNCTION-COUNT`/`E-SH-IR-EXPRESSION` 拒绝，**提高 fuel 不改变结果**；此前登记为「燃料墙」的口径已订正）**（STEP-0558/0564），与 verdict parity 正交 ⇒ 作为前端刀是死路。
- **新主路径（2026-11-07 判定）**：①**先跑 STEP-0575 §4.4 的强制判据**——396 站点按 match 密度分账，对照「耗尽 335 / 全等 32」两桶；若耗尽桶不超配 ⇒ match 主刀预测回绿 ~0、**不应开工**（测量片 **STEP-0579**）；②若①证实常数主导 ⇒ 唯一定位路径是 **op 级燃料仪器化剖析**（需 owner 授权 `runner/sico-runner` 生产侧 fuel 读数/epoch·opcode 钩，见 STEP-0572 §5；另需本机 gcc 安装授权，见 STEP-0572 §1），否则走 owner **D8** 限额裁决（fuel 1.6T 重基线 vs 收窄语料/闭包口径；**不得无证据上调**）。**在 owner 裁决前，M22 线只允许测量片/诊断片，不允许特征实现片**（含 `parser.sico`/`codegen.sico` 热文件写入）**【2026-11-08 订正：该限制已由 owner D-C 裁决解除，见 §10.1.4 末 2026-11-08 addendum；历史表述保留】**。
- **STEP-0579 执行结果（2026-11-07，addendum）**：§4.4 判据「字面成立但**无判别力**」——耗尽桶密度中位 **9** vs 全等桶 **1**（9×），但基线口径仅 **1.04×**；**混淆检验为决定性证据**：密度↔span Spearman **ρ=0.883**，全等桶全为微函数（span≤71、密度≤3），固定密度层耗尽函数 span 始终大 3–4× ⇒ 「9× 超配」由**尺寸梯度**驱动，无法分离 match 项与「每操作常数×操作数（∝尺寸）」。**裁决：match 主刀不值得作为主破墙刀开工**（预测回绿 ~0–1%，与 STEP-0556 §11 六刀 0–1%、STEP-0575 两具名源 <0.1% 一致）⇒ **M22 收敛为二选一**：op 级燃料剖析（需 owner 授权 runner 生产钩 + gcc；最小变更清单见 STEP-0579 §4）或 owner **D8** 限额裁决。**同片文档治理只读评估（未重编号未改表，重编号属 owner 裁决项）**：README 现 **26 组同号**（19 组双语重复 / 0175 两文件 / 0279–0283 跨线 / 0338 双线）、**5 行未转义 `|`**（STEP 0285/0328/0502/0520/0539）、**0440–0485 悬空**。
- **STEP-0602 执行结果（2026-11-07，addendum）——M22 存在两个正交面**：把 `selfhost_compiler` 的 **5 个出口阻塞红测**逐项 `--exact` 复跑取证后确认——**4 个 codegen 红测全部是 guest 前端「快速 typed refusal」（2.3–2.9 s，远低于 800e9 上限）**：①`cgs_result_return_hex`（EXPRESSION 层，`ERR:E-SH-IR-EXPRESSION`）②`cgs_value_ty`（call-shape 层，`ERR:E-SH-IR-CALL-SHAPE`）③`cgs_value_slot`（同 ② 形状）④`cgs_function_instr_hex`（前端非 Output，嵌套 `while`）；第 5 个是**冻结 match-site 账本漂移**（期望 vs 实测失配；已核 36/95 组 0 失配）⇒ **这 5 个红测与燃料墙无依赖耦合**（燃料墙 = 账本里 331 个 FuelExhausted 站点，是独立且更大的问题），**也不被「收窄语料/闭包口径」顺带解决**。**建议下一刀（若 owner 恢复 M22 实现）**：**STEP-0571 §3 配方落地（`step0519`+`step0520`，唯一已验证路径：转绿 + 逐字节等同）+ 冻结账本重冻结（绑定变更，因配方致 130 站行号位移 ⇒ `LEDGER_STALE`）**，一次协调变更最多解锁 3/5；**不必等 op 级燃料剖析**（剖析解决的是燃料墙本体，与本组红测正交）。**诚实收窄**：每测试只确定首个失败函数（panic 即终止），`step0520`/`step0526` 精确码未重测（取自 STEP-0571 §2 基线），账本漂移精确站点未定位（完整 95 组 ≈20–27 min 超预算）。
- **STEP-0604 执行结果（2026-11-07，addendum）——诊断复闭 + 三处口径订正**：对 STEP-0602 的「5 个出口阻塞红测」做**只读独立复核**（磁盘 sha + `git log` 冻结点，未重跑 cargo，见 [`STEP-0604`](./docs/steps/STEP-0604-m22-exit-blocking-red-tests-diagnostic-reclosure.md)），结论**方向成立但需三处订正**：①**第 5 个（冻结账本）当前并不是红的**——`match-component-coverage-v1.json`、`codegen.sico`、`parser.sico` **最后一次修改均在同一提交 `6440890`（2026-11-06）**，账本与被测源**同步冻结、当前不失配**；账本**快速结构相位**（schema `sico.m22.match-coverage.v1`、396 站点计数、逐站结构字段）在当前源上**通过**（慢速 95 组逐组重算未运行）⇒ STEP-0602/0571 所称「漂移 / `LEDGER_STALE`」是**应用 STEP-0571 §3 配方之后才发生的有条件重冻结成本**（130 站行号位移），**不是当下红**，不得读作「已失配待修」。②**§10.11-A 的 emitter-vocabulary 前提已过期并关闭**——`emitter-vocabulary-v1.json` 行 168 的 `guest_emitter.source_sha256` = `013f2d7e…` **逐字符等于**当前 `codegen.sico` sha ⇒ `census-m22-emitter-vocabulary.py --check` **不再判 `EMITTER_VOCABULARY_RED`**；旧前提引用的 `da0fec5f…`/`3d8705b1…` 两 sha 均已过期（易混淆点：行 110 的 `source_sha256`=`220b8e47…` 属 `backend_helper_rule` 对象，非 `guest_emitter`，不得误读为仍红）⇒ §10.11-A「第 13 拍必红」整改项**关闭**（与 `docs/reports/m22-doc-hygiene-2026-11-06.md`、STEP-0570 一致）。③**wall-clock 上修**：STEP-0602 引的 2.3–2.9 s 是**测试体内单个 `finished in`**；**整测试 wall-clock（含 PreparedPool 槽编译 + 800e9-fuel guest 运行 + harness）为 5.5–7.0 s**（step0507 6.47 s / step0519 6.97 s / step0520 6.69 s / step0526 5.58 s，均 exit 101）⇒ **「快速 typed refusal、与燃料墙正交」的定性不变**。**5 个红测钉到实际测试名/位置**：step0507=`sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`（`:3468`）、step0519=`...step0519...`（`:6376`）、step0520=`...step0520...`（`:6701`）、step0526=`...step0526...`（`:3515`）、ledger=`sico_compiler_reproduces_the_frozen_match_site_ledger`（`:9805`）。**结论维持**：M22 存在两个正交面（本组 = 前端形状缺口 + 配方落地账本绑定代价；燃料墙 = 账本里 331 个 FuelExhausted 站点）；STEP-0571 §3 配方仍被**两项阻塞**——①130 站协调账本重冻结（须与配方同片/同步，不能拆开）②owner 授权（配方落地属特征实现片，须先解除暂停）；**诚实收窄**：#3/#4 精确诊断码未测（panic 串省略 outcome，取得精确码需改测试打印 outcome，超只读诊断片范围；#4 的 Domain-vs-Trap 仍开放）；账本「当前同步」由**同提交冻结 + 快速结构相位**锚定，**未由完整逐组重算独立复证**。**未翻任何门，M22 overall 仍 NO-GO**。
- **STEP-0605 执行结果（2026-11-07，addendum）——STEP-0571 §3 配方落地：两目标红测转绿（逐字节等同）+ MS-081 既有燃料墙阻塞**：STEP-0604 §5 所称「配方落地」已从建议推进为已执行并取证（见 [`STEP-0605`](./docs/steps/STEP-0605-m22-w2-step0571-recipe-landing.md)）。①**配方已落地**：`selfhost/parser.sico` 现含 STEP-0571 §3.1–§3.4 的 4 处纯插入式编辑（`gw_list_get_subject_packed` ~:13958、`gw_match_open_pack` ~:15703），typed-refusal 臂全保留；工作树 sha256 `b67ec566…`（23168 行，基线 `b6cb6ab0…` +52/−3）。②**两目标红测转绿**（coordinator `--exact` 亲跑）：`step0519`→ok（4.21 s）、`step0520`→ok（3.52 s），均 `*_byte_exactly` ⇒ **guest/Rust 逐字节等同**；**这把 §10.11-B 的 `step0519`/`step0520` 两项由红转绿（本机 `--exact` 口径，全量套件未跑）**。③**隔离守卫不变**：`step0507`（EXPRESSION 层 `ERR:E-SH-IR-EXPRESSION`）、`step0526`（非 Output 嵌套 `while`）**仍红** ⇒ 配方未越界，两者仍是出口阻塞项、各需另片。④**账本随配方重编号**：`match-component-coverage-v1.json` +257/−257（锚点下 parser.sico 站点行号位移，MS-267 14102→14139）；`codegen.sico` 未触碰。⑤**账本 firmware 仍红于 MS-081，但为既有、非配方回归**：MS-081 冻结 verdict `"ir-equal+component-equal"` vs 实测 `FuelExhausted`；证明 (a) HEAD 的 MS-081 同 verdict 同 line 4577，(b) MS-081..084 是 checked_add 站点（行 4577/4590/4622/4629）全在配方锚点**之前**，配方只改泛型 `list.get` ⇒ 不可能影响——正是 §10.1.3 的乐观口径（账本 36 `component-equal` 含 MS-081..084 实为 FuelExhausted）。⑥**MS-081 修正阻塞 = 本机无可用 Python**（仅 WindowsApps stub、`SICO_PYTHON` 未设、`Python312` 无 python.exe）⇒ 无法跑 `census-m22-match-sites.py` 重算 `component_sha256` ⇒ **顺延燃料墙/D8 轨**，本片不擅自上调限额/不臆造 digest。⑦**溯源不确定（如实）**：配方编辑与账本重编号在落地 sub-agent 运行时已在工作树（HEAD 未变 `af6dcb7`），该 sub-agent 未施加编辑；coordinator 核验 diff 恰为 §3 配方且两目标逐字节转绿 ⇒ 以已验证状态采纳，但施加时间/agent 不确定。⑧**诚实收窄**：未跑全量 `selfhost_compiler` 套件（仅 `--exact` 4 目标 + 账本 firmware），落地后套件完整计数未测；账本慢速 95 组逐组重算未跑。**未翻任何门，M22 overall 仍 NO-GO；未提交未推送。**
- **STEP-0606 执行结果（2026-11-07，addendum）——燃料归因诊断：把燃料墙从「常数主导·源未定位」修订为「具名超线性发射器算子主导」，并给出 D8 后果（上调限额买不回站点）**：用新落地的 runner 燃料钩（`run_with_fuel`，见 STEP-0572 §5 缺口）对燃料墙做 op/括号级归因（coordinator 独立读取 sweep 日志 `target/tmp/step_probe/{cost_centers,match_quadratic}.log` 视为 ground truth；harness = `runner/sico-runner/tests/m22_fuel_profile.rs`，探针上限经 env `SICO_M22_PROBE_FUEL` 抬升、in-file 冻结资格上限 `FUEL_CEILING=800_000_000_000`；800e9 单线程；**emit-only 合成闭包 + 一个真实站点**；见 [`STEP-0606`](./docs/steps/STEP-0606-m22-fuel-attribution-diagnostic.md)）。**发现：M22 燃料墙是「工作负载/超线性驱动」，不是「每跑固定常数」**——主导项是**两个具名发射器算子**：①**match-dispatcher 重扫**（判别器 `match_flat`（N 个独立 match）vs `match_one`（1 match + N−1 `set`，**等 body 体量**）把 dispatcher 重扫从 body 扫描中隔离：n=8 flat fuel_mid 115.2e9(ir 21060) vs one 5.6e9(ir 6418)；n=16 flat **EXHAUSTED(>800e9)**(ir 40052) vs one 14.4e9(ir 8394) ⇒ 相近 n 下 **~55× 差距**；flat 的 observed_over_quadratic = 1.000(n=4)/0.778(n=8) ⇒ **干净 O(n²)**）与 ②**`text_concat` 装配**（fuel_mid 14.4e9(n=8)→89.6e9(n=16)→556e9(n=32)→exhausted(n=64)，slope_vs_prev **3.11/3.10** ⇒ 二次；与 STEP-0548 左折叠装配二次结论一致）。`functions` 族 slope_vs_prev 1.22/1.64/1.56 ⇒ 近线性-plus。**负对照 `forwarded_let_control`**：IR 平坦(2069→2088)、fuel **恒定 4.5e8**(n=8/16/32/64)、slope_vs_prev 0.50 纯由 n 翻倍引起 ⇒ **平坦读数可达、方法论自证**（增长族确在测发射器功随 IR 标度）。**真实站点锚 `parser.sico:fixed_literal_const_segment`**（ir 92057 B，emit-only）：6.4e9/64e9/800e9 **三档全耗尽** ⇒ **永不达翻转点 ⇒ 精确烧量未钉死，仅 `>800e9`（`lower_bound_only`）**；刻意未推到 6.4e12（约 15+ min/测试，超预算）。**D8 后果：上调燃料限额（D8 重基线）买不回站点**——二次增长下**限额翻倍只多解锁一步 n**（与 STEP-0600 实测 2× 燃料只转绿 5/331=1.51% 一致）；**真正杠杆 = 降低 match 重扫与 text_concat 装配的复杂度**（与 STEP-0548 装配策略结论收敛）。**与 §10.1.4 的调和（不推翻）**：§10.1.4 原表述「发射器层 + fuel ~ IR² + **常数主导（具体 wasm-op 源仍未定位）**」——本诊断**确认**前两点，并**定位了源**：非「尺寸无关固定常数」，而是上述两个具名超线性算子 ⇒ 把「常数主导·源未定位」**修订为「具名超线性发射器算子主导」**；因「常数主导」本身是带保留措辞（明确承认源未知），此为**对开放问题的解决/收窄，非对既立事实的推翻**。**诚实收窄**：emit-only 合成闭包 + 一个真实站点（**非**完整 396 站点账本、**非** live guest-前端降级）；锚站点精确烧量仅 `>800e9`（`lower_bound_only`）；`FuelTrace::timeline`（`run_with_fuel_timeline`，逐 tick epoch 采样）**已构建但本批探针未行使**——只调 `run_with_fuel`，仅得 `consumed` 总量/括号中点；`fuel_mid` 为**括号中点（约 ±12.5% 带），非精确 consumed**。**未翻任何门，M22 overall 仍 NO-GO；未提交未推送。**
- **owner 三项裁决（2026-11-08，addendum）——D-A 授权 / D-B 否决 / D-C 解除暂停**：owner 就 §10.1.4 收敛出的两个悬决项拍板，本条只做登记（纯登记/文档片，零代码/零 WIT/零测试/零语料/零限额改动）。①**D-A（op 级燃料仪器化剖析 + gcc）已授权、进行中**：`runner/sico-runner` 生产侧 fuel 读数/epoch·opcode 钩授权与本机 gcc 安装授权均已给出，剖析工作由另一代理在跑（本登记片不跑 cargo/剖析）。②**D-B（燃料限额上调 / D8 重基线）正式否决**：**不上调 fuel 限额**——依据 **STEP-0600** 实测 2× 燃料（1.6T）全量 331 站点只转绿 **5/331 = 1.51%**；**STEP-0606** 证明二次增长下「限额翻倍只多解锁一步 n」⇒ **上调限额买不回站点**，真正杠杆是降低 match 重扫与 `text_concat` 装配的复杂度。③**D-C（M22 暂停）解除**：2026-10-30 的「M22 暂停」（只允许测量/诊断片、不允许特征实现片）**正式解除**，`selfhost/parser.sico` / `selfhost/codegen.sico` 热文件可开特征实现片，**仍须遵守 §10.2「两热点文件同时只允许一个写入方」与逐片纪律（HEAD/diff/SHA 记录、并发改动后重验证）**。**由此，上列 §10.1.4 各条中「在 owner 裁决前只允许测量片/诊断片，不允许特征实现片」「须先解除暂停」「D8 限额裁决待裁」等表述自 2026-11-08 起已被本裁决取代**（历史叙述保留、以本 addendum 订正为准）；**「下一刀」据 D-B/D-A 订正为：op 级燃料剖析 → 据剖析结果定优化刀（match-dispatcher 重扫 / `text_concat` 装配二选一或并进），不再走 B3（§10.1.4 已实测否决）、也不走限额上调（D-B 否决）**。**门状态不变**：未翻任何门，M22 overall 仍 NO-GO。
- **STEP-0609 执行结果（2026-11-08，addendum）——D-C 解除暂停后首个特征实现片：`step0507` 出口阻塞红测转绿、`step0526` 同步转绿 ⇒ 出口阻塞红测 5→3**：落地片改 `selfhost/parser.sico`（**+149/−0，纯插入式**，工作树起始 HEAD `90e37db`，未提交未推送）。**真实根因（订正 STEP-0602/0604/0605 的记录）**：guest 前端无法降低**返回位置的**无参 `sico.*` 内建（general）——stub 生成器为 `List[U64]` stub 发射 `return sico.list.empty[U64]()`，`cgs_result_return_hex` 调 `cgs_value_index`（返回 `List[U64]`）⇒ 失败 `ERR:E-SH-IR-EXPRESSION`；既有 `direct_host_call_return_ir` 只声明 `sico.bytes.length` ⇒ SKIP。**修复 = 收窄的纯插入式路由 `direct_list_empty_return_ir`**：只声明 `signature → return sico.list.empty[<U64|Text>]() → end function`，否则 `return "SKIP"` ⇒ **全部 typed-refusal 臂保留**。**`step0526` 与 `step0507` 同根因（CORRECTION——STEP-0602/0604/0605 所记「`step0526` = 独立的非 Output 嵌套 `while`」作废）**。**账本 = 纯行号位移**：229 个站点行号移动、**零 digest/verdict 变化**、无新增 census 站点、`EXPECTED_SITES` 仍 396、`--check` = `LEDGER_CURRENT sites=396`。**剩余出口阻塞项**：原 5 项中只剩**账本 firmware（MS-081 燃料墙）族**。**诚实收窄**：本片只跑 **8 个 `--exact` 测试**（非全量套件、非账本慢速 95 组逐组重算）；账本 firmware 仍红于 MS-081；coordinator 复核 step0507+step0526 两测转绿、step0519/0520 仍绿、整份 parser IR pin ok、账本 HEAD-vs-worktree verdict+sha256 多重集**完全相同**。**未翻任何门，M22 overall 仍 NO-GO；未提交未推送。**
- **STEP-0608 执行结果（2026-11-08，addendum）——W3-a 整份 `formatter.sico` 组件面的首次测量（仅测量片）**：首次对 W3-a 的**整文件组件面**做测量——整份 `formatter.sico` 经 guest `--emit-component-hex`（**标准预算，未上调**）⇒ typed refusal `ERR:E-SH-CG-IR-EMIT`，仅耗 **~0.03% fuel** ⇒ **与燃料墙正交**（不是 entry-mapping）。**具名缺口 = `sico.bytes.is_utf8` 被用作 `if` 条件**（`formatter.sico:818`）。⇒ **W3-a 未闭合**：guest 在发射前即拒绝、无字节可比对，**需要补一个发射器臂**（本片未做）。coordinator 复核：STEP-0608 探针 clippy 干净、runner fmt 干净。**未翻任何门，M22 overall 仍 NO-GO。**
- **STEP-0610 执行结果（2026-11-08，addendum）——W3-a `is_utf8` 臂落地，但真挡板改判为两侧共享的 multi-block-`run` 缺口（W3-a 仍 open）**：落地片改 `selfhost/codegen.sico`（**+71/−0**，未提交未推送）——guest 发射器补 `sico.bytes.is_utf8` 臂：新函数 `cgs_is_utf8_hex`（`:3857-3919`）+ `cgs_intrinsic_hex` 一行分派（`:3851-3853`），采用 STEP-0560 `cgs_list_length_u64_hex` 委托模式（**delegation 必须，非风格**：inline 版实测打破 `step0513` 自举——`ERR:E-SH-IR-STATEMENT` 语句前沿）。**核心再定性**：`is_utf8` 臂**必要但不充分**——W3-a 真挡板是**两侧共享的 multi-block-`run` 缺口**：guest `cgs_result_return_hex`（`:4788-4790`）拒任何 >1 基本块的 `run`、`cgs_run_body_hex` 发 `terms=0`（无分支 terminator）；formatter `main` 的 `if sico.bytes.is_utf8(...)` 恰好使 `run` 多块；**Rust oracle 同拒**（`Unsupported{function:"run",feature:"stdlib helper layout"}`，`lib.rs:4408-4412`）⇒ **非 guest 独有缺口** ⇒ **W3-a 关缺需两侧 multi-block-`run` 发射（分支 terminator + 多 return result-area 推导），属 owner 级设计决策点，尚未开工**。新钉测试 `sico_is_utf8_lowers_byte_exactly_but_the_gated_run_emission_is_refused`（双侧拒收定位：oracle `stdlib helper layout` vs guest `unsupported codegen source shape`）。词表刷新：`emitter-vocabulary-v1.json` +`is_utf8`、`guest_emitter.source_sha256` 刷新，`--check` 绿，`EMITTER_VOCABULARY_CURRENT sites=340`。验证：coordinator 独立 `--exact` 亲跑 **7 守卫全绿**（新钉 + step0507/0513/0519/0520/0526 + 整份 formatter IR pin）；fmt/clippy 净。诚实收窄：臂无法端到端行使直至两侧 multi-block-`run` 落地（STEP-0608 整文件探针仍 `unsupported codegen source shape`）；全量套件未跑；W3-a 仍 open；**未翻任何门，M22 overall 仍 NO-GO**。见 [`STEP-0610`](./docs/steps/STEP-0610-m22-w3a-is-utf8-emitter-arm.md)。
- **仍属 owner 待裁（2026-11-08 更新：其中 D-A/D-B/D-C 三项已裁决，见上一条 addendum；余项仍开放）**：STEP-0576 文档治理（README 20 组同号 / 0175 重编号 / 0440–0485 悬空引用）处置方式（重编号是 owner 裁决项，**不得自动执行**）。

### 10.2 Global Constraints

- 不修改 M22 七门、ADR-0015 的 `A == B == C`、证据等级或 Rust oracle；不预留 STEP 号，开工时按共享序列顺取。
- 每项实现先写会失败的差分/拒绝测试，再写最小实现；合法源逐字节等同，范围外和畸形输入 typed refusal，禁止静默近似。
- 真实 S 站点、合成形状、冻结语料是三种不同覆盖口径，分别报告，不得互相替代；无逐站点证据时只能写“候选覆盖集”。
- `selfhost/parser.sico` 与 `selfhost/codegen.sico` 同时只允许一个写入方；每次开工和收尾记录 HEAD、worktree diff 与文件 SHA，发现并发改动后重新验证最新快照。
- 可复现验证器必须版本化到 `tools/`；不得只依赖 `target/tmp` 脚本或日志。文档只在命令最终 exit 0 后写结果，提交/推送状态以本地 SHA 和远端 SHA 为准。
- 保留无关 dirty/untracked 文件；三个遗留 scratch 文件不得进入候选提交，清理须另行确认。仅在 owner 明确授权后 commit/push，CI 必须绑定精确 SHA。

### 10.3 Review Focus

- 真实 S 的复杂 case 体、嵌套控制流和多形参函数：必须由站点账本逐条执行，而不是由 4 个合成 fixture 代替。
- 同槽数但不同类型的 Result 投影、非 variant 值上的 variant 模式：必须同时被 guest 与 Rust oracle 拒绝。
- `checked_add` 溢出、`checked_sub` 下溢、ok/error 投影及多槽 Result：必须覆盖实际字节和拒绝边界。
- 1 MiB 级 IR/Component/hex 中间量：必须保持线性组装并记录预算；失败后不得上调限额追认。
- 178 条拒绝、hostile manifest、未知字段、重复路径、截断、尾随数据和 limit+1：必须核对稳定身份、span 与 fail-closed 结果。

### 10.4 R0：STEP-0535 证据整改（先于任何新能力片）

**Files:** `selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、`runner/sico-runner/tests/fixtures/`、`tools/validate-m22-s5.ps1`、`tools/probe-m22-match-coverage.ps1`、`selfhost/match-component-coverage-v0.json`、`docs/reports/m22-support-surface-match.md`、`docs/STATUS.md`、`docs/handoff-m22.md`、`docs/steps/STEP-0535-*.md`。

- [ ] **R0.1 修正声明。** 把“42/519 形状面闭合”降为“4 个合成形状通过、42 个候选站点待复核”；同步修复登记表遗留的 `0/519`、旧 CI 链接和 STATUS 的 STEP-0320/旧审计链接。历史 STEP 保留原文，以 addendum 订正；当前 STATUS/plan/handoff 必须使用最新口径。
- [ ] **R0.2 恢复可复现电池。** 新建版本化 `tools/validate-m22-s5.ps1`，串联 fmt、`run-ci.ps1`、0245、0262、前端/IR 前沿探针、comment discipline、`git diff --check` 和真实站点覆盖；消除依赖 `target/tmp/step0526/integrity.py` 的硬编码 dirty-list。每步记录命令、耗时和 exit，任何一步失败即整体失败。
- [ ] **R0.3 先写畸形 IR 拒绝测试。** 在 runner 测试侧建立 canonical-IR 注入夹具，不新增产品 CLI、不改 Rust oracle：同槽数异型投影、非 variant 值模式、非法嵌套 payload 均要求 guest typed refusal 且 Rust 拒绝。随后为 guest 增加完整 flat-type 比较和 variant-layout 检查，跑至测试转绿。
- [ ] **R0.4 建立真实站点账本。** `match-component-coverage-v0.json` 先登记 42 个候选站点的 path/function/head/case-shape/verdict/digests；探针只读冻结 S，逐条比较 guest/Rust IR 与 Component，并跑真实 validator。`is_ident_byte`、`type_json` 必须单列；未过项保持 typed refusal 或待实现，不得改写源文件规避。
- [ ] **R0.5 补齐布局测试。** 覆盖 ok/error 投影、Result cell、布尔模式、I64 checked 的明确拒绝、U64 add 溢出和 sub 下溢；断言 guest/Rust 字节或双方拒绝身份，不只断言组件长度。
- [ ] **R0.6 重跑并同步证据。** 下列命令全绿后才可把 STEP-0535 记为本机完成；保留原始日志路径，handoff 的 commit/push 句子只有在远端 SHA 实际出现后才可以保留。

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& $cargo fmt --all --manifest-path .\runner\sico-runner\Cargo.toml -- --check
& $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- -D warnings
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1
.\tools\validate-m22-s5.ps1
git diff --check
```

**R0 exit:** 173+ 固件、版本化电池和 42/42 候选站点均有当前快照证据；畸形 IR 测试绿；文档只声明实测范围；STEP-0535 独立 CI 仍须由精确提交快照取得。

### 10.5 R1：闭合 `match` 与共享算子关键路径（S5-W2 前半）

**Files:** `selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、`selfhost/match-component-coverage-v0.json`、`docs/reports/m22-support-surface-match.md`。

- [ ] **R1.1 M-g3'：** 实现 `sico.list.get` 的宿主调用传输，15 个真实站点逐条 IR/Component 全等；泛型、越界和错误臂保持 typed outcome。
- [ ] **R1.2 M-g1：** 移除 checked-arithmetic 路由的“恰好 2 形参”限制，354 个站点按真实形参计数复核；不得改变既有值 id、SSA 或 range。
- [ ] **R1.3 M-g2：** 补 `checked_sub`、`checked_mul`、`checked_div` 前端及发射，46 个站点加溢出/下溢/除零边界全部通过。
- [ ] **R1.4 M-g3：** 补 `sico.bytes.slice` 匹配头，21 个站点逐条复核；非法范围仍由既有 typed error 传播。
- [ ] **R1.5 共享算子：** 按实测阻塞补 `U64.less_than` 及 `U64.equal` 之外的固定算子发射，并闭合 `T2-g3` 的 7 个操作数位置；每个算子核对名称、元数、类型、结果槽数四道闸门。
- [ ] **R1.6 族级收口：** 将账本扩至 519 个 `match` 站点，519/519 均有前端和组件 verdict；成功项逐字节全等，拒绝项身份一致，validator 通过，独立 CI 绿后才能宣告族闭合。

**R1 exit:** `match` 六项族级判据全部满足，T2 的 332 个站点重新复核；不得以形状代表抽样替代 519 站点账本。

### 10.6 R2：完成剩余宿主族与整源前端（S5-W2 后半）

- [ ] 对 T3–T20 逐族执行既有“真值 T → 前端 F → codegen C → 收口 X”四拍；D9 串行修改热点，D10 不允许在 S 尚有真实缺口时降级。
- [ ] 每族登记表固定名字、元数、实参类型、结果槽数、真实调用点、冻结语料覆盖和未覆盖形状；真实 S 调用点全部有 verdict，不能只测最小 fixture。
- [ ] 整份 `codegen.sico`、`compiler.sico` 和其余 9 文件冻结 S 均经 guest 降低为 Rust canonical IR；任何剩余拒绝都进入具名缺口，不得宣称整源自举。
- [ ] 复测 1 MiB 规模曲线、`cgs_data_offset` 等已知二次路径；若超过事前预算，按 ADR/RFC 先修表示或算法，失败后才允许登记新 attempt。

**R2 exit:** S 内实际使用的宿主族和语言形状全部满足 IR 差分及对应组件差分；W2 支持面汇总无“语料覆盖 0 条”的完成族。

### 10.7 R3/R4：冻结语料与完整 bundle（S5-W3/W4/W5）

- [ ] **W3-a：** guest 对 `formatter.sico` 的 Component 与 Rust 生产输出逐字节等同并通过 validator。
- [ ] **W3-b：** 37 条 accepted 语料逐条满足 guest Component == Rust 生产输出 == 冻结 `component_sha256`，不得以组内抽样代替。
- [ ] **W4：** 新建 `tools/probe-m22-corpus-parity.ps1` 和 `selfhost/corpus-parity-v0.json`，178 条拒绝逐条核对 `(verdict, diagnostic id, span)`；116 LEXICAL、28 SCRIPT_ABI、34 散码分别出账。
- [ ] **W5：** 依次完成 5 文件 bundle 和 9 文件全集 Component 全等；A 的产出脚本、源 SHA、组件 SHA 和预算记录版本化，1 MiB 级输出仍满足事前限额。

**R3/R4 exit:** 门 2 才能由 PARTIAL 申请 GO；此时仍不得把 bundle 全等当作门 3 的 `A == B == C` 执行证据。

### 10.8 R5：ADR-0015 自举闭环与门 4 实测（S5-W6）

- [ ] 固化 `A = R(S)`、`B = run(A,S)`、`C = run(B,S)` harness；三者 Component 校验通过且字节全等，失配产生带 stage/digest 的 typed bootstrap failure。
- [ ] 用 R 与 B 双跑完整冻结语料，产物字节和 typed refusal 全等；Rust IR verifier 始终保留。
- [ ] 将 B 经 RFC-0015 canonical `.sapp`、RFC-0016 显式本地信任和 ADR-0006 发布演练打包，由 `sico-app` 真实执行自编译；guest 无 ambient 权限。
- [ ] 证据清单覆盖源/编译器/工具链/target/语料/A/B/C/package/signer 摘要，以及未知字段、重复路径、穿越、截断、尾随、溢出和 limit+1。
- [ ] 对 **B(S)** 记录 cold/warm wall time、fuel（无读数则明确标上限）、峰值 guest memory、hostcall 数/fuel、输入输出字节、timeout/cancellation，并登记 M19 吞吐对照。

**R5 exit:** 门 3 才可申请 GO；门 4 只有上述 B(S) 记录完整后才可恢复 GO，IR-only 的 25.93s 不再作为自编译完成证据。

### 10.9 R6：最终回归、独立 CI 与七门复审（S5-W7）

- [ ] 在干净、唯一、已提交的候选 SHA 上运行 `tools/run-ci.ps1`、runner 全工作区、版本化 M22 电池、全部站点/语料探针和模块边界检查；不得混入 scratch 或并发未提交改动。
- [ ] 推送仅经 owner 授权；独立 `m22-w1-ci.yml` 在相同 SHA 上最终 completed/success，记录 run id、head_sha 和日志。红灯必须取得失败步骤后再归因，不能仅凭后续绿灯断言环境抖动。
- [ ] 新建出口审计逐门裁决 1–7，附证据矩阵和剩余缺口；门 6 要求完整候选 M0–M21 回归绿，门 7 要求每项声明可回溯到执行证据。
- [ ] 同步 STATUS、ROADMAP、M22 plan、handoff、steps README 和支持面登记；历史报告用 addendum 保留可审计性。按 D14，不与 `origin/dev` rebase/merge，集成留 M25。
- [ ] 只有七门全部满足才将 M22 改为 GO；任一缺项保持 NO-GO。R3 止损或外部证据缺失不得包装成自举完成。

**最终验证：**

```powershell
.\tools\run-ci.ps1
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml -- --test-threads=1
.\tools\validate-m22-s5.ps1
.\tools\probe-m22-match-coverage.ps1
.\tools\probe-m22-corpus-parity.ps1
.\tools\validate-module-boundaries.ps1
git diff --check
```

### 10.10 已完成清单（仅列已有执行证据的工作）

> 这里的“完成”指对应工作片或证据项完成，不代表 M22 或其所属出口门完成。

- [x] **M22 入口与自举合同**：S0 语言前置、ADR-0015、ADR-0016、RFC-0047 和双实现纪律已建立；Rust 前端、IR verifier、后端继续作为永久 oracle（门 5 GO）。
- [x] **L1 formatter/checker 声明子集**：formatter 冻结差分和幂等、checker 声明子集语料电池绿；formatter canary 30/30、full-source exit 0（门 1 限声明子集 GO）。
- [x] **`parser.sico` 前端 IR 差分**：census 149/149 函数闭包字节全等；整份 `parser.sico` 经 guest 降低的 canonical IR 与 Rust oracle 字节全等。
- [x] **S5-W1 规模可行性测量**：一次成型的线性组装在既定限额内完成 1 MiB 输出，判读为“可行”；增量拼接和简单分块的二次增长边界已登记。
- [x] **初版 Script Component 发射链**：参考形、error 臂、多函数、用户调用、`sico.bytes.length`、局部 cell 和分支等已测形状均有生产后端逐字节对照。
- [x] **T1 `sico.text.concat` 族**：声明形状的真值表、IR/Component 差分、validator、前沿探针、固件和独立 CI 六项族级判据已闭合；这不代表整文件或 M22 闭合。
- [x] **T2 的已测子形状**：语句位、嵌套调用实参及发射端已测单元格有差分证据；当前登记的前端覆盖为 325/332，但 T2 整族尚未闭合。
- [x] **`match` 前端局部覆盖**：57/519 个 S 站点的已测形状可降，包含 `U64.checked_add` 小形参函数和 `sico.list.get` 系列。
- [x] **STEP-0535 的 4 个合成形状**：2 形参 set/return 1215 B、1 形参 1214 B、return/return 1167 B、嵌套 match 1389 B，guest Component 与 Rust 生产输出逐字节等同；`selfhost_compiler` 最新复跑 173/173、runner clippy 和 fmt 通过。
- [x] **STEP-0534 独立 CI**：精确 SHA `54b6d9758a4709f5e6fb9d6982854704fbcbd285` 的 run `40053245939` 为 completed/success；该证据不外推到 STEP-0535。
- [x] **P0/P1/P2 装配与尺度片（STEP-0548/0549/0550）**：D1 绿（1 MiB 分片装配 42.5 M fuel / 38 ms）；四个升维折叠改分片 + 一次 join（64 KiB 字面量从 trap 变为字节全等）；闭包上限放开到 256 fn/1 MiB 后 396 站点首次全部有 verdict，回绿 0/364 ⇒ **D2 结构性**、下一步 B2。组件轴仍 32/396，七门不变。**口径提醒**：词表 artifact 同步扩到 340 站点，`sico.list.length[U64]` 缺 7 站点（`general_while_function_ir`）。
- [x] **B2 值索引片（STEP-0551）**：`selfhost/codegen.sico` 每函数建一次 stride-2 `List[U64]` 值索引（`2k` 槽号 / `2k+1` 类型偏移，poison 语义逐条复刻），`cgs_value_slot`/`cgs_value_ty` 改 O(1)，两个发射根改游标遍历，新增 canonical id 前置门 `cgs_value_ids_refusal`（稀疏/重排/前导零 ⇒ `ERR:E-SH-CG-IR-EMIT`）。95 组 **6,040.99 s** 复算：回绿 **4/364 = 1.10%**（`parser.sico:fixed_literal_const_segment`，全部来自既有 140；P2 新开测 195 个 **0 回绿**），既有 32 全等站点逐字节不变、无退化，组件轴 **36/396**；七门不变、M22 仍 NO-GO。**残差可数**：`cgj_array_item` 仍 88 处（45 处在循环内），加每调用全模块查询 ⇒ 下一刀 **B3**。
- [x] **IR-only 预算观察**：整份 parser IR 降低已有 wall-time 和限额内通过记录；它只是门 4 的前置观察，不是 `B(S)` 完整自编译预算。

### 10.11 未完成清单（当前必须继续做的工作）

#### A. STEP-0535 立即整改

- [ ] **完整可信电池**：版本化 `tools/validate-m22-s5.ps1`，消除临时 integrity 脚本问题并全步 exit 0；当前 `target/tmp/step0535/battery.log` 不能支持 `BATTERY_GREEN 23/23`。
- [ ] **真实 42 站点复核**：逐条运行候选账本，覆盖 `is_ident_byte`、`type_json` 等复杂 case 体；在复核前不得宣称“42/519 形状面闭合”。
- [ ] **畸形 IR fail-closed**：补同槽数异型 Result 投影、非 variant 值模式、非法嵌套 payload 的双方拒绝测试及实现。
- [ ] **布局与边界补测**：ok/error 投影、Result cell、多槽 Result、布尔模式、I64 checked 拒绝、U64 溢出/下溢均需明确证据。
- [ ] **文档与候选卫生**：修正旧 STATUS/CI/覆盖声明，保持历史 addendum；三个 scratch 遗留文件不得进入候选；STEP-0535 尚需精确 SHA 的独立 CI。

> **出口阻塞红测现状（2026-11-08 addendum，详见 §10.1.4）**：原 5 项出口阻塞红测已收敛到 **3 项**——`step0519`/`step0520` 由 STEP-0605 配方落地转绿、`step0507`/`step0526` 由 STEP-0609 的 `direct_list_empty_return_ir` 转绿（**两者同根因**：返回位置无参 `sico.*` 内建无法降低，STEP-0602/0604/0605 所记「`step0526` = 独立非 Output 嵌套 `while`」作废）；**原 5 项中只剩账本 firmware（MS-081 燃料墙）族仍红**。诚实口径：STEP-0609 仅跑 8 个 `--exact` 测试（非全量套件、非账本慢速 95 组），账本 `--check` = `LEDGER_CURRENT sites=396`、HEAD-vs-worktree verdict+sha256 多重集相同（纯行号位移 229 站）。**未翻任何门。**

#### A2. 电池基础设施与 STEP-0545 收口（2026-10-26 新增）

- [x] **分层策略落地（owner 决策）**：`tools/validate-m22-s5-dev.ps1`（开发期子集，`-Set fast|probes|firmware|all`）+ `tools/m22-s5-fingerprint.ps1`（修正后的指纹：`target` / `__pycache__` / `*.pyc` 三项排除，STEP-0545 §6.1 的自污染修复入库）⇒ 见 §10.1.2。
- [ ] **可续跑封装器入库**：可续跑包装器（含状态文件、逐拍落盘、指纹闸门）仍只在 `target/tmp/`，未版本化到 `tools/` ⇒ 下一个电池仍可能踩同一个坑（§10.2 要求可复现验证器必须版本化）。
- [x] **STEP-0545 电池第 13 拍必红（已归因，实测）——已于 2026-11-07 关闭/过期（见 STEP-0604）**：`selfhost/emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` 仍是 B1 修复前的 `da0fec5f…`，而当前 `selfhost/codegen.sico` 是 `3d8705b1…` ⇒ `census-m22-emitter-vocabulary.py --check` 判 `EMITTER_VOCABULARY_RED`。**词表本身未变**（145/145 站点逐条相同，helpers / intrinsics / ops 三集合全同）⇒ 只是源码指纹过期。该工件**只被 `tools/census-m22-emitter-vocabulary.py` 读**（未进任何 `include_str!`）⇒ 刷新它不影响其它拍。处置：在 STEP-0545 的 worktree 内 `--update` 刷新 + 记录归因 + 续跑第 13 拍。 **【2026-11-07 关闭】本条前提已过期：`emitter-vocabulary-v1.json` 行 168 的 `guest_emitter.source_sha256` 现为 `013f2d7e…`，逐字符等于当前 `codegen.sico` sha ⇒ 不再判 `EMITTER_VOCABULARY_RED`，词表指纹已同步、无需刷新；详见 [`STEP-0604`](./docs/steps/STEP-0604-m22-exit-blocking-red-tests-diagnostic-reclosure.md) §3.4。**
- [ ] **主树脏文件清理**：`selfhost/codegen.sico`（= step0545 修复副本）、`runner/sico-runner/tests/selfhost_compiler.rs`（**旧草稿**，`fmt (runner)` 红；权威版是 `4449efa` 的 436 行版）、`AGENTS.md` §10、`tools/` 两个新工具。

#### B. S5-W2 核心能力与真实覆盖

- [ ] **M-g3'**：`sico.list.get` 宿主调用传输及 15 个真实站点。
- [ ] **M-g1**：checked-arithmetic 形参计数门及 354 个站点。
- [ ] **M-g2**：`checked_sub`、`checked_mul`、`checked_div` 前端和发射及 46 个站点。
- [ ] **M-g3**：`sico.bytes.slice` 匹配头及 21 个站点。
- [ ] **共享算子和 T2 残余**：`U64.less_than` 等发射缺口、T2-g3 的 7 个站点及其跨族依赖；T2 整族仍未闭合。
- [ ] **完整 `match` 族**：519/519 真实站点账本、六项族级判据和独立 CI；当前只有 4 个合成 Component 形状通过。
- [ ] **T3–T20 与剩余语言形状**：逐族完成 T/F/C/X 四拍和真实 S 调用点覆盖；不得以最小 fixture 代替族闭合。
- [ ] **9 文件整源前端**：`codegen.sico`、`compiler.sico` 及其余冻结 S 均需 guest canonical IR 全等；目前只有 parser/formatter 等已声明范围完成。

#### C. 冻结语料、组件尺度和自举闭环

- [ ] **W3-a**：`formatter.sico` 整组件与 Rust 生产输出逐字节等同。 **【2026-11-08 addendum：已首次测量（STEP-0608，仅测量片）——整份 `formatter.sico` 经 guest `--emit-component-hex`（标准预算，未上调）⇒ typed refusal `ERR:E-SH-CG-IR-EMIT`，仅耗 ~0.03% fuel ⇒ 与燃料墙正交。具名缺口 = `sico.bytes.is_utf8` 被用作 `if` 条件（`formatter.sico:818`）；guest 发射前即拒绝、无字节可比对 ⇒ 需补一个发射器臂（本片未做），W3-a 未闭合。】**
- [ ] **W3-b**：37 条 accepted 语料逐条 Component 全等并匹配冻结 SHA。
- [ ] **W4**：178 条拒绝逐条核对 `(verdict, diagnostic id, span)`，包括 116 LEXICAL、28 SCRIPT_ABI、34 散码。
- [ ] **W5**：5 文件 bundle 和 9 文件全集 Component 全等，同时满足事前预算和线性组装要求。
- [ ] **W6 固定点**：执行 `A = R(S)`、`B = run(A,S)`、`C = run(B,S)`，三者 Component 校验且 `A == B == C`。
- [ ] **M7 信任链**：B 的 canonical `.sapp`、显式本地信任、发布演练、`sico-app` 自编译，以及 hostile manifest/limit+1 全部完成。
- [ ] **门 4 完整预算**：对 B(S) 记录 cold/warm wall、fuel、峰值内存、hostcall、I/O、timeout/cancellation 和 M19 对照；IR-only 数据不能替代。

#### D. 最终出口

- [ ] **门 2**：上述冻结语料和全部 artifact 全等后才可由 PARTIAL 申请 GO。
- [ ] **门 3**：固定点与 `.sapp` 执行证据完成前保持 NO-GO。
- [ ] **门 6**：干净唯一候选 SHA 的 M0–M21 全量回归及独立 Windows GNU CI 最终 success。
- [ ] **门 7 与 S7**：所有声明映射到当前执行证据，形成新的七门出口审计；七门全部满足前 M22 overall 不得改为 GO。
- [ ] **交付登记**：汇总支持面、同步 STATUS/ROADMAP/plan/handoff/steps README；`origin/dev` 集成按 D14 留 M25，不作为 M22 完成的替代条件。
