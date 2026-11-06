# STEP-0520 m22-w2 S5 **W2 族共享前置（cell 支持）**：`write_local` / `read_local` ⇒ 整组件与 Rust 后端**生产输出**逐字节等同（1114 B）

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0519](./STEP-0519-m22-w2-s5-w2-t1-codegen-knife.md)（上一片 · T1 codegen 刀 C）、[STEP-0518](./STEP-0518-m22-w2-s5-w2-t1-frontend-knife.md)（T1 前端刀 F）、[STEP-0517](./STEP-0517-m22-w2-s5-w2-t1-text-concat-truth.md)（T1 真值测量片）
> - gate: **不推进任何出口门** —— 这是 W2 **所有族共享的前置片**（cell 支持），**不是** T1 族的第 4 拍；T1 族仍未闭合
> - numbering: **STEP-0520**（取号前已核对 `docs/steps/`：最大号 0519；*0520 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| D10 的实测结论要求**继续闭合**而不是降级 T1 族。 | ✅ §1.1：S 内 **2,843 / 2,857** 个 `sico.text.concat` 调用点落在**带 cell 的函数**里 |
| 缺口已定位到**组件层**（前端已能逐字节降低）。 | ✅ §2.1：canonical IR **2630 B** 与 Rust 前端逐字节相同 |
| 发射真值已从 oracle **生产输出**取到（不猜）。 | ✅ §2.2：反汇编 oracle 组件的 code 段，逐槽核对 |
| 本拍是**实现片**：改 `selfhost/codegen.sico`，oracle 生产代码**零改动**。 | ✅ §4.1 |
| 出口测试有界。 | ✅ §3：6 条新钉 |

⇒ **取号合法**。

## 1. 这一片回答的问题

### 1.1 为什么 cell 支持是**全族共享前置**（D10）

STEP-0519 之后，T1 族（`sico.text.concat`）的组件面在**不带 cell** 的形状上已经闭合。但准备片的普查（生产 Rust 前端，冻结源集 S = `selfhost/*.sico` 9 个文件）给出：

| 指标 | 实测 |
|---|---|
| S 内 `sico.text.concat` 调用点 | **2,857** |
| 其中落在**带 cell 的函数**里的 | **2,843（99.5%）** |
| S 内 `write_local` / `read_local` 指令 | **12,204** / **18,569** |

⇒ 按 D10（族降级后若 S 内该族仍有未覆盖调用点形状 ⇒ **不得降级，必须继续闭合**），T1 族**不能**降级到「无 cell 的子集」；而**任何**族（`sico.text.length` 起）都会撞上同一堵墙：`let` 绑定一旦被 `set` 赋值，Rust 前端就把它物化成 **cell**，组件层必须会发射 cell。

⇒ **cell 支持是 W2 全部 20 个族的共享前置**，必须单独一片做完，且必须**先于**任何族的 codegen 刀。

### 1.2 本片要回答的三个问题

1. **后端的槽位布局到底是什么？** ⇒ 必须从后端源码与 oracle 生产输出**对出来**（§2.2）。
2. **unit 型指令结果占几个槽？** ⇒ **0 槽 0 组**（后端 `inferred_local_layout` 对 `WriteLocal` 明确「no result slots」）。
3. **cell 的类型子集怎么定？** ⇒ 必须与后端 `lower_local_types` 的语义**一致**，不一致就**拒**（§2.3）。

## 2. 实施设计

### 2.1 参考形状（第一个需要 cell 的形状）

```
function join(a: Text, b: Text) returns Text:
  let x = a
  set x = b
  return sico.text.concat(a, b)
end function
```

Rust 前端把它降成（canonical IR，**2630 B**）：

- `locals`: `[{"name":"x","ty":{"kind":"string"}}]`
- 指令 1：`{"result":2,"ty":{"kind":"unit"},"operation":{"op":"write_local","data":{"local":0,"value":0}}}`
- 指令 2：`{"result":3,"ty":{"kind":"unit"},"operation":{"op":"write_local","data":{"local":0,"value":1}}}`
- 指令 3：`{"result":4,"ty":{"kind":"string"},"operation":{"op":"intrinsic","data":{"name":"sico.text.concat","arguments":[0,1]}}}`
- terminator：`{"kind":"return","data":4}`

⇒ **被赋值的 `let` 绑定 = cell**；cell 写的结果类型是 **unit**（0 槽）。

