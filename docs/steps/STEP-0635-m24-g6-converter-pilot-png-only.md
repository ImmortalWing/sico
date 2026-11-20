# STEP-0635 — M24 G6 转换器 pilot（PNG-only，clean-room-consumer）

> **迁移备注（2026-11-19）**：本 STEP 记录的转换器应用已依 owner 2026-11-19 治理政策从
> `pilots/g6-converter/` 迁至 **`案例项目/gui-converter/`（PRJ-0001）**——GUI 转换器属外部应用项目，
> 用 `案例项目/<name>/` + `PRJ-xxxx` 独立编号、不占 M/STEP 共享序列。下文中的 `pilots/g6-converter/*`
> 路径为**迁移前历史记录**，现对应 `案例项目/gui-converter/*`。迁移后端到端测试
> `runner/sico-runner/tests/g6_converter_pilot.rs` 仍 **16/16 绿**（`include_str!`/`include_bytes!`
> 已改指新路径）。RFC-0057 落地（STEP-0637）后 re-encode 已通、§8.5 三组由 blocked 转绿，
> PNG↔PNG 半边闭合（详见 [`STEP-0637`](./STEP-0637-rfc0057-bytes-u64-and-i64-cast-intrinsics.md)
> 与 [`PRJ-0001 README`](../../案例项目/gui-converter/README.md)）。

状态：implementation complete locally（2026-11-19，MSVC 证据基底，`--locked --offline`）。
**本片不翻任何 §3 gate 数值**：G6 转换器应用本体编写（PNG-only 切片），但 image-codec 核心
未重冻结、§8.5 corpora 未整体绿、JPEG（E2）为具名外部门 → **gate 5 仍 NO-GO**。
证据等级：**`clean-room-consumer`**（真实 Sico 源码编译 + 运行端到端，非 internal-fixture）。

## 1. 目标与诚实边界

M24 门 5「GUI 格式转换器 pilot」的本体应用：以 Sico 源码编写的 GUI 格式转换器，基于
`sico.ui.*`（RFC-0052 A1 host 面）+ `image-codec@1` M7 包（RFC-0050 按 A1 修正），在 Windows
evidence host 运行，§8.5 fail-closed corpora 全绿，无静默 fallback / 无 native escape / 无核心
compiler·Runtime 补丁。

**诚实收窄（本片）**：v0 语言缺口使本片只能做 **PNG-only 且不写文件** 的切片。**已实测确认**
（见 §3.1）Sico v0 **无 bytes→U64 读取器、无 I64→U64 强制转换**：`sico.bytes.at` 只回 `I64`，
`encode-png` 的宽度/高度参数是 `U64`，解码负载 16 字节 LE u64 头无法回喂给 `encode-png`。
故 re-encode 步为 **typed refusal**（`"re-encode unsupported: v0 lacks bytes->U64 and I64->U64 cast"`），
任何路径都不写文件。§8.5 需要写的三项（save-path failure / byte-stable repeat / batch 采样）
**不可完成**，作为 blocked 登记（§4）。

## 2. 交付物

| 文件 | 内容 |
|---|---|
| `pilots/g6-converter/converter.sico` | G6 转换器应用源码（PNG-only）：`interface ImageCodec version 1`（`decode_png`/`encode_png`）+ `use pkg image_codec version 1 expose ImageCodec`；`enum ui_event`（click/select/closed，钉定形）；`UiHandles` 记录；`build_tree()`（window→column→status text→Convert 按钮→progress→image_preview）；`do_convert()`（picker.open→fs.read→decode_png→picker.save→re-encode typed refusal）；`le_byte`/`add_u64`/`le_u64`（`sico.bytes.at`+`I64.shl`/`I64.bit_or` 组装 16 字节头，I64 视图）；有界事件循环（`POLL_BOUND`=8 次空 poll，关闭/耗尽退出） |
| `pilots/g6-converter/sico-lock.json` | 包锁（`image_codec` v1，`pkgs/image_codec.sapp` sha256 `9ba7c2f0…`） |
| `pilots/g6-converter/pkgs/image_codec.sapp` | 已生成包二进制（195389 B，与 `image_codec_package()` 逐字节同） |
| `runner/sico-runner/tests/g6_converter_pilot.rs` | 端到端 clean-room-consumer 测试：`sico_cli::run` 编译源码（staged 锁+包）→ 真实 `Runner` + 确定性 `ScriptedUi`（句柄 1..6，事件队列）+ `ScriptedPicker`（可配置 open/save，计数）+ 真实包 `image_codec_package()`；断言 §8.5 typed 状态、零写、取消、授权 default-deny 零触及 |
| `docs/evidence/m24/g6-converter-pilot.json` | 证据记录，`class: clean-room-consumer` |

### 2.1 §8.5 corpora 覆盖

**绿（PNG-可完成，全部 typed，无写、无 trap、无 hang、无静默回退）**：

| §8.5 案例 | 断言状态 | save 触及 |
|---|---|---|
| bad signature | `decode: bad-signature` | 0 |
| truncated file | `decode: truncated` | 0 |
| trailing data | `decode: trailing-data` | 0 |
| oversized dimensions limit+1 | `decode: oversized` | 0 |
| unsupported color type (palette) | `decode: unsupported-profile` | 0 |
| unsupported bit depth 16 | `decode: unsupported-profile` | 0 |
| interlaced | `decode: unsupported-profile` | 0 |
| cancellation mid-conversion（save 空路径） | `cancelled`（无写） | 1 |
| decode-ok → re-encode | `decoded 2x2 | re-encode unsupported: …`（无写） | 1 |
| cancelled open | `cancelled` | 0 |
| select 事件（宿主面 declared-but-not-produced） | `select unsupported in this pilot` | 0 |
| 授权 default-deny（UI 未授） | `build_tree: authority-denied`，后端零触及 | 0 |

