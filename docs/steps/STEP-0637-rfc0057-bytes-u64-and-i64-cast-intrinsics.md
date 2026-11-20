# STEP-0637 — RFC-0057 language v1 batch 5: `bytes`→`U64` read + `I64`→`U64` cast (implementation)

> - **status: WITHDRAWN（owner 2026-11-20 裁决「3」回退）**。实现已回退（5 个 oracle crate 复原 HEAD、RFC-0057 测试文件删除）。理由：唯一消费者 GUI 转换器已随 gate 5 重定划为外部项目、不再需要；其 oracle 改动经实测证明**非** `selfhost_compiler` 红根因（回退后红依旧，真因 = checkpoint `94ea1fa` 的 guest↔oracle 自举分叉，归 M22 轨道）。详见 [`RFC-0057`](../rfc/RFC-0057-language-v1-batch5-bytes-to-u64-and-i64-cast.md)（已标 withdrawn）。
> - status（原）: implementation complete locally（**未 commit/push、不翻任何 gate、未触碰 `selfhost/*`/冻结工件**）
> - date: 2026-11-19（withdrawn 2026-11-20）
> - contract: [`RFC-0057`](../rfc/RFC-0057-language-v1-batch5-bytes-to-u64-and-i64-cast.md)（原 accepted owner 2026-11-19，**2026-11-20 withdrawn**）
> - validator: `cargo test --manifest-path runner/sico-runner/Cargo.toml --test bytes_u64_cast` + `--test g6_converter_pilot` + `crates/sico-codegen-wasm` intrinsic probe + 既有 `*_byte_exactly` pin 守卫
> - evidence class: `internal-fixture`（语言核心 intrinsic）；下游 G6 pilot 为 `clean-room-consumer`

## 1. 目的

落地 RFC-0057 的两个纯语言 intrinsic，补 Sico v0 的 `Bytes`→`U64` 读取与显式 `I64`→`U64` 转换缺口，解锁 M24 G6 转换器 pilot 的 PNG 重编码（STEP-0635 实测的 v0 决定性缺口）。

## 2. 改动面（Rust-oracle 路径，零 `selfhost/*`）

- `crates/sico-semantics/src/lib.rs` — `infer_stdlib_call`：+2 臂（`sico.bytes.le_u64(Bytes,U64)`、`sico.conv.u64_from_i64(I64)`，返回 `Result[U64,Text]`，error 类型直接构造 `Text`、非 `NumericError`）。
- `crates/sico-ir/src/lib.rs` — `intrinsic_signature`：+2 臂（`Type::Result{ok:U64,error:String}`，绕开钉 `NumericError` 的 `result_of` 闭包）。lowering 数据驱动，`sico-ir/lower.rs` 零改动。
- `crates/sico-codegen-wasm/src/canonical.rs` — +`INTRINSIC_ERROR_TEXTS = &["out-of-bounds","negative"]`。
- `crates/sico-codegen-wasm/src/lib.rs` — 数据段 builder 加 **guard 的** intrinsic error-text intern（appended 在既有表之后，程序不用该 intrinsic 则数据段字节不变）+ `emit_intrinsic` 两臂（4-slot `Result[U64,Text]` 布局 `[I32 tag, I64 value, I32 err_ptr, I32 err_len]`；ok=`[0,u64,0,0]`、error=`[1,0,ptr,len]`；OOB 判 `len<8 || len-8<offset` 防 `U64::MAX` 回绕；`u64_from_i64` 位保持 + 符号检查）。

**关键纪律**：数据段 intern **guard 化** ⇒ 既有程序（不用新 intrinsic）数据段逐字节不变 ⇒ 全部既有 `*_byte_exactly` pin 不动。**未触碰** `selfhost/codegen.sico`/`parser.sico`/`emitter-vocabulary-v1.json`/`corpus-parity-v0.json`（并发 M22 代理持有）、`image-codec-core.wasm`、WIT、包接口、`sico-lexer/parser/hir/runtime/cli`。G6 走 `sico build --profile script-v0` = Rust oracle（`compile_script_program`），guest 自举路径非 G6 所需。

## 3. 语义（fail-closed）