### 2.2 后端真值（`crates/sico-codegen-wasm/src/lib.rs`，生产代码）

`LocalLayout::plan`（`:1812`）的槽位顺序：

| 顺序 | 内容 | 声明 |
|---|---|---|
| 1 | 形参 | **不声明**（占形参槽） |
| 2 | 各指令结果 | 每个结果槽一组 `(1, valtype)`；**unit 结果 0 槽 0 组** |
| 3 | **cells** | 每个 cell 的 flat 类型各一组（`lower_local_types`） |
| 4 | dispatcher | 1 组 `(1, i32)` |
| 5 | Script scratch | 1 组 `(2, i32)` |

发射规则：`WriteLocal` = 逐槽 `local.get(源)` + `local.set(cell)`；`ReadLocal` = 逐槽 `local.get(cell)` + `local.set(结果)`；源宽与 cell 宽不等即 typed 拒绝。

**oracle 组件反汇编**（`target/tmp/step0520/dump_core.py`，冻结工件 `concat-g2-oracle-component.hex.bin`，**1114 B**）：

```
core function 0 (= join): locals groups = [(1,127),(1,127),(1,127),(1,127),(1,127),(2,127)]  (7 槽)
  4100 2108 0240 0340 2008 4100 46 0440
  2000 2106 2001 2107            <- write_local{local:0,value:0}: a(0,1) -> cell(6,7)
  2000 2001 1005 2105 2104       <- write_local{local:0,value:1}: b(2,3) -> cell(6,7)
                                 --   + intrinsic concat: local.get 0/1/2/3, call 5
  2004 2005 0f                   <- return: local.get 4, local.get 5, return
```

⇒ 槽位：`a`=0,1 `b`=2,3 concat 结果=4,5 **cell x=6,7** dispatcher=**8** scratch=9,10；locals 声明 = **5 组 `(1,i32)` + 1 组 `(2,i32)`**。

> **反汇编工具本身也被交叉验证**：同一脚本打出的 alloc / realloc / post / concat helper 四个 body 与 guest 里**已冻结**的常量逐字节相符（helper 的 locals 前缀 `01027f` 也在内）—— 这正是 STEP-0519 §4.6 教训（§4.6）的落实。

### 2.3 cell 的类型子集（fail-closed）

`cgs_cell_flat_hex` 只认后端 `lower_local_types` 在 Script profile 下**确实这样降**的类型：

| cell 类型 | flat | 槽数 |
|---|---|---|
| `bool` | `017f` | 1 |
| `i64` / `u64` | `017e` | 1 |
| `string` / `bytes` | `017f017f` | 2 |
| **其它** | — | **拒**（`""` ⇒ `ERR:E-SH-CG-IR-FLAT`） |

**为什么不用 `cgs_flat_hex`（通用 flat 函数）**：后端的 `lower_local_types` 与 `flat_ir_types` 在 **`Result[I64|U64, NumericError]`** 上**不同**（前者 `[I32, I64]` 两槽，后者 `[I32, I64, I32]` 三槽）⇒ 通用函数在这一族上会**算错槽数**。因此 cell 走**独立的窄函数**，与后端的 `lower_local_types` **逐类型对齐**，其余一律拒。

### 2.4 落点（`selfhost/codegen.sico`）

| 项 | 落点 |
|---|---|
| **新增** 8 个函数 | `cgs_is_unit_ty`（unit 型判定）、`cgs_cell_flat_hex`（cell 类型 flat）、`cgs_cell_ty` / `cgs_cell_width` / `cgs_cell_slots` / `cgs_cell_slot`（cell 的类型/宽度/总宽/槽位）、`cgs_write_local_hex` / `cgs_read_local_hex`（两个 cell 操作的发射） |
| **重塑** 4 个既有函数 | `cgs_function_local_hex`（unit 结果 0 组 + cell 组）、`cgs_value_slot`（unit 结果不占槽）、`cgs_dispatcher_slot`（unit 结果 + cell 宽度）、`cgs_op_hex`（`write_local` / `read_local` 路由） |
| 合计 | 文件 **3979 → 4277 行**（**+298 行**，按 LF 计；函数数 **141 → 149**） |
| oracle 生产代码（`crates/*`） | **零改动** |
| `selfhost/parser.sico` | **零改动** |

**四道闸门（全 fail-closed，ADR-0016 决策 6）**：

