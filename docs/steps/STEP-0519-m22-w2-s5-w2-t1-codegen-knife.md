# STEP-0519 m22-w2 S5 **W2 族 T1 codegen 刀（C）**：`sico.text.concat` ⇒ 整组件与 Rust 后端**生产输出**逐字节等同（1094 B）

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0518](./STEP-0518-m22-w2-s5-w2-t1-frontend-knife.md)（上一片 · T1 前端刀 F）、[STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 真值测量片）、[STEP-0513](./STEP-0513-m22-w2-s5-host-call-component.md)（codegen 刀的方法学先例）
> - gate: **不推进任何出口门** —— W2 的**第 3 拍（codegen 刀 C）**；**族仍未闭合**（按计划 §3.3.5 的族级出口判据，**五项缺一不算闭合**）
> - numbering: **STEP-0519**（取号前已核对 `docs/steps/`：最大号 0518；*0519 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| owner 已授权 W2 族按「每族四拍」推进。 | ✅ 计划 §3.3.5 |
| 上一拍（前端刀 F）已落地，canonical IR 逐字节等同。 | ✅ STEP-0518：2295 B == 2295 B |
| 发射真值已从 oracle **生产输出**取到（不猜）。 | ✅ §2.1：type/function/export/code 四段的实测 |
| 本拍是**实现片**：改 `selfhost/codegen.sico`，oracle 生产代码**零改动**。 | ✅ §4.1 |
| 出口测试有界。 | ✅ §3：3 条新钉 + 1 条真值钉翻转 |

⇒ **取号合法**。

## 1. 这一片回答的问题（阶段的决策棱镜）

STEP-0517/0518 把前端面的活干完后，剩下的问题**全部在组件层**，而且与 STEP-0513（`sico.bytes.length`）**不同构**：

- `sico.bytes.length` 是**内联**发射（`local.get` + `i64.extend_i32_u` + `local.set`），**不产生新函数**；
- `sico.text.concat` **必须发射一个 helper 函数体**（arena 分配 + 两段 `MemoryCopy`），**还要**在 type/function/code 三段各多出一条目，**还要**让调用点指向它。

于是本片的问题：

1. **helper 的形状与索引从哪来？** ⇒ 必须**从 oracle 生产输出测**，不能按直觉写。
2. **「模块用没用到 concat」怎么判定？** ⇒ 必须**结构化扫描 IR**，**不得**用子串搜索（否则 `sico.text.concat` 出现在别处会误判）。
3. **helper 条目怎么条件化？** ⇒ 模块**没用到** concat 时，type/function/code 三段必须**保持三条目**（既有组件 1025 B 逐字节不变）。
4. **新代码本身能不能被 guest 自己逐字节降低？** ⇒ 自举形状前沿必须**不推进**。

## 2. 实施设计

### 2.1 发射真值（实测，oracle 生产输出）

STEP-0519 的准备片把 oracle 组件的 type/function/export/code 四段**解析**出来（`target/tmp/step0519/core-sections.txt`；**段结构可信，但 code body 的 locals 前缀被脚本漏掉**，见 §4.6 —— helper 体的真值最后是**从后端源码逐条对出来的**）：

| 段 | 实测 |
|---|---|
| type | 6 条。type 0 = 模块函数 `join` 的 `(i32,i32,i32,i32)->(i32,i32)`；type 1 = `run` 的 `(i32×4)->(i32)`；type 2/3/4 = 三个 arena helper（`60017f017f` / `60047f7f7f7f017f` / `60017f00`）；**type 5 = concat helper** = `60047f7f7f7f027f7f` |
| function | 6 条，**`type_index` = 自身下标**（0..5） |
| export | **不变**（memory 0 / run 1 / cabi_realloc 3 / cabi_post_run 4） |
| code | 6 个 body：模块函数 ×2 + alloc + realloc + post + **concat helper** |

**helper 的 core function index = 模块函数数 + 3**（后端规则：`helper_count = import_count + module.functions.len()`，helper index = `helper_count + 3 + i`；Script profile 的 `import_count = 0`，`used_helpers` 是**按名字排序**的集合 ⇒ `sico.text.concat` 恒为 `i = 0`）。

**helper body（42 B）**：`01027f`（1 组 × 2 个 i32 局部） + `200120036a2205`（`a_len + b_len` → `local.tee 5`） + `10 <alloc>`（`call` arena 分配） + `21 04`（`local.set 4` = 结果 pointer） + `200020012001fc0a0000`（`memory.copy` 第 1 段） + `200420016a20022003fc0a0000`（`memory.copy` 第 2 段） + `200420050b`（返回 `(ptr, len)`）。

**调用点**：`local.get a_ptr` / `local.get a_len` / `local.get b_ptr` / `local.get b_len` → `call <helper>` → `local.set <result len>` → `local.set <result ptr>`。

### 2.2 落点（`selfhost/codegen.sico`）

| 项 | 落点 |
|---|---|
| **新增** 4 个结构化扫描函数 | `cgs_instruction_is_text_concat` / `cgs_block_has_text_concat` / `cgs_function_has_text_concat` / `cgs_module_has_text_concat`（共 **93 行**函数体） |
| **新增** 1 个 helper 体发射函数 | `cg_concat_helper_body_hex`（**9 行**函数体） |
| **搬家** 1 个既有函数 | `cgs_function_count` 从 `:3381` 上移到 `cgs_intrinsic_hex` 之前（**函数体逐字节不变**，见 §4.2） |
| **重塑** 4 个既有函数 | `cgs_intrinsic_hex`（新增 concat 分支）、`cgs_core_types_hex` / `cgs_core_funcs_hex` / `cgs_core_code_hex`（三段条件化） |
| 合计 | 文件 **3744 → 3979 行**（**+235 行**，按 LF 计） |
| oracle 生产代码（`crates/*`） | **零改动** |

### 2.3 四道闸门（**全 fail-closed**）

`cgs_intrinsic_hex` 新增的 `sico.text.concat` 分支，四道闸门任一不过 ⇒ 一律 `ERR:E-SH-CG-IR-EMIT`（**不静默取近似**，ADR-0016 决策 6）：

| 闸门 | 条件 | 越界时的身份 |
|---|---|---|
| 名字 | `sico.text.concat` | `ERR:E-SH-CG-IR-EMIT` |
| 实参个数 | **恰好 2**（第 3 个实参存在即拒） | `ERR:E-SH-CG-IR-EMIT` |
| 实参类型 | 两个都是 `string`（= `Text`） | `ERR:E-SH-CG-IR-EMIT` |
| 结果槽数 | **恰好 2**（pointer, length） | `ERR:E-SH-CG-IR-EMIT` |

**元数边界不在这里**：1 个 / 3 个实参由 **Rust 前端自己**拒 `stdlib intrinsic arity`（语义层，STEP-0517 已钉）⇒ guest **不得**自造元数检查；本闸门只是**不再往下走**。

### 2.4 「模块用没用到 concat」= **结构化扫描**

不是子串搜索。扫描沿后端**同一套**层级走：`functions` → 每个函数的 `blocks` → 每个 block 的 `instructions` → 每条指令的 `operation.op == "intrinsic"` → `operation.data.name == "sico.text.concat"`。

- 四级各自一个函数，**每个函数只有一个 `while`**（flag 模式，不用嵌套 `continue`）；
- 任一层命中即 **early return 1**，不命中返回 0；
- 结果驱动三段条目：`cgs_core_types_hex` 多一条 type、`cgs_core_funcs_hex` 多一条 function（`type_index` 恒等于自身下标 ⇒ 计数 +1 就够）、`cgs_core_code_hex` 多一条 code body。

### 2.5 helper 条目**条件化**

`cgs_core_types_hex` / `cgs_core_funcs_hex` / `cgs_core_code_hex` 三处都改成「模块用了 concat ⇒ 计数 +1，并追加 helper 的 type / function / code 条目」。模块**没用到**时三段**保持三条目** ⇒ 既有组件（host-call fixture **1025 B**、echo fixture）**逐字节不变**。

## 3. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 |
|---|---|---|
| 1 | `oracle_text_concat_component_truth_is_pinned`（**STEP-0517 真值钉翻转**） | 组件 **1094 B** == oracle 生产输出（**逐字节相同**）+ 过真实 `wasmparser::Validator`（oracle 与 guest 两侧各一次） |
| 2 | `sico_codegen_guest_emits_the_text_concat_helper_index_from_the_module_function_count` | 三函数模块（`join` + `size` + `run`）的组件**逐字节** == oracle ⇒ **helper 索引是推导出来的**（模块函数数 + 3），不是写死的 |
| 3 | `sico_codegen_guest_keeps_the_modules_without_the_concat_helper_unchanged` | 不用 concat 的模块仍是 **1025 B**，逐字节 == oracle ⇒ **helper 条目确实条件化** |
| 4 | `sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly` | **自举形状前沿**：本片新增/重塑的 **10** 个 codegen 函数经 guest 自己逐字节降低 == Rust 前端 ⇒ 未推进形状边界 |

⇒ 固件从 **129** 增至 **132** 条。

## 4. 实施记录（2026-10-23）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/codegen.sico` | 新增 5 个函数（4 扫描 + 1 helper 体）+ 搬家 1 个（`cgs_function_count`）+ 重塑 4 个 ⇒ 文件 **3744 → 3979 行**（**+235 行**） |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新增 3 条钉 + **翻转** 1 条 STEP-0517 真值钉（`sico_codegen_guest_refuses_the_text_concat_component_outside_the_subset` → `oracle_text_concat_component_truth_is_pinned`） |
| `selfhost/parser.sico` / oracle 生产代码（`crates/*`） | **零改动** |
| `docs/*` | 同步本片结论（5 处） |

### 4.2 自证：把本片**反向撤销**，量出它的**全部足迹**

本片**不是**纯插入（它搬了一个函数、重塑了四个），所以「改了哪些行」不能靠声明。`target/tmp/step0519/audit_footprint.py` 把本片的**九处改动逐条反向撤销**，重建出**本片之前**的文件：

```
STEP-0519 reverse-applied footprint
  reconstructed_lines=3745
  current_lines=3980
  delta_lines=235
STEP0519_FOOTPRINT_GREEN edits_reversed=9
```

（两个行数都按同一种口径计：文件以 LF 结尾，`split("\n")` 的末位空元素计入 ⇒ 换算成 LF 口径即 **3744 → 3979 行**。）

⇒ **九处反向撤销全部干净命中**（任何一处对不上 ⇒ 脚本报红）⇒ **本片对 `selfhost/codegen.sico` 的改动面就是这九处、净 +235 行**，没有别的暗改。脚本另外交叉核对：重建文件里三个 arena helper 的**冻结常量**仍在（重建没把别的东西弄坏）。

`target/tmp/step0519/audit_codegen_delta.py` 再机械检查三件事：① 文件**纯 LF、无 BOM**；② `cgs_function_count` **只声明一次**且其**函数体与搬家前逐字节相同**（把旧体钉在脚本里比对）；③ 本片新增的函数**全部先于其首次使用声明**（guest 前端只把调用降低到**已声明**的函数 ⇒ 这条规则直接决定了 `cgs_function_count` 必须上移）。

```
STEP0519_CODEGEN_DELTA_GREEN
```

### 4.3 出口达成

| 项 | 值 |
|---|---|
| 整组件 | **1094 B** == oracle 生产输出 **1094 B**（**逐字节相同**） |
| `wasmparser::Validator` | oracle 组件与 guest 组件**两侧都通过** |
| helper 索引 | 模块函数数 + 3（两函数模块 ⇒ **5**；三函数模块 ⇒ **6**）⇒ 由钉 #2 证明是**推导**的 |
| 条件化 | 不用 concat 的模块仍是 **1025 B**，逐字节不变（钉 #3） |
| 自举前沿 | 本片 10 个 codegen 函数逐字节等同 ⇒ **未推进形状边界** |
| 固件 | **132 passed / 0 failed**（STEP-0518 的 129 + 本片 3 条） |
| 前沿探针 | **28/28** ⇒ `FRONTIER_UNCHANGED`（codegen 刀**不动前端前沿**，见 §5.4） |

### 4.4 没有静默放宽

- **别的 helper 型内建仍然全拒**：guest 的 codegen 现在**只**认 `sico.bytes.length`（内联、无 helper）与 `sico.text.concat`（本片认领）这两个名字；任何其它名字在 `cgs_intrinsic_hex` 处一律 fail-closed `ERR:E-SH-CG-IR-EMIT` ⇒ 整组件退回 typed 拒绝，**不产生近似字节**。**这是代码层判定**（`cgs_intrinsic_hex` 的最后一行就是那句 typed 拒绝），**不是端到端钉** —— 某个内建能不能走到这一层，还取决于前端能否把它降下来。
- **四道闸门全 fail-closed**：见 §2.3。
- **STEP-0517 的前端未闭合形状一个都没被认领**：两个字面量实参 / 混合实参对 / 嵌套宿主调用仍 `ERR:E-SH-IR-EXPRESSION`（STEP-0518 已钉，本片未动）。
- **元数边界仍在语义层**：guest 不自造元数检查。

### 4.5 自造缺陷 ①：**直线体里 `let` 跟在 `set` 之后 ⇒ `ERR:E-SH-IR-CALL-TYPE`**

`cg_concat_helper_body_hex` 初稿写成「`let` → `set` → `set` → **`let`** → `set` → `set`」。自举前沿钉直接报 `ERR:E-SH-IR-CALL-TYPE`。

**根因（实测，不是猜）**：临时探针把该形状拆成 6 个最小变体逐一试，结论是 —— **直线函数体里，`let` 绑定必须全部排在任何 `set` 之前**；一旦 `let` 出现在 `set` 之后，guest 前端拒绝整个函数（`ERR:E-SH-IR-CALL-TYPE`）。注意：**在 `if` 分支内部**同样的写法是可降的（`cgs_intrinsic_hex` 的 concat 分支里就有 `let` 跟在 `set` 之后，且逐字节等同）—— 所以这条规则**只约束直线体**。

**修法**：把 `let cgchbh_alloc = cg_leb_hex(alloc_index)` **提到所有 `set` 之前**。**教训**：guest 前端的形状规则是**逐路由**的，不能把「`if` 分支里可以」外推成「直线体里也可以」。

### 4.6 自造缺陷 ②：**准备片的反汇编脚本把 locals 段丢了**

STEP-0519 准备片的 `dump_component_sections.py` 打出的 code body **不含 locals 段** ⇒ helper body 一开始被读成 **39 B**（而不是真实的 **42 B**），差点把 helper 体少写 3 字节。

**怎么发现的**：把同一脚本打出的 alloc / realloc / post 三个 body 与 guest 里**已冻结**的常量比对 —— 三个都对不上，差的正好是各自的 locals 前缀（`01027f` / `01027f` / `00`）⇒ 说明**脚本**错了，不是**真值**错了。

**修法**：真值**改从后端源码 `crates/sico-codegen-wasm/src/stdlib.rs` 的 `emit_concat` 逐条对出来**（`Function::new(vec![(2, ValType::I32)])` ⇒ locals = `01027f`），再用 oracle 组件的**总长度**与 `wasmparser::Validator` 交叉验证。**教训**：**反汇编工具本身也要被交叉验证**——拿它去和一个**已知正确的冻结常量**比对，是最便宜的自检。

### 4.7 自造缺陷 ③：**「callee 必须先声明」这条规则逼出了一次函数搬家**

`cgs_intrinsic_hex` 的 concat 分支需要**模块函数数**（才能算出 helper 索引 = 模块函数数 + 3），而 `cgs_function_count` 原本声明在 `cgs_intrinsic_hex` **之后** ⇒ 既是 forward reference，又违反 guest 前端「只把调用降低到已声明函数」的规则。

**修法**：把 `cgs_function_count` **原样上移**到 `cgs_intrinsic_hex` 之前（**函数体逐字节不变**，§4.2 有机械比对）。**代价**：本片**不是**纯插入（与 STEP-0518 的前端刀不同）—— 这是 codegen 刀的真实成本，记在此处以便审计。

## 5. 诚实性

### 5.1 本片**只闭合组件面**，**族仍未闭合**

- 按计划 §3.3.5 的族级出口判据（canonical IR 逐字节等同 **且** 整组件逐字节等同 **且** `wasmparser::Validator` 通过 **且** 前沿探针期望值已翻转 **且** 独立 CI 绿，**五项缺一不算闭合**）：前四项**本机**已具备（前沿探针的翻转在 STEP-0518 已发生），**独立 CI 尚未运行** ⇒ **T1 族尚未闭合**。
- **T1 拍 4（收口片 X）** 未开始。

### 5.2 未闭合的 T1 形状

- **前端**：两个字面量实参、混合实参对、嵌套宿主调用、let 绑定位置的字面量/cell 变体、**体内另有语句**的其它位置变体 ⇒ 仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`。
- **组件**：guest 的 codegen 只认**一个** helper 型内建（`sico.text.concat`）；其它 helper 型内建仍 fail-closed。
- **语料覆盖仍 0 条**（S 内 2,882 个调用点的绝大部分未覆盖）。

### 5.3 本片**没有**改动的

- oracle 生产代码（`crates/*`）**零改动**；`selfhost/parser.sico` **零改动**；前沿探针**零改动**。
- **不改任何预算常量**、不改任何既有断言的期望值（只**翻转** STEP-0517 那条**正好被本片认领**的真值钉）。
- **未提交、未推送**；**独立 CI 尚未运行**。

### 5.4 前沿探针为什么不动

`tools/probe-m22-frontend-frontier.ps1` 判的是**前端**前沿（IR 能否降下来），codegen 刀**不改变前端能降的形状集合** ⇒ 期望值表**不变**，仍是 **28/28** `FRONTIER_UNCHANGED`。这不是「探针漏了」，而是**探针的判据层次**如此；组件层的判据由本片的 3 条钉承担。

### 5.5 下一刀（W2 族 T1 的第 4 拍 / 收口片 X）

1. **钉住**：把本片的组件真值（1094 B）、helper 索引推导、helper 条目条件化、三函数模块变体**并入族级真值表**；
2. **纠偏**：把 §5.2 列出的未闭合形状**逐条**登记进支持面登记表（D15）；
3. **前沿探针期望值**：确认无需前移并把理由写进探针文件；
4. **固件电池 + 独立 CI**（提交后）；
5. **诚实收窄段**：族级结论写入 STATUS/ROADMAP，并给出「T1 族闭合」与「T2 族开工」之间的门槛。

## 6. 验证记录（2026-10-23）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉（4 条定向） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --exact <4 条> --test-threads=1` | ✅ **4 passed / 0 failed** |
| 固件全量 | 同上，去掉 `--exact` | ✅ **132 passed / 0 failed**（114 s） |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | ✅ **28/28** ⇒ `FRONTIER_UNCHANGED` |
| codegen 改动审计 | `python .\target\tmp\step0519\audit_footprint.py` + `python .\target\tmp\step0519\audit_codegen_delta.py` | ✅ `STEP0519_FOOTPRINT_GREEN edits_reversed=9`（**+235 行**）+ `STEP0519_CODEGEN_DELTA_GREEN` |
| 注释纪律 | `python .\tools\validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 本机验证电池 | `target/tmp/step0519/battery.ps1`（11 步） | 见 §6.2 |

### 6.1 T1 族状态速查

| 拍 | 状态 | 证据 |
|---|---|---|
| 第 1 拍（真值测量 T） | ✅ 完成 | STEP-0517：10 变体规则表 + 子集边界 + 发射真值 |
| 第 2 拍（前端刀 F） | ✅ 完成 | STEP-0518：canonical IR 逐字节等同（2295 B）+ 自举前沿未推进 |
| **第 3 拍（codegen 刀 C）** | ✅ **本片完成** | 本片：整组件逐字节等同（1094 B）+ Validator 双侧通过 + helper 索引推导 + 条目条件化 |
| 第 4 拍（收口片 X） | ❌ 未开始 | 需在前三拍之后（钉住 + 纠偏 + 电池 + 独立 CI） |

⇒ **T1 族：3/4 拍完成，尚未闭合**（缺独立 CI 与收口片）。

### 6.2 本机验证电池（11 步）

见 §6.4（电池在本片收尾时整轮重跑）。

### 6.3 未做的事

- **只认领一个 helper 型内建**、**四道闸门 fail-closed** ⇒ T1 的其余形状与 S 内 2,882 个调用点**仍未覆盖**；
- **未提交、未推送**；**独立 CI 尚未运行**；
- **不推进任何出口门**，M22 仍为 NO-GO。

### 6.4 本机验证电池（11 步）

本轮电池 **11/11 全绿**（`target/tmp/step0519/battery.ps1`，日志 `target/tmp/step0519/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 430 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试 ⇒ **固件 132 条在本步已跑过**） |
| `validate-0245` | 148 | 0 | 内含固件全量 + clippy（`-D warnings` 零警告） |
| `validate-0262` | 145 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 193 | 0 | 前端前沿 **28/28** ⇒ `FRONTIER_UNCHANGED`（codegen 刀**不动前端前沿**） |
| `probe-ir-plan-frontier` | 25 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 3 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN`（**26** 个文件，含本片新增的 STEP-0519 文档） |

⇒ `BATTERY_GREEN 11/11`。
