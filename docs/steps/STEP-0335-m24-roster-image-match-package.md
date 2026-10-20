# STEP-0335: M24 gate 3 roster 1/3 — `sico:user/image-match@1` SAD 模板匹配（Rust 编译纯 package + 独立 oracle 内容语料）

> - status: complete（实现切片；**不移动 §3 gate 3**——三包仅落其一）
> - phase: M24 §3 gate 3 / §8.3 确定性视觉包 roster（三包：image-match / image-grid / image-contour 之首）
> - completed: 2026-10-01
> - contracts: [`RFC-0043`](../rfc/RFC-0043-deterministic-vision-packages-v0.md) §Summary `image-match@1`（结果由 [`Amendment A1`](../rfc/RFC-0043-amendment-A1-region-packed-bytes.md) 冻结、输入由 [`Amendment A2`](../rfc/RFC-0043-amendment-A2-bitmap-input-serialization.md) 冻结，二者 owner re-adjudication 2026-10-01 接受）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)（Rust 编译 package-core 生产路径）、[`RFC-0041`](../rfc/RFC-0041-image-data-contract-v0.md)（`record region`/`image-error`/D4 2^28 ceiling）
> - evidence: runtime-verified（Windows x64 真实 wasmtime Component 路径、空 linker、`1.98.0-x86_64-pc-windows-gnu`，无外部 crate）；内容语料独立 oracle = **纯 stdlib Python SAD 重实现**（本机无 numpy，与 Rust core 无共享代码）
> - support-matrix row: [`docs/evidence/m24/image-match-support.json`](../evidence/m24/image-match-support.json)（schema `sico.m24.roster-support.v0`）

## Objective

落地 M24 §3 gate 3 / §8.3 确定性视觉 roster 三包中的**第一个**：
`sico:user/image-match@1` 的 `template(bmp, tpl) -> result<region, error>`
（SAD 模板匹配，整数、行主序、ties 取最低 `(y,x)`）作为**纯 package
Component**，按 ADR-0019 Rust 编译 core 生产路径产出，对冻结的独立-oracle
内容语料**逐元素字节精确**、五类 `image-error` 逐 case 类型化拒绝、字节稳定。
零 compiler/Runtime 改动。严格镜像 #2 PNG（STEP-0331）路径。

## Contract background（为何需要 A2）

A1（2026-10-01 接受）只冻结了 roster **结果**编码（`region`→16 B 定宽小端
packed `Bytes`、`image-error`→案例名 `Text`），并把签名输入塌成裸 `bmp: Bytes`
却**未定义 `record bitmap`→输入 `Bytes` 的打包形制**（RFC-0041 §3 明文推迟
canonical serialization）。径建需实现者自拟输入 wire 形制 ⇒ 触 AGENTS §5 +
不静默改合同。故起草 [`RFC-0043-A2`](../rfc/RFC-0043-amendment-A2-bitmap-input-serialization.md)，
owner 于 2026-10-01 裁决**接受 A2**（自描述 packed header，复用 image-codec
载荷 header 先例）。A1+A2 双双冻结后三包方可建。本 STEP 是裁后首片。

## Changes

- **guest crate（workspace 外，ADR-0019 §2）**：[`guests/image-match/`](../../guests/image-match)
  —— 纯整数 SAD，**零外部依赖**（区别于 codec 的 `png =0.18.1`）；release
  profile 与 codec 逐字段钉死（`opt-level=3, lto, codegen-units=1, panic=abort,
  debug=0, strip`）；导出 canonical-ABI `realloc` + `template`，无 import（纯）。
  已加入根 workspace `exclude`。
  - **A2 输入解码 `decode_bitmap`**：`width u32 LE · height u32 LE · format u8
    (0=bgra8,1=grey8) · reserved u8[3]=0 · pixels(w·h·bpp)`；**冻结检查序**（选
    唯一返回 case）：短头/保留位非零/零轴/长度不符→`stride-mismatch`；format∉
    {0,1}→`unsupported-format`；`w·h·bpp` 溢出 u64→`dimension-overflow`；
    像素字节 > 2^28→`allocation-limit`。
  - **SAD 内核**：整数、行主序（y 外 x 内），严格 `<` 保首个最小 ⇒ ties 取最低
    `(y,x)`；`bmp.format≠tpl.format`→`unsupported-format`；模板大于图→
    `region-out-of-bounds`。
  - **A1 结果编码**：`region{x,y,width=tw,height=th}`→16 B 小端。
