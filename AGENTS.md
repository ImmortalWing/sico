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
- M22 编译器自举轨 in progress / 出口 NO-GO：主动线 `codex/m22-w2` 已于 2026-11-06 并入 `dev`（merge commit `7bcdc35`）；R0 已收口（owner 审计 2026-10-24）、W3-a 已闭环（STEP-0626）；门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不沿用 GO。逐步执行状态与燃料墙判读**只以 [`docs/STATUS.md`](./docs/STATUS.md) 与 [`M22 计划`](./docs/plans/M22-compiler-self-host.md) 为准**（摘要见 §10.1.4 执行脉络表），本文件不复制每步易变的数字。
- M23 language v1 batch 3：项 1/2 已落地（RFC-0048/RFC-0049，STEP-0328/0329），出口审计 item 1 GO / item 2 GO / item 3 deferred-external / item 4 closed（[M23 审计](./docs/reports/m23-exit-audit-2026-09-30.md)）；item 3 的 owner 凭据语料复测未关闭，不得读作 M23 整体 GO。
- M24 vision closure + GUI 格式转换器（含 Sico 原生 UI 库工作流）：实现切片 STEP-0330~0338 已落（合同复裁、#2 PNG、`#5 ui@1` 绑定+默认拒绝授权、出口审计、gate 4 provenance schema、gate 3 roster 三包全绿、**§8.6.1 Fluent-subset Direct2D 渲染器 + 冻结 token 表 + 确定性帧语料**；实施细节——完整 `windows` crate 钉 0.52 走预编译 import lib、DIB 须 `BITMAPV5HEADER`+`BI_BITFIELDS`+BGRA mask、像素格式须 `B8G8R8A8_UNORM`+`PREMULTIPLIED`——见 [STEP-0338](./docs/steps/STEP-0338-m24-native-ui-fluent-d2d-renderer.md)）。**但仍只是渲染器核心：OS 窗口壳/事件泵/runner 接线未做 ⇒ 无 guest 可见 UI 能力，无任何 §3 gate 翻转**；M24 overall 本机仍 NO-GO（#3 JPEG 与 gate 1 加速器为具名外部门，gate 4 余「预算运行期强制+取消」）。**ADR-0018 状态两线登记不一致**（dev 线 STEP-0323 按 owner 指令裁决 accepted；m22-w2 线登记「仍为 proposed」）——按 §1 取下界声称（不当已接受），差异留 owner 裁决。
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

#### 10.1.1 Owner 工作量看板（先看这里）— 摘要
> 原看板（R0–R6 台账 / 判读规则 / 65–120 片估算）已移至[归档](./docs/reports/m22-agents-addenda-archive.md)；看板自 2026-10-26 起降为残差台账，执行顺序以 §10.1.3 / §10.1.4 为准。

| 阶段 | 状态 |
|---|---|
| R0 证据整改 | **已收口**（owner 审计 2026-10-24） |
| R1 `match` 与共享算子 | 进行中：M-g1 门已拆、B4 已转绿；519/519 未闭合 |
| R2 宿主族与整源前端 | 未闭合（T3–T20 未动） |
| R3/R4 语料与 bundle | W3-a 已闭环（STEP-0626）；W3-b/W4/W5 未开始 |
| R5 自举与预算 | 未进入 |
| R6 最终出口 | 未进入（门 6 有 2026-10-25 GO 快照，终裁未做） |

#### 10.1.2 Owner 决策（2026-10-26）：电池分层
| 层 | 工具 | 何时跑 | 证据效力 |
|---|---|---|---|
| 开发期子集 | `tools/validate-m22-s5-dev.ps1`（fast ≈1 min / probes ≈12 min / firmware ≈28 min） | 每次 guest 改动 | 非出口证据 |
| 出口整轮 | `tools/validate-m22-s5.ps1`（15 拍 ≈2.7 h） | 族收口片 X / 候选 SHA / 门 6、门 7 出口 | 出口证据，缺一不算 |

分层只改节奏、不降判据（族级判据、退出门、证据等级、独立 CI 不变）。**原决策二（下一步 B2）已被 §10.1.4 判据取代**：B2 完成（回绿 4/364）、B3 实测否决；指纹复用规则见 `tools/m22-s5-fingerprint.ps1`。