| 闸门 | 条件 | 越界时的身份 |
|---|---|---|
| cell 类型 | 在 §2.3 的子集内 | `ERR:E-SH-CG-IR-FLAT` |
| cell 下标 | cell 存在 | `ERR:E-SH-CG-IR-EMIT` |
| 宽度一致 | 源宽 == cell 宽 | `ERR:E-SH-CG-IR-EMIT` |
| unit 结果 | 结果类型是 unit ⇒ **0 槽**，不查槽位 | —（这是**合法**路径，不是闸门） |

## 3. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 |
|---|---|---|
| 1 | `sico_compiler_lowers_the_text_concat_with_an_assigned_cell_byte_exactly` | 参考形状的 canonical IR **2630 B** == Rust 前端（逐字节）+ IR 里确有 `write_local` ×2、`locals`、`unit` 结果型 |
| 2 | `oracle_local_cell_component_truth_is_pinned` | 组件 **1114 B** == oracle 生产输出（**逐字节**）+ 过真实 `wasmparser::Validator`（两侧各一次） |
| 3 | `sico_codegen_guest_emits_the_local_cell_reads_byte_exactly` | **cell 读**形状（2 写 + 2 读）组件 **1138 B** == oracle + Validator ⇒ `read_local` 也真的被发射了 |
| 4 | `sico_codegen_guest_emits_the_i64_local_cell_byte_exactly` | **I64 cell** 形状组件 **1116 B** == oracle + Validator ⇒ cell 子集不止 Text |
| 5 | `sico_codegen_guest_refuses_the_local_cell_types_outside_the_reached_subset` | `Bytes` cell / `List[Text]` cell ⇒ 前端 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`，组件发射 typed 拒绝「unsupported codegen source shape」⇒ **不产生近似字节** |
| 6 | `sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly` | **自举形状前沿**：本片 **12** 个 codegen 函数（8 新 + 4 重塑）经 guest 自己逐字节降低 == Rust 前端 ⇒ **未推进形状边界** |

⇒ 固件从 **132** 增至 **138** 条。

## 4. 实施记录（2026-10-23）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/codegen.sico` | 新增 8 个函数 + 重塑 4 个 ⇒ 文件 **3979 → 4277 行**（**+298 行**），函数数 141 → 149 |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新增 **6** 条钉 |
| `selfhost/parser.sico` / oracle 生产代码（`crates/*`） | **零改动** |
| `docs/*` | 同步本片结论（5 处） |

### 4.2 自证一：把本片**反向撤销**，量出它的**全部足迹**

本片**不是**纯插入（它重塑了四个既有函数），所以「改了哪些行」不能靠声明。`target/tmp/step0520/audit_footprint.py` 把本片的**11 处改动逐条反向撤销**，重建出**本片之前**的文件：

```
STEP-0520 reverse-applied footprint
  reconstructed_lines=3980
  current_lines=4278
  delta_lines=298
STEP0520_FOOTPRINT_GREEN edits_reversed=11
```

**交叉验证（独立见证）**：重建文件再喂给 **STEP-0519 的足迹审计脚本**（独立写的 9 处反向撤销），得到

```
  reconstructed_lines=3745
  current_lines=3980
  delta_lines=235
STEP0519_FOOTPRINT_GREEN edits_reversed=9
```

⇒ **3745 / 3980 / +235 三个数字与 STEP-0519 当时记录的值逐个相同** ⇒ 重建文件**就是**本片之前的那份，不多不少。

`target/tmp/step0520/check_functions.py` 再机械核对被重塑的 4 个函数与前态**逐函数行数相同**：`cgs_function_local_hex` 60 / `cgs_value_slot` 66 / `cgs_dispatcher_slot` 54 / `cgs_op_hex` 55 ⇒ **全部 ok**。

`target/tmp/step0520/check_guest.py` 机械核对 guest 侧的三条硬规则：① 纯 LF、无 BOM；② 本片新增的 8 个函数**全部先于其首次使用声明**；③ **同一函数内 `let` 名字不重复**（§4.5 缺陷 ②）。

### 4.3 出口达成

| 项 | 值 |
|---|---|
| 前端 | 参考形状 canonical IR **2630 B** == Rust 前端（逐字节） |
| 整组件 | **1114 B** == oracle 生产输出（逐字节） |
| `wasmparser::Validator` | oracle 组件与 guest 组件**两侧都通过** |
| cell 读 | **1138 B** 形状（2 写 2 读）逐字节等同 ⇒ `read_local` 真被发射 |
| I64 cell | **1116 B** 形状逐字节等同 ⇒ cell 子集不止 Text |
| 既有形状不退化 | a / b / d / g1 四个形状仍逐字节等同 oracle（**1094 B**） |
| 自举前沿 | 本片 12 个 codegen 函数逐字节等同 ⇒ **未推进形状边界** |
| 固件 | **138 passed / 0 failed** |

