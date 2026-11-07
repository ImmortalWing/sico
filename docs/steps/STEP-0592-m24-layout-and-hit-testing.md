# STEP-0592: M24 §8.6.1 — 渲染器布局计算 + 点→句柄命中测试（G4 铺路片；纯计算层，不改任何渲染字节）

> - status: implementation complete locally（**新增纯计算布局层 + 命中测试**；不改任何渲染字节、不翻任何 M24 §3 gate、不改变任何证据等级）
> - phase: M24 计划 §8.6.1（Sico-language native UI library 前置工作流，ADR-0018 §1/§2/§3/§4，RFC-0052 §2/§3 + A1）
> - completed: 2026-11-07
> - contracts: [`ADR-0018`](../adr/ADR-0018-native-renderer-fluent-subset-v0.md)（accepted，STEP-0323）、[`RFC-0052`](../rfc/RFC-0052-native-ui-source-binding-v0.md)+A1（accepted，同上）——本 STEP 只消费、不修改
> - evidence: **internal-fixture**（纯计算、平台中立；Windows x64 复跑 + 跨进程摘要一致性；**非** guest 级、**非** pilot 级、**非**生产声明）
> - artifacts: [`crates/sico-ui-renderer-d2d/src/layout.rs`](../../crates/sico-ui-renderer-d2d/src/layout.rs)、[`crates/sico-ui-renderer-d2d/src/lib.rs`](../../crates/sico-ui-renderer-d2d/src/lib.rs)、[`crates/sico-ui-renderer-d2d/tests/layout_corpus.rs`](../../crates/sico-ui-renderer-d2d/tests/layout_corpus.rs)、[`crates/sico-ui-renderer-d2d/examples/layout_digest.rs`](../../crates/sico-ui-renderer-d2d/examples/layout_digest.rs)

## Objective

STEP-0590 的 C-C 与 STEP-0585 的剩余缺口 #1 都指向同一处：`ui@1` **确有**
`click(ui-handle)` / `select(ui-selection)` 变体（`wit/script-ui-v1/ui.wit:59-63`），但
点→句柄命中测试缺**渲染器布局**（`windowed.rs:24-26`，`events.rs:65-82`）——窗口壳只能给
client 坐标 `WindowEvent::Clicked{x,y}`，坐标点击被宿主侧跳过（`skipped_click_count`，
STEP-0585）且**明确否决**了「任意点一律归窗口根」（无布局 ⇒ 子元素无矩形 ⇒ 错误归属）。
本片补上让归属性正确的那块：一个**纯计算**的布局层 + 点→句柄命中测试，**不改变任何渲染
字节**。它是 G4（`poll-event` 交付 `click(handle)` 且节点路由正确）的**铺路片**：布局与
命中测试就绪，但 G4 还需 runner 侧交付 `click(handle)`（后续片）。

## 侦察结论（本片逐条复验，只读）

以下为**实际读文件**的结论（非转述 STEP 摘要）。括号内为证据来源。

### 坐标系 —— client 绝对坐标，原点在客户区左上，96-DPI 逻辑像素

- `WM_LBUTTONDOWN` 的 `lParam` 低/高字即**客户区坐标**（`shell/win.rs:280-281`：
  `low_word(lparam.0)` / `high_word(lparam.0)`），经 `translate` 映射为
  `WindowEvent::Clicked{x, y}`（`events.rs:79-81`）。
- **坐标系为 client 绝对坐标**（非窗口相对）：原点 = 客户区左上角，`x`/`y` 为
  `i32`。壳请求 DPI-unawareness 使客户区保持 96-DPI 逻辑像素（`shell/mod.rs:15-21`、
  `shell/win.rs:377-378`）。
- `translate` 过滤 `x < 0 || y < 0`（`events.rs:109-113`），故 `Clicked` 的坐标
  非负；但可**等于** `width`/`height`（边界），命中测试须把 `>= w` 或 `>= h` 视为
  窗外（本片 `Rect::contains` 半开区间 `[x, x+w)`，见下）。
- **结论**：布局矩形与命中测试的输入点用**同一 client 坐标空间**，无需任何换算。

