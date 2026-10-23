# STEP-0528 m22-w2 S5 **W2 族 T2 真值测量片（`sico.text.length`）**：**条件位已经能逐字节降，语句位不能** —— 族未闭合

> - status: **measurement complete locally（本机实测绿，随本片提交并推送；独立 CI 裁决待下一批回填）**
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0527](./STEP-0527-m22-w2-s5-w2-t1-family-closure.md)（上一片 · **T1 族收口**）、[STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）、[STEP-0511](./STEP-0511-m22-w2-s5-host-call-truth.md)（宿主调用真值测量片的方法学先例）
> - gate: **不推进任何出口门** —— W2 的**第 1 拍（真值测量片 T）**，按计划 §3.3.5 的「每族四拍」模板执行；**guest 生产代码零改动**、**oracle 生产代码零改动**
> - numbering: **STEP-0528**（取号前已核对 `docs/steps/`：最大号 0527；*0528 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t2-text-length.md)（D15 风格，**族未闭合**）

## 0. 入口门槛

| 入口条件 | 状态 |
|---|---|
| 上一族 **T1** 已按计划 §3.3.5 的**族级出口判据六项**闭合（STEP-0527）⇒ 允许开下一族 | ✅ |
| owner 已授权 W2 族按「每族四拍」推进（D4–D15 全部裁决关闭）。 | ✅ 计划 §3.3.5 |
| 本拍是**纯测量**：只读 oracle **生产输出**，不写 guest 生产代码。 | ✅ 方法学先例 STEP-0501 / 0505 / 0511 / 0517 |
| 出口测试有界。 | ✅ §3：4 条钉 + 5 条前沿用例 |

⇒ **取号合法**。*测量已完成*，见 §4。

## 1. 这一片回答的问题

计划 §3.3.5 把 W2 定义为 **20 个宿主内建族**的逐字节编解码，按 **S 内调用点降序**排序：

| 序 | 内建 | 计划记的 S 内调用点 | **本片实测** | 本片状态 |
|---|---|---|---|---|
| T1 | `sico.text.concat` | 2,882 | 2,943（STEP-0521 订正） | ✅ **已闭合**（STEP-0527） |
| **T2** | **`sico.text.length`** | **318** | **322** | **本片（真值测量片）** |
| T3 | `sico.text.starts_with` | 316 | — | 未开始 |

> **口径订正**：计划 §3.3.5 的族表记 **318**，本片按**文本出现次数**（`sico.text.length(`）实测 **322**。差的 4 个是 S 在计划成文之后的增长（STEP-0522…0526 在 `codegen.sico` 里新增了 concat 调用，T2 的调用点也随之增加）。**两个数字口径不同、不互相矛盾**：318 是计划成文时的快照，322 是本片的实测。

本片要回答三个问题：

1. **规则表**：T2 的各个形状，Rust 前端的 canonical IR 与 Rust 后端的**生产组件**分别是什么字节？
2. **子集边界**：T2 的「名字 / 实参个数 / 实参类型 / 结果槽数」四道边界分别卡在哪一层？
3. **下一刀切点**：guest 侧缺的是**能力**、**接线**、还是**可达性**？

**本片的结论（先说结果）**：**guest 前端在「条件位」上已经能逐字节降 `sico.text.length`**（`if` 条件 / `while` 条件 / 分支块里的语句），**在「语句位」上不能**（`let` 绑定 / `set` / 体内只有一条 `return`）—— 原因是**路由分家**，不是能力缺失；**组件层则完全没闭合**（guest 发射器不认识这个算子，fail-closed 拒绝）。

## 2. 测量设计

### 2.1 真值来源

| 真值 | 取法 |
|---|---|
| canonical IR | Rust 前端 `lower_core` + `canonical_json`（**生产路径**） |
| 整组件字节 | Rust 后端 `sico_codegen_wasm::compile_script_program`（**生产输出**，非重实现） |
| core module code 段 | 解析**生产组件**的 core module，导出每个函数体的 locals 组与指令字节（**反汇编**） |
| guest 判读 | **真实 runner**（fuel 800 B / timeout 120 s / 内存 2560 MiB）—— 权威通道 |

### 2.2 形状谱系（12 个变体）

覆盖「位置 × 实参形态 × 实参个数」三个维度；`a`–`j` 是 10 个形状，`k` / `l` 是两个**元数探针**。

### 2.3 S 内调用点普查（脚本 `target/tmp/step0528/census_t2.py`）

