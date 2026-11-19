# M22 `selfhost_compiler` 套件回归归因（2026-11-19）

- 结论：`selfhost_compiler` 套件大批转红（实测 ~55 FAILED / 135 ok，且仍在跑时已见此比例），**归因于并发 M24 代理对 Rust oracle 核心 crate 的未提交改动，非 M22 门 7 改动（STEP-0639, commit `71dcf71`）**。
- 本片性质：归因/协调登记，**零源码改动、不碰并发代理文件、不翻任何 gate**。

## 1. M22 门 7 交付（本人，绿）

- `every_match_site_is_measured_on_the_component_axis` 按选项 B 重定范围（接纳 25 个 `scalar_ir` pin 背书 not-comparable 残差），**套件上下文实测 ok**。见 [`STEP-0639`](../steps/STEP-0639-m22-gate7-component-axis-option-b-narrowing.md)，commit `71dcf71`。
- 该改动**只触及那一个测试函数体**；测试函数相互独立，改一个不可能改变其它测试结果。

## 2. 回归现象

全量 `selfhost_compiler` 套件（`cargo test --locked --offline --test selfhost_compiler`）大批 `*_byte_exactly` / pin 测试 FAILED，样本：

- `sico_compiler_lowers_the_text_concat_as_the_whole_expression_byte_exactly`：挂在 **oracle IR 尺寸 pin**（`tests/selfhost_compiler.rs:6447`）—— `left: 2349 / right: 2295`，oracle canonical IR 涨 54 B。
- 另有 `the_t2_g2_nested_call_shapes_lower_byte_exactly` / `the_t2_g3_operator_operand_gap_is_measured_and_attributed` / `the_t2_g2_s_site_refusals_belong_to_other_families` / `the_match_construct_truth_is_measured_and_pinned` / 多个 `sico_codegen_guest_*` / `sico_compiler_lowers_*_byte_exactly` 等。

## 3. 归因（为何不是 M22 门 7 改动）

- 失败全是**降级/发射 vs Rust oracle** 的字节/尺寸 pin；oracle = `sico-semantics` / `sico-ir` / `sico-codegen-wasm`。
- 并发 M24 代理对这些 oracle crate 有**未提交**改动（`git diff --stat`）：
  - `crates/sico-codegen-wasm/src/lib.rs` **+136**
  - `crates/sico-codegen-wasm/src/canonical.rs` +6
  - `crates/sico-ir/src/lib.rs` +17
  - `crates/sico-semantics/src/lib.rs` +17
  - 合计 **+176**（RFC-0057 bytes→U64 / I64→U64 cast intrinsic + RFC-0051-A2）。
- 这些改动移动了 oracle 的 canonical IR / 组件输出 ⇒ guest 侧（冻结 `selfhost/*.sico` + guest 编译器，未变）与 oracle 不再逐字节相等 ⇒ 大批 pin 转红。
- 反证：M22 门 7 改动只改一个测试函数体，不触及 oracle crate，也不可能改变其它测试函数的结果。

## 4. 建议（需 owner 协调，本片不擅动）

1. **M24 代理自行收口**：落地 RFC-0057 / RFC-0051-A2 时，需把被移动的 oracle IR 尺寸 pin / byte-exact pin **一并重基线**（其 STEP 文字声称「既有 byte-exact pin 逐字节不变」，实测变了 ⇒ 需复核是预期重基线还是回归）。
2. **owner 裁优先级**：M22 套件全绿是门 7 / 出口整轮 / 独立 CI 的前提；M24 的 oracle 改动持续打破它 ⇒ M22 出口证据取不到。需 owner 排序两线。

## 5. 边界

- 本片**未修改**任何 `crates/*`（并发代理 in-flight 工作，§10.2/§9 不碰）。
- 未跑全量到结束（小时级且信号被并发回归污染；门 7 单测结果已取得）。
- **未翻任何 M22 出口门；M22 overall 仍 NO-GO**；仅 internal-fixture 证据。