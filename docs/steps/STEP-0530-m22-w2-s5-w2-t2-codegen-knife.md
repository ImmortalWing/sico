# STEP-0530 m22-w2 S5 **W2 族 T2 codegen 刀（C）**：`sico.text.length` 的**组件层**逐字节下来了

> - status: **本机实测绿（随本片提交并推送；独立 CI 裁决待下一批回填）**
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0528](./STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（上一片 · **T2 拍 1 真值测量**）、[STEP-0529](./STEP-0529-m22-w2-s5-w2-t2-frontend-knife.md)（上一片 · **T2 拍 2 前端刀 F**）、[STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 拍 1 的方法学先例）、[STEP-0525](./STEP-0525-m22-w2-s5-w2-t1-call-arg-nested-call.md)（G-f2 刀）
> - gate: **不推进任何出口门** —— W2 的**第 3 拍（codegen 刀 C）**，按计划 §3.3.5 的「每族四拍」模板执行；**oracle 生产代码（`crates/*`）零改动**
> - numbering: **STEP-0530**（取号前已核对 `docs/steps/`：最大号 0529；*0530 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表](../reports/m22-support-surface-t2-text-length.md)（D15 风格，**族仍未闭合**）

## 0. 入口门槛

| 入口条件 | 状态 |
|---|---|
| T2 族的**拍 2（前端刀 F）**已完成（STEP-0529）⇒ 允许开**拍 3（codegen 刀 C）** | ✅ |
| 上一刀**只登记缺口、不预支能力**：STEP-0529 §7.3 明确写下「下一刀 = 让 `cgs_intrinsic_hex` 认 `sico.text.length`（**四道闸门全 fail-closed**）」 | ✅ |
| 出口测试有界 | ✅ §2：2 条新钉 + 1 条改钉 + 前沿探针 50 → **51** 条用例 |
| oracle 生产代码零改动 | ✅ §4.1 |

⇒ **取号合法**。*实现已完成并本地实测绿*，见 §5。

> **口径**：计划 §3.3.5 的族表记 **318** 个 `sico.text.length(` 调用点，STEP-0528 实测 **322**（S 在计划成文后的增长）⇒ 本片沿用 **322** 这个实测口径。

## 1. 这一片回答的问题

STEP-0529 之后的状态是：**前端能把 `sico.text.length` 逐字节降下来（条件位 3 + 语句位 5），但组件层整个族仍是 typed 拒绝**——guest 发射器 `cgs_intrinsic_hex` 只认 `sico.bytes.length` 与 `sico.text.concat` 两个算子。

**本片要回答的问题**：

1. 能不能**只做接线**（不新造机器）就让 guest 发射器认识 `sico.text.length`，并让**组件**逐字节等同 oracle 的生产组件？
2. 发射端认领这个算子之后，**helper 的索引规则**到底怎么算（两个内建 helper 共存时）？
3. 认领之后，**guest 发射端剩下的拒绝**里，哪些还是 T2 的缺口，哪些**根本不是**？

**本片的结论（先说结果）**：

- **能**。本片**零新造机器**：复用既有 `cgs_intrinsic_hex` / `cgs_core_types_hex` / `cgs_core_funcs_hex` / `cgs_core_code_hex`，新增 **1 个函数 + 4 个探针 + 1 个分支 + 6 处接线**，**+204 / −0 行**（纯插入，既有行**一行不重排** ⇒ T1 族的字节等同性不受影响）。
- **helper 索引规则（实测，不是设计）**：helper 的 core function 索引 = `模块函数数 + 3 + 名字排序位`；两个内建 helper 按**点分名**排序 ⇒ `sico.text.concat` 恒为 0、`sico.text.length` 为 1 ⇒ **只用 length 时 = `函数数+3`，concat 与 length 同模块时 = `函数数+4`**。
- **组件逐字节等同 10 个形状**（见 §2 #1）；**剩下的拒绝逐条归因**：**只有 1 条还是 T2 的缺口**（T2-g2 嵌套调用实参，**在前端层**，留给下一刀），其余全部是**与 T2 无关的既有窄一格**（`while` 循环形状、`U64.bit_or`/`U64.shr`），已用**对照形状**钉住（§4.4）。

## 2. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 | 状态 |
|---|---|---|---|
| 1 | `sico_codegen_guest_emits_the_text_length_component_byte_exactly`（**新**，替代 STEP-0528 的「组件层拒绝」钉） | **10 个形状**的组件**逐字节** == Rust 后端生产组件（1167 / 1183 / 1091 / 1091 / 1113 / 1197 / 1139 / 1091 / 1111 / 1187 B），且**两侧都过真实 `wasmparser::Validator`** | ✅ 新增 |
| 2 | `sico_codegen_guest_refuses_the_loop_shape_outside_the_text_length_subset`（**新**） | **对照形状**：不含 length 的 `while` 体（1142 B）**同样被 guest 发射端拒** ⇒ 该拒绝属**`while` 循环形状**，不属 T2；含 length 的循环体（1209 B）与 `while` 条件位（1187 B）亦拒 | ✅ 新增 |
| 3 | `the_guest_frontend_narrower_than_the_oracle_on_the_u64_bit_operators`（**新**） | **对照形状**：不含 length 的 `U64.bit_or` / `U64.shr`（各 1039 B，oracle 发射得出）被 guest **前端**拒 `ERR:E-SH-IR-EXPRESSION` ⇒ 属**前端窄一格**，不属 T2 | ✅ 新增 |
| 4 | `sico_compiler_keeps_the_nested_text_length_arguments_typed_refused`（**既有**） | **2 个嵌套调用实参形状**仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`（**身份不变**）⇒ **T2-g2 仍未闭合** | ✅ 保持 |
| 5 | `sico_guest_arena_accommodates_full_parser_selfhost_input`（**既有**） | **整 `parser.sico` 自举**（1 MB 输入）仍绿 | ✅ 保持 |

⇒ 固件从 **159** 增至 **161** 条；前沿探针从 **50** 增至 **51** 条用例（**1 条新增**：把 T2-g2 的边界钉到 **return 位**的嵌套调用实参；**0 条期望翻转**——本片只动发射端，不动前端；实测 `FRONTIER_UNCHANGED`）。

## 3. 实施设计

### 3.1 「接线」而不是「新机器」

`cgs_intrinsic_hex` 已经有 `sico.bytes.length` 与 `sico.text.concat` 两个分支，各自带**四道 fail-closed 闸门**（名字 / 实参个数 / 实参类型 / 结果槽数）。本片**照抄这套形状**加第三个分支，不引入任何新的抽象：

| 闸门 | `sico.text.length` 的取值 | 不满足时 |
|---|---|---|
| 名字 | `sico.text.length` | 落到函数末尾的 `ERR:E-SH-CG-IR-EMIT` |
| 实参个数 | **恰好 1**（第 2 个实参必须不存在） | `ERR:E-SH-CG-IR-EMIT` |
| 实参类型 | 必须是 `string`（canonical `Text`） | `ERR:E-SH-CG-IR-EMIT` |
| 结果槽数 | **恰好 1**（`U64`） | `ERR:E-SH-CG-IR-EMIT` |

### 3.2 组件层的三处接线（与 concat 家族同构）

| 段 | concat 家族 | **length 家族（本片新增）** |
|---|---|---|
| type 段 | `60047f7f7f7f027f7f`（`(i32,i32,i32,i32)->i32`） | **`60027f7f017f`**（`(i32,i32)->i32`） |
| func 段 | 计数 +1 | **计数 +1** |
| code 段 | helper body 追加在 arena helper 之后 | **helper body 追加在 concat helper 之后** |

**探针镜像**：concat 家族有 `cgs_instruction_is_text_concat` / `cgs_block_has_text_concat` / `cgs_function_has_text_concat` / `cgs_module_has_text_concat` 四个探针；本片**逐字镜像**出四个 `…_text_length` 探针（**不改共用路径** ⇒ 不动 concat 的既有行为）。

### 3.3 helper 索引规则（**实测**，非设计）

```
helper_index = 模块函数数 + 3 + (模块里有 sico.text.concat ? 1 : 0)
```

- `+ 3` = 三个 arena helper（`alloc` / `realloc` / `post`）；
- `+ 名字排序位` = 两个内建 helper 按**点分名**排序（`sico.text.concat` < `sico.text.length`）；
- 实测：只用 length 的模块（2 个模块函数）⇒ helper = **5**；concat + length 同模块（2 个模块函数）⇒ concat = **5**、length = **6**。

### 3.4 调用点与 helper body（**实测**）

- **调用点**：`local.get <ptr>; local.get <len>; call <helper>; i64.extend_i32_u (0xad); local.set <result>`（`Text` 的扁平整数槽位是 **(pointer, length)** 对；结果 1 槽 `U64`）。
- **helper body**：**54 B** = locals 声明 `01 02 7f`（**1 组、2 个 i32**）+ **51 B** 代码（数的是 **UTF-8 码点**，非续字节）。

### 3.5 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/codegen.sico` | **+204 / −0 行**（文件 4523 → **4727** 行，函数数 156 → **161**）：**新增 1 个函数**（`cg_text_length_helper_body_hex` :173）+ **4 个探针**（:2850 / :2876 / :2901 / :2930）+ **1 个分支**（`cgs_intrinsic_hex` :3089–3146）+ **6 处接线**（type 段计数与条目、func 段计数、code 段计数与 helper body） |
| `runner/sico-runner/tests/selfhost_compiler.rs` | **+176 / −32 行**：1 条新钉（逐字节等同，**10** 形状）+ 2 条新钉（归因对照）+ 1 条改钉（组件层拒绝 ⇒ 组件层逐字节等同） |
| `tools/probe-m22-frontend-frontier.ps1` | **+10 / −0 行**：用例 **50 → 51**（1 条新增） |
| oracle 生产代码（`crates/*`） | **零改动** |

> **纯插入**：新增代码一律**追加**，**既有行不重排** ⇒ T1 族（`sico.text.concat`）的字节等同性**不受本片影响**。

## 4. 实施记录（2026-10-24）

### 4.1 第一轮实现：组件差 **3 字节**

首轮发射后，guest 的组件与 oracle 的**逐字节比对差 3 字节**（guest 1164 B vs oracle 1167 B）。分段比对（type / func / code 三段各自比对）把差值定位到 **code 段的 helper body**：

- guest 的 helper body 开头是 `04 4020030f0b…`（**51 B 的代码**）；
- oracle 的是 `03 40200220014f…`（**54 B**）。

**根因**：**wasm 的函数体 = locals 声明 + 代码**。oracle 的 helper body 是 `01 02 7f`（**1 组、2 个 i32** 的 locals 声明）+ 51 B 代码；首轮实现只发了**代码**，**漏了 locals 声明**。

**修法**：把 helper body 的**取值来源**从「手抄代码字节」改成「**从实测的生产组件里读出整个函数体**」（含 locals 组），并加两条断言（**长度 = 54 B**、**以 `01 02 7f` 开头**）⇒ 字节**不可能再与真值漂移**。

> **教训**：**helper body 不许手抄**。STEP-0528 §4.5 只记了「body **51 B**」（那是**代码**的长度，不是函数体的长度）——本片第一次落地就踩了这个坑。**函数体 = locals 组 + 代码**，两者都要算。

### 4.2 真值与形状集（**全部来自 oracle 的生产输出**）

组件真值走 **Rust 后端的生产路径** `compile_script_program`（**不走 CLI**：`crates/sico-cli/src/lib.rs:675` 的 script-v0 profile 要求 `ScriptErrorCode` 恰为 `["InvalidInput","ResourceLimit","DomainError","Cancelled"]` 且入口名为 `main`，与 Script-profile fixture 的 ABI 不同 ⇒ CLI 能量不了本族 fixture）。

**形状集**（`target/tmp/step0530/shapes.txt`，**Rust 探针与 Python 比对器共用同一份文件** ⇒ 两侧永不漂移）：

| 形状（`shapes.txt` 标签） | oracle 组件 | guest 组件 | 判定 |
|---|---|---|---|
| `a-length-only`（`if` 条件 · 形参） | **1167** | 1167 | ✅ **逐字节等同** |
| `b-length-and-concat`（concat + length 同模块） | **1183** | 1183 | ✅ **逐字节等同** |
| `c-length-only-statement`（`let` 绑定 · 形参） | **1091** | 1091 | ✅ **逐字节等同** |
| `d-length-local`（`let` 绑定 · 局部 cell） | **1091** | 1091 | ✅ **逐字节等同** |
| `e-length-set`（`set` 位 · 形参） | **1113** | 1113 | ✅ **逐字节等同** |
| `g-length-twice`（同一函数两个 length 调用） | **1197** | 1197 | ✅ **逐字节等同** |
| `i-length-two-functions`（两个函数各一个调用） | **1139** | 1139 | ✅ **逐字节等同** |
| `q-return-position`（return 位 · 形参） | **1091** | 1091 | ✅ **逐字节等同** |
| `r-return-literal`（return 位 · 字面量） | **1111** | 1111 | ✅ **逐字节等同** |
| `t-inside-if-block`（`if` 块内的语句 · 形参） | **1187** | 1187 | ✅ **逐字节等同** |
| `j-nested-argument`（嵌套调用实参，return 位） | **1183** | **前端拒** `ERR:E-SH-IR-EXPRESSION` | ⏳ **T2-g2**（下一刀） |
| `p-while-concat`（`while` 体 + 嵌套调用实参） | **1301** | **前端拒** `ERR:E-SH-IR-CALL-SHAPE` | ⏳ **T2-g2** + 循环形状 |
| `k-length-bytes-argument`（`Bytes` 实参） | **语义层拒** `E2001` | **前端拒** `ERR:E-SH-IR-EXPRESSION` | ✅ 两层都拒（不是缺口） |
| `f-concat-only`（字面量里嵌 intrinsic） | **语义层拒** `E2001` | **前端拒** `ERR:E-SH-IR-EXPRESSION` | ✅ 两层都拒（不是缺口） |
| `h-length-in-while`（`while` 体 · 含 length） | **1209** | **发射端拒** | ↩️ **循环形状**（§4.4 对照） |
| `o-while-no-length`（`while` 体 · **不含** length） | **1142** | **发射端拒** | ↩️ **循环形状**（**对照形状**） |
| `s-while-condition`（`while` 条件 · 含 length） | **1187** | **发射端拒** | ↩️ **循环形状** |
| `l-bit-or-only`（`U64.bit_or`，**不含** length） | **1039** | **前端拒** | ↩️ **前端窄一格**（**对照形状**） |
| `n-shr-only`（`U64.shr`，**不含** length） | **1039** | **前端拒** | ↩️ **前端窄一格**（**对照形状**） |

⇒ **组件逐字节等同 10 个形状**；**拒绝逐条归因**（见 §4.4）。

> **注意 `t-inside-if-block`**：`if` **块内**的语句**能**逐字节发射（1187 B）——发射端认识**分支块**；发射端不认识的是 **`while` 循环**（三个位置全拒，且**对照形状不含 length 也拒**）。

### 4.3 拒绝身份**分层记**（前端拒绝 vs 发射端拒绝 vs 语义层拒绝）

| 形状 | oracle（Rust） | guest | 归属 |
|---|---|---|---|
| j 嵌套调用实参（return 位） | **降下来并发射**（1183 B） | **前端拒** `ERR:E-SH-IR-EXPRESSION` | ⏳ **T2-g2**（真实缺口） |
| k `Bytes` 实参 | **前端降下来**，**语义层拒**：`E2001` / `TYPE_MISMATCH` / "expected Text, found Bytes" | **前端拒** `ERR:E-SH-IR-EXPRESSION`（return 位）／`ERR:E-SH-IR-CALL-TYPE`（let 位） | ✅ **不是缺口**（两层都拒） |
| l 字面量里嵌 intrinsic | **前端降下来**，**语义层拒**：`E2001` / "expected U64 literal in range, found non-literal Int" | **前端拒** `ERR:E-SH-IR-EXPRESSION` | ✅ **不是缺口**（两层都拒） |

> 「**某形状被拒绝**」≠「**该形状没实现**」（AGENT 指南 §5）。本片把 **前端拒绝** 与 **发射端拒绝** **分开记**，并注明 oracle 侧的**语义层拒绝**在第三层。

### 4.4 剩下的拒绝：**逐条用对照形状证伪**（不靠猜）

本片最重要的诚实性工作：**发射端认领 `sico.text.length` 之后，guest 发射端仍有拒绝**。这些拒绝**是不是** T2 的缺口？**用不含该算子的对照形状**逐一判定：

| 拒绝 | 对照形状（**不含** `sico.text.length`） | 对照结果 | 判定 |
|---|---|---|---|
| 发射端拒 `while` 体 | `o-while-no-length`：`while` 体里**没有** length 调用（1142 B，oracle 发射得出） | **同样被拒** | ↩️ **`while` 循环形状的既有窄一格**，**不是 T2 缺口** |
| 发射端拒 `while` 条件 | 同一个对照形状（`while` 循环整体被拒，与 length 在哪个位置无关） | **同样被拒** | ↩️ **同上** |
| 前端拒 `U64.bit_or` / `U64.shr` | `l-bit-or-only` / `n-shr-only`：**不含** length 的位运算（各 1039 B，oracle 发射得出） | **同样被拒** | ↩️ **前端窄一格**，**不是 T2 缺口** |

⇒ **T2 族只剩 1 条真缺口：T2-g2（嵌套调用实参，前端层）**。这一条**固件钉住**（`sico_compiler_keeps_the_nested_text_length_arguments_typed_refused`），并且**前沿探针新增的用例把它钉到 return 位**。

> **顺带确认**：发射端**认识 `if` 分支块**（`t-inside-if-block` 逐字节等同 1187 B）⇒ 发射端不认识的是 **`while` 循环**这一个形状，**不是**「分支」。

### 4.5 前沿探针：**0 条期望翻转 + 1 条新增**

本片只动**发射端**，不动**前端** ⇒ 前沿探针的判定**不动**（`FRONTIER_UNCHANGED`）。新增 1 条用例，把 T2-g2 的边界钉到 **return 位**：

```
text-length-over-a-nested-call-argument-in-a-return-position
  function size(t: Text) returns U64:
    return sico.text.length(sico.text.concat(t, "x"))
  end function
  → refused:ERR:E-SH-IR-EXPRESSION
```

（CLI 口径已实测确认该判词；oracle 对同一形状**降下来并发射** 1183 B ⇒ 它**正是**下一刀要搬的边界。）

## 5. 验证记录（2026-10-24）

| 验证 | 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`，冷构建） | ✅ **161 passed / 0 failed**（§6.2） |
| 组件逐字节比对 | `python target/tmp/step0530/emit_probe.py`（guest 发射 vs oracle 生产组件） | ✅ `EMIT_CHECK_GREEN`：**10 个形状逐字节等同** |
| 前端判定分层 | `python target/tmp/step0530/frontend_verdict.py` | ✅ 每个形状的拒绝层（前端 / 发射端）分开记 |
| oracle 真值 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test scratch_step0530_probe`（临时探针，**交付前删除**） | ✅ 18 个形状的组件字节数 / 语义层判词 |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | 见 §6.2 |
| 本机电池 | `target/tmp/step0530/battery.ps1`（**19 步**） | 见 §6.2 |

### 6.1 T2 规则表更新（组件列翻转）

见 [支持面登记表 §2](../reports/m22-support-surface-t2-text-length.md#2-族级真值表13-行--1-行类型探针实测)：

- **组件逐字节等同 0 → 10 行**；
- **前端 typed 拒绝 2 → 2 行**（身份不变，仍是 T2-g2）；
- **组件层 typed 拒绝 = 全部可降形状** → **不再成立**（只余 `while` 循环形状与 T2 无关）。

### 6.2 电池

本片电池 = `target/tmp/step0530/battery.ps1`（**19 步** = STEP-0529 的 19 步原样复用，把
`step0529-doc-check` 换成 `step0530-doc-check`），日志 `target/tmp/step0530/battery.log`。
**首轮即绿**（与 STEP-0529 的首轮红 3 步不同——本片的固件改动在提交前已过 `cargo fmt`）。

| 步骤 | 耗时 | 裁决 |
|---|---|---|
| `workspace-integrity` | 51s | ✅ |
| `fmt-runner` | 2s | ✅ |
| `ci-one-key` | 409s | ✅ |
| `validate-0245` | 122s | ✅ |
| `validate-0262` | 140s | ✅ |
| `probe-frontend-frontier` | 340s | ✅ |
| `probe-ir-plan-frontier` | 26s | ✅ |
| `validate-0124` | 1s | ✅ |
| `validate-0131` | 2s | ✅ |
| `validate-0156` | 1s | ✅ |
| `git-diff-check` | 0s | ✅ |
| `line-ending-audit` | 0s | ✅ |
| `selfhost-comment-discipline` | 0s | ✅ |
| `step0526-footprint-audit` | 0s | ✅ |
| `step0526-census-branches` | 0s | ✅ |
| `step0526-differential` | 84s | ✅ |
| `step0528-census-t2` | 0s | ✅ |
| `step0530-doc-check` | 0s | ✅ |
| `workspace-integrity-final` | 51s | ✅ |

⇒ **BATTERY_GREEN 19/19**

### 6.3 独立 CI 回填（run #36 / run #37）

- **STEP-0529** 推送（`e5f657a`）触发的 **run #36**（run id `40016972272`，head_sha `e5f657a7455e9f173032ada52d5f43ccc75d0266`）= ✅ **success**（2026-10-24 本机经 GitHub API 复核）。
- **本片**推送（`13507b8`）触发的 **run #37**（run id `40024577187`，head_sha `13507b80862b694ca8b765cde73db55e12c9426b`）= ✅ **success**（2026-10-24 本机经 GitHub API 复核，**由 STEP-0531 回填**）。

## 7. 诚实性

### 7.1 本片**只推进组件层**

- `selfhost/parser.sico` **零改动** ⇒ 前端前沿**不动**（前沿探针 `FRONTIER_UNCHANGED`）。
- oracle 生产代码（`crates/*`）**零改动**。
- **不推进任何出口门**；**M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。

### 7.2 未闭合的（承自本片）

- **T2-g2 嵌套调用实参**（S 内 **2** 个调用点：`compiler.sico:57` 的 `sico.text.length(sico.bytes.utf8_decode(src))`、`compiler.sico:361` 的 `sico.text.length(positive_sleb_hex(initial))`）仍 typed 拒绝 ⇒ **T2 版的 G-f2**，需要**另一刀**。
- **语料覆盖仍 0 条**：S 内 322 个 T2 调用点一个都没进差分语料。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- **guest 发射端仍不认循环 / 分支形状**（**与 T2 无关**）：`while` 体、分支块内的语句一律 `unsupported codegen source shape` ⇒ 登记在 §4.4，**不进 T2 的账**。
- **guest 前端比 oracle 前端窄一格**：`U64.bit_or` / `U64.shr` 前端拒（**与 T2 无关**）；`sico.text.length` 喂 `Bytes` 时 oracle 降下来再由语义层拒、guest 在前端拒（**层次不同**）。

### 7.3 下一刀（W2 族 T2 的第 4 拍 / **T2-g2 刀**）

**T2-g2 刀**：让 guest 前端认**实参本身是嵌套调用**的 `sico.text.length`（`sico.text.length(sico.text.concat(t, "x"))` 与 `sico.text.length(sico.bytes.utf8_decode(src))` 两个形状）。做法应沿用 **T1 族 G-f2 刀（STEP-0525）**的先例：**新形状必须落在既有可降形状的延长线上**——`sico.text.concat` 的实参下降器（`stc_call_operand_packed`）已经能递归进嵌套调用 ⇒ 形状**可复用**，但**不得**顺带把 `sico.text.length` 的**元数**或**类型**闸门放宽。之后是 **T2 族收口片（X）**。
