# STEP-0337: M24 #3/3 — `sico:user/image-contour@1` 连通域闭合区域（Rust 编译纯 package + 拒绝语料）

> - status: complete（实现切片；补齐 gate 3 roster 三包之第三片）
> - phase: M24 gate 3 roster 队列 #3/3（`find`）
> - completed: 2026-10-01
> - contracts: [`RFC-0043`](../rfc/RFC-0043-deterministic-vision-packages-v0.md)（A1 结果 list + A2 输入 header，owner 2026-10-01 复裁决接受）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)（Rust 编译 package-core 生产路径）
> - evidence: runtime-verified（Windows x64 真实 wasmtime `=47.0.2` Component 路径，`1.98.0-x86_64-pc-windows-gnu`，**零外部依赖** core）；内容语料独立 oracle = 纯 stdlib Python 4-连通域标注重实现（与 Rust core 无共享代码）
> - support-matrix row: [`docs/evidence/m24/image-contour-support.json`](../evidence/m24/image-contour-support.json)（schema `sico.m24.roster-support.v0`）

## Objective

落地 M24 §3 gate 3 roster 的第三片（3/3）：`sico:user/image-contour@1` 的
`find(bmp: Bytes, min-area: U64) -> Result[Bytes, Text]` 作为**纯 package
Component**，按 ADR-0019 Rust 编译 core 生产路径产出，严格镜像 image-codec
（STEP-0331）/ image-match（STEP-0335）/ image-grid（STEP-0336）。输入按 RFC-0043-A2
冻结 header 解码，输出 `list<region>` 按 RFC-0043-A1 冻结 count-前缀 packed `Bytes`
编码，对独立 oracle 语料字节稳定、RFC-0041 可达 `image-error` 闭集逐行类型化。零
compiler/Runtime 改动。

## Changes

- **guest crate（workspace 外，ADR-0019 §2）**：[`guests/image-contour/`](../../guests/image-contour)
  —— `crate-type=["cdylib"]`、`publish=false`、**零 `[dependencies]`**（纯整数
  连通域标注）；release profile 逐字段钉死（`opt-level=3, lto, codegen-units=1,
  panic=abort, debug=0, strip`）；已加入根 workspace `exclude`。ABI 脚手架
  （`realloc`/`alloc_bytes`/`return_area`/`ok_area`/`err_area`/`Bitmap`/
  `bytes_per_pixel`/`decode_bitmap`/`list_blob`）与 image-match/image-grid **逐字节
  相同**（guest crate 不共享 lib，按 ADR-0019 各自复制），`decode_bitmap` 沿用
  **同一 A2 冻结拒绝序**。
  - `#[export_name="find"]` + `find_inner(b, min_area)`：`decode_bitmap` →
    grey8-only（`format!=1`→`unsupported-format`）→ 前景 `px>=128` →
    **4-连通（左右上下）显式栈洪泛标注**（push 时标记，无重访/无递归）→ 每个未访问
    前景像素（行优先种子）开一域，域记 bbox`{minx,miny,maxx-minx+1,maxy-miny+1}` +
    `area`（前景像素计数）→ `area>=min_area`（**含边界**，`min-area` 是过滤器非
    参数接受门，故过大 `min-area` 产空结果而非拒绝）纳入 → `list_blob` A1 编码。
    bbox/area 是集合属性，pop 顺序不影响产物 ⇒ **确定性**；种子行优先 ⇒ ties 取最低
    `(y,x)`。整数-only，`total<=2^28`。
- **提交件 = digest-pinned core `.wasm`（ADR-0019 §3）**：
  [`crates/sico-codegen-wasm/artifacts/image-contour-core.wasm`](../../crates/sico-codegen-wasm/artifacts/image-contour-core.wasm)
  （18856 B）+ `.sha256`
  （`b260e36b0d058b21be7de5f19f2ad1ec2e489441f1c0d62fd7d21ea27f98e4df`）。
- **producer**：`package_builder.rs::image_contour_package()` —— `include_bytes!`
  嵌入提交 core，调用同一 `build_package_component("image-contour", 1, …)`，函数面
  A1/A2 扁平形（`find(Bytes,U64)->Result[Bytes,Text]`，全落 E8017 接受集）。
- **复现校验器（ADR-0019 §4）**：[`tools/validate-package-core.ps1`](../../tools/validate-package-core.ps1)
  注册表加 `image-contour` 行（现覆盖 image-codec+image-match+image-grid+
  image-contour 四 core）。