### 句柄标识形式 —— `u64`，`1..=len`，`0` 为无效

- `UiTree::insert` 返回 `nodes.len() as u64`（`tree.rs:189`），句柄自 `1` 起连续；
  `tree.node(handle)` 用 `index = handle - 1`（`tree.rs:341-347`）。`0` 无效。
- 窗口根是第一个插入的节点（句柄 `1`）；`column`/`row`/`text`/`button`/`dropdown`/
  `image-preview`/`progress` 均为同一句柄空间（`ui.wit:66-73`）。
- `ui-event` 的 `click(ui-handle)` 携带该句柄（`ui.wit:59-63`）；本片命中测试返回的
  `Hit::Node(u64)` 即此句柄。

### 可用度量 —— 冻结 token 表，表内无任何环境派生值

- 几何：`CONTROL_HEIGHT=32`、`CONTROL_PADDING_H=12`、`CONTROL_PADDING_V=6`、
  `SPACING=8`、`WINDOW_PADDING=16`、`PROGRESS_HEIGHT=4`、`MIN_BUTTON_WIDTH=64`、
  `DROPDOWN_CHEVRON_WIDTH=20`、`TEXT_MAX_WIDTH=4096`（`tokens.rs:49-75`）。
- 排版：`BODY_SIZE=14`、`BODY_LINE_HEIGHT=20`（`tokens.rs:86-99`）。
- **表内无任何环境派生值**（无 DPI 比例、无系统字体度量、无时间、无主题/语言），
  `TOKEN_SET_VERSION="fluent-subset-v0"`（`tokens.rs:17`）。

### 既有渲染器布局 —— 不适用本片（且存在两处「不能用」）

- `render/win.rs::place`（`render/win.rs:327-424`）用 **DirectWrite 度量文本**
  （`render/win.rs:154-178`：`CreateTextLayout` + `GetMetrics`）——**宿主相关**（字形光栅
  化因机器而异），不能用于跨机器确定的命中测试。
- `natural_size(Root|Column|Row) = (0.0, 0.0)`（`render/win.rs:429-430`）⇒ 容器 own box
  为 **0×0**：两个**兄弟容器**在渲染器里会重叠（潜在 bug，冻结帧语料未覆盖）。本片命中
  测试**不能**沿用（容器须有真实 content extent 才能把「在容器内但不在子节点」归到容器）。

## 实现

### 1. 纯计算布局层（`src/layout.rs`）

- **`Rect`**（`layout.rs:62`）：`x/y/width/height`（`f32`，client 坐标）。
  `contains` 用半开区间 `[x, x+w) × [y, y+h)`（`layout.rs:73-81`）——右/下边**独占**，
  故 `(width, height)` 为窗外，`(0,0)` 为窗内。
- **`Layout`**（`layout.rs:126`）：`entries: Vec<Option<Entry>>`，按 `handle-1` 索引；
  `Entry = (rect, depth, kind, parent)`。方法：`rect`/`kind`/`depth`/`parent`/`root`/
  `width`/`height`/`len`。
- **`compute(tree, width, height)`**（`layout.rs:295`）：纯函数。
  - 无根 → `Err(NoWindow)`；`width==0 || height==0` → `Err(InvalidArgument{client-size})`。
  - 根矩形 = **整个客户区** `(0,0,w,h)`（`layout.rs:306-321`）——与渲染器 0×0 根 cell
    不同，命中测试需要根覆盖全客户区，使「窗内但无子节点覆盖」归根、窗外为 Miss。
  - 根子节点从 `(WINDOW_PADDING, WINDOW_PADDING)` 起垂直堆叠，间距 `SPACING`。
  - `place`（`layout.rs:357`）递归放置子树，返回该节点的 **content extent**：
    - 容器（Root/Column）＝子节点矩形并集（`extent_w = max(child_w)`，
      `extent_h = last_child_bottom - y`，含间距间隙）；Row 横向同理。
    - 叶子 ＝ 固有尺寸（`natural_size`，`layout.rs:458`）。
    - `stretch_width`：垂直容器的子节点中 `Text`/`Progress` 取满 `available_width`，
      其余保持固有宽度（镜像渲染器规则 `render/win.rs:347-349`）。
  - **有界**：节点数/深度沿用树模型限额（`tree.rs:11-13`；`place` 深度超 `MAX_DEPTH`
    → `Err(Limit{tree-depth})`，未知句柄 → `Err(UnknownHandle)`，`layout.rs:368-373`）；
    不 panic、不截断半成品（任何越界/畸形输入都是类型化拒绝）。
