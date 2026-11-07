# STEP-0595: M24 §8.6.1 — G5 **guest 级帧确定性**正式专测（纯证据片；不改生产语义）

> - status: implementation complete locally（**guest 级帧确定性专测新增**；不翻任何 M24 §3 gate；未 commit/push）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）、[`STEP-0590`](./STEP-0590-m24-ui-contract-amendments-draft.md)（验收条款 G5）、[`STEP-0591`](./STEP-0591-m24-guest-visible-ui-evidence.md)（附带确定性证据，本片升格为正式专测）——本 STEP 只消费、不修改
> - evidence: **runtime-verified window + internal-fixture guest**（Windows x64 真实 `CreateWindowExW` 窗口 + GDI DIB 呈现，经 hand-encoded guest Component 经真实 Runner 准备/运行路径；**非**试点级、**非**生产声明）
> - artifacts: [`runner/sico-runner/tests/ui_frame_determinism.rs`](../../runner/sico-runner/tests/ui_frame_determinism.rs)（新增）、本文件
> - **引用注意**：`runner/sico-runner/src/ui.rs`、`runner/sico-runner/src/lib.rs` 与 `crates/sico-ui-renderer-d2d/**` **正被并发代理编辑**（STEP-0592 布局/命中测试、并发代理在动）。本文件全部 `文件:行号` 均为 **2026-11-07 在 worktree 实测锚点**，会漂移；实现者须对已提交快照重新锚定后再引用。

## 背景

STEP-0590（验收条款）把「guest 可见 UI 能力」定义为 **G1 ∧ G2 ∧ G3 ∧ G4**，其中 **G5** 为确定性
伴随分项：**guest 同树渲染 N 次 → N 帧逐字节一致**（ADR-0018 §3 `ADR-0018:40-44`）。STEP-0591
（G2∧G3 的 guest 级证据）在 G2 的核验里顺手证明了**跨实例字节一致**（窗口后端呈现帧 == 独立
`D2dUiBackend` 离屏帧，`STEP-0591:40,129`），但明确声明：

> G2 的字节一致性同时是**跨实例确定性**证据……但 G5 的正式声明（同树 N 次 → N 帧逐字节一致）
> 仍需专测，本片**不**声称 G5（`STEP-0591:40`）；G5 状态 = **未达成（仅渲染器核心语料，
> STEP-0338）**（`STEP-0591:132`）。

本片就是那一次正式专测：把 G5 从「附带证据 + 渲染器核心语料」升格为 **guest 级可复核**的
确定性声明。

## 侦察结论（本片只读复验，非转述 STEP 摘要）

### 1. guest 触发呈现与宿主读回通道

- guest 经 `run()` 触发呈现（`ui.wit:85`）；`run()` 无 guest 读回通道（`STEP-0591` 合同缺口 1）。
- 帧字节读回仅宿主侧：`WindowedUiBackend::read_presented_frame()`（`crates/sico-ui-renderer-d2d/src/windowed.rs:483-485`），
  经 `ReadbackBackend`（本测试文件）暴露给测试。
- `WindowedUiBackend::run()` = `self.tree.render_frame()`（离屏 D2dUiBackend 渲染）+ `self.shell.present(frame)`
  （`windowed.rs:467-471`）；`present` 是**纯字节拷贝**（STEP-0577），故读回帧 == 离屏渲染帧。
- **结论**：G5 的「逐字节可核」是「guest 触发 + 宿主读回核验」，同 G2 的收窄口径；guest 无读回
  通道（WIT 变更属 owner 裁决，STEP-0590 C-A 模式），本片不改变该合同缺口。

### 2. 渲染器为纯函数（ADR-0018 §3），无环境派生量

- 冻结 token 表：`crates/sico-ui-renderer-d2d/src/tokens.rs`——`fluent-subset-v0`，全部常量；
  注释明确「no token may be derived from the environment (system theme, DPI scale, locale,
  accent colour)」（`tokens.rs:3-8`）。
