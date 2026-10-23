# M22 支持面登记表 · T2 族（`sico.text.length`）

> - status: **STEP-0530 落地（codegen 刀 C 已完成；族未闭合）** —— **前端**已逐字节降（STEP-0529），**组件层**已逐字节发射（STEP-0530，**10 个形状**），**嵌套调用实参**仍 typed 拒绝；语料覆盖 **0 条**
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**：族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）
> - depends: [STEP-0517](../steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）、[STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（上一族 T1 收口）、[STEP-0528](../steps/STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（本族的拍 1 真值测量片）、[STEP-0529](../steps/STEP-0529-m22-w2-s5-w2-t2-frontend-knife.md)（本族的拍 2 前端刀）、[STEP-0530](../steps/STEP-0530-m22-w2-s5-w2-t2-codegen-knife.md)（**本族的拍 3 codegen 刀**）
> - 口径: **所有数字来自 oracle 的生产输出**（Rust 前端 `lower_core` + `canonical_json`，Rust 后端 `compile_script_program`），**不是设计值**

## 1. 子集边界（D15 要求登记的六项）

| 项 | 值 | 证据 |
|---|---|---|
| 支持的名字 | **仅** `sico.text.length` | STEP-0530：guest 发射器 `cgs_intrinsic_hex` 现在认 `sico.bytes.length` / `sico.text.concat` / **`sico.text.length`** 三个算子 ⇒ 其余 `ERR:E-SH-CG-IR-EMIT` |
| 实参个数 | **恰好 1** | STEP-0528：**元数边界在语义层**（0 个 / 2 个实参被 **Rust 前端自己**拒 `stdlib intrinsic arity`，区间 358..376 / 365..387）⇒ guest **不自造**元数检查 |
| 实参类型 | **一个 `Text`**（canonical `string`；**2 槽**：pointer + length） | STEP-0528：后端 `emit_intrinsic` 取 `[pointer, length]` 对（`crates/sico-codegen-wasm/src/lib.rs:4466`）；**STEP-0529**：guest 前端会**查实参类型**，类型不符 ⇒ `ERR:E-SH-IR-CALL-TYPE`（oracle 侧同一形状在**语义层**判 `E2001` / `TYPE_MISMATCH` / "expected Text, found Bytes"） |
| 结果槽数 | **1**（`U64`） | STEP-0528：后端把 helper 的 i32 结果 `I64ExtendI32U`（`0xad`）后落槽 |
| 语料覆盖 | **0 条** | S 内 **322** 个 `sico.text.length(` 文本调用点**一个都没进语料** |
| 未覆盖形状 | **1 类**：**嵌套调用实参**（S 内 **2** 个调用点）。**组件层**已闭合（STEP-0530）；**前端层**仍缺 | STEP-0530 实测（§4.4 用对照形状逐条证伪了「循环形状」「位运算」两类拒绝） |

## 2. 族级真值表（13 行 + 1 行类型探针，实测）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。**每一行都由固件钉住**（STEP-0528 的 4 条钉 + STEP-0529 的 3 条新钉 / 1 条改钉 + STEP-0530 的 3 条新钉 / 1 条改钉）。

| # | 形状（位置 · 实参） | IR | 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| a | return（体内只有 return） · 形参 | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| b | let 绑定 · 形参 | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| c | return（体内只有 return） · 字面量 | 2252 | 1111 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| d | let 绑定 · 局部（cell） | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| e | let 绑定 · 嵌套用户调用 | 2656 | 1145 | ❌ `ERR:E-SH-IR-EXPRESSION`（**身份不变**） | ⏳ **T2-g2**（前端层，未到发射端） |
| f | let 绑定 · 嵌套宿主调用 | 2369 | 1570 | ❌ `ERR:E-SH-IR-EXPRESSION`（**身份不变**） | ⏳ **T2-g2**（前端层，未到发射端） |
| g | `set` · 形参 | 2765 | 1113 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| **h** | **`if` 条件 · 形参** | **2991** | **1167** | ✅ **逐字节等同** | ✅ **逐字节等同**（STEP-0530） |
| **i** | **`while` 条件 · 形参** | **3446** | **1187** | ✅ **逐字节等同** | ↩️ **`while` 循环形状**（**与 T2 无关**，§4.4 对照形状证明） |
| **j** | **分支块内的语句 · 形参** | **3437** | **1187** | ✅ **逐字节等同** | ✅ **逐字节等同**（STEP-0530） |
| k | 元数探针 · 0 个实参 | — | — | ❌ **Rust 前端自己拒** `stdlib intrinsic arity` | — |
| l | 元数探针 · 2 个实参 | — | — | ❌ **Rust 前端自己拒** `stdlib intrinsic arity` | — |
| **m** | **类型探针 · `Bytes` 实参** | — | — | ❌ **guest 前端拒** `ERR:E-SH-IR-CALL-TYPE`（oracle：语义层 `E2001` / `TYPE_MISMATCH`） | — |

**STEP-0530 追加的组件层形状**（不在上表的 IR 口径内，但组件真值同样实测）：

| 形状 | 组件 | guest 组件 |
|---|---|---|
| concat + length 同模块（两个内建 helper 共存） | **1183** | ✅ **逐字节等同** |
| 同一函数两个 `sico.text.length` 调用 | **1197** | ✅ **逐字节等同** |
| 两个函数各一个 `sico.text.length` 调用 | **1139** | ✅ **逐字节等同** |
| 嵌套调用实参（return 位） | **1183** | ⏳ **T2-g2**（前端层） |
| `while` 体 · 含 length | **1209** | ↩️ **`while` 循环形状**（与 T2 无关） |
| `while` 体 · **不含** length（**对照形状**） | **1142** | ↩️ **`while` 循环形状**（与 T2 无关） |
| `U64.bit_or`（**不含** length，**对照形状**） | **1039** | ↩️ **guest 前端窄一格**（与 T2 无关） |
| `U64.shr`（**不含** length，**对照形状**） | **1039** | ↩️ **guest 前端窄一格**（与 T2 无关） |

⇒ **前端逐字节等同 8 行**；**组件逐字节等同 10 行**（上表 a/b/c/d/g/h/j + 追加 3 行）；**前端 typed 拒绝 2 行**（身份不变）；**语义层拒绝 2 行**（元数）；**类型探针 1 行**。

## 3. 缺口登记（D10 判定用）

| 缺口 | 症状 | 根因 | S 内是否出现 | 归属 |
|---|---|---|---|---|
| ~~**T2-g1** 语句位的 `sico.text.length`~~ | ~~前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`~~ | **路由分家**：通用宿主调用路由 `gw_intrinsic_call_packed`（`selfhost/parser.sico:7001`）认 `sico.text.length`，但**直线路由家族** `straight_cell_function_ir` / `straight_environment_function_ir` 只认 `sico.text.concat` | **是**（S 内 **320** / 322 个调用点落在「局部 / 形参」实参上） | ✅ **T2-g1 闭合**（**STEP-0529**）：5 处接线 + 2 个新函数，**零新造机器** |
| **T2-g2** 实参本身是**嵌套调用** | 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION` | `gw_intrinsic_call_packed` 与新路由都只认**形参 / 局部 cell / 字面量**三种实参 | **是**（**2** 个调用点：`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))`、`compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`） | ⏳ **未闭合**（STEP-0528 登记，STEP-0529 / STEP-0530 未动）⇒ T2 版的 G-f2，**下一刀** |
| ~~**T2-c1** 发射端不认识 `sico.text.length`~~ | ~~组件层 typed 拒绝 `unsupported codegen source shape`~~ | guest 发射器 `cgs_intrinsic_hex` 只认两个算子 ⇒ **任何**用到该算子的模块都撞上它 | **是**（全部 320 个可降形状） | ✅ **T2-c1 闭合**（**STEP-0530**）：`cgs_intrinsic_hex` 新增 `sico.text.length` 分支（**四道闸门全 fail-closed**：名字 / 实参个数 = 1 / 实参类型 = `string` / 结果槽数 = 1）+ helper body + 类型 / 函数 / 代码三段接线 + **名字排序索引**（**+204 / −0 行**，纯插入） |

### 3.1 D10 复核（**不得降级**）

STEP-0530 闭合的是 **T2-c1**（**一个缺口**）。S 内 T2 族的 **322** 个调用点里：

- **320** 个（局部 306 + 形参 14）落在 **T2-g1** ⇒ **已闭合**；其**组件层**亦已闭合（**T2-c1**，STEP-0530）；
- **2** 个是**嵌套调用实参** ⇒ 仍属 **T2-g2**，**未覆盖**。

⇒ **T2-g2 未闭合 ⇒ 本族不得降级、不得宣告闭合**。

> **STEP-0530 的诚实性做法**：发射端认领算子之后仍有拒绝，本片**没有**把它们一律记成 T2 缺口，而是**逐条用不含该算子的对照形状证伪**：`while` 循环形状（对照形状 1142 B，同样被拒）与 `U64.bit_or` / `U64.shr`（对照形状各 1039 B，同样被前端拒）**都与 T2 无关**，不进本族的账。

## 4. S 内 T2 调用点普查（口径：文本出现次数）

| 口径 | 数值 |
|---|---|
| `sico.text.length(` 调用点 | **322**（计划 §3.3.5 的族表记 **318** ⇒ **订正**：S 在计划成文后的增长） |
| 按文件 | `parser.sico` **223** / `codegen.sico` **59** / `compiler.sico` **24** / `compiler_semantics.sico` **10** / `compiler_parser.sico` **3** / `checker.sico` **2** / `formatter.sico` **1** |
| **实参形态** | **局部（cell）306** / **形参 14** / **调用 2** / 字面量 **0** |
| **位置** | `set` **131** / `if` 条件 **119** / `let` 绑定 **69** / `while` 条件 **2** / `return` **1** |

## 5. 组件层真值（core module code 段反汇编，STEP-0530 实测）

- **调用点**：`local.get pointer; local.get length; call <helper>; i64.extend_i32_u (0xad); local.set result`；
- **helper 索引**：core function 索引 = `模块函数数 + 3 + 名字排序位`（**两个内建 helper 按点分名排序** ⇒ `sico.text.concat` 恒为 0、`sico.text.length` 为 1）；实测：只用 length ⇒ `函数数 + 3`，concat + length 同模块 ⇒ `函数数 + 4`；
- **helper 类型**：`(i32, i32) -> i32` = `60027f7f017f`；**helper body** = **locals 声明 `01 02 7f`（1 组、2 个 i32）+ 51 B 代码**，合计 **54 B**，**数的是 UTF-8 码点**（非续字节）；
- ⇒ **STEP-0528 §4.5 记的「body 51 B」是代码长度，不是函数体长度**：wasm 的函数体 = **locals 组 + 代码**（STEP-0530 §4.1 首轮就踩了这个坑，差 3 字节）。

## 6. 族级出口判据（六项缺一不算闭合）

| 判据 | 状态 |
|---|---|
| `canonical IR` 逐字节等同 | ✅ **8** 个形状（条件位 3 + 语句位 5）—— **嵌套调用实参 2 行仍未覆盖** |
| `整组件` 逐字节等同 | ✅ **10** 个形状（STEP-0530）—— **嵌套调用实参 2 行仍未覆盖**（前端层就拒了，到不了发射端） |
| `wasmparser::Validator` 通过 | ✅ **两侧都过**（STEP-0530：oracle 的生产组件与 guest 发出的组件都过真实 validator） |
| 前沿探针期望值已翻转 | ✅ STEP-0530：**50 → 51** 条用例（**0 条翻转 + 1 条新增**：`text-length-over-a-nested-call-argument-in-a-return-position` = `refused:ERR:E-SH-IR-EXPRESSION`）⇒ 实测 **`FRONTIER_UNCHANGED`**（本片只动发射端，不动前端） |
| 固件电池绿 | ✅ 本机 **BATTERY_GREEN 19/19**（固件 **161 / 161**） |
| **独立 CI 绿** | ✅ run #36（STEP-0529 `e5f657a`）= **success**（run id `40016972272`，**本片回填**）；**本片**推送后触发新一轮，裁决由**下一片**回填 |

⇒ **T2 族未闭合**：剩余**一项**缺口（**T2-g2 嵌套调用实参**，前端层）待落地；**组件层（`T2-c1`）已闭合**。
