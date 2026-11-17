# STEP-0614 · M22 · W3-a 燃料墙第一次字节恒等提速尝试：`cgs_helper_index` intrinsic-membership 预计算位掩码 —— 字节恒等（7/7 守卫绿）但对整份 `formatter.sico` main-entry 燃料**零效果**（主导 op 不在该查询族）

> **片型** = 实现 + 测量片。本片落地一个**字节恒等**的发射器提速（`selfhost/codegen.sico`），用新增燃料迹探针对整份 `formatter.sico` main-entry 发射做 before→after 实测，跑 7 守卫与质量门，并如实登记**负结果**：提速字节恒等成立，但**不降低整份 formatter main-entry 的燃料**（仍是 800e9 上限耗尽）。
>
> **约束遵守**：不上调任何 fuel/内存/超时限额（owner D-B 否决维持）；不动 `emitter-vocabulary-v1.json`；不改 oracle 语义/七门/ADR-0015；函数体内无 `//` 注释；不预留 STEP 号（本片取共享序列下一号 **STEP-0614**）；未 commit/push；未翻任何门，M22 overall 仍 **NO-GO**。
>
> **改动面**：`selfhost/codegen.sico`（+6 新函数 `cgs_intrinsic_bit`/`cgs_instruction_intrinsic_bit`/`cgs_block_intrinsic_bits`/`cgs_function_intrinsic_bits`/`cgs_intrinsic_mask`/`cgs_intrinsic_present`；`cgs_helper_index` 由「每调用 ~15 次 `cgs_module_has_intrinsic` 全模块重扫」改为「每调用一次 O(module) 位掩码构建 + 15 次 O(1) 位测试」）；`runner/sico-runner/tests/selfhost_formatter_component_measurement.rs`（+1 个 `#[ignore]` 燃料迹探针 `m22_w3a_whole_formatter_main_entry_fuel_trace`，before/after 共用）。
>
> **复现路径**：探针 = `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_formatter_component_measurement -- --ignored --exact --nocapture m22_w3a_whole_formatter_main_entry_fuel_trace`（`SICO_TEST_WASMTIME` 指向 `target/tooling/wasmtime-v46.0.1-x86_64-mingw/wasmtime.exe`；标准预算 800e9，未上调）。

## 0. 结论摘要（一句话）

**STEP-0611 推荐的「下一刀」——把 intrinsic-membership 查询族从「每次查询重扫整份 IR JSON」改为「每模块一次预计算 + O(1) 查询」——已在本片以字节恒等形式实现（7/7 守卫绿、质量门全绿），但对整份 `formatter.sico` main-entry 发射燃料**零效果**：before 与 after 均为 `FuelExhausted @ 800e9`（84.2s vs 84.25s，噪声内），耗尽函数与调用栈不变。⇒ **整份 formatter main-entry 的主导 op 不是 `cgs_helper_index` 的 intrinsic-membership 重扫**；耗尽发生在发射器另一条扫描链（根 `component_hex_for_ir` → `cgs_mul`/`cgs_sub`/`cgs_add` → 内层扫描帧，最内层 wasm func#74、offset 56632）。**未翻任何门，M22 overall 仍 NO-GO。**

## 1. 背景与本片目标

STEP-0606 定位燃料墙为「match-dispatcher 重扫 + `text_concat` 装配」两个具名超线性算子；STEP-0611 将其共同底层 op 钉死为 `cgj_value_end` JSON 逐字节扫描（由 intrinsic-membership 三级重扫 `cgs_module_has_intrinsic`→`cgs_function_has_intrinsic`→`cgs_block_has_intrinsic`→`cgs_instruction_is_intrinsic` 驱动），并给出「下一刀」= 预计算索引/位掩码使查询 O(1)。本片把该刀在 `cgs_helper_index`（9 个发射臂共用、每臂 ~15 次 `cgs_module_has_intrinsic` 全模块重扫）上实现为**每调用一次 O(module) 位掩码构建 + 15 次 O(1) 位测试**，并实测其对**整份 `formatter.sico` main-entry** 发射的效果（W3-a 的真挡板在 multi-block-`run` 落地后即为此燃料墙，见 STEP-0610/0612）。

## 2. 改动（字节恒等的论证）

`cgs_helper_index`（codegen.sico）原逻辑：`cgs_function_count` 一次（O(F)）+ 对每个词表 helper 各一次 `cgs_module_has_intrinsic(bytes, name)`（每次 O(F×B×I×value_scan)），共 ~15 次全模块重扫，用于算 helper rank。本片改为：