**blocked（需 re-encode）**：save-path failure（不可写/拒写）；byte-stable repeat；long-run batch
sampling。三者皆因本片 re-encode 为 typed refusal（§3.1）不可执行，登记为 blocked 而非伪造。

## 3. 关键事实与证据

### 3.1 v0 语言缺口（已实测，决定性）

- `sico.bytes.at(bytes: Bytes, index: U64, fallback: I64) -> I64`（semantics lib.rs:4159-4162；
  codegen lib.rs:5636-5656；越界返回 fallback，无 trap）。
- 探针：`U64.shl(U64.literal(1), <I64>)` → `E2001 expected U64, found I64`；
  `U64.literal(<I64 var>)` → `E2001 expected Int, found I64` + `expected U64 literal in range,
  found non-literal Int`。
- 全量检索 v0 stdlib：仅 `sico.u64.to_text`/`sico.i64.to_text` 与 `I64.*`/`U64.*` 定宽算子
  （`checked_add/sub/mul/div`、`equal`、`less_than`、`bit_and/bit_or/bit_xor`、`shl`、`shr`）。
  **无 bytes→int 读取器、无 int 强制转换**。
- 后果：`encode-png` 的 `U64` 宽度/高度无法从解码负载的 16 字节 LE u64 头取得；E8016 戳记门
  亦禁止把接口参数声明为 `I64`（与冻结戳 `u64` 冲突 → E8016 拒绝）。⇒ **re-encode 不可在纯
  Sico v0 实现**。本片按纪律「typed refusal / 诚实收窄，不伪造」处理。

### 3.2 已测（2026-11-19，本机 MSVC，`--locked --offline`）

| 命令 | 结果 |
|---|---|
| `sico build --profile script-v0 --output target/tmp/g6probe/converter.component.wasm pilots/g6-converter/converter.sico` | 成功（源码 + `use pkg` 解析，产 Component） |
| `cargo test -p sico-cli --offline --test generate_pilot_packages -- generate_image_codec_package --ignored --nocapture` | 1/1 绿；`image_codec: 195389 bytes sha256=9ba7c2f0…` |
| `cargo test --offline --manifest-path runner/sico-runner/Cargo.toml --test g6_converter_pilot` | （见 §5 实跑结果） |

## 4. 具名残余 / 未完成

1. **re-encode 阻塞**（§3.1）：v0 无 bytes→U64 / 无 I64→U64，`encode-png` 不可从解码负载
   头取宽高 ⇒ §8.5 三项 blocked（save-path failure / byte-stable repeat / batch）。
2. **image-codec 核心 `.wasm` 未重冻结**：committed `image-codec-core.wasm`（sha `28917071…`，
   PNG-only）≠ 含 JPEG 重建（sha `cc47a3ea…`）；`validate-package-core.ps1` 判 `NOT-reproducible`；
   须 owner 协调刻意 re-pin（ADR-0019 digest-pinned 冻结工件）。**本片不动核心，不 re-freeze**。
3. **JPEG（E2）**：具名外部门（需离线 JPEG codec + 独立 oracle；`jpeg-decoder`/`jpeg-encoder`
   已离线 cache 且原生往返 1/1，但 committed core 未含 JPEG —— 见 4.2）。
4. **原生窗口限制**：窗口创建为 **HIDDEN**；点击为 **host-injected 坐标**（经
   `route_click_event`），非真实鼠标；`select` 在真实宿主面 declared-but-not-produced
   （RFC-0052-A3=A1 收窄）；标题 ≤256 UTF-8 字节；事件队列 256。见
   `docs/user-guide/NATIVE-UI.md`。本片确定性证据用 `ScriptedUi`（与真实后端同 typed
   `click(handle)` 面），真实 Direct2D 帧 + OS 窗口壳 + 原生 Win32 对话框为具名残余
   （ADR-0018 #4、RFC-0055 §7）。
5. **本片不翻 gate**：gate 5 仍 NO-GO（G6 应用 PNG-only 已写，但 codec 核心未重冻结 +
   §8.5 未整体绿 + JPEG 未达）；未翻其它 gate。

## 5. 验证（执行记录）

本片验证命令与结果（MSVC 工具链，`CARGO_TARGET_DIR=target/g6` 独立目录，避免与并发代理
`target/debug/sico.exe` 锁冲突）：

```
sico build --profile script-v0 --output target/tmp/g6probe/converter2.component.wasm pilots/g6-converter/converter.sico   # 成功（13394 B Component）
cargo test --offline --manifest-path runner/sico-runner/Cargo.toml --test g6_converter_pilot   # 13/13 绿（2.39s）
cargo clippy --offline --manifest-path runner/sico-runner/Cargo.toml --test g6_converter_pilot -- -D warnings   # 净（exit 0）
```

13/13 覆盖：bad-signature / truncated / trailing-data / oversized / unsupported-profile×3 /
cancelled open / decode-ok→save-cancelled / decode-ok→re-encode refusal / select typed /
picker cancelled flattening / default-deny zero-touch。`git diff --check` = 净。
**未 commit / 未 push**。并发代理预存脏文件（`selfhost/codegen.sico`、
`selfhost/corpus-parity-v0.json`、`selfhost/emitter-vocabulary-v1.json`、
`docs/STATUS.md`、`docs/steps/STEP-0636-…`、`docs/reports/m22-w4-refusal-reconcile-status.md`、
`tools/reconcile-m22-w4-refusals.py`）**未触碰**。
