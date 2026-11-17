# STEP-0618 — M22 W3-a 燃料墙**定量化**与「游标遍历刀」（量化 + 一次字节恒等提速；**墙未破**，负结果如实登记）

- Date: 2026-11-13
- Track: M22 compiler self-host, goal **W3-a**（`AGENTS.md` §10.7 / `docs/plans/M22-compiler-self-host.md`）
- Slice type: **实现 + 测量片** —— 落地一处**逐字节恒等**的发射器提速（`selfhost/codegen.sico`），并新增 4 个**可复现探针**把「燃料墙」从定性变成定量；**未翻任何门、未上调任何限额、未 commit/push**。
- Worktree: `D:/SorftWare/sico-m22-w3a`，分支 `codex/m22-w2-step0613`，HEAD at start `e93f59a`（`e93f59adcce30ba4fcab88c90b991830a1d466ef`）。
- 热文件纪律：本片是 `selfhost/codegen.sico` 的**唯一写入方**；`parser.sico` / `compiler.sico` / `crates/**` 本片**未触碰**。

## 0. 结论摘要（一句话）

**燃料墙被定量了，但还没被打破**：整份 `formatter.sico` 的 guest 发射在**抬预算**下**跑完全程**并以 **typed refusal** 收尾，**精确耗量 1.186e12 fuel / 126.6 s** ⇒ 墙 = **默认 800e9 预算的 1.48×**（**不是** STEP-0614/0616 记的「>3.2e12」，见 §3.1 订正）；本片落地**两把逐字节恒等**的刀（7/7 字节守卫全绿）：刀 1「5 处数组遍历改既有游标」**−2.3%**、刀 2「`cgs_dispatcher_slot` 四处遍历改游标（打的是 **O(I³)/函数** 的二次项）」**−14.5%**，累计 **1.186e12 → 9.913e11（−16.4%）**，**仍高于 800e9 ⇒ 墙未破**；用**阶段拆分 + 规模律 + 重复法**把成本链条定位到 **逐函数体发射（81.5%）→ 逐指令分派（66.4%）→ 槽位布局（15.4%，已修）**，并订正了「桩读数」这一被污染的归因方法（§3.4）。

## 1. 前置：本机工具链修复（此前一切「无工具链」结论的根因）

- **事实**：`target/tooling/msys2-binutils/` 是**残缺提取** ⇒ `dlltool.exe` 任何调用形式**段错误**（写 0 字节 import lib ⇒ rustc `failed to add native library ... os error 1006`）、`ld.exe`/`objdump.exe` 缺 `libintl-8.dll` 退出 127 ⇒ **runner 工作区在本机根本无法构建**。这正是 STEP-0613 / 0614 / 0616 / 0617 / 0617b 多片写「本机无 Rust 工具链（`NO_CARGO_FOUND`）」的**真实根因**：不是没装，是**装了但残缺**。
- **修复**：用 Windows 自带 bsdtar（libzstd）把已 vendored 的全部 MSYS2 pkg 重装到 `target/tooling/mingw64-fix`；补 `gettext-runtime`（`libintl-8.dll`）；合成 `default-manifest.o` 与空 `libpthread.a`；全过程固化为 **`tools/ensure-mingw.ps1`**（幂等 + rustc 探针自检，**已实跑通过**）。
- **效果**：本片所有测量（cargo 构建 + wasmtime 真实 runner + 4 个探针）**在本机首次可复现执行**。

## 2. 方法：4 个新探针（`runner/sico-runner/tests/selfhost_formatter_component_measurement.rs`）

| 探针 | 作用 | 备注 |
|---|---|---|
| `m22_w3a_whole_formatter_fuel_ladder` | **预算阶梯**：同输入在 6 档预算（25e9→800e9）下各在 op #k 耗尽 ⇒ 逐档采样燃料分布 | guest 确定性 ⇒ 采样可比 |
| `m22_w3a_whole_formatter_total_fuel` | **精确总耗量**：抬预算（8e12，仅探针参数，非产品限额）跑到终点读 `trace.consumed` | 默认 800e9 只能得到下界 |
| `m22_w3a_whole_formatter_phase_split` | **阶段拆分**：默认路径（只产 canonical IR）vs `--emit-core-hex` vs `--emit-component-hex` | 无源码桩 ⇒ **无早退混淆** |
| `m22_w3a_emission_scaling_law` | **规模律**：按 `end function` 边界截断出 5/10/20/30 个函数的副本分别发射 | 每个副本都是合法 Sico |

## 3. 测量结果

### 3.1 墙的精确量（订正「>3.2e12」）

