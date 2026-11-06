# STEP-0526 m22-w2 S5 **发射端分支支持（G-d2）**：按块发射终结符 + `equal_fixed` 算子（G-d3）

> - status: **implementation complete locally**（本机电池 **BATTERY_GREEN 17/17**；固件 **153 / 153**；独立 CI 裁决待下一批回填）
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0525](./STEP-0525-m22-w2-s5-w2-t1-gf2-nested-call-argument.md)（G-f2 闭合）、[STEP-0523](./STEP-0523-m22-w2-s5-w2-t1-gd-single-return-route.md)（G-d1 与「G-d = 两个缺口」的判定）、[STEP-0521](./STEP-0521-m22-w2-s5-w2-t1-family-truth-table.md)（族级真值表 + D15 登记）
> - gate: **不推进任何出口门** —— 本片是 **M22 S5 发射端能力**（计划 §3.3.5「独立条目登记：G-d2」），**T1 族仍未闭合**（收口片 X 尚未开始）
> - numbering: **STEP-0526**（取号前已核对 `docs/steps/`：最大号 0525；*0526 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t1-text-concat.md)（D15，已更新）

## 0. 为什么这一片现在可以开（入口门槛）

计划 §3.3.5 的 **G-d2 独立条目**给了三个入口条件与一个出口判据：

| 入口条件 | 状态 |
|---|---|
| ① 多块发射的 IR 计划 | ✅ 本片落点：发射端按块循环，逐块发终结符（`cgs_function_instr_hex` 的 `terms` 标志） |
| ② Wasm 侧的控制流与分支值槽 | ✅ 条件值槽取自 `cgs_value_slot`，与后端 `layout.scalar` 同口径（§2.2） |
| ③ **先量 oracle** | ✅ §2：Rust 后端**生产组件**与 core module code 段反汇编（真值，不是设计） |
| **出口判据** | 某个 `if` 形状的**整组件与 Rust 后端逐字节等同** + 过真实 `wasmparser::Validator` + **自举形状前沿不推进** + 固件绿 + 独立 CI 绿 |

**归属不变**：G-d2 **不是** T1 族的一拍（它对**任何**用到分支的族都成立），本片排在 T1 收口片 X **之前**。

## 1. 这一片回答的问题

**guest 的发射器能不能把「函数体里有 `if`（有分支、有跳转、有多个返回）」的模块发射成与 Rust 后端**逐字节相同**的组件？**

## 2. 真值测量（生产输出）

测量口径：**Rust 后端 = `sico build --profile script-v0` 的生产组件**（与固件里的 `compile_script_program` 同一函数）；**Rust 前端 = `lower_core` + `canonical_json`**；**guest = `selfhost/compiler.sico`**（真燃料 800B，与固件同一 runner）。

### 2.1 形状表

| 形状 | Rust IR | Rust 组件 | guest（改前） | guest（改后） |
|---|---|---|---|---|
| **AA 行**：`if` 块里的 concat（族级真值表唯一一条组件层拒绝） | **3119** | **1198** | **组件层 typed 拒绝** | **1198 B，逐字节相同** |
| **S 真实形状 `cgs_zero_hex`**：`if` 块里的 concat + 一次调用实参 | **3753** | **1266** | **组件层 typed 拒绝** | **1266 B，逐字节相同** |
| **S 真实形状 `declaration_name`**：`if` 块里的两实参调用 + 嵌套 concat | **4387** | **1316** | **组件层 typed 拒绝** | **1316 B，逐字节相同** |
| `if` / `else`（两个分支 + 一条 `unreachable`） | 2853 | 1100 | 组件层 typed 拒绝 | **1100 B，逐字节相同** |
| `while` 循环（回边是 `jump`） | 3293 | 1116 | 组件层 typed 拒绝 | **1116 B，逐字节相同** |
| 一个函数体里两个 `if`（6 块、2 分支、2 跳转） | 3530 | 1178 | 组件层 typed 拒绝 | **1178 B，逐字节相同** |
| （既有 33 行真值表，回归） | — | — | 逐字节等同 | **逐字节等同（一个字节都没变）** |

⇒ **S 内 G-d2 的 4 个调用点（2 个在 `cgs_zero_hex`、2 个在 `declaration_name`）现在逐字节发射** ⇒ **G-d2 闭合**。

### 2.2 终结符的字节编码（从后端生产输出反汇编读出，不是设计）

后端 `compile_function`（`crates/sico-codegen-wasm/src/lib.rs:1678`）对**每个块**写：块头（`local.get disp; i32.const <id>; i32.eq; if`）+ 指令 + **终结符** + `end`；块循环之后是 `unreachable; end; end; unreachable; end`。