#### 10.1.3 复审（2026-10-26，STEP-0547）：装配策略优先于形状覆盖 — 摘要
全量复核见 [`m22-review-2026-10-26.md`](./docs/reports/m22-review-2026-10-26.md)：门 2/3 瓶颈 = `codegen.sico` 的 `sico.text.concat` 左折叠装配（O(N²)），非形状覆盖 ⇒ 执行序 P0 尺度预检 → P1 装配重写 → P2 批量复算 → P3–P6。**现状**：P0–P2 已完成（D1 绿；D2 = 结构性，回绿 0/364），P3–P6 未做。Standing 约束：看板降台账；不新增 per-shape packer；不上调 fuel/内存限额追认；`parser.sico:scalar_ir` 24 站受 runner 8 MiB stdout 上限阻挡（不得直接上调）；ADR-0017 Option A 挂起至 M25 之后。原文与 P0–P6 决策门表见归档。

#### 10.1.4 复审（2026-11-07，主线评审）：B3 已被实证否决；燃料墙改判「发射器层 · 常数主导」；主路径改判

> 依据：STEP-0555 §3.1/§4/§10.6、STEP-0556 §3/§10/§11、STEP-0562、STEP-0567、STEP-0572 §1/§5、STEP-0575 §3.2–3.3/§4.4、2026-11-05/06 STATUS 条目。**本节为 addendum 订正，§10.1.1–§10.1.3 原文保留**。**2026-11-18 两轮精简**：原判定点 4 条与 12 条 STEP-06xx 执行结果原文已逐字移至[归档](./docs/reports/m22-agents-addenda-archive.md)，本节只留当前判定、执行脉络表与 owner 裁决。

- **B3 = 已否决，不应重开**（STEP-0555）：FuelExhausted 331→344、component-equal 36→23，0 回绿 / 13 回退；游标化 fuel 净负、模块索引 ≈0.01% 且 331/335 耗尽站点全在 `parser.sico:scalar_ir`（不调 codegen helper）⇒ 触达不到墙；§10.1.2 决策二与 §10.1.3「B3 为主路径」前提失效。
- **燃料墙**：墙在发射器层、fuel ~ IR²；STEP-0606 归因 = **match-dispatcher 重扫 + `text_concat` 装配**两个具名超线性算子，STEP-0611 钉死热叶 `cgj_value_end`（intrinsic-membership 三级重扫驱动）。
- **口径（历史，2026-11-07）**：组件轴 = 335 exhausted / 32 equal / 24 trap(8 MiB) / 5 refused-guest；W4 parity 剩余 3/150 经 STEP-0600 实测**并非燃料墙**。上列数字已被 STEP-0626 部分翻转（MS-081..084 → equal）⇒ 当前站点轴一律以账本 `LEDGER_CURRENT` 与 STATUS 为准。
- **新主路径（2026-11-07 判定 → 2026-11-08 owner 裁决取代）**：① STEP-0579 已跑：match 密度分账无判别力 ⇒ match 主刀否决；② op 级剖析已落地（STEP-0611，D-A）→ 优化刀落地中（0625 intrinsic mask、0626 W3-a 闭环）；D8 限额上调被 D-B 正式否决（**不得无证据上调**）。

- **执行脉络（2026-11-18 精简）**：上列各 STEP 执行结果的完整原文（含每条「诚实收窄」）已逐字移至 [`docs/reports/m22-agents-addenda-archive.md`](./docs/reports/m22-agents-addenda-archive.md)；逐步权威登记以 [`docs/STATUS.md`](./docs/STATUS.md) 与 [M22 计划](./docs/plans/M22-compiler-self-host.md) 为准，本节不复制每步易变数字（与 §2 口径一致）。下表只留判定级结论：