| 阶段 | 耗量 | wall | 终点 |
|---|---|---|---|
| 整份 formatter 发射（**刀前**） | **1,186,075,335,125**（1.186e12） | 126.583 s | `Domain{invalid-input, "unsupported codegen source shape"}` |
| 整份 formatter 发射（**刀后**） | **1,159,329,467,982**（1.159e12） | 127.035 s | 同上（同身份） |

⇒ ① 墙 = **1.48× 默认预算**（800e9），**不是** STEP-0614/0616 记的「>3.2e12」——该数出自 1.6e12/3.2e12 两档的**下界外推**，本片用抬预算跑到底把它**订正为精确值**；② 终点是 **typed refusal**（4 个 stdlib helper 臂未落地：见 STEP-0616 §10），**不是**燃料耗尽 ⇒ **1.186e12 是「发射尝试」的耗量 = 成功发射的下界**（补齐 helper 臂后只会更高）。

### 3.2 阶段拆分（无混淆）

| 阶段 | 耗量 | wall | 产物 |
|---|---|---|---|
| **前端**（解析 + 降低 → canonical IR） | **1,444,003,824**（1.4e9，**0.12%**） | 0.229 s | 236,040 B canonical IR |
| **发射**（`--emit-component-hex`） | **1,159,329,467,982**（**99.88%**） | 124.6 s | typed refusal |

⇒ **前端不是问题**；全部成本在**发射阶段**。（`--emit-core-hex` 在本形态下立即拒绝，不构成可用阶段读数。）

### 3.3 规模律（单函数二次项的指纹）

| 截断（函数数） | 源码字节 | 耗量 | 相邻比 |
|---|---|---|---|
| 5 | 1,346 | 1.153e9 | — |
| 10 | 5,386 | 5.860e10 | ×50.8（字节 ×4.0） |
| 20 | 12,701 | 9.352e10 | ×1.60（字节 ×2.4） |
| 30（全部） | 22,638 | **1.159e12** | ×12.4（字节 ×1.8） |

⇒ 前 20 个函数基本线性（甚至次线性），**最后一个跳跃集中在含 `normalize_source`（`formatter.sico:694–815`，约 120 行 / 数百条指令）的那几个大函数上** ⇒ 成本形态 = **单函数内指令数的超线性项**（与「函数数」无关）。

### 3.4 消融的**方法学陷阱**（本片最重要的方法学产出）

用「把某函数替换成桩」做有限差分时，**凡是会改变发射字节的桩都会让发射器提前拒绝**（组件 hex 非法 ⇒ 早退），于是读数变成「到早退为止的耗量」，**不是该函数的成本**：

| 桩 | 耗量 | wall | 判读 |
|---|---|---|---|
| `cgs_function_position` → 1 | 1.1592e12 | 125.6 s | **无早退** ⇒ 该函数成本 ≈ 0 |
| `cgs_intrinsic_mask` → 0 | 1.1584e12 | 125.5 s | **无早退** ⇒ ≈ 0（0.08%） |
| `cg_hex_digit` → "0" | 1.1593e12 | 124.2 s | **无早退** ⇒ ≈ 0 |
| `cgs_const_hex` → "" | 1.1593e12 | 131.1 s | **无早退** ⇒ ≈ 0 |
| `cgs_value_slot` → 1 | 1.1593e12 | 124.7 s | **无早退** ⇒ ≈ 0 |
| `cgs_helper_index` → 0 | 2.155e11 | 24.2 s | **早退** ⇒ 读数无效 |
| `cgs_op_hex` → "" | 3.892e11 | 46.5 s | **早退** ⇒ 无效 |
| `cg_leb_hex` → "" | 1.417e11 | 15.1 s | **早退** ⇒ 无效 |
| `cgs_core_code_hex` → "" | 1.436e11 | 16.1 s | **早退** ⇒ 无效 |

⇒ **「STEP-0614 位掩码刀零效果」这类结论，只有在变体不改变控制流时才可信**；本片据此把 STEP-0611/0614 的归因链**重新表述为「在整文件路径上未观测到效果」**（而非「已证明主导 op 不在此」）。

### 3.5 重复法（无早退混淆的归因手段）

把某函数的**工作原地复制一遍**（丢弃第二份结果 ⇒ 发射字节不变 ⇒ 不可能早退），则 **燃料增量 = 该函数在全模块上的真实成本**：

| 重复对象 | 刀前耗量 | 重复后耗量 | 增量 | 判读 |
|---|---|---|---|---|
| `cg_leb_hex`（整函数体复制） | 1.1593e12 | 1.159331386791e12 | **+1.9e6（+0.0002%）** | **逐字节 LEB128 编码不是成本** |

