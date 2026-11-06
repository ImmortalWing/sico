# STEP-0507 m22-w2 S5 多函数模块：组件级派生（**已实现，本机全绿**）

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0506](./STEP-0506-m22-w2-s5-error-arm-component.md)（`error` 臂整组件）、[STEP-0504](./STEP-0504-m22-w2-s5-derived-component.md)（派生组件）、[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位计划）
> - gate: 门 3 的 S5 波次 = guest IR 摄取 + run 体发射 —— **本片是发射侧的第七刀（多函数模块）**
> - numbering: M22 线顺延为 **STEP-0507**（本线已占用 STEP-0491..0499、STEP-0501..0506；已核对 `docs/steps/` 与并行 worktree `E:\github\sico-m24` 占用的 STEP-0330–0337，**均无 0507**）

## 0. 为什么这一片现在可以开（入口门槛）

按仓库纪律「**不为让路线图看起来有排期而占用未来 STEP**；入口门槛满足且有有界出口测试时才取号**」：

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ 见 §1、§2（组件解剖 + 局部布局规则 + 返回形式 + 调用 IR 形状） |
| 有**有界出口测试** | ✅ 见 §3（一个双函数 fixture 的**整组件逐字节等同**） |
| 前端是否需要刀已判定 | ✅ 见 §4（双函数模块的 IR **已经**逐字节等同；只有「表达式位置的调用」被拒） |

⇒ 取号合法。**实现已完成**，见 §7。

## 1. fixture 与真值

### 1.1 fixture（两个源函数；helper 未被调用）

```
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

function helper(value: I64) returns I64:
  return value
end function

function run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(7)))
end function
```

### 1.2 Rust IR（backend 生产输出）

- `functions` = **[helper (id 1), run (id 2)]** —— **模块声明顺序**，`run` **不是**第一个（`run` 的位置由名称决定）。
- `helper`：1 个参数（`value: I64`），返回类型 `i64`，单个 block，terminator = `return 0`（返回参数本身）。
- `run`：`project` × 2、`const_i64`、`construct ScriptOutput`、`variant ok`。

### 1.3 组件解剖（oracle `compile_script_program` 的生产输出 = **1017 B**）

| 段 | 多函数（实测） | 单函数（参考形态） |
|---|---|---|
| 核心模块 | **574 B** | 488 B |
| 段号集合 | **{1,3,5,6,7,10}**（本 fixture 无字面量 ⇒ **无 data 段**） | {1,3,5,6,7,10,11} |
| types 载荷 | **31 B** | 26 B |
| functions 载荷 | **6 B** | 5 B |
| exports 载荷 | 47 B | 47 B |
| code 载荷 | **441 B** | 349 B |

**types 载荷（实测 hex）**

```
05 60 017e 017e  60 04 7f7f7f7f 017f  60 017f 017f  60 04 7f7f7f7f 017f  60 017f 00
```

= **5 个类型**，顺序 = **源函数（模块顺序）→ 三个冻结 helper**：

| # | 类型 | 来源 |
|---|---|---|
| 0 | `(i64) -> (i64)` | **派生**：helper 的参数 flat + 返回类型 flat |
| 1 | `(i32,i32,i32,i32) -> (i32)` | **冻结**：Script ABI 的 `run`（结果经返回区 ⇒ 1 个 i32 指针） |
| 2 | `(i32) -> (i32)` | 冻结：`alloc` |
| 3 | `(i32,i32,i32,i32) -> (i32)` | 冻结：`cabi_realloc` |
| 4 | `(i32) -> ()` | 冻结：`cabi_post_run` |

**functions 载荷（实测 hex）** = `05 00 01 02 03 04` ⇒ **5 个函数**，类型索引 = 源函数顺序（0,1）+ 三个冻结 helper（2,3,4）。

**exports 载荷（实测 hex）**

```
04 06 6d656d6f7279 02 00 | 03 72756e 00 01 | 0c 636162695f7265616c6c6f63 00 03 | 0d 636162695f706f73745f72756e 00 04
```

⇒ 导出**数量不变（4）**，但**函数索引整体平移**：冻结段里 `"run" → 00` 变成 **`"run" → 01`**（run 在核心函数表里的位置），`cabi_realloc → 02` 变 **`03`**、`cabi_post_run → 03` 变 **`04`**。

**code 载荷（实测）** = **5 个函数体**：`helper`（29 B）、`run`、`alloc`、`realloc`、`post`（后三个**逐字节等于**冻结常量 ✓）。

### 1.4 局部布局规则（backend `LocalLayout::plan`，已测得）

对 **Script profile 的任意函数**都成立：

1. **参数**占用 wasm 参数位（**不写进局部声明**）；
2. **每条指令结果的每个 slot 单独一组**（`01 7f` / `01 7e`），按 IR 顺序；
3. 之后是 **dispatcher**（`01 7f`）；
4. 之后是 **Script scratch 对**（`02 7f`）。

⇒ `helper`（0 条指令）的局部声明 = `02` + `017f` + `027f`（**2 组 / 3 槽**）—— 与实测逐字节一致。
⇒ guest 现有的 `cgs_local_hex` 已经实现了这条规则，只是**写死了 `run`**（按函数 id 取指令结果 + dispatcher + scratch）⇒ 泛化成「按函数 id 取」即可。

### 1.5 非 `run` 函数的返回形式（backend `emit_return`）

`CoreAbi::Direct` 且返回类型**不是** checked fixed `Result` 时 = **逐 slot `local.get` + `return`**：

```
helper 的 block 0 尾部： 0440 2000 0f      # if(void) { local.get 0 ; return }
```

⇒ guest 需要一条**通用的「直接返回」**发射路径（`run` 的结果区返回是**另一条**路径，不能混用）。

### 1.6 通用函数体的骨架（实测）

与 `run` 的 dispatcher 骨架**同构**：`i32.const <entry>; local.set <dispatcher>; block(void) loop(void) local.get <dispatcher>; i32.const <block id>; i32.eq; if(void) { <block 体（含 terminator）> }`，随后固定尾声 `0b 00 0b 0b 00 0b`。

### 1.7 分配器的核心索引（实测，本片新增的派生点）

backend 的 `ScriptEmit`：

```rust
let helper_count = import_count + u32::try_from(module.functions.len())?;
alloc_index: helper_count, realloc_index: helper_count + 1, post_return_index: helper_count + 2,
```

Script profile 的核心模块**没有 import**（`import_count = 0`）⇒ **分配器的核心函数索引 = 模块函数数**：单函数模块 = **1**，双函数模块 = **2**。

⇒ guest 原先的**冻结常量** `cgs_frozen_alloc_index() = 1` 只描述单函数模块；本片把它改成**派生**。

## 2. 调用形状的 IR 真值（供「表达式位置的调用」那一刀用）

实测（`helper(I64.literal(7))` 出现在 `ScriptOutput` 的 `exit_code` 字段值位置）：

```json
{"result": 4, "ty": {"kind": "i64"}, "operation": {"op": "call", "data": {"function": 1, "arguments": [3]}}, "range": {"start": 600, "end": 622}}
```

| 字段 | 规则 |
|---|---|
| `function` | 被调者的**声明序号**（= IR 的 function id） |
| `arguments` | 实参的 **value id**；实参指令**先发射**、id **更小**（3 < 4） |
| `ty` | 被调者的**声明返回类型**（这里是 `i64`） |
| `range` | 从被调者名字起点到右括号 `)` 之后（`600..622` = `helper(I64.literal(7))`） |

**id 记账约束（实现要点）**：guest 现在的 construct 降级假设「**一个字段值 = 一个 result id**」；调用形状会吃掉 **1 + 实参个数** 个 id ⇒ 调用方必须相应推进 id，否则字段引用会指向错误的值。

## 3. 出口测试（有界）

1. **固件钉（组件级）**：guest 对 §1.1 fixture 发射的**整个组件**与 oracle `compile_script_program` 的生产输出**逐字节相同**（含 `wasmparser::Validator` 校验）；组件长度应为 **1017 B**。⇒ `sico_codegen_guest_emits_the_multi_function_component_byte_exactly`。**两个 fixture**：helper 参数为**单槽**（`I64`）与**多槽**（`Text` ⇒ 2 个 Wasm slot）各一个 ⇒ 派生的核心类型覆盖 1 个与 2 个 flat group。
2. **回归**：STEP-0506 的两个钉（`error` 臂 939 B、ok 臂参考形态）**仍绿** ⇒ 单函数路径未被泛化改坏。
3. **fail-closed**：形状超出本片支持的（例如非 `run` 函数返回 checked result）⇒ **typed 拒绝**（`ERR:E-SH-CG-IR-PLAN` 或 `ERR:E-SH-CG-IR-EMIT`），**不得**静默回退到单函数骨架。⇒ `sico_codegen_guest_refuses_the_component_emission_outside_the_subset`（**语义已随本片更新**：多函数不再是拒绝对象）。
4. **自举形状前沿**：本片新增/改写的 guest 代码必须能被 **guest 自己**逐字节降低 ⇒ `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`。

## 4. 已测得的前端事实（决定本片要不要前端刀）

| 形状 | guest 前端 | 证据 |
|---|---|---|
| **双函数模块**（helper 未被调用） | ✅ **IR 逐字节等同** Rust IR（2069 B，`equal=true`） | 不需要前端刀 |
| **构造字段值 = 调用**（`exit_code: helper(I64.literal(7))`） | ❌ `ERR:E-SH-IR-CONSTRUCT-VALUE` | construct 的字段值形状表目前只有：字符串字面量、字段投影、枚举 case、`I64/U64` 字面量 |
| **语句位置的调用** | ⚠️ 已有路径，但返回类型**写死** `{"kind":"u64"}` | 类型推断缺口 |

⇒ **本片（组件级派生）不需要前端刀**；「表达式位置的调用」是**下一刀**（STEP-0508 候选），它需要解决 §2 的三件事：调用形状的识别与实参降级、**被调者返回类型的解析**（从源码声明取）、以及 id 记账。

## 5. 实施提纲（已完成）

1. `cgs_function_at_index` / `cgs_function_count`：按**模块顺序**取函数（`run` 不在首位 ⇒ 一切索引都按位置算，不按 id 算）。✔
2. `cgs_function_local_hex(bytes, fn_id)`：把 `cgs_local_hex` 泛化成「按函数 id」——规则不变（§1.4）。✔
3. `cgs_function_instr_hex(bytes, fn_id)`：把 `cgs_run_instr_hex` 泛化成「按函数 id」。✔
4. `cgs_function_return_hex(bytes, fn_id)`：通用**直接返回**（逐 slot `local.get` + `return`），与 `run` 的结果区返回**分开**。✔
5. `cgs_core_type_hex(bytes, fn_id)`：`(参数 flat) -> (返回类型 flat)`；`run` 用**冻结**的 Script ABI 类型。✔
6. `cgs_core_types_hex` / `cgs_core_funcs_hex` / `cgs_core_exports_hex` / `cgs_core_code_hex`：按 §1.3 派生。✔
7. `cg_core_module_hex`：改用派生段；`cgs_data_hex` 的收集顺序扩到「模块内函数顺序 → 块顺序 → 指令顺序」（backend 的 `script_data` 规则）。✔
8. **自举形状前沿**：新代码必须能被 guest 自己降低（实测规则见 §7.3）。✔
9. **派生 vs 冻结的分界**：组件骨架（WIT 记录、三个 helper 体、内存、globals 上限）继续冻结；**函数表、类型表、导出索引、data 段、分配器核心索引**改为派生。✔

## 6. 诚实性

- 本记录里的每一个数字都来自 **backend 的生产输出**（`compile_script_program`），不是设计文档，也不是 `sico check` 的通过。
- **oracle 生产代码零改动**。
- 测量用的临时探针**已从测试文件移除**；工件留在 `target/tmp/step0507/`（未跟踪目录）。
- **本片只覆盖一个 fixture**（双函数、helper 未被调用）；见 §8 的收窄清单。
- M22 整体仍为 **NO-GO**（门 3 未动）；独立 CI 尚未运行；未提交未推送。

## 7. 实施记录（2026-10-22）

### 7.1 codegen 侧的派生（`selfhost/codegen.sico`）

| 动作 | 内容 |
|---|---|
| **删掉冻结段常量** | `cg_core_types_hex` / `cg_core_funcs_hex` / `cg_core_exports_hex`（三张单函数冻结表）被派生版取代 |
| **新增派生函数** | `cgs_function_at_index` / `cgs_function_count` / `cgs_run_index`（1-based）/ `cgs_flat_valtypes` / `cgs_core_type_hex` / `cgs_core_types_hex` / `cgs_core_funcs_hex` / `cgs_core_exports_hex` / `cgs_function_return_hex` / `cgs_function_body_hex` / `cgs_core_code_hex` |
| **泛化既有函数** | `cgs_local_hex` → `cgs_function_local_hex(bytes, fn)`；`cgs_run_instr_hex` → `cgs_function_instr_hex(bytes, fn)`；`cgs_data_hex` 改为**跨全部模块函数**收集字面量 |
| **分配器核心索引派生** | `cgs_frozen_alloc_index()`（冻结 1）→ `cgs_alloc_index(bytes)` = **模块函数数**（§1.7）；`cgs_frozen_alloc_call_hex()` → `cgs_alloc_call_hex(index)`；索引一路线程化到 `cgs_result_list_hex`（新增形参）、`cgs_result_field_hex`、`cgs_result_error_field_hex`、`cgs_result_return_hex`（ok/error 两臂） |
| **失败模式** | `cgs_core_type_hex` 对 `result` / `option` 返回类型返回 `""` ⇒ 上层 fail-closed ⇒ **typed 拒绝**；`cgs_function_local_hex` 对带**声明局部**的函数返回 `ERR:E-SH-CG-IR-FLAT`；`cgs_function_return_hex` 对 **branch 终结**（多 block）返回 `ERR:E-SH-CG-IR-PLAN` |

### 7.2 自举形状前沿的实测规则（下一刀直接用）

用「同签名桩」把 `codegen.sico` 的单个函数隔离出来，交给 **guest 前端**降低（前端无法解析未声明的被调者），与 Rust 前端的 canonical IR 逐字节比对。实测得到的**拒绝形状**：

| 形状 | guest 前端的身份 |
|---|---|
| `return <user call>`（调用直接做 return 表达式） | `ERR:E-SH-IR-CALL-ARGUMENT` → **【STEP-0510 实测纠正（2026-10-22）】本行过宽**：这个形状 guest 前端**能**逐字节降低——本片自己的固件钉 `cgs_alloc_index`（`return cgs_function_count(bytes)`）就是它，**至今仍绿**；下表的 `ERR:E-SH-IR-CALL-ARGUMENT` 实际对应的是**实参本身是调用**的那一类，不是「调用做 return 表达式」 |
| `sico.text.concat(<literal>, <user call>)` 直接做 return 表达式 | `ERR:E-SH-IR-EXPRESSION` |
| call 的**实参是另一个 call** | `ERR:E-SH-IR-CALL-ARITY` |
| `match <host call>`（两臂 `ok`/`error`） | `ERR:E-SH-IR-CALL-SHAPE` |
| `let x = <host call>(<host call>(...))`（host 套 host） | `ERR:E-SH-IR-GWPACK-OTHER` |

⇒ **可用的规避写法**：一律**先过 cell**——`let` 取值 → `set` 拼接 → `return` cell；实参先落进 cell 再传。

据此本片把 **4 个新函数 + 4 个既有函数**改写成可降的形状（**发射字节不变**，由组件钉保证）：

| 函数 | 改写 |
|---|---|
| `cgs_alloc_call_hex` | `return sico.text.concat("10", cg_leb_hex(x))` ⇒ `let` + `set` + `return` |
| `cgs_flat_valtypes` | `match sico.bytes.slice(...)` ⇒ `cgs_flat_slot_kind` + 显式 `i32`/`i64` 映射（更严格：未知组 ⇒ `""` fail-closed） |
| `cgs_function_body_hex` / `cgs_core_code_hex` | `cg_leb_hex(U64.shr(sico.text.length(...)))` ⇒ 拆成 `len` / `shr` / `hex` 三个 cell |
| `cgs_local_hex` / `cgs_run_instr_hex` | `return <user call>` ⇒ 先过 cell |
| `cg_run_body_hex_from_ir` / `cg_core_globals_hex` | 同上（host 套 host / return 表达式） |

### 7.3 固件钉（新增 / 改写）

| 钉 | 内容 | 状态 |
|---|---|---|
| `sico_codegen_guest_emits_the_multi_function_component_byte_exactly` | **整组件**与 oracle 生产输出逐字节相同（**单槽 + 多槽两个 fixture**）+ `wasmparser::Validator` 通过 | ✅ 新增 |
| `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly` | **21 个**本片新增/改写的 codegen 函数（+ 4 个既有改写函数）逐个被 guest 前端**逐字节**降低（同签名桩隔离） | ✅ 新增 |
| `sico_codegen_guest_refuses_the_component_emission_outside_the_subset` | **语义更新**：多函数不再是拒绝对象；保留「非 Script profile」与「非 `run` 函数返回 checked result」两个 **typed 拒绝** | ✅ 改写 |
| STEP-0504 / STEP-0506 的三个组件钉 | 单函数路径（参考形态 / `error` 臂 / 派生形态）仍逐字节相同 | ✅ 回归绿 |

**固件全量：108 passed / 0 failed**（`--test-threads=1`，`SICO_SELFHOST_COMPONENT_CACHE=0`）。

### 7.4 交付前清理

- 删除临时诊断模式 `--emit-parts`（`selfhost/compiler.sico`）与临时探针 `probe_step0507_parts`（测试文件）。
- 临时工件只在 `target/tmp/step0507/`（未跟踪）：`closure.py` / `stub_prefix.py` / `probe/*.sico`（前沿测量用）。

### 7.5 本机验证电池（11 步）

首轮：**9/11 绿**，2 步红。两步红都是**环境敏感项**（非代码缺陷）：

| 步骤 | 红因 | 证据 |
|---|---|---|
| `ci-one-key` | `prepared_program_reruns_are_isolated_and_survive_a_trap` 的 **20 ms warm-median 预算断言**（实测 **20.626 ms**）。该断言**自己**就是 M19 计划 §3.1 登记的两条 **env-sensitive** 断言之一（源码注释写明「fails under heavy parallel load, passes solo」）；本片只改 selfhost 源码与测试文件，**不触碰 runner 热路径** | **单跑通过**：warm median **18.625 ms** < 20 ms；CI 其余 **11 步全 PASS** |
| `validate-0245` | `Launch("页面文件太小，无法完成操作。 (os error 1455)")` = 宿主 **Windows 提交限额**耗尽（当时一个 python 进程常驻 **5.7 GB**、空闲物理内存仅 **1.7 GB**） | **单跑该测试通过**；`RUST_TEST_THREADS=1`（**只降并发，不改断言、不改验证脚本**）下 `validate-step-0245.ps1` **exit=0**（内含固件 **108 passed** + clippy） |

⇒ **电池 11/11**（红项均以「降低并发 + 单跑」复跑确认）。

### 7.6 过程中发现并修掉的**自造缺陷**（记录在案）

1. **函数表类型索引偏移一**：派生函数表时把 helper 的类型索引写成 `count+1/2/3`（应为连续的 `0..count+2`）⇒ 单函数路径的 `funcs` 段从 `0400010203` 变成别的 ⇒ 由单函数组件钉当场暴露并修正。
2. **分配器核心索引仍是冻结常量**：`cgs_frozen_alloc_index()` 恒为 1，而 backend 的 `alloc_index = 模块函数数` ⇒ 双函数组件差 **1 字节**（guest `call 1` vs oracle `call 2`）⇒ 改为派生后转绿。
3. **`cgs_flat_valtypes` 的组索引错位**：改写为「按 group 取 valtype」时把**字节偏移**（0,4,8…）当成了**组号**（0,1,2…）传给 `cgs_flat_slot_kind`（后者内部再乘 4）⇒ 单组 fixture 看不出来，**双组（`Text` 参数）会落到范围外 ⇒ fail-closed**。**补测**：新增一个**多槽参数**的 fixture（`helper(text: Text) returns Text`）⇒ 该缺陷当场暴露并修正（改为按组号迭代 + 消费字节数校验）。**教训**：派生规则必须用**能跨过边界**的 fixture 覆盖，不能只用最小 fixture 验收。


## 8. 诚实收窄（未关闭的缺口）

1. **只覆盖一个 fixture**：双函数、helper **未被调用**；「**表达式位置的调用**」仍未实现（前端 `ERR:E-SH-IR-CONSTRUCT-VALUE`）。
2. **checked result 的通用函数**：非 `run` 函数返回 `Result[...]` / `Option[...]` ⇒ **typed 拒绝**（packed 的 checked-result 返回形式未实现）。
3. **多 block 的通用函数**未支持（terminator 为 branch ⇒ `ERR:E-SH-CG-IR-PLAN`）。
4. **整文件自举仍未关闭**：guest 前端还不能整体降低 `codegen.sico`（`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）⇒ 前沿按**函数**推进，不是整文件。
5. **组件里仍有冻结骨架**：WIT 记录、导入/别名/导出段、三个 helper 体、内存与 globals 上限仍是冻结常量。
6. **门 3 仍 NO-GO**；独立 CI 尚未运行；未提交未推送。