| 口径 | 数值 |
|---|---|
| `sico.text.length(` 的文本调用点 | **322** |
| 按文件 | `parser.sico` **223** / `codegen.sico` **59** / `compiler.sico` **24** / `compiler_semantics.sico` **10** / `compiler_parser.sico` **3** / `checker.sico` **2** / `formatter.sico` **1** |
| **实参形态** | **局部（cell）306** / **形参 14** / **调用 2** / 字面量 **0** |
| **位置** | `set` **131** / `if` 条件 **119** / `let` 绑定 **69** / `while` 条件 **2** / `return` **1** |

⇒ **S 内 T2 调用点没有一个是字面量实参**；**只有 2 个**是「实参本身是调用」（`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))` 与 `compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`）⇒ 这 2 个是 T2 版的 **G-f2 面**（嵌套调用实参）。

## 3. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 |
|---|---|---|
| 1 | `sico_compiler_lowers_the_text_length_in_a_condition_position_byte_exactly` | 3 个**条件位**形状（`if` 条件 2991 B / `while` 条件 3446 B / 分支块内 3437 B）⇒ guest IR **逐字节** == Rust canonical IR；intrinsic 名字保持 `sico.text.length` |
| 2 | `sico_compiler_keeps_the_unclosed_text_length_positions_typed_refused` | 7 个**语句位**形状仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（**不静默放宽**） |
| 3 | `the_text_length_arity_is_refused_by_the_rust_frontend_itself` | 0 个 / 2 个实参 ⇒ **Rust 前端自己**拒 `stdlib intrinsic arity`（**元数边界在语义层**） |
| 4 | `sico_codegen_guest_refuses_the_text_length_component_outside_the_subset` | oracle 生产组件 = **1167 B**（钉住）+ 过真实 `wasmparser::Validator`；guest 发射侧 **fail-closed** 拒绝 `unsupported codegen source shape` |

⇒ 固件从 **153** 增至 **157** 条；前沿探针从 **41** 增至 **46** 条用例（新增 5 条 T2 用例，期望值 = 本片实测）。

## 4. 实施记录（2026-10-24）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `runner/sico-runner/tests/selfhost_compiler.rs` | **新增 4 条钉**（§3），纯追加 |
| `tools/probe-m22-frontend-frontier.ps1` | 前沿用例 **41 → 46 条**（新增 T2 五条：3 条 `lowered` + 2 条 `refused:ERR:E-SH-IR-EXPRESSION`） |
| `selfhost/*.sico` | **零改动**（纯测量） |
| oracle 生产代码（`crates/*`） | **零改动** |
| `docs/plans/M22-compiler-self-host.md` / `docs/STATUS.md` / `docs/ROADMAP.md` / `docs/handoff-m22.md` / `docs/steps/README.md` / `docs/reports/README.md` | 同步本片结论；新增 [T2 支持面登记表](../reports/m22-support-surface-t2-text-length.md) |

### 4.2 T2 规则表（实测，fixture 形状）

`IR` = Rust 前端 canonical IR 字节数；`组件` = Rust 后端**生产组件**字节数；`guest 前端` = guest 的判定。

| # | 形状 | 位置 | 实参 | IR | 组件 | **guest 前端** | **guest 发射端** |
|---|---|---|---|---|---|---|---|
| a | `text-length-a` | return（体内**只有** return） | 形参 | 2212 | 1091 | ❌ `ERR:E-SH-IR-EXPRESSION` | — |
| b | `text-length-b` | let 绑定 | 形参 | 2212 | 1091 | ❌ 同上 | — |
| c | `text-length-c` | return（体内只有 return） | 字面量 | 2252 | 1111 | ❌ 同上 | — |
| d | `text-length-d` | let 绑定 | 局部（cell） | 2212 | 1091 | ❌ 同上 | — |
| e | `text-length-e` | let 绑定 | **嵌套用户调用** | 2656 | 1145 | ❌ 同上 | — |
| f | `text-length-f` | let 绑定 | **嵌套宿主调用** | 2369 | 1570 | ❌ 同上 | — |
| g | `text-length-g` | `set` | 形参 | 2765 | 1113 | ❌ 同上 | — |
| **h** | `text-length-h` | **`if` 条件** | 形参 | **2991** | **1167** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| **i** | `text-length-i` | **`while` 条件** | 形参 | **3446** | **1187** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| **j** | `text-length-j` | **分支块内的语句** | 形参 | **3437** | **1187** | ✅ **逐字节等同** | ❌ typed 拒绝 |
| k | `text-length-k` | let 绑定 | **0 个实参** | — | — | ❌ **Rust 自己拒** `stdlib intrinsic arity`（358..376） | — |
| l | `text-length-l` | let 绑定 | **2 个实参** | — | — | ❌ **Rust 自己拒** `stdlib intrinsic arity`（365..387） | — |