- 新增 `cgs_intrinsic_bit(name) returns U64`：把 15 个词表 intrinsic 名各映射到一个唯一 bit（1<<0..1<<14；未知名 → 0）。bit 表与 `cgs_helper_index` 的 rank 链一一对应。
- 新增 `cgs_instruction_intrinsic_bit(bytes, instr) returns U64`：对单条指令，若其为 `intrinsic` op（`operation`→`op`=="intrinsic"→`data`→`name`，**与 `cgs_instruction_is_intrinsic` 相同的 fail-closed 结构检查**），返回该指令自带 intrinsic 名的 bit；否则 0。
- 新增 `cgs_block_intrinsic_bits`/`cgs_function_intrinsic_bits`/`cgs_intrinsic_mask`：单遍走 functions→blocks→instructions，OR 入所有 bit，得到模块级位掩码（一次 O(module) 遍）。
- 新增 `cgs_intrinsic_present(mask, name) returns U64`：`bit_and(mask, bit(name)) != 0`，O(1)。
- `cgs_helper_index`：在 `sico.bytes.equal` 提前返回之后构建一次 `cchi_mask = cgs_intrinsic_mask(bytes)`，把 15 处 `cgs_module_has_intrinsic(bytes, "X")` 全部换成 `cgs_intrinsic_present(cchi_mask, "X")`（含 `is_utf8` 回退臂）。

**字节恒等论证**：旧 `cgs_module_has_intrinsic(bytes, X)` 返回 1 当且仅当「存在某指令为 intrinsic op 且其 name==X」；新 `cgs_intrinsic_present(mask, X)` 返回 1 当且仅当「`cgs_intrinsic_bit(name)` 含 bit(X)」且该 bit 在掩码中被某指令 OR 入，即「存在某指令为 intrinsic op 且其 `cgs_intrinsic_bit(name)`==bit(X)」。因 `cgs_intrinsic_bit` 在词表上单射、未知名映射 0，二者对全部 15 个被查询名逐名相同 ⇒ rank 序列、`cgs_helper_index` 返回值逐字节相同 ⇒ 发射字节不变。**仅当 mask 构建正确（结构与旧 `cgs_*_has_intrinsic` 逐级相同、fail-closed 保留）时成立；7/7 守卫字节全等证实。**

## 3. 实测（before → after，整份 `formatter.sico` main-entry）

探针 `m22_w3a_whole_formatter_main_entry_fuel_trace`：Rust oracle 产出 oracle 组件（`oracle_bytes_A_main = 28459`），guest `--emit-component-hex` 全编译器跑整份 `formatter.sico`，`run_with_fuel`（800e9 标准预算）记录 `fuel_consumed`/wall/耗尽调用栈/字节等同。

| 阶段 | fuel_consumed | wall | outcome | 字节 |
|---|---|---|---|---|
| before（原 WIP codegen.sico） | `Some(800000000000)` | 84.221s | `FuelExhausted` | n/a（耗尽，无字节） |
| after（本片改动） | `Some(800000000000)` | 84.252s | `FuelExhausted` | n/a（耗尽，无字节） |

**读数**：两者都恰在 800e9 上限耗尽（`fuel_consumed` 被钉在上限，真值 >800e9，只能得下界）；wall 84.221s→84.252s（+0.031s，噪声内）。**字节未产出**（耗尽发生在发射完成前），故无逐字节可比对；oracle 字节仍为 28459B（STEP-0612 已证明 oracle 侧可发射整份 formatter）。

**耗尽调用栈（after，与 before 同构，仅偏移因 codegen.sico 变更轻微位移）**：最外层 wasm func#34 = `component_hex_for_ir`（发射入口），经 `cgs_mul`#69 → `cgs_sub`#68 → `cgs_add`#67 → 内层扫描帧 #87/#81/#80/#79 → 最内层 **#74（offset 56632）**；另有 2 个 shim 适配帧 #244/#245。**`cgs_helper_index` 不在耗尽栈上** —— 与本片「零效果」自洽。

## 4. 关键发现（订正 STEP-0611 推断对整份路径的适用性）