- **`text_advance`**（`layout.rs:494`）：文本推进宽度 = `UTF-16 码元数 × BODY_SIZE`，
  上限 `TEXT_MAX_WIDTH`。**token 派生代理**，非字体度量（见「度量来源与确定性论证」）。

### 2. 命中测试（`src/layout.rs`）

- **`Hit`**（`layout.rs:102`）：`Node(u64)` | `Miss`。
- **`Layout::hit(x, y)`**（`layout.rs:197`）：
  - 点**在客户区外**（`px<0 || py<0 || px>=w || py>=h`）→ **`Hit::Miss`**（即使退化
    子矩形如超大图预览延伸到窗外，`layout.rs:200-202`）。
  - 否则取**最深**（depth 最大）包含该点的节点；同深并列取最小句柄（handle 序迭代，
    `layout.rs:203-214`）。`Hit::Node(root)` ⇔ 点在窗内但**无任何子节点矩形覆盖**（如
    窗口 padding、容器空隙）。
- **语义与 STEP-0585 否决的「任意点归根」不同**：根只在**确实无子节点覆盖**时返回；
   子节点上的点击归子节点（最深层）。「窗外 → Miss」是明确的无命中。

### 3. 冻结语料 + 摘要工具

- **`layout::corpus`**（`layout.rs:545`，`#[doc(hidden)]`）：**单一来源**的 4 场景
  （`empty`/`text`/`controls`/`row`，形状镜像冻结帧语料 `frame_corpus.rs:22-47`）+
  `probes(layout)` 探针（四角 + 每节点中心 + 每容器相邻子节点间隙中点，
  `layout.rs:651`）。测试与示例共用，**不能漂移**。
- **`Layout::digest()`**（`layout.rs:225`）与 **`Layout::hit_digest(points)`**
  （`layout.rs:254`）：固定版本化 wire 形式（LE；`f32` 以 `to_bits()` 序列化；
  节点按 handle 序；`Hit` 为 tag + handle）的 SHA-256。
- **`tests/layout_corpus.rs`**：`CORPUS` 冻结 4 场景的 `(layout_sha256, hit_sha256)`
  （`layout_corpus.rs:34`）+ 语义断言（叶子中心→自身、窗外→Miss、无根/零尺寸→类型化拒绝、
  同树两次布局逐字节同摘要）。
- **`examples/layout_digest.rs`**：再生成工具，镜像 `event_digest`/`frame_digest` 形态；
  与测试共享 `layout::corpus` ⇒ 打印值 == 测试冻结值（跨进程一致）。

## 度量来源与确定性论证

**度量只来自冻结 token 表**的理由：

1. **DirectWrite 字形度量是宿主相关的**（`render/win.rs:154-178`）：同一文本在不同
   Windows 构建/机器上的 glyph 光栅化与 advance 可不同，`frame_corpus.rs:10-13` 因此把
   帧语料限定为「evidence-host local」。若命中测试用 DirectWrite，**同一棵树在不同机器
   上的矩形/命中会不同** ⇒ 无法跨机器复现，语料只能按机器冻结（失去「跨进程一致」）。
2. **token 表是冻结常量**（`tokens.rs:1-12`，`TOKEN_SET_VERSION="fluent-subset-v0"`，
   表内无环境派生值）。本片所有度量（`CONTROL_HEIGHT`/`CONTROL_PADDING_H/V`/`SPACING`/
   `WINDOW_PADDING`/`PROGRESS_HEIGHT`/`MIN_BUTTON_WIDTH`/`DROPDOWN_CHEVRON_WIDTH`/
   `TEXT_MAX_WIDTH`/`BODY_SIZE`/`BODY_LINE_HEIGHT`）均为该表的 `const f32`，无 DPI、无
   系统字体、无时间、无 RNG、无环境访问。