| 终结符 | 后端编码 | 字节 |
|---|---|---|
| `Return(v)` | 值的各槽 `local.get` + `return` | `20 <slot>`… + `0f` |
| `Branch{cond,then,else}` | `local.get cond; if; i32.const then; local.set disp; else; i32.const else; local.set disp; end; br 1` | `20 <c>` `0440` `41 <t>` `21 <d>` `05` `41 <e>` `21 <d>` `0b` `0c01` |
| `Jump(target)` | `i32.const target; local.set disp; br 1` | `41 <t>` `21 <d>` `0c01` |
| `Unreachable` | `unreachable` | `00` |
| `Match{…}` | （本片**未实现**，见 §6.1） | — |

**分支值槽**：`cgs_value_slot` 与后端 `layout.scalar` 同口径 —— 形参（不声明）→ 各指令结果（每槽一组）→ cells → dispatcher；guest 的 `cgs_value_slot` 是 **1 基**，落地时减 1（`cgs_function_return_hex` 原来就是这么做的，本片沿用）。

## 3. 落点（实施记录）

落点文件：**`selfhost/codegen.sico`**（**oracle 生产代码 `crates/*` 零改动**）。文件 **4413 → 4524 行（+111）**，函数数 **152 → 156（+4）**。diff：**+142 / −31**。

### 3.1 形状与改动

| 函数 | 动作 | 说明 |
|---|---|---|
| `cgs_function_instr_hex` | **重塑** | 多一个 `terms: U64` 标志；`terms == 1` 时逐块追加「终结符 + `0b`」 |
| `cgs_flat_return_hex` | **新增**（替换 `cgs_function_return_hex`） | 按**值**取平坦返回（原函数按**块 0**取，且遇 `branch` 直接拒） |
| `cgs_branch_hex` | **新增** | 分支终结符（§2.2 的第二行） |
| `cgs_jump_hex` | **新增** | 跳转终结符（§2.2 的第三行） |
| `cgs_block_terminator_hex` | **新增** | 终结符分派：`return` / `branch` / `jump` / `unreachable`，其余 typed 拒绝 |
| `cgs_equal_fixed_hex` | **新增** | `equal_fixed` 算子（§3.3） |
| `cgs_op_hex` | **加一支** | 分派 `equal_fixed` |
| `cgs_function_body_hex` | **改** | 去掉「块 0 的平坦返回」，尾从 `0b000b0b000b` 改成 `000b0b000b` |
| `cgs_run_body_hex` | **不变** | `run` 仍走结果区返回（§3.4） |

### 3.2 **单块函数的字节不变**（为什么既有 33 行真值表一个字节都没动）

改前：尾 = `0b000b0b000b`（第一个 `0b` 闭合**唯一**那个 `if`）。
改后：每块尾部自己写 `0b`，函数尾 = `000b0b000b`。
⇒ 单块时 `… + 0b + 000b0b000b` 与 `… + 0b000b0b000b` **完全相同** ⇒ **既有 33 行真值表逐字节不变**（实测）。

### 3.3 **G-d3（本片新登记）= 发射端缺 `equal_fixed` 算子**

AA 行的条件是 `U64.equal(U64.literal(1), U64.literal(1))`，Rust 前端把它降成定宽比较 `equal_fixed`：

```json
{"result":3,"ty":{"kind":"bool"},"operation":{"op":"equal_fixed","data":{"left":1,"right":2}}}
```

guest 的发射器**不认识这个算子** ⇒ 即使分支终结符到位，AA 行仍被拒。⇒ 本片补上它：后端 `emit_fixed_comparison`（相等形式）写的是「两个操作数槽 + `i64.eq` + 结果槽存储」，guest 写同样的字节（`20 <l> 20 <r> 51 21 <res>`）。

**为什么它是独立的一条**：它**不是控制流**缺口，是**算子覆盖**缺口 ⇒ 登记为 **G-d3**，与 G-d2 同片落地（同族 precedent：STEP-0522 同片闭合 G-b 与 G-e）。

### 3.4 **`run` 的多块仍 fail closed**（诚实收窄）

`run` 的返回走**结果区**（`cgs_result_return_hex`），那是**单块**形状 ⇒ `run` 多块仍被拒（typed 身份 `ERR:E-SH-CG-IR-EMIT`）。

⇒ 这条**不是**「不支持分支」：**分支与跳转都支持**，只是 `run` 的返回形态要求单块。固件新增 `sico_codegen_guest_refuses_the_branching_run_function` 把这条边界钉住（前端逐字节降下来 930 B，发射端带 typed 身份拒绝）。

## 4. 撞到并修掉的 2 条自造缺陷