### 4.4 没有静默放宽

- **cell 类型子集是窄的**（§2.3），越界即 `ERR:E-SH-CG-IR-FLAT` ⇒ 整组件退回 typed 拒绝。
- **宽度不等即拒**（源宽与 cell 宽不一致 ⇒ `ERR:E-SH-CG-IR-EMIT`），与后端 `local cell layout` 的拒绝语义一致。
- **`read_local` 只认 cell 读**，不认别的来源。
- **前端未闭合的形状一个都没被认领**：STEP-0517/0518 列出的字面量实参 / 混合实参对 / 嵌套宿主调用仍 typed 拒绝。

### 4.5 自造缺陷 ①：**`U64.equal(<Bool>, 0)` ⇒ 前端类型错误 `E2001`**

初稿用 `if U64.equal(cgslh_is_unit, U64.literal(0)):` 表达「不是 unit」。

**实测**：Rust 前端直接报 `E2001 expected U64, found Bool`（`equal` 是**定宽数值**方法，参数不是数值就拒）。Sico **没有** `Bool.equal`。

**修法**：改用仓库既有的 if/else 标志惯用法（`if <bool>: let x_ok = I64.literal(1) else: <work> end if`）。**教训**：**Bool 只能进 `if` 的条件位**，不能参与数值比较 —— 这条此前没有踩过。

### 4.6 自造缺陷 ②：**同一函数里两个同名 `let` ⇒ `cell redeclared`**

修完缺陷 ① 后，guest 构建报 `unsupported cell redeclared at bytes 68238..68270`。

**根因（实测，不是猜）**：Rust 前端把**一般 CFG**里的每个 `let` 都变成 **cell**（`lower.rs` 的 `lower_straight_line` 之外一律物化），因此**同一函数内 `let` 名字只能绑定一次**；我的 `cgs_value_slot` 里两处都叫 `cvs_unit_ok` ⇒ 第二次绑定撞上已存在的 cell。

**修法**：两处标志改名（`cvs_found_unit_ok` / `cvs_skip_unit_ok`）。**教训**：**`let` 名字在函数内唯一**是 guest 源码的隐含硬规则；已把这条规则加进 `check_guest.py`，避免下次再踩。

### 4.7 自造缺陷 ③：**足迹审计脚本自己的还原语义错了三轮**

审计脚本用「当前文件的锚点区间 → 手工转写的旧文本」来还原前态。前三轮都还原错了：

| 轮次 | 现象 | 根因 | 修法 |
|---|---|---|---|
| ① | 重建比前态**少 7 行** | 锚点区间在**前态**里对应的内容不是手写的那段（改动的**插入点**才是边界）；`cgs_op_hex` 的还原漏掉了 6 行结果行 | 还原文本改成「改动插入点 ↔ 前态」的对应关系 |
| ② | 重建比前态**多 4 行** | `cgs_function_local_hex` 的指令循环还原把**未改动**的 `let cgslh_instruction_ty` 三行重复了一次 | 还原只覆盖**真正改动**的那几行 |
| ③ | 重建比前态**少 1 行** | `cgs_dispatcher_slot` 与下一个函数之间的**空行**落在还原区间里 | 还原补上那 1 个空行 |

**怎么发现的**：**不是**靠肉眼 —— 而是靠**独立见证**：① STEP-0519 的审计脚本喂给重建文件，要求它给出与 STEP-0519 当时**记录值完全相同**的 3745 / 3980 / +235；② 被重塑的 4 个函数逐个核对前态行数（60/66/54/55）。**教训**：**自证脚本本身也会说谎**，任何自证结论必须与「本片声称做了什么」交叉核对（这与 STEP-0519 的教训一脉相承）。

### 4.8 实测的 cell 覆盖面（准备片的探针）