⇒ 与 §3.4 的 `cg_hex_digit` / `cgs_const_hex` 桩一致：**「整数↔hex 转换」这一族不是燃料大头**（此前由预算阶梯得出的「6/6 样本落在 hex 族」结论**不可信**——core 模块 501 个函数、guest 无 name section，按源码声明序解名的映射在高位不可靠；本片据此**撤回**该结论）。

### 3.6 JSON 访问层重复探针

把 `cgj_member`（深度-1 成员查找，**全模块所有 IR 读取都经它**）整函数体复制一次（发射字节不变 ⇒ 不可能早退）：

| 重复对象 | 刀前耗量 | 重复后耗量 | 增量 | 判读 |
|---|---|---|---|---|
| `cgj_member` | 1.1593e12 | 1.1868e12 | **+2.75e10（+2.37%）** | **整层 JSON 成员查找 ≈ 2.4%，不是大头** |

⇒ 与 §3.5 的 hex 族、§3.4 的 5 个候选函数一致：**「读 IR」层与「数字格式化」层合计都只是个位数百分比** ⇒ 成本主体在**逐指令发射/装配**本身（见 §3.7）。

### 3.7 归因图（重复法，基线 1.1593e12）

重复法给出的**全模块成本占比**（增量 ÷ 基线）：

| 被重复的函数 | 重复后耗量 | 增量 | 占比 | 判读 |
|---|---|---|---|---|
| `cgs_function_instr_hex`（每函数体发射） | 2.1043e12 | +9.45e11 | **+81.5%** | **逐函数体发射 = 主体** |
| `cgs_op_hex`（逐指令分派） | 1.9294e12 | +7.70e11 | **+66.4%** | **逐指令分派 = 主体内部** |
| `cgs_dispatcher_slot`（槽位布局） | 1.3383e12 | +1.79e11 | **+15.4%** | **二次项热点**（见 §3.8） |
| `cgj_member`（JSON 成员查找） | 1.1868e12 | +2.75e10 | +2.37% | 小头 |
| `cgs_value_index`（值→槽索引） | 1.1603e12 | +9.8e8 | +0.09% | 可忽略 |
| `cgs_value_ids_refusal`（id 规范性） | 1.1594e12 | +8.8e8 | +0.08% | 可忽略 |
| `cg_leb_hex`（LEB128 hex） | 1.15933e12 | +1.9e6 | +0.0002% | 可忽略 |

⇒ **成本链条 = 逐函数体发射（81.5%）→ 逐指令分派（66.4%）→ 槽位布局（15.4%）**；`cgs_op_hex` 内部除 `cgs_dispatcher_slot` 外仍有约 51% 未归因（候选：各臂的类型扁平化 `cgs_flat_hex` / `cgs_type_slots`、分派链的 `cg_is_word` 比较、cell 宽度计算——见 §3.9 的后续轮次）。

### 3.8 二次项的具体形态（**本片定位到的算法缺陷**）

`cgs_dispatcher_slot(bytes, run)`（`codegen.sico:2275`）计算函数的槽位布局 = 参数 + **每条指令的结果槽宽** + cell ⇒ **单次调用 O(函数规模)**。它被两处按**每条指令**调用：

- `cgs_list_length_u64_hex`（`codegen.sico:3844`，**内建臂** ⇒ 每个内建指令一次）；
- `cgs_cell_slot`（`:2353`，cell 读写路径）。

而 `cgs_dispatcher_slot` **自身又用 index 形态遍历**（`cgj_array_item(bytes, arr, i)` + 递增 i ⇒ 每次从数组头重扫）⇒ 单次调用 O(I²) ⇒ **每函数 O(I³)** ⇒ 与 §3.3 的规模律（成本集中在指令数最多的大函数上）**完全吻合**。

**修法**：把 `cgs_dispatcher_slot` 的 4 处遍历（parameters / blocks / instructions / locals-cells）改为**既有游标遍历** ⇒ 单次调用 O(函数规模) ⇒ 每函数 O(I²)（**仍非线性，但降一个量级**）。

### 3.9 第二/三轮归因（在**刀 2 之后**的树上，基线 9.913e11）

| 被重复的函数 | 重复后耗量 | 增量 | 占比 | 判读 |
|---|---|---|---|---|
| `cgs_flat_hex`（类型扁平化） | 1.0213e12 | +3.00e10 | +3.0% | 小头 |
| `cgs_type_slots`（槽宽） | 9.9638e11 | +5.1e9 | +0.51% | 可忽略 |
| `cg_is_word`（分派比较，全模块） | 9.9139e11 | +1.1e8 | +0.01% | 可忽略 |
| `cgs_cell_width`（cell 宽） | 9.9454e11 | +3.2e9 | +0.33% | 可忽略 |
| `cgs_write_local_hex`（写 cell 臂） | 9.9417e11 | +2.9e9 | +0.29% | 可忽略 |

