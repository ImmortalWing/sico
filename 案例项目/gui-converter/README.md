# PRJ-0001: GUI 格式转换器（Sico 原生 UI 库 clean-room consumer）

> - 编号: **PRJ-0001**（外部应用项目,独立于 M/STEP 共享序列,依 owner 2026-11-19 治理政策）
> - 状态: 应用已编写并跑绿 PNG↔PNG 转换；JPEG↔PNG 双向待 image-codec 核心含 JPEG 的 re-pin（E2）
> - 日期: 2026-11-19
> - 语言核心依赖: [`RFC-0057`](../../docs/rfc/RFC-0057-language-v1-batch5-bytes-to-u64-and-i64-cast.md)（`sico.bytes.le_u64` + `sico.conv.u64_from_i64`，accepted + 已落地 STEP-0637）
> - 证据等级: `clean-room-consumer`

## 这是什么

一个用 **Sico 语言编写**的 GUI 图片格式转换器,作为 Sico 原生 UI 库（`sico.ui.*`）+ 包系统（`image-codec@1`）+ 文件能力（`sico.picker.*`/`sico.fs`）的 **clean-room consumer**。它把一张 PNG 输入经 解码 → 取宽高 → 重编码 → 写文件,产出**字节稳定**的 PNG 输出。

迁移自原 `pilots/g6-converter/`（依 owner 2026-11-19 治理政策,GUI 转换器属外部应用,不占 M/STEP 序列）。

## 文件

| 文件 | 说明 |
|---|---|
| `converter.sico` | 应用源码：`interface ImageCodec version 1`（`decode_png`/`encode_png`）+ `use pkg image_codec version 1 expose ImageCodec` + `sico.ui.*` 13-func 面 + `sico.picker.*` + `sico.fs.read/write`；事件循环 window→column→status text→Convert 按钮→progress→image_preview→`run()`→有界 `poll_event()`；Convert = `picker.open → fs.read → decode_png → le_u64(取宽高) → encode_png → fs.write`（写仅在编码成功后 ⇒ 失败/取消不产部分文件）；全 typed 状态、无静默回退、无 trap |
| `sico-lock.json` | 包锁（`image_codec` v1,sha `9ba7c2f0…`） |
| `pkgs/image_codec.sapp` | 包二进制（wrapped + dev-signed,195389 B） |

## 运行 / 验证

- 构建：`sico build --profile script-v0 --output <comp.wasm> 案例项目/gui-converter/converter.sico`（`use pkg` 经 `sico-lock.json` 解析）。
- 端到端证据测试：`runner/sico-runner/tests/g6_converter_pilot.rs`（**16/16 绿**,MSVC）。该测试 `include_str!`/`include_bytes!` 引用本目录源码,经真实 Runner + 确定性 UI/picker 后端 + 真实包驱动,断言字节稳定重复、各 fail-closed typed 拒绝、取消无部分输出。测试留在 runner 测试树（Rust,属可执行证据）,源码在本项目目录。

## §8.5 fail-closed corpora（M24 计划）

九组全 typed（无 trap / 无 hang / 无部分文件 / 无静默回退）：bad-signature、truncated、trailing-data、oversized（limit+1）、unsupported-profile（PNG color-type：palette / depth-16 / interlaced）、cancellation-mid-conversion、save-path-failure、byte-stable repeat、long-run batch sampling —— **PNG 可覆盖的全部绿**。

## 具名残余

- **JPEG↔PNG 双向未达**：committed `image-codec-core.wasm` 为 PNG-only（sha `28917071…`）≠ 含 JPEG 重建（sha `cc47a3ea…`）,`validate-package-core.ps1` 判 `NOT-reproducible`,须 owner 协调刻意 re-pin（ADR-0019 digest-pinned）。这是 M24 gate 5 的 E2 外部门。
- 原生窗口隐藏 / 点击 host-injected 坐标 / `select` declared-but-not-produced（RFC-0052-A3=A1）——文档化限制,非本项目缺口。

## 关联

- 语言核心依赖：[`RFC-0057`](../../docs/rfc/RFC-0057-language-v1-batch5-bytes-to-u64-and-i64-cast.md) / [`STEP-0637`](../../docs/steps/STEP-0637-rfc0057-bytes-u64-and-i64-cast-intrinsics.md)
- 历史（迁移前登记,含 PNG-only 切片 + v0 缺口实测）：[`STEP-0635`](../../docs/steps/STEP-0635-m24-g6-converter-pilot-png-only.md)
- 审计：[`m24-exit-audit-2026-11-19-refresh.md`](../../docs/reports/m24-exit-audit-2026-11-19-refresh.md) gate 5