| 形状 | 前端 | 组件 |
|---|---|---|
| Text cell（写） | ✅ 逐字节等同 | ✅ **1114 B** 逐字节等同 |
| Text cell（写 + 读） | ✅ | ✅ **1138 B** 逐字节等同 |
| I64 cell | ✅ | ✅ **1116 B** 逐字节等同 |
| `Bytes` cell | ❌ `ERR:E-SH-IR-EXPRESSION` | ❌ typed 拒绝 |
| `List[Text]` cell | ❌ `ERR:E-SH-IR-EXPRESSION` | ❌ typed 拒绝 |
| 记录型 cell | ⚠️ **前端降出的 IR 与 Rust 前端不同**（不是逐字节等同） | ❌ typed 拒绝 |

⇒ **当前可达的 cell 子集是 Text 与 I64**；`Bytes` / `List[Text]` 被**前端**挡在前（不是组件层挡的）；**记录型 cell 的前端 IR 与 Rust 前端不一致** —— 这是**前端**的已知差异，本片**没有**认领它，也没有把它算成组件层的成果。

## 5. 诚实性

### 5.1 本片**只闭合组件面的 cell 形状**，**族仍未闭合**

- T1 族（`sico.text.concat`）按计划 §3.3.5 的族级出口判据（五项缺一不算闭合）：本片补齐了**带 cell 的形状**这一项，但**族级收口片 X 未开始**、**独立 CI 尚未运行** ⇒ **T1 族尚未闭合**。
- **本片不是 T1 族的第 4 拍**：它是**全族共享的前置**（§1.1），做完之后 T1 族才具备收口的条件。

### 5.2 未闭合的

- **组件**：guest 的 codegen 仍只认 `sico.bytes.length` 与 `sico.text.concat` 两个内建；其它 helper 型内建仍 fail-closed。
- **前端**：两个字面量实参、混合实参对、嵌套宿主调用仍 typed 拒绝 `ERR:E-SH-IR-EXPRESSION`。
- **cell 子集**：`Bytes` / `List[Text]` / 记录型 cell 仍不可达（前端挡的，§4.8）。
- **语料覆盖仍 0 条**（S 内 2,857 个调用点仍未进语料）。
- **整文件自举**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）与 **S6 bootstrap 闭环**（ADR-0015 `A == B == C` + `.sapp`）**未动** ⇒ **门 3 未动，M22 仍为 NO-GO**。

### 5.3 本片**没有**改动的

- oracle 生产代码（`crates/*`）**零改动**；`selfhost/parser.sico` **零改动**；前沿探针**零改动**。
- **不改任何预算常量**、**不改任何既有断言的期望值**（只**新增** 6 条钉）。
- **未提交、未推送**；**独立 CI 尚未运行**。

### 5.4 前沿探针为什么不动

`tools/probe-m22-frontend-frontier.ps1` 判的是**前端**前沿（IR 能否降下来）。本片**不改前端能降的形状集合** ⇒ 期望值表**不变**，仍是 **28/28** `FRONTIER_UNCHANGED`。组件层的判据由本片的 6 条钉承担。

### 5.5 下一步（W2 族 T1 的收口片 X）

1. **钉住**：把本片的组件真值（1114 B / 1138 B / 1116 B）与既有 4 个形状并入**族级真值表**；
2. **纠偏**：把 §5.2 的未闭合形状逐条登记进**支持面登记表**（D15）；
3. **前沿探针期望值**：确认无需前移并把理由写进探针文件；
4. **固件电池 + 独立 CI**（提交后）；
5. **诚实收窄段**：族级结论写入 STATUS/ROADMAP，并给出「T1 族闭合」与「T2 族开工」之间的门槛。

## 6. 验证记录（2026-10-23）

