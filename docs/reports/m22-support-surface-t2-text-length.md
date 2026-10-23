# M22 支持面登记表 · T2 族（`sico.text.length`）

> - status: **STEP-0529 落地（前端刀 F 已完成；族未闭合）** —— **语句位**（`let` / `set` / 单 `return`）**已能逐字节降**（STEP-0529），**嵌套调用实参**仍 typed 拒绝，**组件层未闭合**；语料覆盖 **0 条**
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**：族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）
> - depends: [STEP-0517](../steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）、[STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（上一族 T1 收口）、[STEP-0528](../steps/STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（本族的拍 1 真值测量片）、[STEP-0529](../steps/STEP-0529-m22-w2-s5-w2-t2-frontend-knife.md)（**本族的拍 2 前端刀**）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`，Rust 后端 `compile_script_program`），**不是设计值**

## 1. 子集边界（D15 要求登记的六项）

| 项 | 值 | 证据 |
|---|---|---|
| 支持的名字 | **仅** `sico.text.length` | STEP-0528：guest 发射器 `cgs_intrinsic_hex` 只认 `sico.bytes.length` 与 `sico.text.concat` ⇒ 其余 `ERR:E-SH-CG-IR-EMIT` |
| 实参个数 | **恰好 1** | STEP-0528：**元数边界在语义层**（0 个 / 2 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity`，区间 358..376 / 365..387）⇒ guest **不自造**元数检查 |
| 实参类型 | **一个 `Text`**（canonical `string`；**2 槽**：pointer + length） | STEP-0528：后端 `emit_intrinsic` 取 `[pointer, length]` 对（`crates/sico-codegen-wasm/src/lib.rs:4466`）；**STEP-0529**：guest 前端会**查实参类型**，类型不符 ⇒ `ERR:E-SH-IR-CALL-TYPE`（oracle 侧同一形状在**语义层**判 `E2001` / `TYPE_MISMATCH` / "expected Text, found Bytes"） |
| 结果槽数 | **1**（`U64`） | STEP-0528：后端把 helper 的 i32 结果 `I64ExtendI32U`（`0xad`）后落槽 |
| 语料覆盖 | **0 条** | S 内 **322** 个 `sico.text.length(` 文本调用点**一个都没进语料** |
| 未覆盖形状 | **2 类**：①**嵌套调用实参** 2 个变体（S 内 2 个调用点）；②**组件层全部**（**语句位已闭合**，STEP-0529） | STEP-0529 实测 |

## 2. 族级真值表（12 行 + 1 行类型探针，实测）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。**每一行都由固件钉住**（STEP-0528 的 4 条钉 + STEP-0529 的 3 条新钉 / 1 条改钉）。

| # | 形状（位置 · 实参） | IR | 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| a | return（体内只有 return） · 形参 | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529 翻转） | — |
| b | let 绑定 · 形参 | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529 翻转） | — |
| c | return（体内只有 return） · 字面量 | 2252 | 1111 | ✅ **逐字节等同**（STEP-0529 翻转） | — |
| d | let 绑定 · 局部（cell） | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529 翻转） | — |
| e | let 绑定 · 嵌套用户调用 | 2656 | 1145 | ❌ `ERR:E-SH-IR-EXPRESSION`（**身份不变**） | — |
| f | let 绑定 · 嵌套宿主调用 | 2369 | 1570 | ❌ `ERR:E-SH-IR-EXPRESSION`（**身份不变**） | — |
| g | `set` · 形参 | 2765 | 1113 | ✅ **逐字节等同**（STEP-0529 翻转） | — |
| **h** | **`if` 条件 · 形参** | **2991** | **1167** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| **i** | **`while` 条件 · 形参** | **3446** | **1187** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| **j** | **分支块内的语句 · 形参** | **3437** | **1187** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| k | 元数探针 · 0 个实参 | — | — | ❌ **Rust 前端自己拒** `stdlib intrinsic arity` | — |
| l | 元数探针 · 2 个实参 | — | — | ❌ **Rust 前端自己拒** `stdlib intrinsic arity` | — |
| **m** | **类型探针 · `Bytes` 实参** | — | — | ❌ **guest 前端拒** `ERR:E-SH-IR-CALL-TYPE`（oracle：语义层 `E2001` / `TYPE_MISMATCH`） | — |

⇒ **逐字节等同 3 → 8 行**（条件位 3 + 语句位 5）；**前端 typed 拒绝 7 → 2 行**（且身份不变）；**语义层拒绝 2 行**（元数）；**类型探针 1 行**（§1「实参类型」）；**组件层 typed 拒绝 = 全部可降形状**（发射器仍不认识这个算子）。

## 3. 缺口登记（D10 判定用）

| 缺口 | 症状 | 根因 | S 内是否出现 | 归属 |
|---|---|---|---|---|
| ~~**T2-g1** 语句位的 `sico.text.length`~~ | ~~前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`~~ | **路由分家**：通用宿主调用路由 `gw_intrinsic_call_packed`（`selfhost/parser.sico:7001`）认 `sico.text.length`，但**直线路由家族** `straight_cell_function_ir` / `straight_environment_function_ir` 只认 `sico.text.concat` | **是**（S 内 **320** / 322 个调用点落在「局部 / 形参」实参上） | ✅ **T2-g1 闭合**（**STEP-0529**）：5 处接线（cell 路由的语句位 / return 位、env 路由的 let 位 / return 位、`scalar_ir` 路由梯的整体表达式位）+ 2 个新函数，**零新造机器** |
| **T2-g2** 实参本身是**嵌套调用** | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | `gw_intrinsic_call_packed` 与新路由都只认**形参 / 局部 cell / 字面量**三种实参 | **是**（**2** 个调用点：`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))`、`compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`） | ⏳ **未闭合**（STEP-0528 登记，STEP-0529 未动）⇒ T2 版的 G-f2 |
| **T2-c1** 发射端不认识 `sico.text.length` | **组件层** typed 拒绝 `unsupported codegen source shape` | guest 发射器 `cgs_intrinsic_hex` 只认两个算子 ⇒ **任何**用到该算子的模块都撞上它 | **是**（全部 320 个可降形状） | ⏳ **未闭合**（STEP-0528 登记）⇒ codegen 刀 |

### 3.1 D10 复核（**不得降级**）

STEP-0529 闭合的是 **T2-g1**（**一个缺口**）。S 内 T2 族的 **322** 个调用点里：

- **320** 个（局部 306 + 形参 14）落在 **T2-g1** ⇒ **已闭合**；
- **2** 个是**嵌套调用实参** ⇒ 仍属 **T2-g2**，**未覆盖**。

⇒ **T2-g2 未闭合 ⇒ 本族不得降级、不得宣告闭合**。

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
| `canonical IR` 逐字节等同 | ✅ **8** 个形状（条件位 3 + 语句位 5）—— **嵌套调用实参 2 行仍未覆盖** |
| `整组件` 逐字节等同 | ❌ **0** 行（发射器不认识该算子） |
| `wasmparser::Validator` 通过 | ✅ oracle 侧通过（固件钉住 1167 B 的组件） |
| 前沿探针期望值已翻转 | ✅ STEP-0529：**46 → 50** 条用例（3 条 `lowered` 翻转 + 4 条新增：`text-length-in-a-return-position` / `text-length-with-a-literal-argument` 为 `lowered`，`text-length-over-a-nested-call-argument` 为 `refused:ERR:E-SH-IR-EXPRESSION`；另有 STEP-0512 时代的旧用例 `host-call-in-return-position`（源码同为 `return sico.text.length("abc")`）由 `refused:ERR:E-SH-IR-EXPRESSION` 翻成 `lowered`） |
| 固件电池绿 | ✅ 本机 **BATTERY_GREEN 19/19**（固件 **159 / 159**；首轮红 3 步 = `cargo fmt`，按纪律整轮重跑） |
| **独立 CI 绿** | ✅ run #35（STEP-0528 `25e752a`）= **success**（run id `40004257414`，**本片回填**）；**本片**推送后触发新一轮，裁决由**下一片**回填 |

⇒ **T2 族未闭合**：剩余两项缺口（T2-g2 嵌套调用实参 / T2-c1 发射端）待落地。