- 渲染核心 `D2dUiBackend` 不持有任何 COM 对象，每帧从零重建（`backend.rs:36-39`）；`render_frame()`
  克隆树 + 固定 client 尺寸后渲染（`backend.rs:231-238`）。
- `Frame::sha256()` 是像素负载的 SHA-256（`frame.rs:68-77`），为帧身份。
- 冻结帧语料：`crates/sico-ui-renderer-d2d/tests/frame_corpus.rs`（4 场景 SHA-256）+
  `docs/evidence/m24/ui-frame-corpus.json`；语料注明「verified in-process and across two
  separate processes」（`ui-frame-corpus.json:9`）。
- **结论**：渲染器无 timer/动画/随机/线程依赖，帧是 (树, token 表, 宿主光栅化器) 的纯函数。

### 3. 本片使用的场景 = 冻结语料 `text` 场景

`frame_corpus.rs::scene_text`（`frame_corpus.rs:54-60`）：`window("Text",320,160) → column →
text("Sico") → text("Fluent-subset renderer")`，冻结 digest `0bd324c8…`。本片 guest 精确构建同一
场景，使 guest 级帧**锚定到权威冻结语料**（不只 N 帧内部一致）。

### 4. 跨进程先例

STEP-0338 的跨进程一致性通过 `crates/sico-ui-renderer-d2d/examples/frame_digest.rs`（`cargo run
-p sico-ui-renderer-d2d --example frame_digest`，独立进程打印各场景 digest）与语料测试对照完成。
本片镜像该形态：① 测试内自生成子进程（`--exact emit_offscreen_text_digest`）；② 手动运行
`frame_digest` 示例（独立进程）复算同一场景 digest。

## 实现（本片唯一生产外新增：`tests/ui_frame_determinism.rs`）

纯证据片：**不改任何生产代码**（`ui.rs`/`lib.rs`/`windowed.rs`/`backend.rs`/`tree.rs`/`tokens.rs`/
WIT 全部只读），仅新增一个 integration test 文件。

### 1. guest 树 = 冻结语料 `text` 场景（hand-encoded Component）

- 导入 `window`/`column`/`text`/`run`/`poll-event`（`wasm-encoder` 手编码，复用 STEP-0591 的
  index 记账规则；`column` 追加为 `Func(15)`）。
- guest `run` 体：`window("Text",320,160)` → `column` → `text("Sico")` → `text("Fluent-subset
  renderer")` → `run()` → `poll-event()`；成功 `exit = 1`；各步错误 → `110/100/80/90 + disc`；
  `poll-event` 意外 `Some` → `70 + disc`（全部 `0..=119`）。
- **调试记录**：首版把第一个 `text` 的返回句柄写回父句柄局部，导致第二个 `text` 以 Text 叶节点
  为父 → `WrongParentKind`（exit 81 = 80 + disc 1）。修正：保持父句柄 = `column` 句柄贯穿两个
  `text` 调用。这是**诚实失败**（测试暴露真实句柄传播错误），非放宽断言。

### 2. `ReadbackBackend`（同 STEP-0591 形态）

包装 `WindowedUiBackend`（生产后端），13 个 `ui@1` 操作按 `D2dUiRenderer` 相同方式委托，另暴露
宿主侧 `window_title`/`read_presented_frame`；错误折叠按 `ui.rs::fold_error` 复刻。**不是新渲染器**。

### 3. 三类断言

- **① 跨 Store（主 G5）**：`N = 6` 个**完全独立**的 (Runner, `PreparedProgram::run` Store,
  `WindowedUiBackend`, OS 窗口) 代，每代 guest 渲染同一树一次并读回一帧；断言 6 帧 `pixels()`
  逐字节与 `sha256()` 全等，且每帧 == 冻结语料 digest `0bd324c8…`，且 == 进程内离屏 oracle。
