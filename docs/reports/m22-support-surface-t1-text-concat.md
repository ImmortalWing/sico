# M22 支持面登记表 · T1 族（`sico.text.concat`）

> - status: **STEP-0522 更新**（G-b / G-e 两刀闭合；T1 族仍未闭合）
> - date: 2026-10-23
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**：族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）
> - depends: [STEP-0517](../steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（真值测量）、[STEP-0518](../steps/STEP-0518-m22-w2-s5-w2-t1-frontend-knife.md)（前端刀）、[STEP-0519](../steps/STEP-0519-m22-w2-s5-w2-t1-codegen-knife.md)（codegen 刀）、[STEP-0520](../steps/STEP-0520-m22-w2-s5-w2-shared-local-cell-support.md)（cell 共享前置）、[STEP-0521](../steps/STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表）、[STEP-0522](../steps/STEP-0522-m22-w2-s5-w2-t1-escaped-literal-decoder.md)（G-b / G-e 两刀）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`，Rust 后端 `compile_script_program`），**不是设计值**

## 1. 子集边界（D15 要求登记的六项）

| 项 | 值 | 证据 |
|---|---|---|
| 支持的名字 | **仅** `sico.text.concat` | STEP-0519：`cgs_intrinsic_hex` 的名字闸门；其它 helper 型内建仍 fail-closed |
| 实参个数 | **恰好 2** | STEP-0517：**元数边界在语义层**（1 个 / 3 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity`）⇒ guest **不自造**元数检查 |
| 实参类型 | **两个 `Text`**（canonical `string`） | STEP-0519：类型闸门 `cgs_value_ty` == `string` |
| 结果槽数 | **2**（pointer, length） | STEP-0519：结果槽闸门 `cgs_type_slots` == 2 |
| 语料覆盖 | **0 条** | S 内 2,943 个文本调用点**一个都没进语料** |
| 未覆盖形状 | **1 类缺口**（§3：G-d） | STEP-0522 普查 |

## 2. 族级真值表（20 行，实测）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。**每一行都由固件钉住**（`sico_codegen_guest_pins_the_t1_family_truth_table` + `sico_codegen_guest_derives_the_module_wide_data_offsets`）。

| # | 形状 | IR | 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| A | let 绑定位置（形参 ×2） | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| B | let 绑定位置（cell ×2） | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| C | return 位置 + 体内另有 `let` | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| D | return 位置 + 体内**只有**一条 return | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| E | 被赋值的 `let`（cell 写） | 2630 | 1114 | 逐字节等同 | 逐字节等同 |
| F | 被赋值的 `let`（cell 写 + 读） | 2868 | 1138 | 逐字节等同 | 逐字节等同 |
| G | I64 cell | 2837 | 1116 | 逐字节等同 | 逐字节等同 |
| H | 两个字面量实参（体内只有 return） | 2372 | 1124 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| I | 形参 + 字面量（体内只有 return） | 2334 | 1113 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| J | 嵌套宿主调用（体内只有 return） | 2447 | 1112 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| K | 两个字面量实参（体内另有语句） | 2485 | 1137 | 逐字节等同 | 逐字节等同 |
| L | 形参 + `"\t"`（体内另有语句） | 2334 | 1113 | 逐字节等同 | **逐字节等同（STEP-0522 修正后）** |
| M | 嵌套 `concat`（体内另有语句） | 2447 | 1112 | 逐字节等同 | 逐字节等同 |
| N | 形参 + 普通字面量 `"x"`（体内另有语句） | 2333 | 1113 | 逐字节等同 | 逐字节等同 |
| O | `"a"` + `"\t"`（体内另有语句） | 2486 | 1137 | 逐字节等同 | **逐字节等同（STEP-0522 修正后）** |
| Q | 形参 + `"\n"`（体内另有语句） | 2334 | 1113 | 逐字节等同 | **逐字节等同（STEP-0522 修正后）** |
| R | 形参 + `"\t"`（体内只有 return） | 2334 | 1113 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| S | **两个函数各带一个字面量** | 3060 | 1188 | 逐字节等同 | 逐字节等同（STEP-0521 修正后） |
| T | **同一字面量出现两次** | 2673 | 1145 | 逐字节等同 | 逐字节等同（STEP-0521 修正后） |
| U | 字面量只出现在后一个函数 | 2721 | 1155 | 逐字节等同 | 逐字节等同 |

⇒ **“某形状被拒绝”≠“该形状没实现”**：H / I / J 之所以被拒，是**路由门**（函数体只有一条 `return`）而不是能力缺失 —— 加一条无关语句后，**同样两个操作数**（K / N / M）就逐字节降下来了。

⇒ **逐字节等同 15 行**、**组件层 typed 拒绝 0 行**、**前端 typed 拒绝 4 行**（H / I / J / R）、**修正前静默错字节 1 行（已修）**。

## 3. 未覆盖形状与缺口登记（D10 判定用）

| 缺口 | 症状 | 根因 | S 内是否出现 | 归属 |
|---|---|---|---|---|
| **G-b** 含 JSON 转义的字面量（`"\t"` / `"\n"`） | 组件层 typed 拒绝 | `cgs_literal_text` **故意**拒任何含 `\` 的字面量（guest 不反转义 JSON 字符串） | **是**（S 内 **952** 个操作数是带转义的字面量） | STEP-0522 **已闭合**（字面量身份改为**解码后字节**，解码器在发射端） |
| **G-e** 空字面量（`""`） | 组件层 typed 拒绝 | 同一个哨兵把「空字面量」和「不是字面量」混为一谈 | **是**（S 内 **5** 个操作数是空字面量，都在 `parser.sico`） | STEP-0522 **已闭合**（`cgs_is_const_string` 当闸门） |
| **G-d** 单语句 `return` 的非形参操作数 | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | 路由梯把 `straight_cell_*` 家族门控在 `body_statement_count > 0`；专用路由只认两个 `Text` 形参 | **是**（S 内 **10** 个调用点落在没有 `let` / `set` 的函数体里，且**没有一个**是形参 ×2） | ⏳ **待修**（前端） |
| **G-a** 模块级 data 段偏移 | **同长但字节不同**（数据指针差 1）⇒ **静默错字节**，不是 typed 拒绝 | `cgs_data_offset` 只扫**当前函数**的指令，而后端的静态 data 段收**整个模块**（函数 → 块 → 指令） | **是**（S 的多个文件都有多函数字面量） | STEP-0521 **已修正** |
| **G-c** 重复字面量 | 组件层 typed 拒绝 | `cgs_data_offset` 把重复字面量当作不可推导 | **是**（S 内 `"\t"` 等重复出现） | STEP-0521 **已修正** |

### 3.1 S 内 T1 调用点普查（口径：文本出现次数）

| 口径 | 数值 |
|---|---|
| S = `selfhost/*.sico` 9 个文件 | **2,943** 个 `sico.text.concat(` 文本调用点（STEP-0521 记 2,941；差的 2 个是 STEP-0522 在 `codegen.sico` 里新增的 concat） |
| 其中**操作数含字面量** | 1,576（⇒ **G-b** 是固定点的硬阻塞） |
| 落在**没有 `let` / `set`** 的函数体里 | **10**（⇒ **G-d** 是固定点的硬阻塞） |
| 操作数形态（**按操作数**计，共 5,888 个） | 名字 3,308 / 调用或成员 1,004 / **带转义的字面量 952** / 普通字面量 619 / **空字面量 5** |

> **口径说明**：STEP-0520 记的 **2,857** 是 **IR 级**（Rust 前端实际降出来的调用点）计数；本表的 **2,943** 是**文本**计数（含 `if` / `while` 体内的调用点）。两个数字口径不同，**不互相矛盾**。本表的「操作数形态」一栏按**操作数**计（STEP-0521 的同栏按**调用点**计），两表口径不同、**不互相矛盾**。

### 3.2 自举形状前沿的已知缺口

**已闭合**（STEP-0522）：`cgs_literal_text` 曾是 G-b 的闸门，且**在前沿之外**（guest 前端报 `ERR:E-SH-IR-CALL-ARITY`）。把字面量逻辑搬到**字节域**（hex 字节对）后，不再需要任何 text 形状的宿主调用 ⇒ 前沿问题自然消失。实测 8 个新增/改动函数（`cgs_is_const_string` / `cgs_literal_hex` / `cgj_string_hex` / `cgj_escape_hex` / `cgs_hex_bytes` / `cgs_data_offset` / `cgs_const_string_hex` / `cgs_data_hex`）**全部**能被 guest 前端逐字节降低（新钉 `sico_compiler_lowers_the_step0522_codegen_functions_byte_exactly`）。

> **附带事实**：`U64.checked_div` 这条路由**不在** guest 前端的路由梯里（实测 `ERR:E-SH-IR-CALL-TARGET`）⇒ 字节数按 2 步长计数得到，**不引入除法**。

## 4. 族级出口判据（五项缺一不算闭合）

| 判据 | 状态 |
|---|---|
| `canonical IR` 逐字节等同 | ✅ 已达成的 15 个形状（真值表钉住） |
| `整组件` 逐字节等同 | ✅ 同上（STEP-0522 后**组件层已无 typed 拒绝**） |
| `wasmparser::Validator` 通过 | ✅ oracle 与 guest 两侧都过（真值表逐行验证） |
| 前沿探针期望值已翻转 | ✅ STEP-0521 新增 3 条用例（28 → **31**），STEP-0522 再加 3 条（31 → **34**），全部判 `lowered` |
| 固件电池绿 | ✅ 本机电池全绿（**16 步**，BATTERY_GREEN 16/16） |
| **独立 CI 绿** | ✅ **run #28 = success**（批次提交 `a5e0b65` ⇒ `codex/m22-w1-ci`）；STEP-0522 的推送会触发下一轮，回填在收口片 X |

⇒ **T1 族仍未闭合**，而且**不得按 D10 降级**：**G-d** 在 S 内**真实存在**（§3），降级会让固定点（guest 能编译 S）无法达成。
