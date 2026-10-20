# STEP-0331: M24 #2 — `sico:user/image-codec@1` PNG 双向（Rust 编译纯 package + 拒绝语料）

> - status: complete（实现切片；不移动任何 §3 gate）
> - phase: M24 切片队列 #2（codec slice 1，PNG 双向）
> - completed: 2026-09-30
> - contracts: [`RFC-0050`](../rfc/RFC-0050-image-codec-jpeg-png-v0.md)（A1 扁平面，owner 复裁决接受）、[`ADR-0019`](../adr/ADR-0019-rust-compiled-package-core-production-v0.md)（Rust 编译 package-core 生产路径，owner 复裁决接受）
> - evidence: runtime-verified（Windows x64 真实 wasmtime `=47.0.2` Component 路径，`1.98.0-x86_64-pc-windows-gnu`，`png =0.18.1`）；解码语料独立 oracle = Python `zlib` 手写 PNG 分帧（与 `png` crate 无共享代码）
> - support-matrix row: [`docs/evidence/m24/image-codec-support.json`](../evidence/m24/image-codec-support.json)（schema `sico.m24.codec-support.v0`）

## Objective

落地 M24 §8.4 转换引擎的第一块：`sico:user/image-codec@1` 的 **PNG 双向**
（`decode-png` / `encode-png`）作为**纯 package Component**，按 ADR-0019 的
Rust 编译 core 生产路径产出，对冻结语料字节稳定、§3 拒绝面逐行类型化，全程
经真实 runner Component 路径验证。零 compiler/Runtime 改动（RFC-0050 §4）。

## Changes

- **guest crate（workspace 外，ADR-0019 §2）**：[`guests/image-codec/`](../../guests/image-codec)
  —— 由 `image-codec-probe` 探针 guest 提升为生产件；`png =0.18.1`，release
  profile 钉死（`opt-level=3, lto, codegen-units=1, panic=abort, debug=0,
  strip`）；导出 canonical-ABI `realloc` + `decode-png` + `encode-png`，无
  import（纯）。已加入根 workspace `exclude`，根 `--locked --offline` 构建不
  触及它。
  - **A1 对齐**：encode 侧像素缓冲长度 != `width·height·4` 现返回
    `encode-buffer-mismatch`（探针期临时记为 `corrupt-stream` 的已量缺陷，A1
    接受后改正）。
- **提交件 = digest-pinned core `.wasm`（ADR-0019 §3）**：
  [`crates/sico-codegen-wasm/artifacts/image-codec-core.wasm`](../../crates/sico-codegen-wasm/artifacts/image-codec-core.wasm)
  （194111 B）+ `.sha256`
  （`28917071c355bc5fc826502d4d2ca85a14c7b11d7f225f164d8d6d3b34ad9322`）。
- **producer**：`package_builder.rs::image_codec_package()` —— `include_bytes!`
  嵌入提交 core，调用与手工发射包**同一个** `build_package_component
  ("image-codec", 1, …)`，函数面为 A1 扁平形
  （`decode-png(Bytes)->Result[Bytes,Text]`、
  `encode-png(Bytes,U64,U64)->Result[Bytes,Text]`）。消费者与 runner 看不到与
  `image_vision_package()` 的差别。
- **复现校验器（ADR-0019 §4）**：[`tools/validate-package-core.ps1`](../../tools/validate-package-core.ps1)
  —— 从干净树重建 guest（`--offline --locked`），断言重建 core 与提交件
  **字节相同**（SHA-256 比对）；含 core 注册表，#3 JPEG 可加行复用。
- **冻结解码语料**：[`runner/sico-runner/tests/corpus/image-codec/`](../../runner/sico-runner/tests/corpus/image-codec)
  —— 9 个 PNG（1 ok + 8 类型化拒绝）+ `manifest.json`（schema
  `sico.m24.codec-fixtures.v0`，含每例期望 payload-hex / case 名），由独立
  Python oracle 生成并提交冻结。