- **② 跨进程**：同一树在**独立进程**（本测试二进制自生成子进程，`--exact emit_offscreen_text_digest
  --nocapture`，env `SICO_G5_DIGEST_CHILD=1`）离屏复算 digest，断言 == 进程内 guest 读回 digest
  == 冻结 digest。
- **③ 同树重复呈现（补充）**：一个后端，guest 先呈现一次（读回帧 0），再宿主侧 `run()` +
  读回 `PRESENT_ROUNDS=5` 次；6 帧全部逐字节一致 == 冻结 digest。

## 确定性论证（影响帧字节的输入逐项）

帧字节 = 渲染核心输出（像素负载），经 `present` 纯字节拷贝后由 `read_presented_frame` 读回。
候选输入与处理：

| # | 输入 | 处理 |
|---|---|---|
| 1 | **UI 树**（节点种类、内容、父子、client 尺寸） | **固定**：guest 每代精确构建冻结 `text` 场景（`Text`/`Sico`/`Fluent-subset renderer`，320×160），内容与结构逐代相同 |
| 2 | **冻结 token 表**（颜色/几何/字体度量） | **固定**：`fluent-subset-v0` 全常量（`tokens.rs`）；契约禁止环境派生（ADR-0018 §2） |
| 3 | **DirectWrite 光栅化** | **固定**：字体族 `Segoe UI Variable`、locale `en-us`（`tokens.rs:79-83`）均为冻结常量；光栅化是同宿主确定性函数（进程内/跨进程复算一致，见验证） |
| 4 | **Direct2D 设备/后备缓冲** | **固定**：渲染核心不持 COM 对象、每帧从零重建（`backend.rs:36-39`）；无跨帧可变状态 |
| 5 | **呈现路径** | **固定**：`present` 为纯字节拷贝（STEP-0577），读回帧 == 离屏渲染帧 |
| 6 | **OS 窗口生命周期** | **排除**：client 区精确 320×160（STEP-0577 shell 设定）；标题/边框非 client 帧字节；建/毁窗不改帧像素 |
| 7 | **环境派生量**（主题/DPI/locale/强调色/系统时间） | **排除**：渲染核心无此输入（`tokens.rs:3-8`）；无 timer/动画/随机源 |
| 8 | **线程/进程身份** | **排除**：树经 `Mutex` 克隆，帧在调用线程由不可变克隆计算；shell 线程仅拷贝字节；无调度依赖 |
| 9 | **Wasm Store / runner 代** | **排除**：每代新 Store（`PreparedProgram::run`，`lib.rs:2388-2404`）；树内容与 Store/代无关 |

**结论**：帧字节仅依赖固定树 + 冻结 token 表 + 同宿主光栅化器 ⇒ 同树 N 帧逐字节一致；且因树
与冻结 `text` 场景相同、token 表为 `fluent-subset-v0`，帧 == 冻结语料 digest `0bd324c8…`。
**未发现任何不确定来源**；若发现，本片纪律要求停下并报告（见「诚实收窄」）。