**逐字节等同的口径**：guest 的 canonical IR 输出与 Rust 前端 `canonical_json` 的输出**按字节相等**（不只是同形、不是同长）。

### 4.3 关键发现：**路由分家**，不是能力缺失

`h` / `i` / `j` 三个形状**已经**逐字节等同 —— guest 前端**实现了** T2 的下降，而且实现得完全正确。

**分叉点在哪（源码定位）**：

- `selfhost/parser.sico:7001-7003` —— **通用宿主调用路由** `gw_intrinsic_call_packed` 认 `sico.text.length`（实参可以是**形参**或**局部 cell**）；
- `selfhost/parser.sico:9707` 与 `:10289` —— **直线路由家族** `straight_cell_function_ir`（承担 `let` / `set` / `return` 位的语句）**只认 `sico.text.concat`** ⇒ 语句位一律落回 `ERR:E-SH-IR-EXPRESSION`。

⇒ **下一刀切点（精确）**：**让直线路由家族认 `sico.text.length`**（一个 `Text` 实参 ⇒ 一个 `U64` 结果），**不需要新造机器** —— `gw_intrinsic_call_packed` 这台既有机器已经能正确处理「形参 / 局部 cell」两种实参。**S 内 322 个调用点里有 320 个**（局部 306 + 形参 14）落在这个形状上；**只有 2 个**是嵌套调用实参（T2 版的 G-f2 面），需要另一刀。

> **方法学复现（与 STEP-0517 同族）**：「某个形状被拒绝」**不等于**「该形状没实现」。本片第一轮的结论差点写成「guest 前端未实现 T2 的语句位下降」—— 源码里 `gw_intrinsic_call_packed` **本来就有**这条能力，只是**够不着**那三种位置。

### 4.4 T2 子集边界（登记进支持面登记表）

| 维度 | 边界 | 卡在哪一层 |
|---|---|---|
| **名字** | 仅 `sico.text.length` | guest 发射器 `cgs_intrinsic_hex` 只认 `sico.bytes.length` 与 `sico.text.concat` ⇒ 其余 `ERR:E-SH-CG-IR-EMIT` |
| **实参个数** | 恰好 **1** | **语义层**（Rust 前端自己拒 `stdlib intrinsic arity`）⇒ guest **不得**自造元数检查 |
| **实参类型** | 一个 `Text`（canonical `string`；**2 槽**：pointer + length） | 后端 `emit_intrinsic` 取 `[pointer, length]` 对（`crates/sico-codegen-wasm/src/lib.rs:4466`） |
| **结果槽数** | **1**（`U64`） | 后端把 helper 的 i32 结果 `I64ExtendI32U`（`0xad`）后落槽 |
| **语料覆盖** | **0 条**（S 内 322 个调用点**一个都没进语料**） | — |
| **未覆盖形状** | ①**语句位**（`let` / `set` / 单 `return`）7 个变体；②**嵌套调用实参** 2 个变体；③**组件层全部** | — |

### 4.5 组件层真值（core module code 段反汇编）

**调用点**（core function 0 = 模块函数 `size`，33 B）：

```
41 00 21 03      i32.const 0; local.set 3          # dispatcher 外壳
02 40 03 40      block; loop
20 03 41 00 46   local.get 3; i32.const 0; i32.eq
04 40            if
20 00            local.get 0                      # 形参 pointer
20 01            local.get 1                      # 形参 length
10 05            call 5                           # sico.text.length helper
ad               i64.extend_i32_u
21 02            local.set 2                      # 结果（U64）
20 02 0f         local.get 2; return
0b 00 0b 0b 0b   end; unreachable; end; end; end
```

**helper 事实**（core function 5 = `sico.text.length`，**51 B**，locals 组 = `2 × i32`）：