| # | 症状 | 根因 | 修法 |
|---|---|---|---|
| 1 | 固件 `sico_compiler_emits_the_run_instruction_segment_byte_exactly` **红**：期望 182 B、实得 **350 B** | 第一版把终结符放进**共用的** `cgs_function_instr_hex`，于是 `run` 的**指令段**里长出了结果区返回与块尾 `end` ⇒ **破坏了 STEP-0502 的契约**（`--emit-run-instr-hex` = 「到结果区返回**之前**为止」） | 加一个 `terms: U64` 标志：`run` 路径要**纯指令段**（`terms = 0`），其它模块函数要**带终结符**（`terms = 1`）⇒ **一个循环、两条路径**；`run` 的字节回到原样 |
| 2 | 嵌套 `if` 的形状仍被拒 ⇒ 差点误判成「发射端还不够」 | 那条拒绝来自**前端**（`ERR:E-SH-IR-STATEMENT`），**不是**发射端 ⇒ 两类拒绝必须分开记 | 差分脚本加第三种判定 `frontend-refused`，把「嵌套 `if` = 前端缺口」与「分支 `run` = 发射端缺口」分开钉 |

⇒ 第 1 条的教训与 STEP-0525 §4 同类：**共享热点函数上的改动会同时改掉别人依赖的字节** ⇒ 必须按**调用方**分开，而不是按「谁需要」分开。

## 5. 自举形状前沿（**没有推进**）

本片新增/重塑的 **8** 个函数（`cgs_function_instr_hex` / `cgs_flat_return_hex` / `cgs_branch_hex` / `cgs_jump_hex` / `cgs_block_terminator_hex` / `cgs_equal_fixed_hex` / `cgs_function_body_hex` / `cgs_run_body_hex`）**全部**能被 guest 前端逐字节降低（探针 `target/tmp/step0525/shape_probe.py`，guest canonical IR 与 Rust oracle **逐字节相同**）⇒ 新钉 `sico_compiler_lowers_the_step0526_codegen_functions_byte_exactly`。

⇒ **自举形状前沿没有推进**：新形状是**复用**既有可降形状（`while` + `if` + 顶层 `return` 收尾）得到的。

## 6. 诚实性

### 6.1 **未闭合 / 未认领**

| 项 | 状态 |
|---|---|
| **`match` 终结符** | ⏳ **未实现**（Rust IR 的第 5 种终结符）⇒ 发射器遇到它 typed 拒绝；但 guest **前端**本来就拒 `match` 形状（`ERR:E-SH-IR-MATCH-SHAPE`），所以这条路径**当前不可达** |
| **嵌套 `if`** | ⏳ **前端**缺口（`ERR:E-SH-IR-STATEMENT`）⇒ **不是**发射端缺口；差分脚本按 `frontend-refused` 钉住 |
| **`run` 的多块** | ⏳ **发射端** fail closed（结果区返回是单块形状）⇒ 固件单独钉住 |
| **其它内建** | guest 发射端只认 `sico.text.concat` 与 `sico.bytes.length` 两个内建 ⇒ 其它 `sico.*` 调用属另一个闸门 |
| **整文件自举** | ⏔ 仍**未**闭合（`List[Text]` 形参等形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）—— 本片**没有**推进它 |
| **语料覆盖** | ⏔ 仍 **0 条** |

### 6.2 **S 内那 4 个调用点现在能发射吗？**

**能** —— `cgs_zero_hex` 与 `declaration_name` 两个形状现在与后端**逐字节相同**（1266 B / 1316 B，§2.1）⇒ **D15 的 G-d2 行可以翻成「已闭合」**。

⇒ 但**族级固定点（guest 能编译整个 S）仍未达成**：S 里还有别的形状（宿主调用内建、`List[Text]` 形参、cell 的记录类型等）落在别的缺口上 ⇒ **T1 族不能在本片收口**，收口片 X 仍需要。

## 7. 出口

| 判据 | 状态 |
|---|---|
| 某个 `if` 形状的**整组件与 Rust 后端逐字节等同** | ✅ 6 个形状（1198 / 1266 / 1316 / 1100 / 1116 / 1178 B，0 字节差异） |
| 过真实 `wasmparser::Validator` | ✅ oracle 与 guest 两侧都过（固件逐行验证） |
| **自举形状前沿不推进** | ✅ 8 个函数逐字节降低（新钉） |
| 固件绿 | ✅ **153 passed / 0 failed**（150 → 153：前沿钉 + 分支 `run` 拒绝钉 + S 内两个分支形状的逐字节钉）；本机电池 **BATTERY_GREEN 17/17**（`target/tmp/step0526/battery.ps1`） |
| 独立 CI 绿 | ⏳ 本片推送后触发下一轮，回填在收口片 X（上一批 **run #32 = success ✅** 已回填） |
