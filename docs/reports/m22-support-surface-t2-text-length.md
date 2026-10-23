# M22 支持面登记表 · T2 族（`sico.text.length`）

> - status: **STEP-0532 落地（T2-g2 实现片 = 前端形状面闭合；族未闭合）** —— **前端**已逐字节降（STEP-0529）、**组件层**已逐字节发射（STEP-0530，**10 个形状**）、**嵌套调用实参**（T2-g2）的前端形状已逐字节降（STEP-0532，主测 **5 个形状** + 补测 STEP-0529 的 `let` 绑定形 **2 个** = **7/7**）；**组件层**主测 **3/5** 逐字节等同、**2/5** 被**发射端**拒（内层算子 `sico.bytes.utf8_decode` = **T11 族**），补测后合计 **4/7 逐字节等同 + 3/7 发射端拒**；**两个真实 S 调用点的宿主函数仍不降**，归因已用对照形状钉死（**T7 族** / **匹配路由**）；语料覆盖 **0 条**
> - date: 2026-10-24
> - owner: autonomous-agent
> - scope: 计划 §3.3.5 的 **D15 支持面登记**：族 ×（名字 / 实参个数 / 实参类型 / 结果槽数 / 语料覆盖 / 未覆盖形状）
> - depends: [STEP-0517](../steps/STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）、[STEP-0527](../steps/STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（上一族 T1 收口）、[STEP-0528](../steps/STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（本族的拍 1 真值测量片）、[STEP-0529](../steps/STEP-0529-m22-w2-s5-w2-t2-frontend-knife.md)（本族的拍 2 前端刀）、[STEP-0530](../steps/STEP-0530-m22-w2-s5-w2-t2-codegen-knife.md)（**本族的拍 3 codegen 刀**）、[STEP-0531](../steps/STEP-0531-m22-w2-s5-w2-t2-g2-truth.md)（**T2-g2 的真值测量片**）、[STEP-0532](../steps/STEP-0532-m22-w2-s5-w2-t2-g2-knife.md)（**T2-g2 的实现片**）
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
| **STEP-0532 更新** | **T2 族的形状面已闭合**：嵌套调用实参在**前端**已逐字节降（主测 **5 个形状** + STEP-0529 `let` 绑定形 **2 个**，合计 **7/7**）；**组件层**逐字节等同 **4 个**（1183 / 1145 / 1162 / 1145 B），**3 个** 被**发射端**拒（内层算子 `sico.bytes.utf8_decode` = **T11 族**，**不进本族账**）。**仍未覆盖**：**比较实参路由**（`if` / `while` 条件里的嵌套实参，S 内无此形状）、**两个真实调用点的宿主函数**（归因见 §3.3）、**语料覆盖 0 条**、**整文件自举未关闭** | STEP-0532 实测 |

## 2. 族级真值表（13 行 + 1 行类型探针，实测）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端生产组件字节数；`guest` = guest 侧的判定。**每一行都由固件钉住**（STEP-0528 的 4 条钉 + STEP-0529 的 3 条新钉 / 1 条改钉 + STEP-0530 的 3 条新钉 / 1 条改钉 + STEP-0532 的 3 条新钉 / 2 条改钉）。

| # | 形状（位置 · 实参） | IR | 组件 | guest 前端 | guest 组件 |
|---|---|---|---|---|---|
| a | return（体内只有 return） · 形参 | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| b | let 绑定 · 形参 | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| c | return（体内只有 return） · 字面量 | 2252 | 1111 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| d | let 绑定 · 局部（cell） | 2212 | 1091 | ✅ **逐字节等同**（STEP-0529） | ✅ **逐字节等同**（STEP-0530） |
| e | let 绑定 · 嵌套用户调用 | 2656 | 1145 | ✅ **逐字节等同**（STEP-0532） | ✅ **逐字节等同**（STEP-0532） |
| f | let 绑定 · 嵌套宿主调用 | 2369 | 1570 | ✅ **逐字节等同**（STEP-0532） | ⏳ **T11 族**（发射端不认识 `sico.bytes.utf8_decode`，**不进本族账**） |
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
| 嵌套调用实参（return 位） | **1183** | ✅ **逐字节等同**（STEP-0532） |
| `while` 体 · 含 length | **1209** | ↩️ **`while` 循环形状**（与 T2 无关） |
| `while` 体 · **不含** length（**对照形状**） | **1142** | ↩️ **`while` 循环形状**（与 T2 无关） |
| `U64.bit_or`（**不含** length，**对照形状**） | **1039** | ↩️ **guest 前端窄一格**（与 T2 无关） |
| `U64.shr`（**不含** length，**对照形状**） | **1039** | ↩️ **guest 前端窄一格**（与 T2 无关） |

⇒ **前端逐字节等同 10 行**（上表 a/b/c/d/e/f/g/h/i/j）；**组件逐字节等同 12 行**（上表 a/b/c/d/e/g/h/j + 追加 4 行）；**前端 typed 拒绝 0 行**；**发射端 typed 拒绝 1 行**（行 f 的内层算子 `sico.bytes.utf8_decode` = **T11 族**，**不进本族账**）；**语义层拒绝 2 行**（元数）；**类型探针 1 行**。

> **行 f 的组件层不是本族的缺口**：它的**内层**算子 `sico.bytes.utf8_decode` 不在 guest 发射器认识的 3 个算子内 ⇒ 那是 **T11 族**的发射端缺口 ⇒ **不进 T2 的账**，也**不得**因此把 T2 读成整体闭合（见 §3.1 与 §3.3）。

## 3. 缺口登记（D10 判定用）

| 缺口 | 症状 | 根因 | S 内是否出现 | 归属 |
|---|---|---|---|---|
| ~~**T2-g1** 语句位的 `sico.text.length`~~ | ~~前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`~~ | **路由分家**：通用宿主调用路由 `gw_intrinsic_call_packed`（`selfhost/parser.sico:7001`）认 `sico.text.length`，但**直线路由家族** `straight_cell_function_ir` / `straight_environment_function_ir` 只认 `sico.text.concat` | **是**（S 内 **320** / 322 个调用点落在「局部 / 形参」实参上） | ✅ **T2-g1 闭合**（**STEP-0529**）：5 处接线 + 2 个新函数，**零新造机器** |
| **T2-g2** 实参本身是**嵌套调用** | ~~前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`~~ | ~~`gw_intrinsic_call_packed` 与新路由都只认**形参 / 局部 cell / 字面量**三种实参~~ | **是**（**2** 个调用点：`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))`、`compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`） | ✅ **闭合**（**STEP-0532**）：前端 **5/5 形状逐字节等同**（2477 / 2656 / 2369 / 2783 / 2374 B），组件层 **3/5** 逐字节等同；**新函数 + 两处接线**（纯插入、既有行零重排） |
| ~~**T2-c1** 发射端不认识 `sico.text.length`~~ | ~~组件层 typed 拒绝 `unsupported codegen source shape`~~ | guest 发射器 `cgs_intrinsic_hex` 只认两个算子 ⇒ **任何**用到该算子的模块都撞上它 | **是**（全部 320 个可降形状） | ✅ **T2-c1 闭合**（**STEP-0530**）：`cgs_intrinsic_hex` 新增 `sico.text.length` 分支（**四道闸门全 fail-closed**：名字 / 实参个数 = 1 / 实参类型 = `string` / 结果槽数 = 1）+ helper body + 类型 / 函数 / 代码三段接线 + **名字排序索引**（**+204 / −0 行**，纯插入） |

### 3.1 D10 复核（**不得降级**）

STEP-0530 闭合的是 **T2-c1**（**一个缺口**）。S 内 T2 族的 **322** 个调用点里：

- **320** 个（局部 306 + 形参 14）落在 **T2-g1** ⇒ **已闭合**；其**组件层**亦已闭合（**T2-c1**，STEP-0530）；
- **2** 个是**嵌套调用实参** ⇒ **T2-g2 的形状面已闭合**（STEP-0532，前端 5/5 逐字节等同）。

⇒ **但本族仍不得宣告闭合**（STEP-0532 实测）：那两个调用点的**宿主函数**仍不降，且**归因都不在本族**（§3.3）⇒ **形状面闭合 ≠ 族闭合**；此外**组件层仍有 2/5** 被**发射端**拒（内层算子属 **T11 族**）、**比较实参路由**未覆盖、**语料覆盖 0 条**。

> **STEP-0530 的诚实性做法**：发射端认领算子之后仍有拒绝，本片**没有**把它们一律记成 T2 缺口，而是**逐条用不含该算子的对照形状证伪**：`while` 循环形状（对照形状 1142 B，同样被拒）与 `U64.bit_or` / `U64.shr`（对照形状各 1039 B，同样被前端拒）**都与 T2 无关**，不进本族的账。

### 3.2 T2-g2 的实测真值（STEP-0531，**实现未做**）

T2-g2 是族内**最后**一条缺口：**实参本身是嵌套调用**。本片把它的**目标形状**全部量了出来（Rust 前端自己的 canonical IR，**不是设计值**），固件用 `the_t2_g2_target_ir_is_the_nested_call_shape` 钉住。

**规则（一句话）**：**嵌套调用是 1 条指令**；它的**结果 id 先于**外层调用的结果 id（**后序分配**）；外层 `sico.text.length` 把那个 id 当作**唯一实参**。

| 形状 | oracle canonical IR（指令序列） | guest 前端 | 归属 |
|---|---|---|---|
| 嵌套**内建**调用（`sico.text.concat`） | `1 const_string "x"` → `2 intrinsic sico.text.concat args=[0,1]` → `3 intrinsic sico.text.length args=[2]` | ❌ `ERR:E-SH-IR-EXPRESSION` | **T2-g2** |
| 嵌套**用户**调用（`helper(t)`） | `1 call 2 args=[0]` → `2 intrinsic sico.text.length args=[1]` | ❌ `ERR:E-SH-IR-EXPRESSION` | **T2-g2** |
| **S 站点** ①：`sico.bytes.utf8_decode(src)` | `1 intrinsic sico.bytes.utf8_decode args=[0]` → `2 intrinsic sico.text.length args=[1]` | ❌ `ERR:E-SH-IR-EXPRESSION` | **T2-g2** |
| **S 站点** ②：`positive_sleb_hex(initial)` | `1 call 2 args=[0]` → `2 intrinsic sico.text.length args=[1]` | ❌ `ERR:E-SH-IR-EXPRESSION` | **T2-g2** |
| S 站点 ① 的 `let` 绑定形 | 同上 | ❌ `ERR:E-SH-IR-EXPRESSION` | **T2-g2** |

**这 5 个形状的拒绝身份全部是 `ERR:E-SH-IR-EXPRESSION`**，而 oracle **全部能降下来** ⇒ 这些是**前端层**的缺口，**不是**语义拒绝 ⇒ **T2-g2 仍未闭合**。

**下一刀要动的两处路由**（实测定位，非猜测）：

| S 站点 | 形状 | 落点 |
|---|---|---|
| `compiler.sico:57`（`let byte_len = sico.text.length(sico.bytes.utf8_decode(src))`） | 嵌套**内建**调用 | **通用宿主调用路由** `gw_intrinsic_call_packed`（`selfhost/parser.sico:940`）的实参分支（现为「形参 / 局部 cell / 否则 `ERR:E-SH-IR-CALL-ARGUMENT`」三路） |
| `compiler.sico:361`（`match U64.checked_div(sico.text.length(positive_sleb_hex(initial)), U64.literal(2))`） | 嵌套**用户**调用 | **固定算子实参路径**（`U64.checked_div` 的实参），**不在**宿主调用路由里 |

> **为什么 T1 家族的既有做法不能直接照搬**：T1 族在 `straight_text_concat_packed` 里已经实现了「嵌套内建调用」（递归自身，`:9200-9230`）与「嵌套用户调用」（`stc_call_operand_packed`，`:9642`）——但那两条路径**只服务 `sico.text.concat` 的直线路由**；T2 的两个 S 站点落在**通用路由 / 固定算子实参路径**上 ⇒ **不能**只改 `straight_text_length_packed`。

### 3.3 T2-g2 的实现刀（STEP-0532）

STEP-0532 把 §3.2 量出的**靶子**落了下去：guest 前端现在认「`sico.text.length` 的实参本身是嵌套调用」这个形状。手法 = **一个新函数 + 两处纯插入接线**，**不新造机器**（嵌套调用复用 `straight_text_concat_packed` / `gw_intrinsic_call_packed` / `user_call_expression_json`）。

| 落点 | 位置 | 覆盖 |
|---|---|---|
| **新函数 `nested_text_call_packed`** | `selfhost/parser.sico:9086` | 把「实参是一个调用」降低成 `<指令数>\n<结果 id>\n<种类>\n<指令 JSON>` 的 pack |
| `straight_text_length_packed`（直线路由家族） | 既有「实参不是单个词 ⇒ `ERR:E-SH-IR-EXPRESSION`」之前**插入**嵌套分支 | `return` / `let` / `set` 语句位（**既有拒绝行原样保留**） |
| `gw_rhs_packed` 的 `sico.text.length` 分支（通用宿主调用路由） | 该分支 `return gw_intrinsic_call_packed(...)` **之前插入** | 通用路由函数（`general_while_function_ir` 家族）的 `let` / `set` 右值、分支块内语句 |

**前端层实测（canonical IR 逐字节等同，oracle vs guest）**：

| 形状 | IR 字节 | 前端 | 组件层 |
|---|---|---|---|
| 嵌套**内建**调用（`sico.text.concat`）· `return` 位 | **2477** | ✅ 逐字节等同 | ✅ **1183 B** 逐字节等同 |
| 嵌套**用户**调用（`helper(t)`）· `return` 位 | **2656** | ✅ 逐字节等同 | ✅ **1145 B** 逐字节等同 |
| 嵌套**内建**调用（`sico.bytes.utf8_decode`）· `return` 位 | **2369** | ✅ 逐字节等同 | ⏳ **发射端拒**（内层算子 = **T11 族**） |
| 嵌套**用户**调用 · S 站点 ② 形 | **2783** | ✅ 逐字节等同 | ✅ **1162 B** 逐字节等同 |
| 嵌套**内建**调用（`sico.bytes.utf8_decode`）· `let` 绑定形（= **S 站点 ① 形**） | **2374** | ✅ 逐字节等同 | ⏳ **发射端拒**（内层算子 = **T11 族**） |
| 嵌套**用户**调用 · `let` 绑定形（STEP-0529 的改钉） | **2656** | ✅ 逐字节等同 | ✅ **1145 B** 逐字节等同（**补测并钉进固件**） |
| 嵌套**内建**调用（`sico.bytes.utf8_decode`）· `let` 绑定形（STEP-0529 的改钉） | **2369** | ✅ 逐字节等同 | ⏳ **发射端拒**（内层算子 = **T11 族**，**补测并钉进固件**） |

⇒ **前端 7/7 形状逐字节等同**；**组件层 4/7 逐字节等同**（1183 / 1145 / 1162 / 1145 B，两侧都过真实 `wasmparser::Validator`）、**3/7 被发射端拒**（内层算子 `sico.bytes.utf8_decode` 不在 guest 发射器认识的 3 个算子内 ⇒ **T11 族，不进 T2 账**）。

> **其中 5 个是本片主测的形状**（前端 **5/5** 逐字节等同、组件层 **3/5** 逐字节等同 + **2/5** 发射端拒）；另 **2 个**是 STEP-0529 的 `let` 绑定形，**本片补测**后同样钉住（前端 2656 / 2369 B 逐字节等同；组件层 1145 B 逐字节等同 / 发射端拒）。**分母不同 ⇒ 两处都写清楚**，不把补测混进主测口径。

**位置边界（实测）**：`return` / `let` / `set` / 分支块内语句 / 通用路由 `let` = **LOWERED**；**比较实参路由**（`if` / `while` 条件里把嵌套实参当比较的一侧）仍 typed 拒 `ERR:E-SH-IR-CALL-SHAPE`（**S 内没有**这种调用点 ⇒ 未覆盖，登记为边界）。

**两个真实 S 调用点的诚实归因（用 `compiler.sico` 逐字原文重测，非简化 fixture）**：

| 真实站点 | 宿主函数 | 改前判词 | 改后判词 | 归因（对照形状证伪） |
|---|---|---|---|---|
| ① `compiler.sico:57` | `lex_words` | `ERR:E-SH-IR-EXPRESSION` | `ERR:E-SH-IR-CALL-SHAPE` | 该函数另含 `sico.list.append(<局部>, <嵌套调用>)`（**T7 族**）；把 T2 那一句摊平后判词**再变**（`ERR:E-SH-IR-CALL-TARGET`）⇒ **T2 那一句不再拦路** |
| ② `compiler.sico:361` | `positive_sleb_length` | `ERR:E-SH-IR-EXPRESSION` | `ERR:E-SH-IR-MATCH-SHAPE` | **匹配路由要求恰好 2 个形参**（该函数 1 个）；其后再叠**固定算子实参路径不接受 `sico.*` 实参**（对照形状：**2 个形参 + 非嵌套实参**同样被拒）⇒ **与嵌套调用形状无关** |

⇒ **两个真实调用点的宿主函数仍不降，但归因都不在本族**（分别归 **T7 族** 与 **匹配路由**）⇒ **形状面闭合 ≠ 族闭合**（**D10** ⇒ 本族不得降级、不得宣告闭合）。

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
| `canonical IR` 逐字节等同 | ✅ **10** 个形状（条件位 3 + 语句位 5 + **嵌套调用实参 2**）—— **STEP-0532** 把嵌套调用实参的前端形状面补齐（**5/5 逐字节等同**） |
| `整组件` 逐字节等同 | ✅ **12** 个形状（STEP-0530 的 10 + STEP-0532 的 2）—— 另有 **3 个嵌套形状被发射端拒**（内层算子 `sico.bytes.utf8_decode` = **T11 族**，**不进本族账**） |
| `wasmparser::Validator` 通过 | ✅ **两侧都过**（STEP-0530：oracle 的生产组件与 guest 发出的组件都过真实 validator；STEP-0532 的 3 个组件形状同样两侧都过） |
| 前沿探针期望值已翻转 | ✅ STEP-0530：**50 → 51** 条用例（**0 条翻转 + 1 条新增**：`text-length-over-a-nested-call-argument-in-a-return-position` = `refused:ERR:E-SH-IR-EXPRESSION`）⇒ 实测 **`FRONTIER_UNCHANGED`**（本片只动发射端，不动前端）；**STEP-0531**：**51 → 53** 条（**再 +2 条**，把 **T2-g2 的两个 S 站点**钉进探针：`text-length-over-a-nested-user-call-argument` / `text-length-over-a-nested-host-call-argument-in-a-let-binding`，均 `refused:ERR:E-SH-IR-EXPRESSION`）；**STEP-0532**：**53 → 54** 条（**4 条翻转** `refused:ERR:E-SH-IR-EXPRESSION` → `lowered` + **1 条新增** `text-length-over-a-nested-call-argument-in-a-set-position`）⇒ 实测 **`FRONTIER_UNCHANGED`**（54/54 全部与期望一致） |
| 固件电池绿 | ✅ 本机 **BATTERY_GREEN 19/19**（固件 **161 / 161**）；**STEP-0531**：固件 **163 / 163**（**+2 条** T2-g2 钉：边界拒绝钉 + **目标 IR 钉**）；**STEP-0532**：固件 **166 / 166**（**+3 条新钉 + 2 条改钉**） |
| **独立 CI 绿** | ⛔ **STEP-0532 的 run #39 = failure**（run id `40038444653`，head_sha `504c849`，job `windows-gnu` 25m 26s）—— workflow 把 `tools/run-ci.ps1` 的 13 个子步骤包在同一个 step 里，公开注解只有 **1 error**（exit code 1）且**无任何 rustc / clippy / 测试诊断**，日志需登录（匿名 REST **403**、本机无 `gh`/token）⇒ **失败子步骤未能取到、未归因**；本机所有可复现口径全绿（电池 19/19 ×2、固件 166/166 ×2、runner 全量 exit 0、探针 54/54）⇒ **不得读成族已闭合**。前序 run #36 / #37 / #38 = **success**；**STEP-0532 的回填提交触发 run #40**，裁决由**下一片**回填 |

⇒ **T2 族未闭合**：剩余**一项**缺口（**T2-g2 嵌套调用实参**，前端层）待落地；**组件层（`T2-c1`）已闭合**。