- `sico.bytes.le_u64(bytes, offset)`：小端定长 8 字节读；`offset+8 > len` → `error("out-of-bounds")`（OOB 判写为 `len<8 || len-8<offset`，防 `offset` 近 `U64::MAX` 时 `offset+8` 回绕）。
- `sico.conv.u64_from_i64(value)`：`value >= 0` → `ok(value as U64)`（位保持）；`value < 0` → `error("negative")`（**不回绕**）。
- 二者纯计算、无 hostcall/IO/新权限、O(1)、受既有 fuel/内存预算约束、不新增 trap 面（错误走 `Result`）。

## 4. 验证（本机 MSVC 实跑，全绿）

| 命令 | 结果 |
|---|---|
| `cargo test -p sico-semantics --lib` | 6 passed |
| `cargo test -p sico-codegen-wasm --lib --tests` | 25 passed, 1 ignored（含新 `rfc0057_bytes_u64_intrinsics_validate`：IR 级确定 + `wasmparser::Validator` pass） |
| `cargo test --manifest-path runner/sico-runner/Cargo.toml --test bytes_u64_cast` | **3 passed**（新 fixture `tests/end-to-end/bytes-u64-cast.sico`：LE 读 ok / OOB→`out-of-bounds` / `u64_from_i64(5)`→ok / `(-1)`→`negative` / 错参数 typed E200x 拒） |
| 既有 `*_byte_exactly` pin 守卫 | `script-echo/error/task-scope-and-collect`（script_abi.rs）、`numeric/control/...-component`（backend.rs）逐字节不变（guard 生效） |
| clippy `-D warnings`（semantics/ir/codegen-wasm + runner tests） | clean；`git diff --check` 净 |

主代理独立复跑：`bytes_u64_cast` **3/3**、`g6_converter_pilot` **16/16**。

## 5. 下游收益（G6 pilot，`clean-room-consumer`）

`pilots/g6-converter/converter.sico` 改用 `sico.bytes.le_u64` 取解码负载宽高，新增 `re_encode`（`sico.bytes.slice` 像素 → `image_codec.encode_png` → `fs.write`）。`g6_converter_pilot.rs` 由 13 增至 **16 测试全绿**，三个此前 blocked 的 §8.5 case 转绿且产**真实字节稳定 PNG 输出**：`save_path_failure_is_a_typed_status`、`byte_stable_repeat_produces_identical_png`、`long_run_batch_sampling_terminates_typed_every_case`。⇒ M24 gate 5 的 **PNG↔PNG 半边闭合**（§8.5 九组 PNG 可绿全绿）；唯 `unsupported-profile` 的 JPEG-profile 半边 + JPEG↔PNG 双向仍待 **E2 核心 re-pin**（owner 外部门）。

## 6. 诚实收窄

- 仅 Rust-oracle 路径；guest 自举（M22）发射这些 intrinsic 需另开 `selfhost/codegen.sico` 臂——**本片未做**（该文件并发代理持有，且非 G6 所需），留 M22 轨协调。
- 单机快照、非独立 CI、未跑全 workspace 套件（过慢）、未跑 `selfhost_compiler`（M22 持有）。
- 未翻任何 M24 gate：gate 5 仍 NO-GO（差 E2 JPEG 双向）；仅 `internal-fixture`（intrinsic）+ `clean-room-consumer`（G6 pilot）证据。
- 治理备注：owner 2026-11-19 治理政策将应用层（GUI 转换器）划为外部项目（`案例项目/`+`PRJ-xxxx`，不占 M/STEP 序列）。本 STEP 只登记**语言核心 intrinsic**（属 M/STEP 序列）；G6 pilot 应用的最终归类/搬迁待 owner 明确。

## Links

- 合同：[`RFC-0057`](../rfc/RFC-0057-language-v1-batch5-bytes-to-u64-and-i64-cast.md)
- 下游：[`STEP-0635`](./STEP-0635-m24-g6-converter-pilot-png-only.md)、**PRJ-0001** [`案例项目/gui-converter/`](../../案例项目/gui-converter/)（原 `pilots/g6-converter/`，依 owner 2026-11-19 治理政策迁为外部应用项目、不占 M/STEP 序列；迁移后 `g6_converter_pilot.rs` 仍 16/16 绿）。
- 审计：[`m24-exit-audit-2026-11-19-refresh.md`](../reports/m24-exit-audit-2026-11-19-refresh.md) gate 5