| 验证 | 口径 | 结果 |
|---|---|---|
| 新钉（6 条定向） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --exact <6 条> --test-threads=1` | ✅ **6 passed / 0 failed** |
| 固件全量 | 同上，去掉 `--exact` | ✅ **138 passed / 0 failed** |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | ✅ **28/28** ⇒ `FRONTIER_UNCHANGED` |
| codegen 改动审计 | `python .\target\tmp\step0520\audit_footprint.py` | ✅ `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（**+298 行**） |
| 独立交叉验证 | `python .\target\tmp\step0520\audit_step0519_copy.py` | ✅ `STEP0519_FOOTPRINT_GREEN edits_reversed=9`（**3745 / 3980 / +235**，与 STEP-0519 记录值逐个相同） |
| 函数行数核对 | `python .\target\tmp\step0520\check_functions.py` | ✅ `STEP0520_FUNCTION_CHECK_GREEN`（60/66/54/55） |
| guest 源码规则 | `python .\target\tmp\step0520\check_guest.py` | ✅ `STEP0520_GUEST_CHECK_GREEN`（纯 LF / 无 BOM / 声明顺序 / `let` 名字唯一） |
| 注释纪律 | `python .\tools\validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 本机验证电池 | `target/tmp/step0521/battery.ps1`（**17 步**） | 见 §6.2 |

### 6.1 T1 族状态速查

| 拍 | 状态 | 证据 |
|---|---|---|
| 第 1 拍（真值测量 T） | ✅ 完成 | STEP-0517：10 变体规则表 + 子集边界 + 发射真值 |
| 第 2 拍（前端刀 F） | ✅ 完成 | STEP-0518：canonical IR 逐字节等同（2295 B）+ 自举前沿未推进 |
| 第 3 拍（codegen 刀 C） | ✅ 完成 | STEP-0519：整组件逐字节等同（1094 B）+ Validator 双侧通过 |
| **共享前置（cell 支持）** | ✅ **本片完成** | 本片：带 cell 的形状逐字节等同（1114 B / 1138 B / 1116 B） |
| 族级真值表 + D15 登记 | ✅ 完成 | STEP-0521：20 行真值表钉进固件 + 支持面登记表 + 缺口定位（并修正一处静默偏移） |
| 第 4 拍（收口片 X） | ❌ 未开始 | 需在 **G-b**（转义字面量）与 **G-d**（单语句 return 的非形参操作数）之后 —— 二者在 S 内真实存在 ⇒ 按 D10 不得降级 |

⇒ **T1 族：3/4 拍 + 共享前置 + 族级真值表完成，尚未闭合**（缺 **G-b / G-d** 与**独立 CI**）。

### 6.2 本机验证电池（17 步）

见 §6.4（电池在本片落地后、并在 STEP-0521 落地后整轮重跑）。

### 6.3 未做的事

- **未认领**前端未闭合的形状；**未认领** `Bytes` / `List[Text]` / 记录型 cell；**未认领**其它内建；
- **未提交、未推送**；**独立 CI 尚未运行**；
- **不推进任何出口门**，M22 仍为 NO-GO。

### 6.4 本机验证电池（17 步）

本轮电池 **17/17 全绿**（`target/tmp/step0521/battery.ps1`，日志 `target/tmp/step0521/battery.log`）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 429 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试） |
| `validate-0245` | 139 | 0 | 内含固件全量 + clippy（`-D warnings` 零警告） |
| `validate-0262` | 152 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 209 | 0 | 前端前沿 **31/31** ⇒ `FRONTIER_UNCHANGED`（STEP-0521 新增 3 条用例；本片**不动前端前沿**） |
| `probe-ir-plan-frontier` | 26 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN`（**29** 个文件，含 STEP-0520 与 STEP-0521 的文档与支持面登记表） |
| `selfhost-comment-discipline` | 0 | 0 | `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| `step0520-footprint-audit` | 0 | 0 | `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（**+298 行**） |
| `step0520-crosscheck-0519` | 0 | 0 | `STEP0519_FOOTPRINT_GREEN edits_reversed=9`（**3745 / 3980 / +235**） |
| `step0521-footprint-audit` | 0 | 0 | `STEP0521_FOOTPRINT_GREEN edits_reversed=3`（**+25 行**） |
| `step0521-crosscheck-0520` | 0 | 0 | `STEP0520_FOOTPRINT_GREEN edits_reversed=11`（**3980 / 4278 / +298**） |
| `step0521-census-s` | 1 | 0 | S 内调用点普查（只读） |

⇒ `BATTERY_GREEN 17/17`。

> **两次电池的留痕（诚实性）**：①本片自己的 16 步电池（`target/tmp/step0520/battery.ps1`）在 `probe-frontend-frontier` 上判**红**，但失败形态是 CLI 的 `source cache failure: Io(...)` **环境抖动**（不是语义移动；单独复跑该探针 ⇒ `FRONTIER_UNCHANGED`）；②上表是**改动落地后整轮重跑**的结果（全绿），且该次运行同时覆盖了 STEP-0521 的落地状态 —— STEP-0521 对 guest 源码的改动**只重塑 `cgs_data_offset`**（模块级 data 偏移），**不改变**本片 6 条 cell 形状钉的组件字节（1114 B / 1138 B / 1116 B 与既有 1094 B 逐字节不变）。