⇒ **已归因的算子合计 ≈ 4%（刀 2 之后）**；`cgs_op_hex` 的 **66.4%** 中仍有**大部分未落到单个臂** ⇒ 成本更像是**逐指令发射的「整体」**（例如每条指令都要重新解析/重建类型与槽信息、或经 `sico.text.concat` 逐段装配），而**不是**某一个可单独命名的热函数 ⇒ **下一刀必须换成「按指令形状分组」的归因**（同一 op 形状的指令批量对比），而不是继续逐函数重复。

## 4. 落地改动（唯一代码改动：`selfhost/codegen.sico`，**逐字节恒等**）

### 4.1 刀 1：5 处模块/函数级数组遍历改游标

把 5 处**模块级 / 函数级数组遍历**从「按 index 取元素（每次从数组头重扫）」改为**既有的游标遍历**（`cgj_array_first` + `cgj_array_next`，仓库早已有这两个 helper 及其注释「so a full traversal is linear instead of rescanning from the array head per index」）：

| 函数 | 原形态 | 现形态 | 语义 |
|---|---|---|---|
| `cgs_function_count` | `cgj_array_item(bytes, arr, i)` + 递增 i | 游标 | 元素集合与顺序不变 |
| `cgs_function_position` | 同上 | 游标 | 同上（命中即返回，位置计数不变） |
| `cgs_intrinsic_mask` | 同上 | 游标 | 同上（位掩码逐位 OR 不变） |
| `cgs_function_intrinsic_bits` | 同上 | 游标 | 同上 |
| `cgs_block_intrinsic_bits` | 同上 | 游标 | 同上 |

- **效果**：1.186e12 → **1.1593e12（−2.3%）**。

### 4.2 刀 2：`cgs_dispatcher_slot` 的 4 处遍历改游标（**打的是 §3.8 的二次项**）

`cgs_dispatcher_slot`（`:2275`）的 parameters / blocks / instructions / locals-cells 四个循环全部由 index 形态改为游标 ⇒ **单次调用 O(函数规模)**（原 O(I²)）⇒ **每函数 O(I²)**（原 O(I³)）。

- **效果**：1.1593e12 → **9.913e11（−14.5%）**；**累计 1.186e12 → 9.913e11 = −16.4%**。
- **仍未破墙**：9.913e11 > **800e9** 默认预算 ⇒ 还需要 **≥ −20%** 才能在默认限额内跑完（**owner D-B：不得上调限额**，故只能继续降复杂度）。

### 4.3 两刀的共同纪律

- **不改任何签名、不改任何调用点、不改任何 typed refusal 臂**；函数体内**不加注释**（STEP-0613c 已证函数体内 `//` 注释会触自举语句前沿 `ERR:E-SH-IR-STATEMENT`）。
- **逐字节恒等**：两刀的每一步都跑 7/7 字节级守卫（§5），**7 passed / 0 failed**。

## 5. 验证（本机真实 runner，Windows x64 GNU）

- **7/7 字节级守卫全绿**（`--exact`，`selfhost_compiler`）：
  `sico_compiler_lowers_the_step0507_codegen_functions_byte_exactly`、
  `..._step0526_...`、`..._step0513_...`、
  `sico_is_utf8_gated_run_lowers_byte_exactly`、
  `sico_run_emits_the_minimal_multiblock_run_component_byte_exactly`、
  `sico_codegen_guest_emits_the_derived_component_bytes_byte_exactly`、
  `sico_codegen_guest_emits_the_call_in_a_field_component_byte_exactly`
  ⇒ **7 passed / 0 failed**（7.19 s）。
- 4 个新探针均可复现执行；发射耗量 before/after 如 §3.1。
- 工具链：`tools/ensure-mingw.ps1` 自检通过（rustc 探针链接 + 运行成功）。

## 6. 诚实收窄