3. **文本宽度用 token 派生代理**：`text_advance = utf16码元数 × BODY_SIZE`（上限
   `TEXT_MAX_WIDTH`），是文本字符串 + 冻结 token 的纯函数，跨机器逐字节一致。它**不是**
   字形精确 advance（诚实收窄，见下），但命中测试的「归属性正确」不依赖字形精度——它
   依赖**确定性**（同一树同一坐标永远命中同一节点）。
4. **`f32` IEEE-754 运算确定**：加法/乘法/`min`/`max`/比较在 Rust 的所有目标上是
   确定性的（本片不引入非确定操作）。
5. **无 `HashMap` 迭代序依赖**：`Layout` 用 `Vec<Option<Entry>>`（按 `handle-1`），
   迭代恒为 handle 序；无 `BTreeMap` 以外的无序容器。
6. **图像尺寸取自树的 `ImageData`**（纯树值，`tree.rs:57-65`），非环境值。

**结论**：布局是 `(tree, client_size)` 的**纯函数**，跨机器/跨进程逐字节一致；摘要由固定
版本化 wire 形式（`f32::to_bits`、LE、handle 序）计算，故 SHA-256 跨进程一致。

## 冻结布局/命中语料（SHA-256，跨两次独立进程一致）

| 场景 | 客户区 | `Layout::digest()` | `hit_digest(probes)` |
|---|---|---|---|
| `empty` | 200×120 | `2f90b55b902998af5d46020abc066dfe69cbb131bfd18ea33292979095716f04` | `a7a471a580609be848816f8aa952caf0fcde37a7d7e19813964eaad2516b5730` |
| `text` | 320×160 | `97a1ffb47e765b02c7f640d46882d7ff63df533a8fe8f18cec99f54caab7d816` | `42585f1da3dd1ba3942602499a1d6f9a6eec19f4dcd1acc4e7ef7616d7d7e63c` |
| `controls` | 360×280 | `27fe362d88a8242e53105f49ae007c93df06fbabe9766b751276fb9cb9eb6500` | `87be33ba52be676ae6bcd1435e185ab94ff3a383794e73a0d2b23ea26f4bd4a0` |
| `row` | 320×120 | `9da940cb69d5bdb55d43a909a9773621290eeb438ac020b7588bb6dcc2512274` | `133db61be41ff487736da7f09eb9fa58181405c48458ea9ee933ef5350c60532` |

- 冻结位置：`tests/layout_corpus.rs::CORPUS`（权威）；再生成：`examples/layout_digest.rs`。
- **跨进程一致证据**：`cargo run --example layout_digest`（进程 A，多轮）输出与测试冻结值
  逐字相同（进程 B 的测试断言），两处共用 `layout::corpus` 单一来源 ⇒ 不可能漂移。
- 探针覆盖：四角（窗内/窗外边界）、每节点中心（最深层规则）、每容器相邻子节点间隙中点
  （在容器内但不在子节点 ⇒ 归容器）。

## 执行验证（本机 Windows x64，`1.98.0-x86_64-pc-windows-gnu`，离线，温热缓存）

基线（改动前）：`cargo test -p sico-ui-renderer-d2d --offline` → **10 lib + 5 event_corpus
+ 5 frame_corpus 全绿**。

改动后：

- `cargo test -p sico-ui-renderer-d2d --offline` → **21 lib（11 新增 layout 单测）+ 5
  event_corpus + 5 frame_corpus + 6 layout_corpus 全绿，0 失败**。
  - **冻结帧哈希未变**：`frame_corpus_digests_are_frozen` 通过，4 个 SHA-256 与
    `docs/evidence/m24/ui-frame-corpus.json` 逐字相同（`039b877a…`/`0bd324c8…`/
    `f407bcd0…`/`9e28e6aa…`）。
  - **事件语料哈希未变**：`event_corpus_digests_are_frozen` 通过，3 个 SHA-256 与
    `docs/evidence/m24/ui-event-corpus.json` 逐字相同（`a5278613…`/`dd345629…`/`46928f99…`）。
  - 渲染字节路径（`render/win.rs`/`shell/win.rs`/`frame.rs`/`tokens.rs`/`tree.rs`/
    `events.rs`/`backend.rs`/`windowed.rs`）**未触碰**。