```
03 40                       loop
20 02 20 01 4f              local.get 2; local.get 1; i32.ge_u
04 40 20 03 0f 0b           if; local.get 3; return; end
20 00 20 02 6a 2d 00 00     local.get 0; local.get 2; i32.add; i32.load8_u
41 c0 01 71 41 80 01 47     i32.const 0xC0; i32.and; i32.const 0x80; i32.ne
04 40 20 03 41 01 6a 21 03 0b   if; count+1; end
20 02 41 01 6a 21 02       i+1; local.set 2
0c 00 00 0b 00 0b          br 0; unreachable; end; unreachable; end
```

⇒ **helper 数的是「非续字节」= UTF-8 码点**（与 Rust 后端 `emit_text_length`，`crates/sico-codegen-wasm/src/stdlib.rs:390` 同构）。

**helper 索引规则（从后端生产代码读出，不是设计）**：`helper_count = import_count + module.functions.len()`；运行时 helper 占 `helper_count` / `+1` / `+2`；**内建 helper = `helper_count + 3 + 名字排序位`**（`used_helpers` 是 `BTreeSet`，**按名字排序** ⇒ `sico.text.concat` 恒为 0，`sico.text.length` 为 1）—— 实测：模块 2 个函数 ⇒ helper 落在 core function **5**。

⇒ **对 codegen 刀的直接含义**：与 `sico.bytes.length`（**内联**发射 `local.get` / `i64.extend_i32_u` / `local.set`，**不需要 helper**）不同，**`sico.text.length` 必须发射一个 helper 函数体**（51 B）+ **调用点**；而且一旦同一模块里同时用 `sico.text.concat` 与 `sico.text.length`，guest 就必须处理**两个**内建 helper 的**名字排序索引**（今天它只处理 `concat` 一个）。

### 4.6 自造缺陷 ①：**第一轮测的是两种不同的模块形状**

第一版测量脚本对 **IR** 用 fixture、对**组件**用「CLI 变体」（多 3 个 enum case、`function main` 替 `function run`）—— 两个不同的模块。**怎么发现的**：固件要钉的 fixture 形状实测出来是 **2212 B**，而第一轮记的是 **2209 B**，差 **3** 字节 ⇒ 差的不是形状，是**函数名长度**（第一轮用 `f`，固件用 `size`）。**修法**：改用临时探针（`runner/sico-runner/tests/scratch_step0528_probe.rs`，**本片收尾时已删除**）直接量**固件真正会构造的那个 fixture 模块**，钉住那组数字。

### 4.7 自造缺陷 ②：**元数探针没记「哪一层拒的」**

第一轮脚本对 k / l 只写了「RUST_REFUSED」，**没记是谁拒的** ⇒ 后来者可能误以为「guest 拒了元数」。**修法**：探针记录**语义层身份** `stdlib intrinsic arity` 与源码区间（358..376 / 365..387），固件钉住这个身份。

### 4.8 自造缺陷 ③：**电池首步红了 —— 完整性白名单还停在上一片**

电池第 1 步 `workspace-integrity` 红了（`exit=1`）：该检查逐个把**受版本控制的文件**与**索引 blob**比对，不在本片「预期变更表」里的任何差异都算**就地损坏**。而那张表还写着 **STEP-0526** 的变更集（`selfhost/codegen.sico` 等）⇒ 本片改的 **8** 个文件（固件 + 前沿探针 + 6 个文档）全部被报成损坏。**修法**：把本片的 8 个路径登记进那张表（其余 1841 个受控文件仍逐字节等于索引 blob ⇒ `mismatches = 0`）⇒ **按纪律整轮重跑电池**（上表是重跑那一轮的结果）。

### 4.9 自造缺陷 ④：**取消电池 ⇒ 留下损坏的 runner 测试二进制**

第二轮电池的第 3 步 `ci-one-key` 红了：`runner/sico-runner/tests/record_types.rs` 的测试进程以 **`STATUS_ACCESS_VIOLATION`（0xc0000005）** 崩掉 —— **一个与本片毫无关系的测试**（record 类型，本片只动了 selfhost 编译器的测试文件与前沿探针）。**怎么定位的**：①单独复跑 ⇒ **可复现**（不是偶发）；②查本片改动面 ⇒ `record_types` 是**独立测试二进制**，与本片改的文件无依赖关系；③回看时间线 ⇒ 本片的**首轮电池被我手动取消**，而取消点正落在 `ci-one-key` 的 runner 测试**编译/链接**阶段 ⇒ **被取消的那次构建留下了损坏的测试二进制**。**修法**：触碰 `record_types.rs` 的时间戳强制重新链接 ⇒ **12 / 12 绿**；随后**按纪律整轮重跑电池**（上表是重跑那一轮的结果）。