- **冻结内容语料**：[`runner/sico-runner/tests/corpus/image-contour/manifest.json`](../../runner/sico-runner/tests/corpus/image-contour/manifest.json)
  —— 16 例（8 ok + 8 类型化拒绝），schema `sico.m24.roster-contour-fixtures.v0`，
  由独立纯 stdlib Python oracle
  [`prototypes/image-contour-probe/tools/make-fixtures.py`](../../prototypes/image-contour-probe/tools/make-fixtures.py)
  生成并提交冻结。ok 面含 single-blob / min-area 边界含（`>=2` 双域）与排除（`>=3`
  单域）/ 纵向域 / **环形含洞单域**（洞不分裂）/ **对角 4-连通断开**（两域）/ 全 OFF 空
  / min-area 过大空；err 面覆盖 A2 全拒绝序 + grey8-only。
- **runner 内容测试**：[`runner/sico-runner/tests/image_contour_package.rs`](../../runner/sico-runner/tests/image_contour_package.rs)
  —— 4 个测试，期望从 manifest 读取（测试与 oracle 不会各自漂移）。
- **支持矩阵行**：[`docs/evidence/m24/image-contour-support.json`](../evidence/m24/image-contour-support.json)。

## Executed validation

- `cargo test --manifest-path runner/sico-runner/Cargo.toml --test image_contour_package`
  → **4 passed; 0 failed**（对 re-pin 后的 core 复跑）：
  - `image_contour_package_is_deterministic_and_pure`：两次构建字节相同；对空 linker
    实例化成功（core 无 import ⇒ 满足 runner 纯度检查）；恰一 `sico:user/image-
    contour@1.0.0` 实例导出。
  - `find_corpus_matches_independent_oracle`：16 例逐行——ok 例 `list<region>` blob
    字节精确并校验 `count(4B)+16B/域` framing；8 例拒绝 case 名精确；恰两例 count-0
    空 ok。
  - `refusal_corpus_covers_the_reachable_image_error_set`：四类可达 image-error
    （`unsupported-format`/`stride-mismatch`/`dimension-overflow`/`allocation-limit`）
    逐 case 覆盖，并断言 `region-out-of-bounds` 对本 op **不可达**。
  - `find_is_byte_stable_and_repeats_across_stores`：非空 ok 例 fresh-store 复调用
    字节相同。
- `tools/validate-package-core.ps1` → **PACKAGE-CORE VERDICT: reproducible**
  （clean 重建 image-contour sha256 == 提交件；image-codec/match/grid/contour **四 core**
  全 `byte-identical: True`）。
- `cargo fmt`（guest/root-codegen/runner）`--check` 绿；`cargo clippy --test
  image_contour_package -- -D warnings` 绿；`git diff --check` 绿。

## Honest residuals

- **gate 3 roster 现 3/3 全绿**（image-match STEP-0335 + image-grid STEP-0336 +
  image-contour STEP-0337，皆 internal-fixture / runtime-verified package-Component
  级）。§3 gate 3 的**聚合 GO/NO-GO 裁决**按 AGENTS §3 留待 M24 出口审计登记（本片
  只补齐三包全绿这一必要条件，不以单片自称翻绿）。
- **`find` 内部语义（4-连通、前景阈值 128、bbox 区域形制、min-area 过滤）为 RFC-0043
  §A4 委托给内容语料的内部参考定义**，非 owner 裁决的跨边界 wire 合同；跨边界形状
  （输入 A2 header / 结果 A1 list）已 owner 冻结。此区分已在 support json
  `semantics-provenance` 如实登记。
- **M24 overall 本机仍 NO-GO**：§3 gate 1（= M17 gate 2 加速器硬件）、#4 Direct2D/
  DirectWrite 离线绑定、#3 离线 JPEG 编解码 + oracle 均为不可在本机关闭的外部门；
  §3 gate 5 GUI 转换器试点随之延期。
- **本机唯一剩余「不需 owner 裁决即可离线推进」的内部线** = task #12 provenance
  run-time 预算强制 + 中途取消（经真实 runner，A1.3 映射既有 `RuntimeLimits`，无合同门）。
- M7 lock（`sico-lock.json`）条目随首个消费该包的试点落地；本片以 artifact
  `.sha256` + 复现校验器完成 digest-pinning。
- STEP 改号：M24 轨 …0335（image-match）/0336（image-grid）/**0337（image-contour）**，
  主线最高 0329；取主线最大值之上首个空号，登记于本文件头。