| 日期 | 片 | 判定级结论 |
|---|---|---|
| 2026-11-07 | STEP-0579 | match 密度分账「字面成立、无判别力」（密度↔span Spearman ρ=0.883，尺寸梯度混淆）⇒ **match 主刀否决**；M22 收敛为 op 级燃料剖析 / owner D8 二选一；附文档治理只读清单（README 26 组同号等，处置属 owner 裁决） |
| 2026-11-07 | STEP-0602/0604 | 出口阻塞红测定性 = guest 前端「快速 typed refusal」族，与燃料墙正交；0604 三处订正（账本同步冻结非当下红、emitter-vocabulary 前提关闭、整测试 wall-clock 上修 5.5–7.0 s） |
| 2026-11-07 | STEP-0605 | STEP-0571 §3 配方落地 ⇒ `step0519`/`step0520` 转绿（逐字节等同）；账本纯行号位移重冻结 |
| 2026-11-07 | STEP-0606 | 燃料墙归因 = **match-dispatcher 重扫 + `text_concat` 装配**两个具名超线性算子（fuel ~ IR²）⇒ **D8 后果：上调限额买不回站点**；真正杠杆 = 降两算子复杂度 |
| 2026-11-08 | STEP-0609 | `direct_list_empty_return_ir` ⇒ `step0507`/`step0526` 转绿（同根因，订正旧登记）⇒ 出口阻塞红测 5→3，剩账本 firmware（MS-081 燃料墙）族 |
| 2026-11-08 | STEP-0608/0610/0612 | W3-a 整文件测量链：`is_utf8` 臂落地 → 真挡板 = **guest-only multi-block-`run`**（0612 订正 0610「两侧共享」——oracle 早已可发整份 formatter）→ 切片 O1→G3；登记 W3-b 杠杆（42 accepted 中 37 条含控制流） |
| 2026-11-08 | STEP-0611 | op 级归因落地（D-A 授权；gcc 未安装未需要）：烧点钉死 **`cgj_value_end`**（intrinsic-membership 三级重扫热叶）⇒ 下一刀 = intrinsic 查询族每模块一次预计算 |
| 2026-11-17 | STEP-0623 | variant 路由前导 `let` 物化 ⇒ B4 转绿（逐字节等同）；账本 403→405 外科式重冻结 |
| 2026-11-18 | STEP-0625 | intrinsic-membership mask 降燃料（字节恒等）⇒ 整份 formatter 从 `FuelExhausted` 翻为快速 typed refusal；揭出 **map 系 helper 无发射臂**（直连 `ERR:E-SH-CG-IR-FLAT`） |
| 2026-11-18 | STEP-0626 | **W3-a 闭环**：map/list cell + `map.get/put/empty[Text,U64]` 臂 + 三处游标 ⇒ 整份 `formatter.sico` **28,459 B 与 Rust oracle 逐字节等同**（总燃料 486.2e9 = 标准预算 800e9 的 60.8%；validator pass）；MS-081..084 翻转 `ir-equal+component-equal`；同步产出全仓 12 项隐藏缺陷与优化方向审计（A1–A5/B1–B2/C1–C5，按 owner 指令只登记不改码）；**M22 overall 仍 NO-GO** |

- **owner 三项裁决（2026-11-08，addendum）——D-A 授权 / D-B 否决 / D-C 解除暂停**：owner 就 §10.1.4 收敛出的两个悬决项拍板，本条只做登记（纯登记/文档片，零代码/零 WIT/零测试/零语料/零限额改动）。①**D-A（op 级燃料仪器化剖析 + gcc）已授权、进行中**：`runner/sico-runner` 生产侧 fuel 读数/epoch·opcode 钩授权与本机 gcc 安装授权均已给出，剖析工作由另一代理在跑（本登记片不跑 cargo/剖析）。②**D-B（燃料限额上调 / D8 重基线）正式否决**：**不上调 fuel 限额**——依据 **STEP-0600** 实测 2× 燃料（1.6T）全量 331 站点只转绿 **5/331 = 1.51%**；**STEP-0606** 证明二次增长下「限额翻倍只多解锁一步 n」⇒ **上调限额买不回站点**，真正杠杆是降低 match 重扫与 `text_concat` 装配的复杂度。③**D-C（M22 暂停）解除**：2026-10-30 的「M22 暂停」（只允许测量/诊断片、不允许特征实现片）**正式解除**，`selfhost/parser.sico` / `selfhost/codegen.sico` 热文件可开特征实现片，**仍须遵守 §10.2「两热点文件同时只允许一个写入方」与逐片纪律（HEAD/diff/SHA 记录、并发改动后重验证）**。**由此，上列 §10.1.4 各条中「在 owner 裁决前只允许测量片/诊断片，不允许特征实现片」「须先解除暂停」「D8 限额裁决待裁」等表述自 2026-11-08 起已被本裁决取代**（历史叙述保留、以本 addendum 订正为准）；**「下一刀」据 D-B/D-A 订正为：op 级燃料剖析 → 据剖析结果定优化刀（match-dispatcher 重扫 / `text_concat` 装配二选一或并进），不再走 B3（§10.1.4 已实测否决）、也不走限额上调（D-B 否决）**。**门状态不变**：未翻任何门，M22 overall 仍 NO-GO。

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

### 10.4 R0：STEP-0535 证据整改 — **已收口**
2026-10-24 owner 审计 **R0 exit 达成**：R0.1 修正声明 / R0.2 版本化电池（`tools/validate-m22-s5.ps1`）/ R0.3 畸形 IR fail-closed / R0.4 真实站点账本（v1）/ R0.5 布局边界补测 / R0.6 证据同步全部交付。原 checklist 与验证命令见归档；**R0 exit ≠ M22 GO**。

