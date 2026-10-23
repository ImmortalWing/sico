# M22 支持面登记表 · T1 族（`sico.text.concat`）

> - status: **STEP-0527 定稿（T1 族收口片 X）**：**T1 族闭合** —— 计划 §3.3.5 的**族级出口判据六项全过**（`canonical IR` 逐字节等同 / 整组件逐字节等同 / `wasmparser::Validator` 通过 / 前沿探针期望值已翻转 / 固件电池绿 / 独立 CI 绿）；G-b / G-e / G-d1 / G-f / G-f2 / **G-d2 / G-d3** **全部闭合**；**语料覆盖仍 0 条**（本登记册上仍为 0 的那一项）⇒ **族闭合 ≠ M22 闭合**（见 §5）
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**：族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）
> - depends: [STEP-0517](../steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（真值测量）、[STEP-0518](../steps/STEP-0518-m22-w2-s5-w2-t1-frontend-knife.md)（前端刀）、[STEP-0519](../steps/STEP-0519-m22-w2-s5-w2-t1-codegen-knife.md)（codegen 刀）、[STEP-0520](../steps/STEP-0520-m22-w2-s5-w2-shared-local-cell-support.md)（cell 共享前置）、[STEP-0521](../steps/STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表）、[STEP-0522](../steps/STEP-0522-m22-w2-s5-w2-t1-escaped-literal-decoder.md)（G-b / G-e 两刀）、[STEP-0523](../steps/STEP-0523-m22-w2-s5-w2-t1-gd-single-return-route.md)（G-d1）、[STEP-0524](../steps/STEP-0524-m22-w2-s5-w2-t1-gf-multi-argument-call-operand.md)（G-f）、[STEP-0525](../steps/STEP-0525-m22-w2-s5-w2-t1-gf2-nested-call-argument.md)（G-f2）、[STEP-0526](../steps/STEP-0526-m22-w2-s5-gd2-emitter-branch-support.md)（G-d2 / G-d3）、[STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（收口片 X / D15 定稿）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`，Rust 后端 `compile_script_program`），**不是设计值**

## 1. 子集边界（D15 要求登记的六项）

| 项 | 值 | 证据 |
|---|---|---|
| 支持的名字 | **仅** `sico.text.concat` | STEP-0519：`cgs_intrinsic_hex` 的名字闸门；其它 helper 型内建仍 fail-closed |
| 实参个数 | **恰好 2** | STEP-0517：**元数边界在语义层**（1 个 / 3 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity`）⇒ guest **不自造**元数检查 |
| 实参类型 | **两个 `Text`**（canonical `string`） | STEP-0519：类型闸门 `cgs_value_ty` == `string`；**调用型操作数**的被调函数也必须返回 `Text` |
| 结果槽数 | **2**（pointer, length） | STEP-0519：结果槽闸门 `cgs_type_slots` == 2 |
| 语料覆盖 | **0 条** | S 内 2,943 个文本调用点**一个都没进语料** |
| 未覆盖形状 | **0 类缺口**（§3：G-d2 与 G-d3 已闭合；**族外**的缺口另计，见 §5） | STEP-0526 闭合，STEP-0527 收口复核 |

## 2. 族级真值表（38 行，实测）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。**每一行都由固件钉住**（`sico_codegen_guest_pins_the_t1_family_truth_table` **36 行** + `sico_codegen_guest_derives_the_module_wide_data_offsets` 2 行）。

| # | 形状 | IR | 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| A | let 绑定位置（形参 ×2） | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| B | let 绑定位置（cell ×2） | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| C | return 位置 + 体内另有 `let` | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| D | return 位置 + 体内**只有**一条 return | 2295 | 1094 | 逐字节等同 | 逐字节等同 |
| E | 被赋值的 `let`（cell 写） | 2630 | 1114 | 逐字节等同 | 逐字节等同 |
| F | 被赋值的 `let`（cell 写 + 读） | 2868 | 1138 | 逐字节等同 | 逐字节等同 |
| G | I64 cell | 2837 | 1116 | 逐字节等同 | 逐字节等同 |
| H | 两个字面量实参（体内只有 return） | 2372 | 1124 | **逐字节等同（STEP-0523 修正后）** | 逐字节等同 |
| I | 形参 + 字面量（体内只有 return） | 2334 | 1113 | **逐字节等同（STEP-0523 修正后）** | 逐字节等同 |
| J | 嵌套宿主调用（体内只有 return） | 2447 | 1112 | **逐字节等同（STEP-0523 修正后）** | 逐字节等同 |
| K | 两个字面量实参（体内另有语句） | 2485 | 1137 | 逐字节等同 | 逐字节等同 |
| L | 形参 + `"\t"`（体内另有语句） | 2334 | 1113 | 逐字节等同 | 逐字节等同 |
| M | 嵌套 `concat`（体内另有语句） | 2447 | 1112 | 逐字节等同 | 逐字节等同 |
| N | 形参 + 普通字面量 `"x"`（体内另有语句） | 2333 | 1113 | 逐字节等同 | 逐字节等同 |
| O | `"a"` + `"\t"`（体内另有语句） | 2486 | 1137 | 逐字节等同 | 逐字节等同 |
| Q | 形参 + `"\n"`（体内另有语句） | 2334 | 1113 | 逐字节等同 | 逐字节等同 |
| R | 形参 + `"\t"`（体内只有 return） | 2334 | 1113 | **逐字节等同（STEP-0523 修正后）** | 逐字节等同 |
| S | **两个函数各带一个字面量** | 3060 | 1188 | 逐字节等同 | 逐字节等同（STEP-0521 修正后） |
| T | **同一字面量出现两次** | 2673 | 1145 | 逐字节等同 | 逐字节等同（STEP-0521 修正后） |
| U | 字面量只出现在后一个函数 | 2721 | 1155 | 逐字节等同 | 逐字节等同 |
| **V** | **调用另一个函数作为操作数**（第二个位置） | 2887 | 1177 | **逐字节等同（STEP-0523 新增）** | 逐字节等同 |
| **W** | **调用另一个函数作为操作数**（第一个位置） | 2887 | 1177 | **逐字节等同（STEP-0523 新增）** | 逐字节等同 |
| **X** | **嵌套 concat 里的调用操作数** | 3152 | 1208 | **逐字节等同（STEP-0523 新增）** | 逐字节等同 |
| **Y** | **调用链**（被调函数本身也是 concat） | 3594 | 1260 | **逐字节等同（STEP-0523 新增）** | 逐字节等同 |
| **Z** | **调用另一个函数（两个实参）**作为操作数 | 3036 | 1185 | **逐字节等同（STEP-0524 修正后）** | 逐字节等同 |
| **AA** | **concat 在 `if` 块里** | 3119 | 1198 | 逐字节等同 | **逐字节等同（STEP-0526 修正后）** |
| **BB** | **调用另一个函数（两个实参，第二个是 `U64.literal(n)`）** —— **S 的真实形状** | 3513 | 1229 | **逐字节等同（STEP-0524 新增）** | 逐字节等同 |
| **BC** | **调用另一个函数（三个实参）** | 3189 | 1193 | **逐字节等同（STEP-0524 新增）** | 逐字节等同 |
| **BD** | **调用另一个函数（两个字面量实参）** | 3120 | 1211 | **逐字节等同（STEP-0524 新增）** | 逐字节等同 |
| **BE** | **调用另一个函数（实参是 `I64.literal(-n)`）** | 3070 | 1190 | **typed 拒绝** `ERR:E-SH-IR-EXPRESSION` | typed 拒绝 |
| **BF** | **调用另一个函数（实参本身是嵌套调用）** | 3635 | 1267 | **逐字节等同（STEP-0525 修正后）** | 逐字节等同 |
| **BG** | **调用另一个函数（内层调用没有实参）** | 3559 | 1261 | **逐字节等同（STEP-0525 新增）** | 逐字节等同 |
| **BH** | **调用另一个函数（内层调用有三个实参）** | 4005 | 1296 | **逐字节等同（STEP-0525 新增）** | 逐字节等同 |
| **BI** | **调用另一个函数（内层调用返回 `U64`）** | 3618 | 1250 | **逐字节等同（STEP-0525 新增）** | 逐字节等同 |
| **BJ** | **调用另一个函数（两层嵌套）** | 3767 | 1281 | **逐字节等同（STEP-0525 新增）** | 逐字节等同 |
| **BK** | **`if` / `else`（两个分支 + 一条 `unreachable`）** | 2853 | 1100 | **逐字节等同（STEP-0526 新增）** | 逐字节等同 |
| **BL** | **`while` 循环（回边是 `jump`）** | 3293 | 1116 | **逐字节等同（STEP-0526 新增）** | 逐字节等同 |
| **BN** | **一个函数体里两个 `if`（6 块、2 分支、2 跳转）** | 3530 | 1178 | **逐字节等同（STEP-0526 新增）** | 逐字节等同 |

⇒ **“某形状被拒绝”≠“该形状没实现”**：H / I / J / R 之所以曾被拒，是**路由门**（函数体只有一条 `return`）而不是能力缺失 —— STEP-0523 把路由改成**复用 `let` 绑定路由一直在用的操作数降低器**之后，这四行与它们的「体内另有语句」版本（K / N / M）**走同一份规则**，于是逐字节降下来了。

⇒ **逐字节等同 35 行**（含 AA —— STEP-0526 起）、**前端 typed 拒绝 1 行**（BE）、**组件层 typed 拒绝 0 行**；加上数据偏移测试的 2 行 ⇒ **族级真值表 38 行**。

> **AA 是 STEP-0526 的收口行**：它曾是族级真值表里**唯一**一条组件层拒绝（前端逐字节降下来、组件发射拒绝）。STEP-0526 给发射端补上了分支 / 跳转 / `unreachable` 三种终结符与 `equal_fixed` 算子 ⇒ AA 与它的三个邻接形状（BK / BL / BN）现在**整组件与 Rust 后端逐字节相同**。

> **BF 是 STEP-0524 的普查产物**：BE 钉的是「实参是 `I64.literal(-n)`」（S 内**不存在**该形状），BF 钉的是「实参**本身是嵌套调用**」（S 内**存在 22 个调用点**）—— 两者都是**真缺口**（Rust 两个 oracle 都能降，guest 不能），不是一个意思。
>
> **BF 已由 STEP-0525 闭合**：BG / BH / BI / BJ 是它的四个邻接形状（内层 0 个实参 / 3 个实参 / 内层返回 `U64` / 两层嵌套），**四行都是逐字节等同** ⇒ G-f2 不再是缺口。

## 3. 未覆盖形状与缺口登记（D10 判定用）

| 缺口 | 症状 | 根因 | S 内是否出现 | 归属 |
|---|---|---|---|---|
| **G-b** 含 JSON 转义的字面量（`"\t"` / `"\n"`） | 组件层 typed 拒绝 | `cgs_literal_text` **故意**拒任何含 `\` 的字面量（guest 不反转义 JSON 字符串） | **是**（S 内 **952** 个操作数是带转义的字面量） | STEP-0522 **已闭合**（字面量身份改为**解码后字节**，解码器在发射端） |
| **G-e** 空字面量（`""`） | 组件层 typed 拒绝 | 同一个哨兵把「空字面量」和「不是字面量」混为一谈 | **是**（S 内 **5** 个操作数是空字面量，都在 `parser.sico`） | STEP-0522 **已闭合**（`cgs_is_const_string` 当闸门） |
| **G-d1** 单语句 `return` 的非形参操作数 | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | 路由梯把 `straight_cell_*` 家族门控在 `body_statement_count > 0`；专用路由只认两个 `Text` 形参 | **是**（S 内 **10** 个调用点，其中 **6** 个是这种形状） | STEP-0523 **已闭合**（路由复用 `straight_text_concat_packed`） |
| **G-d2** `if` 块里的 concat | ~~**组件层** typed 拒绝（前端已逐字节降下来）~~ | ~~**guest 的发射器只支持单块直线代码**（`selfhost/codegen.sico` 只认 `return` 终结符）⇒ **任何** `if` 都撞上它~~ | **是**（S 内 **4** 个调用点：2 个在 `cgs_zero_hex`、2 个在 `declaration_name`） | ✅ **已闭合**（STEP-0526：发射端逐块发终结符 —— `return` / `branch` / `jump` / `unreachable` —— 与后端 `compile_function` 同构；**S 内两个真实形状逐字节等同**（1266 B / 1316 B）） |
| **G-d3** 发射端缺 `equal_fixed` 算子 | **组件层** typed 拒绝 | guest 的 `cgs_op_hex` 不认识定宽比较 `equal_fixed` ⇒ 连**分支的条件**都发不出来（AA 行的条件正是它） | **是**（AA 行的 `U64.equal(...)`） | ✅ **已闭合**（STEP-0526：新增 `cgs_equal_fixed_hex`，与后端 `emit_fixed_comparison` 同字节） |
| **G-f** 调用型操作数带**多个实参** | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | 借用来的降低器 `straight_text_concat_packed` 的调用分支只支持**一个**实参 | **是**（S 内 **117** 个调用点 —— 口径见 §3.1；**订正**：STEP-0523 记的「1 个」是 **G-d 子集**口径） | ✅ **已闭合**（STEP-0524：**新增 `stc_call_operand_packed`**，实参可为**形参 / 局部 / 字符串字面量 / `U64.literal(n)`**；**单实参路径原样保留** ⇒ `let` 路由的 6 行真值表逐字节不变。S 内 117 个调用点里 **95 个**的实参全部落在认领集合内） |
| **G-f2** 调用型操作数的**实参本身是嵌套调用**（`f(g(x), y)`） | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | `stc_call_operand_packed` 的实参分类只认**形参 / 局部 / 字符串字面量 / `U64.literal(n)`**；实参是调用 ⇒ `ERR:E-SH-IR-CALL-ARGUMENT` ⇒ 路由转 `SKIP` | **是**（S 内 **22** 个调用点：`codegen.sico` 10 + `parser.sico` 12） | ✅ **已闭合**（STEP-0525：嵌套调用的逻辑**内联进** `stc_call_operand_packed`（**不新增函数**）—— 见 STEP-0525 §3.1：单独写一个 helper 时 **guest 前端降不下它**（通用调用路由的内建白名单里没有 `sico.text.concat`，且函数体尾部是 `if/else` ⇒ 合并块不终结）⇒ 改为内联进**已被证明可降**的共用降低器，它**递归调用自己**；真值表 **BF** 行翻成逐字节等同，另加 **BG / BH / BI / BJ** 四行邻接形状，全部逐字节等同） |
| **G-a** 模块级 data 段偏移 | **同长但字节不同**（数据指针差 1）⇒ **静默错字节**，不是 typed 拒绝 | `cgs_data_offset` 只扫**当前函数**的指令，而后端的静态 data 段收**整个模块**（函数 → 块 → 指令） | **是**（S 的多个文件都有多函数字面量） | STEP-0521 **已修正** |
| **G-c** 重复字面量 | 组件层 typed 拒绝 | `cgs_data_offset` 把重复字面量当作不可推导 | **是**（S 内 `"\t"` 等重复出现） | STEP-0521 **已修正** |

> **G-d2 是本表最重要的一次订正**：STEP-0521 / STEP-0522 把 G-d 登记成「前端缺口」。STEP-0523 逐个探过剩下那 4 个调用点之后发现，它们的拒绝**不在前端** —— 前端**已经**把它们逐字节降下来了，拒的是**组件发射**。原因与 T1 无关：guest 的发射器只支持**单块直线代码**，所以**任何** `if` 块都撞上它（实测：`if` 块里只返回一个字面量、连 concat 都没有，照样被拒）。把它继续挂在 T1 族名下会让族级出口判据失真 ⇒ 拆成 **G-d1（前端，T1 专属，✅ 已闭合）** 与 **G-d2（发射端分支缺口，✅ 已闭合 —— STEP-0526）**。

### 3.1 S 内 T1 调用点普查（口径：文本出现次数）

| 口径 | 数值 |
|---|---|
| S = `selfhost/*.sico` 9 个文件 | **2,943** 个 `sico.text.concat(` 文本调用点（STEP-0521 记 2,941；差的 2 个是 STEP-0522 在 `codegen.sico` 里新增的 concat） |
| 其中**操作数含字面量** | 1,576（⇒ **G-b** 是固定点的硬阻塞） |
| 落在**没有 `let` / `set`** 的函数体里 | **10**（⇒ **G-d** 是固定点的硬阻塞） |
| 操作数形态（**按操作数**计，共 5,888 个） | 名字 3,308 / 调用或成员 1,004 / **带转义的字面量 952** / 普通字面量 619 / **空字面量 5** |
| **G-f 面**（操作数是**用户函数调用**、被调函数返回 `Text`、实参 ≥ 2 个） | **117** 个调用点（其中 **95** 个实参全部落在认领集合内 / **22** 个带嵌套调用实参 ⇒ **G-f2**，STEP-0525 已闭合）；**G-f + G-f2 之后，G-f 面在 S 内不再有未认领的形状**；另有 **787** 个操作数是**宿主调用**（`sico.*`，属另一个闸门，不在 G-f 面内） |

> **口径说明**：STEP-0520 记的 **2,857** 是 **IR 级**（Rust 前端实际降出来的调用点）计数；本表的 **2,943** 是**文本**计数（含 `if` / `while` 体内的调用点）。两个数字口径不同，**不互相矛盾**。本表的「操作数形态」一栏按**操作数**计（STEP-0521 的同栏按**调用点**计），两表口径不同、**不互相矛盾**。
>
> **G-f 面的口径（STEP-0524 普查，脚本 `target/tmp/step0524/census_gf.py`）**：判据与降低器里的分派**一一对应** —— 嵌套 `sico.text.concat` 走内建分支、`sico.u64.to_text` 走自己的分支、其它 `sico.*` 宿主调用被 `ERR:E-SH-IR-CALL-ARGUMENT` 拒（**都不是** G-f）；只有**用户定义函数**的调用会走到那道**元数闸门**。元数分布：2 个实参 **90** / 3 个 **2** / 4 个 **3** / 5 个 **16** / 6 个 **6**；路由分布：`let`/`set` 路由 **115** / 单语句 `return` 路由 **2**（`compiler_parser.sico:172`，即 S 的真实形状）；按文件：`parser.sico` **80** / `codegen.sico` **34** / `compiler_parser.sico` **2** / `formatter.sico` **1**。

### 3.2 那 10 个调用点的逐个定位（STEP-0523 实测）

| # | 文件:行 | 函数 | 操作数形态 | 状态 |
|---|---|---|---|---|
| 1–2 | `codegen.sico:147` | `cg_post_body_hex` | 字面量 + 嵌套 concat（内含调用） | ✅ 逐字节等同 |
| 3 | `codegen.sico:1630` | `cgs_local_get_hex` | 字面量 + 调用 | ✅ 逐字节等同 |
| 4 | `codegen.sico:1635` | `cgs_local_set_hex` | 字面量 + 调用 | ✅ 逐字节等同 |
| 5 | `codegen.sico:1640` | `cgs_i32_const_hex` | 字面量 + 调用 | ✅ 逐字节等同 |
| 6 | `codegen.sico:1645` | `cgs_i64_const_hex` | 字面量 + 调用 | ✅ 逐字节等同 |
| 7–8 | `codegen.sico:2069/2072` | `cgs_zero_hex` | 字面量 + 调用，**在 `if` 块里** | ✅ **逐字节等同（STEP-0526）** |
| 9–10 | `compiler_parser.sico:172` | `declaration_name` | **两实参调用** + 嵌套 concat，**在 `if` 块里** | ✅ **逐字节等同（STEP-0526）**（**G-f 已由 STEP-0524 闭合**） |

⇒ **10 / 10 闭合**；原先剩下的 4 个**全部**落在 `if` 块里，STEP-0526 把发射端的分支支持补上后**两个真实形状逐字节等同**（1266 B / 1316 B）。

### 3.3 自举形状前沿的已知缺口

**已闭合**（STEP-0522）：`cgs_literal_text` 曾是 G-b 的闸门，且**在前沿之外**（guest 前端报 `ERR:E-SH-IR-CALL-ARITY`）。把字面量逻辑搬到**字节域**（hex 字节对）后，不再需要任何 text 形状的宿主调用 ⇒ 前沿问题自然消失。实测 8 个新增/改动函数（`cgs_is_const_string` / `cgs_literal_hex` / `cgj_string_hex` / `cgj_escape_hex` / `cgs_hex_bytes` / `cgs_data_offset` / `cgs_const_string_hex` / `cgs_data_hex`）**全部**能被 guest 前端逐字节降低（新钉 `sico_compiler_lowers_the_step0522_codegen_functions_byte_exactly`）。

**STEP-0523 未推进前沿**：重塑后的 `direct_text_concat_return_ir` 与它现在调用的 `straight_text_concat_packed` 改前改后**都**在前沿内（新钉 `sico_compiler_lowers_the_step0523_parser_functions_byte_exactly`）。新形状是**复用**既有形状得到的，不是靠改形状硬挤过去的。

**STEP-0524 也未推进前沿**：新增的 `stc_call_operand_packed` 与它所属的 `straight_text_concat_packed` 改前改后**都**在前沿内（新钉 `sico_compiler_lowers_the_step0524_parser_functions_byte_exactly`）。**顺带修掉一条既有隐患**：`sico_compiler_lowers_straight_text_concat_packed_closure_byte_exactly` 那份**手工维护**的闭包清单在本次改动后失效（Rust oracle 报 `unresolved call target stc_call_operand_packed`）⇒ 改成**从文件自动计算**（`parser_closure_source`）。

**STEP-0525 仍未推进前沿**：改动后的 `stc_call_operand_packed` 与它所属的 `straight_text_concat_packed` 改前改后**都**在前沿内（新钉 `sico_compiler_lowers_the_step0525_parser_functions_byte_exactly`）。**本片撞到一条形状边界并绕开了它**：单独写一个 helper 承载嵌套调用时，**guest 前端降不下它**（通用调用路由的内建白名单里没有 `sico.text.concat` ⇒ 「内建调用的实参本身是调用」撞 `ERR:E-SH-IR-CALL-TARGET`；函数体尾部是 `if/else` ⇒ 通用 CFG 路由判 `ERR:E-SH-IR-RETURN-COUNT`）⇒ 逻辑**内联进既有可降形状**。**这条边界没有登记成族级缺口**（它限制的是「新增函数的形状」，不是某个族的操作数能力），但它**约束后续切片**：新形状要落在既有可降形状的延长线上。

> **附带事实**：`U64.checked_div` 这条路由**不在** guest 前端的路由梯里（实测 `ERR:E-SH-IR-CALL-TARGET`）⇒ 字节数按 2 步长计数得到，**不引入除法**。

> **另一条附带事实**（STEP-0523 测得）：STEP-0518 的固件里写着「guest 前端拒前向引用的用户函数调用」——**这条现在不成立**（实测：前向调用与后向调用都逐字节降下来）。该注释已在本片订正，并顺带把那份**手工维护**的闭包清单改成**从文件自动计算**（`parser_closure_source`），免得路由再变时清单又漂移。

## 4. 族级出口判据（六项缺一不算闭合）—— STEP-0527 收口核对

| 判据 | 状态 | 证据 |
|---|---|---|
| `canonical IR` 逐字节等同 | ✅ 已达成的 **35** 个形状（真值表钉住） | `sico_codegen_guest_pins_the_t1_family_truth_table` **36 行** + `sico_codegen_guest_derives_the_module_wide_data_offsets` **2 行**（BE 行是两侧**一致**的前端 typed 拒绝，不是差异行） |
| `整组件` 逐字节等同 | ✅ 同上（**组件层 typed 拒绝 0 行** —— AA 一行已由 STEP-0526 翻转） | STEP-0526：S 内两个真实形状的 Rust 生产组件 **1266 B / 1316 B** 与 guest **整组件逐字节相同** |
| `wasmparser::Validator` 通过 | ✅ oracle 与 guest 两侧都过（真值表逐行验证） | 真值表逐行验证 |
| 前沿探针期望值已翻转 | ✅ STEP-0521 新增 3 条用例（28 → **31**），STEP-0522 再加 3 条（31 → **34**），STEP-0523 再加 4 条并翻 2 条（34 → **38**），STEP-0524 再加 1 条并翻 1 条（38 → **39**），STEP-0525 再加 2 条（39 → **41**），**全部与期望一致**（**30** 条期望 `lowered` —— 含 T1 族 18 条形状；**11** 条期望 typed 拒绝 `ERR:E-SH-IR-*`，**故意钉住的拒绝身份**，不是未翻转） | `tools/probe-m22-frontend-frontier.ps1`（电池步骤 `probe-frontend-frontier`） |
| 固件电池绿 | ✅ 本机电池全绿（**18 步**，BATTERY_GREEN 18/18 —— STEP-0526 的 17 步原样复用 + 本片新增 1 步文档自洽检查）+ 固件 **153 / 153** | `target/tmp/step0527/battery.ps1`（源码树与 STEP-0526 逐字节相同 ⇒ 该轮电池对本片同样成立） |
| **独立 CI 绿** | ✅ run #31（STEP-0524 `a383dfe`）= **success**；run #32（STEP-0525 `1956158`）= **success**（run id `39969358043`）；run #33（STEP-0526 `b4364ef`）= **success**（run id `39990810669`） | GitHub Actions（`codex/m22-w1-ci`）；按仓库既有做法，**每轮裁决由下一片的提交回填** |

⇒ **六项全过 ⇒ T1 族的支持面闭合**（登记口径：**族闭合**，不是 M22 闭合 —— 见 §5）。

## 5. 诚实收窄：T1 族闭合 **≠** M22 闭合（STEP-0527）

| 项 | 状态 |
|---|---|
| **T1 族（`sico.text.concat`）** | ✅ **闭合**（六项族级出口判据全过） |
| **M22 门 1 / 4 / 5 / 7** | ✅ GO（不变） |
| **M22 门 2** | PARTIAL（不变） |
| **M22 门 3 / 6** | ❌ NO-GO（不变） |
| **M22 整体** | ❌ **仍为 NO-GO** |
| **整文件自举** | ⏔ **仍未闭合**：`List[Text]` 形参 ⇒ `ERR:E-SH-IR-RECORD-TYPE`；宿主调用内建只认两个（`sico.text.concat` / `sico.bytes.length`）；嵌套 `if` 的**前端**缺口 `ERR:E-SH-IR-STATEMENT`；`match` 终结符未实现（发射端只发 `return` / `branch` / `jump` / `unreachable` 四种；guest 前端对该形状 typed 拒绝 `ERR:E-SH-IR-MATCH-SHAPE` ⇒ 当前不可达，见 STEP-0526）；`run` 多块仍 fail closed（结果区返回是单块形状） |
| **差分语料覆盖** | ⏔ **仍 0 条** —— T1 族的形状**一个都没进** `selfhost/corpus-v0.json` / `corpus-w1` ⇒ **族的行为只由族级真值表钉住，不由差分语料钉住**（这是本登记册上**仍然为 0 的那一项**） |
| **下一个族** | T2 族（`sico.text.length`，S 内 318 处） |

⇒ **「族闭合」与「固定点达成」是两个判断**：前者问「这个族声明的支持面，guest 是不是逐字节做到了」；后者问「guest 能不能编译整个冻结源集 S」。STEP-0517…0526 走完的正是这条区分：**每个族都用自己的真值表判定，而 S 的固定点由所有族合起来判定** —— T1 闭合之后，S 的固定点仍差着**别的族**（宿主调用内建、`List[Text]` 形参、嵌套 `if` 前端形状、record 泛型、cell 的记录类型 …）。