> **教训**：**不要在 `ci-one-key` 运行中取消电池** —— 被取消的构建会把损坏的二进制留在 `target/debug/deps/`，下一次运行就会在**看起来毫不相关的**测试里以原生崩溃的形式报红。判据：**原生崩溃（0xc0000005 / STATUS_ACCESS_VIOLATION）先怀疑构建产物，而不是先怀疑代码**。

## 5. 诚实性

### 5.1 本片**只测量，不实现**

- `selfhost/*.sico` **零改动**；oracle 生产代码（`crates/*`）**零改动**；**不推进任何出口门**。
- 产出是**规则表 + 子集边界 + 下一刀切点**，**不是能力**。
- 证据等级：全部 `internal-fixture`；运行时主张只来自**真实 runner**（本机 Windows x64 GNU + Wasmtime）。

### 5.2 未闭合的（承自本片）

- **语句位**（`let` / `set` / 单 `return`）仍 typed 拒绝 ⇒ S 内 **320** 个调用点里的绝大多数**还降不下来**。
- **嵌套调用实参**（2 个 S 调用点）仍 typed 拒绝 ⇒ T2 版的 G-f2 面。
- **组件层完全没闭合**：`sico.text.length` 不在 guest 发射器的支持子集内 ⇒ `--emit-component-hex` 仍 fail-closed 拒绝。
- **语料覆盖 0 条**：S 内 322 个 T2 调用点一个都没进语料。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- **M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）；**T1 族闭合不等于 M22 闭合**（STEP-0527 §5）。

### 5.3 下一刀（W2 族 T2 的第 2 拍 / 前端刀）

1. **前端刀（F）**：让**直线路由家族**（`straight_cell_function_ir`）认 `sico.text.length` —— 一个 `Text` 实参（形参 / 局部 cell）⇒ 一个 `U64` 结果；**纯插入、零既有行重排**；复用既有 `gw_intrinsic_call_packed` ⇒ **不新造机器**；新函数必须自证「经 guest 自己逐字节降低 == Rust 前端」（自举形状前沿钉）。**这一刀覆盖 S 内 320 / 322 个调用点。**
2. **codegen 刀（C）**：让 `cgs_intrinsic_hex` 认 `sico.text.length`（**四道闸门全 fail-closed**：名字 / 实参个数 = 1 / 实参类型 = `string` / 结果槽数 = 1），并**发射 helper 函数体**（§4.5 的 51 B）+ **调用点**（`local.get ×2; call; 0xad; local.set`）+ **两个内建 helper 的名字排序索引**。
3. **收口片（X）**：钉住 + 纠偏 + 前沿探针期望值翻转（5 条 T2 用例）+ 固件电池 + 独立 CI（提交后）+ 诚实收窄。

## 6. 验证记录（2026-10-24）