- **runner 内容测试**：[`runner/sico-runner/tests/image_codec_package.rs`](../../runner/sico-runner/tests/image_codec_package.rs)
  —— 5 个测试，期望从 manifest 读取（测试与 oracle 不会各自漂移）。
- **支持矩阵行**：[`docs/evidence/m24/image-codec-support.json`](../evidence/m24/image-codec-support.json)。

## Executed validation

- `cargo test --manifest-path runner/sico-runner/Cargo.toml --test image_codec_package`
  → **5 passed; 0 failed**：
  - `image_codec_package_is_deterministic_and_pure`：两次构建字节相同；组件对
    **空 linker** 实例化成功（core 无 import ⇒ 满足 runner `prepare_package`
    纯度检查）；恰好导出一个 `sico:user/image-codec@1.0.0` 实例。
  - `decode_corpus_matches_independent_oracle`：9 例逐行——ok 例 payload 字节
    精确（16 B LE 头 + RFC-0041 BGRA8），8 例拒绝 case 名精确
    （`bad-signature`/`truncated`/`trailing-data`/`oversized`(limit+1)/
    `unsupported-profile`×3(palette/depth-16/interlaced)/`corrupt-stream`(CRC)）。
  - `refusal_corpus_covers_the_a1_closed_set`：§3 解码可达闭集逐 case 覆盖。
  - `encode_round_trips_and_is_byte_stable`：`decode(encode(x))==x`（payload
    级），且两次 encode 字节相同（冻结 Paeth/level-9/`{IHDR,IDAT,IEND}`）。
  - `encode_rejects_buffer_mismatch_with_the_a1_case`：缓冲长度不符 →
    `encode-buffer-mismatch`（A1 case）。
- `tools/validate-package-core.ps1` → **PACKAGE-CORE VERDICT: reproducible**
  （clean 重建 sha256 == 提交件 == 记录值）。
- `cargo clippy -p sico-codegen-wasm --all-targets -- -D warnings` 绿；
  `cargo clippy --test image_codec_package -- -D warnings` 绿；
  `cargo fmt --check`（codegen + runner）绿。
- `tools/validate-step-0124.ps1 -SelfTest` 绿；`git diff --check` 绿。

## Honest residuals

- 本片**不移动任何 M24 §3 gate**：gate 1（= M17 gate 2 加速器硬件）与 #4
  Direct2D 渲染器仍是外部门（见 STEP-0330 Honest residuals）；M24 overall
  仍非 full GO。
- **仅 PNG**。JPEG（#3）尚未开始，且按 STEP-0330 §4 需先过一支同形独立
  oracle 探针钉死 SOF0/quality-90/4:4:4 输出字节，未过不进实现切片。
- RFC-0050 §4 拒绝语料中的 **cancellation 中途取消** 与 **typed save-failure**
  属 runner 执行框架 + 文件能力路径，需运行 Sico 脚本端到端的转换器试点
  （#6）才能逐行类型化；本片覆盖的是**纯 package Component 级**的解码/编码
  内容合同（profiles/limits/typed refusals/byte-stability），**batch sampling**
  同为 #6 试点级关切。此划分如实登记，不以 package 级证据冒充脚本级证据。
- 提交了一个**新二进制 artifact 类**（`.wasm`）：digest-pinned + 复现校验，
  可审计；codec crate（`png =0.18.1`）的供应链审查属本片范围，已钉版本，任何
  升级触发 chunk-set/字节稳定性复验（guest encode 后自扫 chunk 集即为此设）。
- M24 plan §8.4「conversion engine … provides no codec I/O (honest gap)」与
  §8.4 把 RFC-0050 标为「draft, owner acceptance required」均已陈旧（PNG codec
  已落地、RFC-0050+A1 已接受），但**本片不改 plan 文本**以免触碰
  `validate-step-0124` 的 M24 不变式；统一留待 #8 出口审计同步。
- M7 lock（`sico-lock.json`）条目随首个消费该包的试点（#6）落地；本片以
  artifact `.sha256` + 复现校验器完成 digest-pinning。