### 10.5 R1：闭合 `match` 与共享算子关键路径
目标：M-g3'（15 站）/ M-g1（354 站，门已拆）/ M-g2（46）/ M-g3（21）+ 共享算子 + 519/519 站点账本族级收口。Files 与族级判据原文见归档及 [M22 计划](./docs/plans/M22-compiler-self-host.md)。

### 10.6 R2：完成剩余宿主族与整源前端
T3–T20 逐族 T/F/C/X 四拍；整份 `codegen.sico`/`compiler.sico` 及其余冻结 S 的 guest canonical IR 全等；1 MiB 规模曲线复测。原文见归档及 M22 计划。

### 10.7 R3/R4：冻结语料与完整 bundle
W3-a **已闭环**（STEP-0626，见 §10.1.4 表）；W3-b 语料逐条全等（当前语料 235 条 / accepted 42，开工前重计——STEP-0612）；W4 178 条拒绝对账；W5 bundle/全集。原文见归档及 M22 计划。

### 10.8 R5：ADR-0015 自举闭环与门 4 实测
`A == B == C` 固定点、R/B 双跑语料、`.sapp` 信任链、B(S) 完整预算（cold/warm wall、fuel、峰值内存、hostcall、M19 对照；IR-only 数据不能替代）。原文见归档及 M22 计划。

### 10.9 R6：最终回归、独立 CI 与七门复审
干净唯一候选 SHA 的 M0–M21 全量回归 + 独立 CI + 逐门新审计（门 6 已有 2026-10-25 GO 快照，终裁未做）。最终验证命令见 §8 与 `tools/run-ci.ps1`；原文见归档。

### 10.10 已完成清单（索引；证据级原文见归档）
- [x] **入口与自举合同**（门 5 GO）；**L1 声明子集**（门 1 限子集 GO）；**`parser.sico` 前端 IR 差分** 149/149
- [x] **STEP-0534 独立 CI**（run `40053245939` @`54b6d97`，不外推 0535）；**STEP-0535 四合成形状**逐字节等同
- [x] **T1 `sico.text.concat` 族**六项族级判据闭合；**T2 已测子形状**（前端 325/332，整族未闭）；**`match` 前端局部覆盖** 57/519
- [x] **P0/P1/P2 装配与尺度**（D1 绿 / D2 结构性）；**B2 值索引**（回绿 4/364 = 1.10%，既有 32 站恒等）
- [x] **W3-a 整份 `formatter.sico` 组件闭环（STEP-0626，2026-11-18）**：28,459 B 逐字节、486.2e9 fuel（标准预算 60.8%）、validator pass、MS-081..084 翻转
- [x] **IR-only 预算观察**（门 4 前置观察，非 B(S) 完整自编译预算）

### 10.11 未完成清单（当前必须继续做的工作）

#### A. STEP-0535 立即整改 — **已收口**（原五项清单与「出口阻塞红测」历史块见归档）
2026-10-24 owner 审计 R0 exit 达成；出口阻塞红测 5 项已全部转绿（`step0519`/`step0520`←STEP-0605，`step0507`/`step0526`←STEP-0609，账本 firmware MS-081 族←STEP-0626 翻转）；当前红测状态以固件实测与 STATUS 为准；emitter-vocabulary「第 13 拍必红」前提已关闭（STEP-0604）。

#### A2. 电池基础设施 — 剩余开放项
- [ ] **可续跑封装器版本化入库**（现只在 `target/tmp/`，违 §10.2）。
- [ ] **主树脏文件清理**（保留无关改动；三个遗留 scratch 文件不进候选提交）。

已落地：分层电池 `tools/validate-m22-s5-dev.ps1` + 指纹 `tools/m22-s5-fingerprint.ps1`；「第 13 拍必红」已关闭。原条目见归档。

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

- [x] **W3-a**：`formatter.sico` 整组件与 Rust 生产输出逐字节等同。**已闭环（STEP-0626，2026-11-18）**：整份 `A:main`/`B:run` 各发射 28,459 B、100% 逐字节等同、过 `wasmparser::Validator`（总燃料 486.2e9 = 标准预算 60.8%，未上调）；MS-081..084 翻转入账（`LEDGER_CURRENT sites=405`）。2026-11-08 测量史（is_utf8 缺口、拒燃料墙定性）见归档；详见 [STEP-0626](./docs/steps/STEP-0626-m22-w3a-formatter-component-closure-and-hidden-issues-audit.md)。
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