| 验证 | 口径 | 结果 |
|---|---|---|
| T2 固件钉（4 条） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1 text_length` | ✅ **4 passed / 0 failed**（另有 1 条既有前沿钉 `sico_compiler_lowers_gw_text_length_rhs_byte_exactly` 同批通过） |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | 见 §6.2 |
| 本机电池 | `target/tmp/step0528/battery.ps1`（**19 步** = STEP-0526 的 17 步原样复用 + `step0528-census-t2` + `step0528-doc-check`） | 见 §6.2 |
| 证据文件 | `target/tmp/step0528/`（12 组 fixture / oracle IR / oracle 组件 / guest 判读工件 + `measure-t2.txt` + core module 反汇编） | ✅ |

### 6.1 T2 真值速查

| 项 | 值 |
|---|---|
| canonical IR（条件位） | **2991 / 3446 / 3437 B** |
| canonical IR（语句位） | **2212 / 2212 / 2252 / 2212 / 2656 / 2369 / 2765 B** |
| oracle 生产组件（条件位） | **1167 / 1187 / 1187 B** |
| oracle 生产组件（语句位） | 1091 / 1091 / 1111 / 1091 / 1145 / 1570 / 1113 B |
| canonical intrinsic 形状 | `{"op":"intrinsic","data":{"name":"sico.text.length","arguments":[0]}}` |
| guest 前端判读 | 条件位 ✅ **逐字节等同** / 语句位 ❌ `ERR:E-SH-IR-EXPRESSION` |
| guest 发射端判读 | ❌ `unsupported codegen source shape`（fail-closed） |
| 错元数 | **Rust 前端自己拒** `stdlib intrinsic arity` |
| helper | core function `helper_count+3+名字排序位`；body **51 B**；数的是 **UTF-8 码点** |

### 6.2 电池

本片电池 = `target/tmp/step0528/battery.ps1`（**19 步** = STEP-0526 的 17 步原样复用 + `step0528-census-t2` + `step0528-doc-check`），日志 `target/tmp/step0528/battery.log`。

**首轮电池在第 1 步红了**（完整性白名单还停在 STEP-0526 的变更集，见 §4.8）⇒ 修正后**按纪律整轮重跑**；**第 2 轮又在 `ci-one-key` 红了**（`record_types` 测试 `0xc0000005`；根因是**我取消首轮电池时中断了 runner 测试链接、留下损坏的测试二进制**，见 §4.9）⇒ 触碰 `record_types.rs` 时间戳强制重链 ⇒ **第 3 轮整轮重跑** ⇒ **BATTERY_GREEN 19/19** ✅。

| 步骤 | 耗时 | 裁决 |
|---|---|---|
| `workspace-integrity` | 52s | ✅ `WORKSPACE_INTEGRITY_GREEN`（1841 tracked / 0 mismatch） |
| `fmt-runner` | 2s | ✅ |
| `ci-one-key` | 413s | ✅ `CI GREEN`（fmt / clippy / build / test workspace / test runner / 边界 / 注释纪律 / 规划合同 / 应用 profile / 跨主机矩阵 / whitespace 全 PASS） |
| `validate-0245` | 120s | ✅（`sico-semantics` 单元测试全过） |
| `validate-0262` | 139s | ✅ `STEP_0262_OK`（canary=FORMATTER-CLOSED） |
| `probe-frontend-frontier` | 303s | ✅ `FRONTIER_UNCHANGED`（**46** 条用例全部与期望一致） |
| `probe-ir-plan-frontier` | 25s | ✅ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` / `validate-0131` / `validate-0156` | 1s / 2s / 1s | ✅ 三项 `OK` |
| `git-diff-check` | 0s | ✅ |
| `line-ending-audit` | 0s | ✅ `LINE_ENDING_AUDIT_GREEN`（31 files） |
| `selfhost-comment-discipline` | 0s | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN`（files=2） |
| `step0526-footprint-audit` | 0s | ✅ `FOOTPRINT_GREEN` |
| `step0526-census-branches` | 0s | ✅ `CENSUS_GREEN`（S 内 4672 条分支语句） |
| `step0526-differential` | 83s | ✅ `DIFFERENTIAL_GREEN` |
| `step0528-census-t2` | 0s | ✅ 322 个 `sico.text.length(` 调用点 |
| `step0528-doc-check` | 0s | ✅ `DOC_CHECK_GREEN` |
| `workspace-integrity-final` | 51s | ✅ `WORKSPACE_INTEGRITY_GREEN`（0 mismatch） |

⇒ **BATTERY_GREEN 19/19**（第 3 轮，**含本片全部改动**）。电池之后的**纯文档**编辑（补记本裁决 + 回填 run #34）按 §8 只复跑 `git diff --check` 与 `doc_check.py`。

### 6.3 独立 CI 回填（上一批）

| 批次 | 提交 | run | 裁决 |
|---|---|---|---|
| STEP-0527 | `914197f` | #34 | ✅ **success**（run id `39997745000`，**本片回填**，实测自 GitHub Actions API） |
| **STEP-0528（本片）** | `25e752a` | **#35** | ✅ **success**（run id `40004257414`，head_sha `25e752ab765951bd9be157c8462095ed16b5a303`，**STEP-0529 回填**，实测自 GitHub Actions API） |

⇒ 按仓库既有做法，**每轮裁决由下一片的提交回填**，不为此再推一轮。**STEP-0529** 推送后触发新一轮，其裁决由**再下一片**回填。

### 6.4 未做的事

- **只量了 12 个变体**，不是 T2 形状的穷举（322 个调用点的分布已按「位置 × 实参形态」清点，但未逐条追到函数级）；
- **组件层未实现**（本片只测量，见 §5.2）；
- **语料未动**（语料是 M22 级的工作项）；
- **不推进任何出口门**，M22 仍为 NO-GO。