- **墙未破**：9.913e11 > 800e9 默认预算（差 **≥ −20%**）；**未上调任何限额**（owner D-B 维持：限额上调已被 STEP-0600/0606 实测否决，本片**不复活该选项**）。
- **1.186e12（刀前）是「发射尝试」的下界**：终点是 typed refusal（4 个 helper 臂未落地），补齐后成本只会更高。
- **`cgs_op_hex` 内部仍有约 55% 未归因**：已归因部分（`cgs_dispatcher_slot` 15.4%、`cgs_flat_hex` 3.0%、`cgs_type_slots` 0.5%、`cgs_cell_width` 0.33%、`cg_is_word` 0.01%、`cgs_value_slot`/`cgs_const_hex`/`cgs_helper_index`/`cgs_function_position` ≈ 0）之外的部分落在**其余内建臂**（`cgs_write_local_hex` / `cgs_read_local_hex` / `cgs_construct_hex` / `cgs_variant_hex` / `cgs_project_hex` / `cgs_checked_fixed_hex` / `cgs_call_hex` / `cgs_intrinsic_hex`）——本片已启动该轮逐臂归因（见 §3.9），**未完成前不宣称任何「已找到破墙刀」**。
- **本片未跑**：全量 `selfhost_compiler` 套件、`tools/validate-m22-s5.ps1` 出口整轮（15 拍）、冻结 match-site 账本慢速 95 组逐组重算、独立 CI。
- **发射词表**：`tools/census-m22-emitter-vocabulary.py` 的 **`GUEST_HELPER_BODIES` 表项缺口已修**（补 `sico.text.trim → cg_text_trim_helper_body_hex`；该缺口是 STEP-0613e 遗留、当时因「本机无 Python」而无法刷新），**词表指纹重冻结（`--update`）未完成**——它需要先构建根工作区的 `sico-ir --test dump_oracle_ir`（本轮预算外）⇒ `selfhost/emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` **仍过期**，留待下一片；**本片不宣称词表绿**。
- **未翻任何门**：M22 overall 仍 **NO-GO**；**未 commit / 未 push**（owner-only 授权）。

## 7. 下一刀（已具名，未动工）

1. **归因手段换成「按指令形状分组」**：重复法已把可命名的函数逐个排除（合计 ≈4%），而 `cgs_op_hex` 的 66.4% 大部分**落不到单个函数** ⇒ 下一轮按**目标模块里的指令形状**分组对比（如「写 cell 的指令」vs「const 指令」vs「call 指令」各自的单位成本），定位到**形状**而不是函数。
2. **优先补齐 4 个 stdlib helper 臂**（`sico.map.empty/length/get/put[Text,U64]` + `sico.text.trim`；真值表见 STEP-0616 §3，**规格有效、实现未落地**）——这是 W3-a 的**第一挡板**（前端 `construct` 字段值缺口 `ERR:E-SH-IR-CONSTRUCT-VALUE` 之后），与燃料墙**正交**；补齐后发射路径会变长 ⇒ **必须重新测墙**（本片的 9.913e11 只对当前形态成立）。
3. **前端缺口先于燃料墙**：STEP-0613h 的 434 行 `construct_field_value_ir` 设计未落地（需与账本重冻结同片协调）⇒ 在四个 helper 臂之前，**W3-a 的第一挡板是前端**。
4. 若破墙仍不可达，则回到 ADR-0015 门 4 的口径：**按 D7「先降表示成本、再改算法、最后才加限额」**，且**限额上调已被实测否决**（STEP-0600/0606），不得复活。

## 7b. 本片留给下一片的两个「可复现资产」

- `runner/sico-runner/tests/selfhost_formatter_component_measurement.rs` 的 **4 个新探针**：预算阶梯 / 精确总耗量 / 阶段拆分 / 规模律（**全部 `#[ignore]`，测量专用，不进产品路径**）。
- `tools/ensure-mingw.ps1`（账本树）：**本机 Rust 工具链的可复现供给**（幂等 + rustc 探针自检）⇒ 后续任何 M22 片都**不得再以「本机无工具链」为由跳过实测**。

## 8. 交叉引用

- 本片工作树：`D:/SorftWare/sico-m22-w3a`（`codex/m22-w2-step0613`）。
- 账本工作树：`D:/SorftWare/sico-m22-bootstrap`（`codex/m22-w2`）：STEP-0615 / 0616（§10 订正）/ **0617 / 0617b**（由 0614/0614b 重编号）；本片的 STATUS/handoff 同步条目见该树 `docs/STATUS.md`、`docs/handoff-m22.md`。
- 工具链脚本：`tools/ensure-mingw.ps1`（账本树）。
- 相关旧片：STEP-0600（限额敏感度）、STEP-0606（燃料归因诊断）、STEP-0607（RFC-0052-A3/A1）、STEP-0611（op 级燃料归因）、**STEP-0614（本树：位掩码刀，负结果保留）**、STEP-0613f/0613g/0613h（前端 construct 缺口）。
