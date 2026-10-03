# STEP-0336: M24 gate 3 roster 2/3 — `sico:user/image-grid@1` 水平线段检测（Rust 编译纯 package + 独立 oracle 内容语料）

> - status: complete（实现切片；**不移动 §3 gate 3**——三包仅落其二）
> - phase: M24 §3 gate 3 / §8.3 确定性视觉包 roster（image-match / **image-grid** / image-contour 之二）
> - completed: 2026-10-01
> - contracts: [`RFC-0043`](../rfc/RFC-0043-deterministic-vision-packages-v0.md) §Summary `image-grid@1`（输入 [`A2`](../rfc/RFC-0043-amendment-A2-bitmap-input-serialization.md)、`list<region>` 结果 [`A1`](../rfc/RFC-0043-amendment-A1-region-packed-bytes.md)，二者 owner re-adjudication 2026-10-01 接受）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)、[`RFC-0041`](../rfc/RFC-0041-image-data-contract-v0.md)（`region`/`image-error`/D4 2^28）
> - evidence: runtime-verified（Windows x64 真实 wasmtime Component、空 linker、gnu 工具链、无外部 crate）；内容语料独立 oracle = **纯 stdlib Python 重实现**（本机无 numpy，与 Rust core 无共享代码）
> - support-matrix row: [`docs/evidence/m24/image-grid-support.json`](../evidence/m24/image-grid-support.json)

## Objective

落地 gate 3 roster 三包中的**第二个**：`sico:user/image-grid@1` 的
`detect-lines(bmp, min-run) -> list<region>`（水平线段检测）作为**纯 package
Component**，按 ADR-0019 Rust 编译 core 路径产出，对独立-oracle 内容语料逐元素
字节精确、可达四类 `image-error` 逐 case 类型化拒绝、字节稳定。镜像 #2 PNG /
STEP-0335 image-match 路径。零 compiler/Runtime 改动。

## Frozen v0 语义（§A4 委托语料定形，本片显式钉死）

RFC-0043 §A4 把该 op 的确切行为**委托给确定性规则 + 内容断言语料**（与 image-
match 的 SAD 同理——注意这是 **package 内部参考语义**，非 image-match input 那类
**跨边界 wire 序列化**，后者才需 owner 裁决、已由 A2 裁）。本片钉死的参考定义：
- 输入须为 **grey8**；bgra8 合法 A2 位图对本 op **越界 → `unsupported-format`**
  （§A4「grey8/threshold 栅格族」）；
- 前景 = 灰度值 `>= 128`（固定 v0 阈值，沿用 image-vision occupancy-mask 惯例）；
- 「线」= 某行内前景像素的**极大连通水平段**；长度 `>= min-run` 者成 region
  `{x, y, len, 1}`；
- 输出**行主序**（y 外 x 内）= 确定性序；`list<region>` 由 A1 编码为
  `count u32 LE` + `count×16 B`；
- 无合格段（含 `min-run` 大于任何段）→ `count=0` blob 为 **ok**（A1 §2 空结果
  即 count 前缀 ok；§A4 允许退化参数出空表）。`image-error` 无参数 case，故
  `min-run` 过大**从不**是拒绝。

## Changes

- **guest crate（workspace 外）**：[`guests/image-grid/`](../../guests/image-match)
  —— 零依赖整数段检测，profile 逐字段钉死；A2 `decode_bitmap` 拒绝序与
  image-match **完全一致**（两核对畸形输入同样拒绝）；导出 canonical-ABI
  `realloc` + `detect-lines`，无 import（纯）。根 workspace `exclude` 已加。
- **提交件**：[`crates/sico-codegen-wasm/artifacts/image-grid-core.wasm`](../../crates/sico-codegen-wasm/artifacts/image-grid-core.wasm)
  （17863 B）+ `.sha256`
  （`524810cbde0308301a70b08de5e7c6f65a15db197776d54da9b56dfdcdb8ecfe`）。
- **producer**：`package_builder::image_grid_package()` —— `include_bytes!` +
  同一 `build_package_component("image-grid", 1, …)`，A1/A2 扁平签名
  `detect-lines(Bytes,U64)->Result[Bytes,Text]` 全落 E8017。
- **复现校验器**：`tools/validate-package-core.ps1` 注册表加 `image-grid` 行。
- **冻结内容语料**：[`runner/sico-runner/tests/corpus/image-grid/manifest.json`](../../runner/sico-runner/tests/corpus/image-grid/manifest.json)
  —— schema `sico.m24.roster-grid-fixtures.v0`，13 例（5 ok + 8 拒绝），由
  [`prototypes/image-grid-probe/tools/make-fixtures.py`](../../prototypes/image-grid-probe/tools/make-fixtures.py)
  独立生成。ok：`two-full-hlines`(min5)/`single-row-min3`/`single-row-min2`
  (>= 边界)/`min-run-too-large-empty`(count0 ok)/`mixed-rows-x-offset`(非零 x
  起点)；err：`bad-format-tag`+`bgra8-rejected-grey8-only`(unsupported-format
  两路)/`reserved-nonzero`/`too-short-header`/`zero-axis`/`length-mismatch`
  (stride-mismatch)/`allocation-limit`/`dimension-overflow`。
- **runner 内容测试**：[`runner/sico-runner/tests/image_grid_package.rs`](../../runner/sico-runner/tests/image_grid_package.rs)
  —— 4 测，期望从 manifest 读取。
- **支持矩阵行**：[`docs/evidence/m24/image-grid-support.json`](../evidence/m24/image-grid-support.json)。

## Executed validation

- `cargo test --manifest-path runner/sico-runner/Cargo.toml --test image_grid_package`
  → **4 passed; 0 failed**：determinism/purity（两次构建字节相同 + 空 linker +
  单实例导出）、`detect_lines_corpus_matches_independent_oracle`（13 例逐行，
  list-blob 字节精确，断言恰有 1 例 count=0 空 ok）、`refusal_corpus_covers_the_
  reachable_image_error_set`（四类可达 + 显式断言 region-out-of-bounds 不可达）、
  `detect_lines_is_byte_stable_and_repeats_across_stores`。
- `tools/validate-package-core.ps1` → **PACKAGE-CORE VERDICT: reproducible**
  （image-codec / image-match / image-grid 三 core clean 重建均字节相同）。
- runner `cargo fmt -- --check` 绿；`cargo clippy --test image_grid_package
  -- -D warnings` 绿；`git diff --check` 绿。

## Honest residuals

- **gate 3 现落 2/3**。仍缺 `image-contour@1`（`find`，外部边界跟随连通域包围
  盒）；三包全绿前 **§3 gate 3 保持未达**。
- detect-lines 的算法细节是**本片按 §A4 委托钉死的 package 内部参考定义**（前景
  阈值 128、极大水平段、行主序、空=count0），已在 guest 文档 + 本 STEP +
  evidence `semantics-provenance` 显式登记可审计；这与 image-match **输入 wire
  序列化**（跨边界、已由 A2 owner 裁决）性质不同，不需另裁，也**未**静默改任何
  跨包合同。若未来需 bgra8 或垂直段检测，属新接口版本。
- oracle 用**纯 stdlib Python**（本机无 numpy），满足 RFC-0043 §Corpora「独立固定
  oracle」意图（异语言、与 core 无共享代码），如实登记。
- 不移动任何 M24 §3 gate；gate 1（加速器）/#4（Direct2D）外部门不变；M24 overall
  仍非 full GO。
- STEP 改号：M24 轨 …0334/0335/**0336**，主线最高 0329。
