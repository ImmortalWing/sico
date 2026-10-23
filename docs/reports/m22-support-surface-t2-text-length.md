# M22 支持面登记表 · T2 族（`sico.text.length`）

> - status: **STEP-0528 建立（拍 1 真值已测；族未闭合）** —— 条件位（`if` / `while` / 分支块内）**已能逐字节降**，语句位（`let` / `set` / 单 `return`）仍 typed 拒绝，**组件层未闭合**；语料覆盖 **0 条**
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**：族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）
> - depends: [STEP-0517](../steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）、[STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（上一族 T1 收口）、[STEP-0528](../steps/STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（**本族的拍 1 真值测量片**）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`，Rust 后端 `compile_script_program`），**不是设计值**

## 1. 子集边界（D15 要求登记的六项）

| 项 | 值 | 证据 |
|---|---|---|
| 支持的名字 | **仅** `sico.text.length` | STEP-0528：guest 发射器 `cgs_intrinsic_hex` 只认 `sico.bytes.length` 与 `sico.text.concat` ⇒ 其余 `ERR:E-SH-CG-IR-EMIT` |
| 实参个数 | **恰好 1** | STEP-0528：**元数边界在语义层**（0 个 / 2 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity`，区间 358..376 / 365..387）⇒ guest **不自造**元数检查 |
| 实参类型 | **一个 `Text`**（canonical `string`；**2 槽**：pointer + length） | STEP-0528：后端 `emit_intrinsic` 取 `[pointer, length]` 对（`crates/sico-codegen-wasm/src/lib.rs:4466`） |
| 结果槽数 | **1**（`U64`） | STEP-0528：后端把 helper 的 i32 结果 `I64ExtendI32U`（`0xad`）后落槽 |
| 语料覆盖 | **0 条** | S 内 **322** 个 `sico.text.length(` 文本调用点**一个都没进语料** |
| 未覆盖形状 | **3 类**：①**语句位**（`let` / `set` / 单 `return`）7 个变体；②**嵌套调用实参** 2 个变体（S 内 2 个调用点）；③**组件层全部** | STEP-0528 实测 |

## 2. 族级真值表（12 行，实测）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。**每一行都由固件钉住**（STEP-0528 的 4 条新钉）。

| # | 形状（位置 · 实参） | IR | 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| a | return（体内只有 return） · 形参 | 2212 | 1091 | ❌ `ERR:E-SH-IR-EXPRESSION` | — |
| b | let 绑定 · 形参 | 2212 | 1091 | ❌ 同上 | — |
| c | return（体内只有 return） · 字面量 | 2252 | 1111 | ❌ 同上 | — |
| d | let 绑定 · 局部（cell） | 2212 | 1091 | ❌ 同上 | — |
| e | let 绑定 · 嵌套用户调用 | 2656 | 1145 | ❌ 同上 | — |
| f | let 绑定 · 嵌套宿主调用 | 2369 | 1570 | ❌ 同上 | — |
| g | `set` · 形参 | 2765 | 1113 | ❌ 同上 | — |
| **h** | **`if` 条件 · 形参** | **2991** | **1167** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| **i** | **`while` 条件 · 形参** | **3446** | **1187** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| **j** | **分支块内的语句 · 形参** | **3437** | **1187** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| k | 元数探针 · 0 个实参 | — | — | ❌ **Rust 前端自己拒** `stdlib intrinsic arity` | — |
| l | 元数探针 · 2 个实参 | — | — | ❌ **Rust 前端自己拒** `stdlib intrinsic arity` | — |

⇒ **逐字节等同 3 行**（条件位）、**前端 typed 拒绝 7 行**（语句位）、**语义层拒绝 2 行**（元数）；**组件层 typed 拒绝 = 全部可降形状**（发射器不认识这个算子）。

## 3. 缺口登记（D10 判定用）

| 缺口 | 症状 | 根因 | S 内是否出现 | 归属 |
|---|---|---|---|---|
| **T2-g1** 语句位的 `sico.text.length` | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | **路由分家**：通用宿主调用路由 `gw_intrinsic_call_packed`（`selfhost/parser.sico:7001`）认 `sico.text.length`，但**直线路由家族** `straight_cell_function_ir`（`:9707` / `:10289`）只认 `sico.text.concat` | **是**（S 内 **320** / 322 个调用点落在「局部 / 形参」实参上） | ⏳ **未闭合**（STEP-0528 登记）⇒ 前端刀 |
| **T2-g2** 实参本身是**嵌套调用** | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | 同上（`gw_intrinsic_call_packed` 只认**形参 / 局部 cell** 两种实参） | **是**（**2** 个调用点：`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))`、`compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`） | ⏳ **未闭合**（STEP-0528 登记）⇒ T2 版的 G-f2 |
| **T2-c1** 发射端不认识 `sico.text.length` | **组件层** typed 拒绝 `unsupported codegen source shape` | guest 发射器 `cgs_intrinsic_hex` 只认两个算子 ⇒ **任何**用到该算子的模块都撞上它 | **是**（全部 320 个可降形状） | ⏳ **未闭合**（STEP-0528 登记）⇒ codegen 刀 |

## 4. S 内 T2 调用点普查（口径：文本出现次数）

| 口径 | 数值 |
|---|---|
| `sico.text.length(` 调用点 | **322**（计划 §3.3.5 的族表记 **318** ⇒ **订正**：S 在计划成文后的增长） |
| 按文件 | `parser.sico` **223** / `codegen.sico` **59** / `compiler.sico` **24** / `compiler_semantics.sico` **10** / `compiler_parser.sico` **3** / `checker.sico` **2** / `formatter.sico` **1** |
| **实参形态** | **局部（cell）306** / **形参 14** / **调用 2** / 字面量 **0** |
| **位置** | `set` **131** / `if` 条件 **119** / `let` 绑定 **69** / `while` 条件 **2** / `return` **1** |

## 5. 组件层真值（core module code 段反汇编）

- **调用点**：`local.get pointer; local.get length; call <helper>; i64.extend_i32_u (0xad); local.set result`；
- **helper**：core function 索引 = `helper_count + 3 + 名字排序位`（`helper_count = import_count + module.functions.len()`；运行时 helper 占 0/1/2；**内建 helper 按名字排序** ⇒ `sico.text.concat` 恒为 0、`sico.text.length` 为 1）；body **51 B**（locals 组 `2 × i32`），**数的是 UTF-8 码点**（非续字节）；
- ⇒ **codegen 刀必须同时处理两个内建 helper 的索引**（今天 guest 只处理 `concat` 一个）。

## 6. 族级出口判据（六项缺一不算闭合）

| 判据 | 状态 |
|---|---|
| `canonical IR` 逐字节等同 | ⚠️ **3** 个形状（条件位）—— 语句位 0 个 |
| `整组件` 逐字节等同 | ❌ **0** 行（发射器不认识该算子） |
| `wasmparser::Validator` 通过 | ✅ oracle 侧通过（固件钉住 1167 B 的组件） |
| 前沿探针期望值已翻转 | ⏳ STEP-0528 新增 5 条 T2 用例（41 → **46**）：3 条 `lowered` + 2 条 `refused:ERR:E-SH-IR-EXPRESSION` |
| 固件电池绿 | ✅ 本机 **BATTERY_GREEN 19/19**（固件 **157 / 157**） |
| **独立 CI 绿** | ⏳ **上一批** run #34（STEP-0527 `914197f`）= ✅ **success**（run id `39997745000`）（本片回填）；**本片**推送后触发 run #35，裁决由**下一片**回填 |

⇒ **T2 族未闭合**：三项缺口（T2-g1 语句位 / T2-g2 嵌套调用实参 / T2-c1 发射端）全部待落地。