1. **STEP-0611 的「下一刀」对整份 formatter main-entry 无效**：本片实现该刀（字节恒等、7/7 守卫绿），但整份 formatter 发射燃料**无任何可测下降**。⇒ 整份 formatter 的主导 op **不是** `cgs_helper_index` 驱动的 intrinsic-membership 重扫（STEP-0611 的归因基于 **emit-only 注入**的单一真实站点 + 合成族；**不必然外推到整份 formatter 的全编译器路径**）。
2. **耗尽在发射器另一条扫描链**：根 `component_hex_for_ir` → `cgs_mul`/`cgs_sub`/`cgs_add` → 内层扫描帧 → 最内层 wasm func#74。该链与 `cgs_helper_index` 不相交。
3. **燃料真值 >800e9（下界已知，精确值未知）**：两档都恰在上限耗尽，`fuel_consumed`=上限是被钉死的下界读数，非精确消耗。

## 5. 验证

- **7 守卫全绿（`--exact` 逐个，本机）**：`sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`、`sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly`、`sico_compiler_lowers_the_step0513_codegen_function_byte_exactly`、`sico_is_utf8_gated_run_lowers_byte_exactly`、`sico_run_emits_the_minimal_multiblock_run_component_byte_exactly`、`sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly`、`sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly` —— **7 passed / 0 failed**。**字节恒等保持**（本片改动不改变任何被测发射字节）。
- **质量门**：`cargo fmt --all --manifest-path runner/sico-runner/Cargo.toml -- --check` 净；`cargo clippy --manifest-path runner/sico-runner/Cargo.toml --test selfhost_formatter_component_measurement --test selfhost_compiler -- -D warnings` 净；`python tools/validate-selfhost-comment-discipline.py selfhost/codegen.sico` = `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2`；`git diff --check` 净。

## 6. 诚实收窄（必须记录的口径边界）

1. **燃料数值是下界**：两档都在 800e9 上限耗尽，`fuel_consumed=800000000000` 是被钉死的下界；整份 formatter main-entry 真值 >800e9，精确值未测（测精确需推高预算，违反 D-B 否决/超时）。
2. **最内层耗尽帧未解析到 Sico 名**：整份 formatter 探针用的是**全编译器组件**（compiler.sico + compiler_lexer.sico + compiler_parser.sico + parser.sico + codegen.sico 五文件），其 wasm 函数布局**非** 1:1 对应 codegen.sico 声明序（STEP-0611 的 1:1 映射只对 emit-only 注入组件验证）；func#74 的精确 Sico 名未解析（需五文件布局的 code-section 解析探针，本片未做）。`module_offset` 为 core module 内偏移。
3. **before/after 均未产出字节**：因耗尽在发射完成前，无逐字节可比对（oracle 28459B 已知，但 guest 未达发射完成）。字节恒等由 7 守卫的既有字节全等用例保证，非本片整份 formatter 直接证据。
4. **仅 1 个真实路径点（整份 formatter main-entry）+ 7 守卫**：非 396 站点账本、非完整 emit-only 注入族；未跑全量 `selfhost_compiler` 套件/电池。
5. **codegen.sico 本片改动已保留在工作树**（字节恒等、7/7 守卫绿，但零燃料效果）；因 WIP 未 commit 且本片无法在不冒险恢复 WIP 的前提下干净回退，保留并如实标注「对整份 formatter 燃料零效果」。owner 如需回退可单点撤销本片 6 函数 + `cgs_helper_index` 改写。
6. **未触碰** `selfhost/parser.sico`、`emitter-vocabulary-v1.json`、`match-component-coverage-v1.json`、oracle、七门、ADR-0015；未上调任何 fuel/内存/超时限额；未 commit/push。
7. **并发注意**：本片只取共享序列 **STEP-0614**（核对 `docs/steps/` 最高 0613 + 字母后缀，无 0614/0615 文件）；未覆盖他片 STEP。

## 7. 残余与是否需 owner 级决策

- **整份 formatter main-entry 燃料墙未破**：仍 800e9 上限耗尽。本片证明「intrinsic-membership 预计算」刀对整份路径无效，主导 op 在另一条发射扫描链（根 `component_hex_for_ir`）。
- **下一步（建议，未动工）**：对**整份 formatter main-entry**（全编译器组件）做 op 级耗尽剖析——用五文件布局解析 code-section，把最内层 func#74（offset 56632）解析到精确 Sico 函数名；据其定位真正的超线性扫描/装配链。这需要一条新增的布局解析探针 + 一次全编译器燃料迹，属测量片。
- **是否需 owner 级决策**：**建议是**。本片已实测 STEP-0611 推荐的破墙刀对整份 formatter 无效，而主导 op 在另一条链上；若该链是「装配/重扫」类二次项，破墙可能需要更大重构（非单点缓存）；owner 需在「继续 op 级剖析定位主导链」vs「按 D8 已否决的限额路径收窄语料/闭包口径」间裁决。本片不越权裁决。