- `cargo test -p sico-ui-renderer-d2d --offline --lib -- --ignored` → **1 通过
  （0.22s）**：`real_window_lifecycle_via_windowed_backend`（真实窗口生命周期）。
- `cargo test -p sico-ui-renderer-d2d --offline --test event_corpus -- --ignored` →
  **1 通过（0.22s）**：`real_window_smoke_close_present_quit`（STEP-0577 冒烟，present
  读回逐字节不变）。
- `cargo clippy -p sico-ui-renderer-d2d --offline --all-targets -- -D warnings` → **绿**。
- `cargo fmt -p sico-ui-renderer-d2d -- --check` → **净**（先 `cargo fmt -p
  sico-ui-renderer-d2d`）。
- `git diff --check` → **净**（注：仅覆盖已跟踪文件；`crates/sico-ui-renderer-d2d/` 整目录
  在当前 worktree **未跟踪**——属在途 M24 工作，见「诚实收窄」；已另用 `grep -nP ' +$'`
  检查本片新文件无尾随空白）。
- `tools/validate-module-boundaries.ps1` → **`MODULE_BOUNDARIES_OK model=openjdk-style
  packages=30 exceptions=4`**（与 STEP-0585 基线逐字一致）。

## 诚实收窄（本片未声称的部分）

- **G4 未达成**：本片只交付布局 + 命中测试；G4 还需 runner 侧把 `WindowEvent::Clicked{x,y}`
  经 `compute` + `hit` 映射为 `click(handle)` 并跨 guest 边界交付（后续片）。