## 验证（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线，温热 target 缓存）

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& $cargo test --offline --test ui_frame_determinism -- --test-threads=1   # 新测试
& $cargo test --offline --test ui_guest_visible -- --test-threads=1       # 回归
& $cargo test --offline --test ui_authority -- --test-threads=1           # 回归
& $cargo test -p sico-ui-renderer-d2d --offline                           # 冻结语料回归
& $cargo clippy --offline --test ui_frame_determinism -- -D warnings      # clippy（我动范围）
& $cargo fmt -- --check tests/ui_frame_determinism.rs                     # 格式（我动范围）
& $cargo run -p sico-ui-renderer-d2d --example frame_digest --offline     # 跨进程复算（STEP-0338 镜像）
git diff --check
```

结果：

| 检查 | 结果 |
|---|---|
| `ui_frame_determinism`（新） | **4 passed, 0 failed**：`same_guest_tree_renders_n_byte_identical_frames`、`guest_frame_digest_matches_across_processes`、`same_tree_presented_repeatedly_on_one_backend_is_byte_identical`、`emit_offscreen_text_digest` |
| `ui_guest_visible`（回归） | **3 passed, 0 failed**（STEP-0591 交付物不变） |
| `ui_authority`（回归） | **7 passed, 0 failed** |
| `sico-ui-renderer-d2d`（冻结语料回归） | **5 + 6 passed, 0 failed**（`frame_corpus` 4 项含 `frame_corpus_digests_are_frozen`、`rendering_the_same_tree_twice_is_byte_identical`；`layout_corpus` 6 项）——冻结 digest 未变 |
| `cargo clippy --test ui_frame_determinism -- -D warnings` | 绿（无警告） |
| `cargo fmt -- --check tests/ui_frame_determinism.rs` | 净 |
| `cargo run -p sico-ui-renderer-d2d --example frame_digest`（独立进程） | `text 320x160 0bd324c87c0f4e41c3f61d62d7a10bd8f3b2bc15b7443a791b234673ef592868` —— 与进程内 guest 读回 digest、冻结语料**逐字节一致** |
| `git diff --check` | 净 |

**跨进程复算方式与结果**：① 测试内子进程（`emit_offscreen_text_digest`，`G5_DIGEST
0bd324c8…`）== 进程内 guest 读回 digest；② `frame_digest` 示例（独立进程，STEP-0338 镜像）打印
`text 320x160 0bd324c8…`——两者一致且等于冻结语料。**跨进程一致性成立**。

**N 取值与理由**：`N = 6`（G5 条款要求 N ≥ 5；6 提供 1 个余量样本）。每代完全独立
(Store, 后端, OS 窗口)，故 G5 覆盖**跨 Store + 跨实例 + 跨窗口**，非仅单后端多次调用；另以
「同树重复呈现 6 次（单后端）」覆盖「同树连续渲染」字面读法。

**本机真实路径**：`window_title()` 为 `Some("Text")`；guest `exit=1`；读回帧 320×160，
`pixels()` 逐字节等于独立离屏帧与冻结语料；6 帧全等；子进程与 `frame_digest` 独立进程均复算
出同一 digest。

## G1–G6 达成状态更新表

| # | 证据 | 可执行核验 | 本片前 | 本片后 |
|---|---|---|---|---|
| G1 | guest `window()` 真实窗口 | guest fixture 调 `window()` 得 `Ok(handle)`；宿主 `window_title()` 为 `Some` | **已达成**（STEP-0581/0591，runtime-verified） | **已达成**（本片再验证：`window_title()`=`Some("Text")`，guest exit=1） |
| G2 | guest `run()` 呈现帧证据 | guest 调 `run()`；宿主 `read_presented_frame()` 返回预期 client 尺寸且帧逐字节一致 | **达成（guest 触发 + 宿主读回核验）**（STEP-0591） | **达成（收窄口径，不变）**——本片沿用；guest 无读回通道（合同缺口 1） |
| G3 | guest `poll-event` 交付 `closed` | guest 的 `poll-event` 在关闭后观察到 `closed` | **达成（guest 观测；close 为宿主动作）**（STEP-0591） | **达成（不变）** |
| G4 | guest `poll-event` 交付 `click(handle)`/`select` 且节点路由正确 | guest fixture 在已知子节点边界点击，收到 `click(该句柄)` | **未达成**（命中测试缺失，STEP-0590 C-C；STEP-0592 在途） | **未达成**（STEP-0592 仍在途，本片不触碰） |
| G5 | guest 级帧确定性 | guest 同树渲染 N 次 → N 帧逐字节一致 | **未达成（仅渲染器核心语料 + 附带证据）**（STEP-0338/0591） | **达成（guest 级正式专测）**：guest 同树渲染 N=6 次 → 6 帧 `pixels()` 逐字节一致（跨 Store、跨实例、跨窗口）；每帧 == 冻结语料 `0bd324c8…`；同树单后端重复呈现 6 帧一致；跨进程（子进程 + `frame_digest` 示例）复算一致。**收窄口径**：guest 触发 + 宿主读回核验（guest 无读回通道） |
| G6 | clean-room pilot 应用（gate 5） | Sico 消费者应用经 `ui@1` 构建/运行真实窗口 | **未达成**（STEP-0581:138；gate 5 范围） | **未达成**（gate 5 范围，本片不触碰） |

**判定**：按 STEP-0590 定义（G1∧G2∧G3∧G4），当前仍为 **G1∧G2∧G3**（G4 缺命中测试，STEP-0592
在途），「guest 可见 UI 能力」**仍未整体达成**。本片只把 **G5（确定性伴随分项）** 从「未达成」
推进到「达成（guest 级正式专测）」；G5 不是能力最小证据集的成员（`STEP-0590:106`），故不改变
「当前未达成」的整体判定。G5 的「达成」为**收窄口径**（guest 触发 + 宿主读回核验，单平台
Windows x64、单树冻结 `text` 场景、N=6），非「任意树任意平台」。

## 诚实收窄（本片未声称的部分）

- **未翻任何 M24 §3 gate**；不改任何证据等级（`internal-fixture` guest + `runtime-verified`
  真实窗口，Windows x64——与 STEP-0581/0591 同档，**非**试点级、**非**生产声明）。
- **G5 达成限缩**：单平台（Windows x64 交互桌面）、单树（冻结 `text` 场景）、N=6、单 token
  集（`fluent-subset-v0`）。**不**推广到：任意树/任意平台/跨主机光栅化（语料本身注明为
  evidence-host local，`frame_corpus.rs:10-13`）/其他 token 集/`row`/`controls`/`empty` 场景。
- **G5 的「逐字节」为 guest 触发 + 宿主读回核验**：guest 无读回通道（合同缺口 1，STEP-0591），
  故不是「guest 自行读回断言」；与 G2 同一收窄口径。
- **未改 WIT/源码/既有测试/既有 STEP/冻结语料**：仅新增 `tests/ui_frame_determinism.rs` 与本
  文件。**未创建新 example**（既有 `frame_digest.rs` 已覆盖本片场景，跨进程用其镜像）。
- **未 commit/push**；保留全部既有 dirty/untracked 改动（含并发代理的 `ui.rs`/`lib.rs`/
  `windowed.rs`/`tree.rs` 等，本片未触碰）。
- **未新增依赖**：`wasm-encoder`/`sico-ui-renderer-d2d`/`sha2` 均为既有项；`Cargo.toml`/
  `Cargo.lock` 未因本片改动（既有 `M` 来自并发代理）。
- **环境依赖**：真实窗口路径需交互桌面；headless 时测试走类型化拒绝断言（`111..=115`）+
  `ENV_LIMIT`，不伪造「已呈现」。本机为真实路径（已跑通）。

## 残余缺口与建议主线审核点

- **残余**：G4（命中测试，STEP-0592 在途）、G6（clean-room pilot，gate 5）；`ui-error` 无渲染
  失败判别式与 `event-overflow` 不可观测（STEP-0590 C-A/C-B，owner 裁决项）；G2/G5 的 guest 读回
  通道缺失（WIT 追加属 owner 裁决）。
- **建议审核点**：①本片是否把 G5 记为「分项达成（收窄口径：guest 触发 + 宿主读回核验、单平台
  单树 N=6）」，还是要求升格为「任意树任意平台」（需跨树/跨平台语料，非本片范围）；②冻结语料
  锚定（guest 帧 == `0bd324c8…`）是否被接受为 G5 的强化证据（建议接受——它把 guest 级声明绑定
  到权威 STEP-0338 证据）；③`frame_digest` 示例（STEP-0338 既有）作为跨进程复算工具而非新
  example 是否被接受；④本片未翻 M24 §3 gate 的表述是否与主线一致（G5 非 §3 gate 成员）。