- **提交件 = digest-pinned core `.wasm`（ADR-0019 §3）**：
  [`crates/sico-codegen-wasm/artifacts/image-match-core.wasm`](../../crates/sico-codegen-wasm/artifacts/image-match-core.wasm)
  （17782 B）+ `.sha256`
  （`d06b6a08cc95c8880c06db6eb3f443ce261279b675fc694098399d4625a6351f`）。
- **producer**：`package_builder.rs::image_match_package()` —— `include_bytes!`
  嵌入提交 core，调用与手工发射包**同一个** `build_package_component
  ("image-match", 1, …)`，函数面 A1/A2 扁平形（`template(Bytes,Bytes)->
  Result[Bytes,Text]`，皆落 E8017 接受集）。消费者与 runner 看不到与
  `image_codec_package()` 的差别。
- **复现校验器**：[`tools/validate-package-core.ps1`](../../tools/validate-package-core.ps1)
  core 注册表新增 `image-match` 行，clean 重建断言字节相同。
- **冻结内容语料**：[`runner/sico-runner/tests/corpus/image-match/manifest.json`](../../runner/sico-runner/tests/corpus/image-match/manifest.json)
  —— schema `sico.m24.roster-fixtures.v0`，14 例（5 ok + 9 类型化拒绝），
  期望 region-hex / case 名内联，由 [`prototypes/image-match-probe/tools/make-fixtures.py`](../../prototypes/image-match-probe/tools/make-fixtures.py)
  独立生成并提交冻结。ok 形：`grey-exact-offset`/`grey-best-nonzero`/
  `grey-tie-lowest-yx`/`bgra-exact-offset`/`bgra-best-nonzero`；五类
  `image-error` 各至少一例（含 `allocation-limit`@2^28+1、`dimension-overflow`
  @u64 溢出 limit+1 边）。
- **runner 内容测试**：[`runner/sico-runner/tests/image_match_package.rs`](../../runner/sico-runner/tests/image_match_package.rs)
  —— 4 测，期望从 manifest 读取（测试与 oracle 不各自漂移）。
- **支持矩阵行**：[`docs/evidence/m24/image-match-support.json`](../evidence/m24/image-match-support.json)。

## Executed validation

- `cargo test --manifest-path runner/sico-runner/Cargo.toml --test image_match_package`
  → **4 passed; 0 failed**：
  - `image_match_package_is_deterministic_and_pure`：两次构建字节相同；对**空
    linker** 实例化成功（core 无 import ⇒ `prepare_package` 纯度检查通过）；恰好
    导出一个 `sico:user/image-match@1.0.0` 实例。
  - `template_corpus_matches_independent_oracle`：14 例逐行——5 ok 例 region
    **16 B 字节精确**、9 例拒绝 case 名精确（覆盖全部五类 `image-error`）。
  - `refusal_corpus_covers_the_image_error_closed_set`：五类闭集逐 case 存在。
  - `template_is_byte_stable_and_repeats_across_stores`：同输入 fresh store 重
    跑字节相同。
- `tools/validate-package-core.ps1` → **PACKAGE-CORE VERDICT: reproducible**
  （`image-codec` 与 `image-match` 两 core clean 重建 sha256 均 == 提交件 ==
  记录值）。
- `cargo fmt -p sico-codegen-wasm -- --check` 绿；runner `cargo fmt -- --check`
  绿；`cargo clippy --test image_match_package -- -D warnings` 绿。

## Honest residuals

- **本片只落 gate 3 三包之一**。`image-grid@1`（`detect-lines`）与
  `image-contour@1`（`find`）尚未开始，各需自己的 ADR-0019 core + 独立
  numpy-oracle 内容语料 + runner 内容测 + 各一 STEP。**§3 gate 3 保持未达**。
- **oracle 用纯 stdlib Python 而非 numpy**（本机离线无 numpy）：RFC-0043
  §Corpora 写「Python/numpy」，纯 Python 是满足「独立固定 oracle」意图的有效
  实现（不同语言、与 Rust core 无共享代码），且语料是极小整数窗口、纯循环精确。
  如实登记，不假称 numpy 运行。
- 本片**不移动任何 M24 §3 gate**：gate 1（加速器硬件）与 #4 Direct2D 渲染器仍是
  外部门；M24 overall 仍非 full GO。
- M7 lock（`sico-lock.json`）条目随首个消费该包的试点（#6）落地；本片以
  artifact `.sha256` + 复现校验器完成 digest-pinning。
- STEP 改号：M24 轨 …0330/0331/0332/0333/0334/**0335**，主线最高 0329，取 0334
  之上的下一个空闲号。