- **无任何 M24 §3 gate 翻转**；`wit/**`、`runner/**`、`docs/steps/README.md`、
  `docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、
  `AGENT_GOAL.md`、`selfhost/`、`guests/**`、`docs/plans/**`、`crates/sico-package/**`
  与**任何冻结语料**（`frame_corpus.rs`/`event_corpus.rs`/`docs/evidence/m24/*.json`）
  均未触碰。
- **文本宽度是 token 派生代理**（`utf16码元数 × BODY_SIZE`，上限 `TEXT_MAX_WIDTH`），
  **非字形精确 advance**：与 DirectWrite 真实度量可差 ±，故命中测试的**几何网格**与
  渲染器**视觉网格**可能不一致。本片是命中测试网格，属「正确归属性」所需；「视觉网格
  一致」是后续「绘制子节点」片的契约决策（见下）。
- **渲染器绘制布局仍是旧布局**（容器 0×0 cell + DirectWrite 度量，`render/win.rs`）；
  本片**未**改动它。故两个兄弟容器在**渲染器**里仍潜在重叠（冻结帧语料未覆盖），而本片
  命中测试已给容器真实 extent——**两套布局并存**，须在「绘制子节点」片统一（见下）。
- **退化超大图不钳制**：`image-preview` 可声明任意 `u64` 尺寸（`tree.rs:290-302` 空像素
  允许任意维度），本片镜像渲染器 `natural_size` 直接使用 ⇒ 超大图产生超大（但确定、不
  panic）矩形；这是退化输入的退化结果，确定性保持。命中测试仍保证窗外 → `Miss`。
- **`docs/evidence` JSON 未另建**：布局/命中语料以 `tests/layout_corpus.rs::CORPUS` 为
  权威（镜像 `frame_corpus.rs` 持有冻结帧值），`examples/layout_digest.rs` 为再生成工具，
  两者共用 `layout::corpus` 单一来源；超出本片可动集的证据文件不新建。
- **本片代码无 unsafe/无 FFI**：布局与命中是纯计算，`unsafe` 仍 confined 在既有
  `render/win.rs`/`shell/win.rs`（ADR-0018 §1）。
- **一次瞬时退出访问违规**（`0xc0000005`）：**任何刚重链完成的 GNU/rust-lld 二进制**在
  首跑退出时报访问违规，输出与测试值均正确；随后所有运行 exit 0。本片观察到它命中本片
  新建的示例/测试二进制**以及**预存的 `frame_digest` 示例（因本片改 `lib.rs` 致其重链）——
  故判定为**工具链/加载器瞬时伪影而非代码缺陷**（本片代码为纯 safe Rust，无
  unsafe/FFI/指针；受影响的 `frame_digest` 亦含既有 Win32 代码）。后续重建后首跑仍应复核。
- **单平台**：Windows x64 复跑；布局/命中逻辑平台中立（无 Win32 依赖），在非 Windows
  主机同样编译并运行（语料测试无 `cfg(windows)` 门）。

## 后续片建议

1. **「让子节点真正被绘制」＝ 需审计的受控动作**：渲染器须**切换到本片的 extent 布局**
   （容器真实 extent + token 文本推进），替换旧 `render/win.rs::place`（容器 0×0 cell +
   DirectWrite 度量）。**这会改变帧字节** ⇒ 必须**重新冻结** STEP-0338 的 4 个帧 SHA-256
   （`frame_corpus.rs` + `docs/evidence/m24/ui-frame-corpus.json`），并 bump
   `TOKEN_SET_VERSION` 语义或记录新渲染器版本——**这是需 owner 授权的审计动作**，本片
   **未**执行。若视觉网格需与命中网格一致（推荐），此片同时消除「两套布局并存」。
2. **runner 侧交付 `click(handle)`**（G4 达成片）：`poll_event` 收到 `WindowEvent::Clicked{x,y}`
   时，以当前客户区尺寸 `compute` + `hit`，`Hit::Node(h)` → 交付 `click(ui-handle{h})`；
   `Hit::Miss` → 跳过（可复用 `skipped_click_count` 或按 owner 裁决交付根）；`Hit::Node(root)`
   才交付根（**不是**「任意点归根」）。需 `WindowedUiBackend` 暴露当前客户区尺寸与树。
3. **命中语义的 runner 决策**：`Hit::Miss` vs `Hit::Node(root)` 在 guest 侧的表示（`click`
   需要 `ui-handle`；`Miss` 无句柄可交付）——建议 `Miss` → 跳过 + 计数（既有
   `skipped_click_count`），`Node(root)` → 交付根句柄。
4. **（可选）视觉网格一致性契约**：若绘制片采用 DirectWrite 度量（视觉精确但宿主相关），
   需 ADR/RFC 决定「命中网格（token 代理）与视觉网格（DirectWrite）不一致」是否可接受，
   或让命中测试也接受一个非确定的 DirectWrite 度量变体（破坏跨机器确定，需新语料口径）。

## 建议主线审核点

- **容器 extent 语义**：命中测试给容器真实 content extent（含间距间隙），点在间隙 → 归
  容器（非子节点、非根）。与渲染器「容器 0×0 cell」不同——确认这是预期的归属性语义。
- **根矩形 = 全客户区**：窗内 padding 点击 → `Node(root)`；窗外 → `Miss`。确认根不覆盖
  客户区外的退化子矩形（超大图）——已显式守卫。
- **文本推进代理**（`utf16码元数 × BODY_SIZE`）：作为命中网格的 token 派生宽度是否可接受
  （非字形精确），还是绘制片需引入字形度量（破坏确定性）。
- **语料冻结位置**：布局/命中语料以 `tests/layout_corpus.rs::CORPUS` 为权威（未另建
  `docs/evidence` JSON）；若 owner 要求 JSON 平行记录，属下一片。
- **`crates/sico-ui-renderer-d2d/` 整目录未跟踪**：当前 worktree 的在途 M24 工作，非本片
  引入；提交/推送须待 owner 授权并整体纳入。

## 关系说明

本片是 STEP-0590 C-C / STEP-0585 剩余缺口 #1 的**实现前置片**（纯计算，不接线）。它
**不**改变 STEP-0338 帧语料（4 冻结哈希复跑不变）、STEP-0577 事件语料（3 冻结哈希复跑
不变），**不**翻转任何 M24 §3 gate，**不**改变任何证据等级，**不**改变任何渲染字节。
