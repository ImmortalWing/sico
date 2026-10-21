# STEP-0496 m22-w2 S5 guest IR 摄取：IR JSON 扫描器 + `run` 发射计划提取

> - status: implementation complete locally
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0495](./STEP-0495-m22-w2-s5-result-variant.md)（变体前端 + `--emit-ir-split`）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)（缺口定位 + 发射规格）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是 IR 摄取的第一刀（扫描器 + `run` 发射计划提取）**
> - numbering: M22 线顺延为 **STEP-0496**（本线已占用 STEP-0491..0495；并行 M23 会话占用 STEP-0489/0490；已核对 `docs/steps/` 无冲突）

## 1. 本片目标与结论

**目标**：让 guest **在自己的进程内消费 canonical IR**（`sico.ir.v0`），而不是让 IR 往返 Script v0 通道。这是 [STEP-0495](./STEP-0495-m22-w2-s5-result-variant.md) §2 登记的容量边界的直接修法，也是 STEP-0488 §4 第 5 项合同（"IR JSON 扫描器 + 布局/槽位计划 + op 发射 + Script 返回发射"）的前半。

**结论（实测）**：扫描器与 `run` 发射计划提取已闭合。

- guest 侧新增 `selfhost/codegen.sico` 的 IR 摄取区块（扫描器 + 类型/操作/计划文本提取）；
- guest 侧新增 `--emit-ir-plan` 驱动模式（`selfhost/compiler.sico`）；
- **guest 从扫描到的 IR 推出的计划文本，与 Rust oracle 从 `sico_ir::Module` 推出的计划文本逐字节相同**（新固件钉 `sico_compiler_extracts_the_run_emission_plan_byte_exactly`，4 个 fixture）；
- 计划子集外的四类形状**各有各的 typed identity**，不做近似（新固件钉 `sico_compiler_refuses_ir_shapes_outside_the_run_plan_subset`，4 个负例）；
- 新增可复现探针 `tools/probe-m22-ir-plan-frontier.ps1`（计划层前沿，8/8 判定一致）。

**门 3 仍为 NO-GO**：**run 体发射**（把计划变成 Component 的 run 体字节）、37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。

## 2. 为什么先做"扫描器 + 计划"，而不是直接发射

STEP-0488 §4 第 5 项把 S5 剩余工作写成四件事：IR JSON 扫描器 / 布局/槽位计划 / op 发射 / Script 返回发射。一次性做完体量过大、且**没有中间证据**：一旦 run 体字节对不上，无法区分是"guest 读错了 IR"还是"guest 发射写错了字节"。

本片把中间层**单独做成可逐字节验证的产物**：

```
canonical IR JSON ──guest 扫描器──> run 发射计划 ──(下一片)──> run 体字节
       │                                ▲
       └──── Rust oracle 逐字节对拍 ────┘
```

两侧都从**同一个 IR**出发（guest 从 JSON 文本扫，oracle 从 `sico_ir::Module` 结构），推出**同一段文本**；文本逐字节相同 ⇒ guest 对 IR 的理解与 oracle 一致。下一片只需在这段计划之上做发射，误差面被切到最小。

**oracle 侧不改生产代码**：`sico_ir::Module` / `Type` / `Operation` / `Terminator` 已是 public，计划生成器写在**测试文件**里（`runner/sico-runner/tests/selfhost_compiler.rs`），不污染 oracle crate。

## 3. 计划文本格式（本片冻结）

逐行、LF 结尾：

```
param <id> <name> <type>
inst <result> <type>
op <operation>
term return <value-id>
record <record> <field> <type>
```

| 类别 | 语法 | 例 |
|---|---|---|
| 类型文本 | `unit`/`bool`/`int`/`i64`/`u64`/`float64`/`string`/`bytes` | `i64` |
| | `list(<T>)` / `named(<Name>)` / `result(<ok>,<error>)` | `result(named(ScriptOutput),named(ScriptError))` |
| 操作文本 | `copy <id>` | `copy 3` |
| | `const_int <json>` / `const_i64 <n>` / `const_u64 <n>` / `const_bool <b>` / `const_string <json>` / `const_bytes <json>` | `const_string "bad"` |
| | `call <fn> args=[<id>,...]` | `call 1 args=[]` |
| | `intrinsic <name> args=[<id>,...]` | `intrinsic sico.text.encode args=[0]` |
| | `project base=<id> field=<name>` | `project base=0 field=stdin` |
| | `construct name=<name> fields=[<field>=<id>,...]` | `construct name=ScriptOutput fields=[stdout=1,stderr=2,exit_code=3]` |
| | `variant name=<name> payload=[<id>,...]` | `variant name=ok payload=[4]` |

### 3.1 三条格式决策

1. **名字不带引号**：record / field / parameter / variant / intrinsic 名都是 Sico 标识符（无空格、无转义）。guest 取到 JSON 字符串后**剥引号并校验"重新编码后字节相同"**，即"必须是纯标识符"，否则 fail-closed（`cgj_ident_text`）。
2. **字面量载荷保留规范 JSON 拼写**：`const_string` / `const_int` 的载荷以 canonical JSON 原样携带（`const_string "bad"`）。**guest 因此永远不需要反转义**；Rust 侧用 `serde_json::to_string` 重序列化，而 canonical IR 本身就是 `serde_json::to_string` 的产物 ⇒ 两侧字节必然一致。
3. **每行只有"最后一个字段"长度可变**：类型文本放在行尾（`param` / `inst` / `record` 行），操作文本整体放在 `op` 行 ⇒ 行内无歧义，不需要额外的分隔符语法。

### 3.2 Script-profile fixture 的计划全文（实测）

```
param 0 input named(ScriptInput)
inst 1 bytes
op project base=0 field=stdin
inst 2 bytes
op project base=0 field=stdin
inst 3 i64
op const_i64 0
inst 4 named(ScriptOutput)
op construct name=ScriptOutput fields=[stdout=1,stderr=2,exit_code=3]
inst 5 result(named(ScriptOutput),named(ScriptError))
op variant name=ok payload=[4]
term return 5
record ScriptError code named(ScriptErrorCode)
record ScriptError message string
record ScriptInput arguments list(string)
record ScriptInput stdin bytes
record ScriptOutput stdout bytes
record ScriptOutput stderr bytes
record ScriptOutput exit_code i64
```

（record 行按 `BTreeMap` 键序 = 字典序；每个 record 的字段按**声明顺序**。）

## 4. guest 侧实现（`selfhost/codegen.sico`）

### 4.1 为什么是字节扫描器

guest 没有文本切片，但有 `sico.bytes.slice`；canonical IR 是**单行** JSON。故扫描器全程按字节走，所有"取值"都落成字节区间。

| 函数 | 职责 |
|---|---|
| `cgj_ws` | 跳到下一个非空白字节 |
| `cgj_string_end` | JSON 字符串结束偏移（识别 `\"` 转义） |
| `cgj_container_end` | 对象/数组匹配闭合偏移（深度计数） |
| `cgj_value_end` | 跳过任意一个 JSON 值 |
| `cgj_span_text` / `cgj_value_text` | 字节区间 → 文本 / 任意值的**规范 JSON 原文** |
| `cgj_ident_text` | 剥引号取标识符，**并校验无转义**（重新编码后必须字节相同），否则返回 `""` |
| `cgj_member` | 对象内**深度 1** 键查找（成员值偏移） |
| `cgj_object_keys` | 对象成员键（文档序） |
| `cgj_array_item` | 数组第 n 项偏移 |
| `cgp_type_text` / `cgp_op_text` | 类型文本 / 操作文本 |
| `cgp_id_list` / `cgp_field_list` | id 列表 / construct 字段列表 |
| `cgp_run_plan` | 计划全文 |

### 4.2 fail-closed 的四类身份

| 身份 | 触发 |
|---|---|
| `ERR:E-SH-CG-IR-PLAN` | IR 里没有 `run` 函数 / 结构成员缺失 / record 键不是纯标识符 |
| `ERR:E-SH-CG-IR-TYPE` | 类型超出子集（如 `Map[...]`） |
| `ERR:E-SH-CG-IR-OP` | 操作超出子集（如 `read_local` / `write_local`） |
| `ERR:E-SH-CG-IR-TERMINATOR` | 终止符不是 `return`（如 `branch`）或 `return null` |

**不近似、不回退**：每一类都是 typed identity，调用方（`--emit-ir-plan` 驱动模式）把它作为 `ScriptError` 返回。

## 5. Rust oracle 侧实现（测试文件内）

`runner/sico-runner/tests/selfhost_compiler.rs` 新增：

- `rust_module(text) -> Module`（`lower_core` 的结构化结果）；
- `plan_type_text(&Type)`、`plan_op_text(&Operation)`、`plan_id_list(&[ValueId])`；
- `plan_text(&Module)`：找 `run` 函数 → 形参行 → 逐 block 逐指令的 `inst`/`op` 行 → terminator 行 → record 行。

两侧的对应关系是**一对一**的：guest 的 `cgp_type_text` ↔ `plan_type_text`，guest 的 `cgp_op_text` ↔ `plan_op_text`，guest 的 `cgp_run_plan` ↔ `plan_text`。

## 6. 实现过程中踩到并修正的**两条真缺陷 + 一条新语法纪律**

1. **`from` 是 Sico 的保留关键字（真缺陷）**：首版把扫描器的形参命名为 `from`（`cgj_ws(bytes, from)` 等 7 处），lowerer 立刻报 `cannot lower ...: unsupported atomic expression at bytes 13566..13570`。根因：`crates/sico-lexer/src/lib.rs:371` 把 `"from"` 映射为 `TokenKind::From`，而 `crates/sico-ir/src/lower.rs:3271` 对任何关键字 token 一律走 `_ => Err(unsupported_error("atomic expression", token.range))`。**形参/局部名不能取任何语言关键字**。
2. **同函数内 `let` 无块作用域（真缺陷）**：`cgp_type_text` 里 `named` 分支与 `result` 分支各写了一个 `let cgpt_data`，报 `unsupported cell redeclared at bytes 24173..24218`。**分支里的 `let` 与函数其他位置的 `let` 共用同一命名空间**——这补齐了 [handoff-m22.md](../handoff-m22.md) §5「语法与语义纪律」表的第三条（同函数内 `let` 不可重名）的**另一半**（STEP-0495 §5 记的是"match 分支绑定与 `let` 单元共用命名空间"，本条是"**不同分支之间**的 `let` 也共用命名空间"）。
3. **重写整个文件时手抄冻结常量（过程缺陷，已修）**：本片重写 `selfhost/codegen.sico` 时把三个**冻结 hex 常量**（`cg_core_types_hex` / `cg_alloc_body_hex` / `cg_realloc_body_hex`）抄错了（`0440000b` → `044000b` 等）。由 `git diff` 发现并按 `HEAD` 还原（还原后 diff 为**纯新增 881 行、零删除**）。**冻结常量不得手抄，必须逐字节比对。**

## 7. 燃料成本（诚实记录）

实测（`sico-runner --fuel <cap>` 阶梯，Script-profile fixture）：

| 驱动模式 | 最小燃料门槛 |
|---|---|
| `--emit-ir-split`（纯 IR 通道） | 5M < cap ≤ 10M |
| `--emit-ir-plan`（IR 扫描 + 计划提取） | 10M < cap ≤ 20M |

**后果**：runner CLI 的默认燃料预算（10M）**装不下计划模式**，而 CLI `sico run` 没有暴露 `--fuel`。因此：

- 固件套件用**显式 800B 预算**（`run_guest_with_args`）跑计划模式 ✓；
- 新的计划层探针 `tools/probe-m22-ir-plan-frontier.ps1` **直接驱动 `sico-runner` 并写明预算**（默认 800B），不把成本藏在 CLI 默认值后面。

这不是"绕过限制"，而是**把限制显式化**：字节扫描器的成本随 IR 字节数线性增长，后续发射片必须复核这一项。

## 8. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 计划逐字节等同（4 个 fixture） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_extracts_the_run_emission_plan_byte_exactly` | **ok**（guest 扫描结果 vs Rust oracle 逐字节相同） |
| 计划 fail-closed（4 个负例） | `... --test selfhost_compiler sico_compiler_refuses_ir_shapes_outside_the_run_plan_subset` | **ok**（4 个 typed identity 各自命中） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **95 + 2 + 9 + 1 = 107 passed / 0 failed** |
| 计划层前沿探针 | `tools/probe-m22-ir-plan-frontier.ps1` | **8/8** 判定一致，`PLAN_FRONTIER_UNCHANGED` exit 0 |
| 前沿探针（前端层，未回归） | `tools/probe-m22-frontend-frontier.ps1` | **18/18** `FRONTIER_UNCHANGED` exit 0 |
| 0245 电池 | `tools/validate-step-0245.ps1` | **exit 0**（含 runner fmt/clippy） |
| 0262 电池 | `tools/validate-step-0262.ps1` | **`STEP_0262_OK`** |
| 规划合同 | `tools/validate-step-0124.ps1` | **`STEP_0124_OK`** |
| canary | `tools/report-m22-canary.ps1` | coverage 100%、`full_source_exit` 0 |
| 根工作区 | `cargo fmt --all -- --check` + `clippy -D warnings` + `cargo test --workspace --all-targets --all-features` | 全绿 |

## 9. 诚实性

- 本片**收窄**了计划子集，必须显式记录（ADR-0016 决策 6：越界形状 typed fail-closed，禁止静默回退）：
  - **函数收窄**：只提取名为 `run` 的函数；其它函数不进入计划（`ERR:E-SH-CG-IR-PLAN`）。
  - **类型收窄**：只支持 `unit`/`bool`/`int`/`i64`/`u64`/`float64`/`string`/`bytes`/`list`/`named`/`result`；`map`/`set`/`option`/`capability`/资源/任务/流类型 typed 拒绝。
  - **操作收窄**：只支持 `copy`/`const_int`/`const_i64`/`const_u64`/`const_bool`/`const_string`/`const_bytes`/`call`/`intrinsic`/`project`/`construct`/`variant`；`read_local`/`write_local`/算术/比较/任务操作 typed 拒绝。
  - **终止符收窄**：只支持 `return`（`return null` 亦拒绝）。
  - **块收窄**：多块（分支）函数会被终止符检查拒绝。
- 本片**只做摄取与计划**，**没有**做任何 run 体字节发射；STEP-0487 的 `--emit-component-hex` 仍是**冻结的最小 echo 骨架**（硬编码 run 体），未接到计划上。
- 计划文本**不是**Component 的中间表示，只是一个**可验证的诊断/中间产物**；下一片的发射器可以直接在扫描到的 IR 上工作，不必解析这段文本——但文本必须先与 oracle 对齐，否则发射层无从证明正确。
- 本片**没有**触碰 37 语料阶梯、`.sapp` 链或 ADR-0015。门 3 仍为 NO-GO。

## 10. 下一片：槽位计划 + run 体发射

STEP-0488 缺口表现在只剩**发射**侧：

1. **槽位/布局计划**：从计划文本（或直接从扫描到的 IR）推出 `run` 函数的 cell 布局（记录按字段序摊平、列表按元素槽），对接 STEP-0487 §3 的槽位计划规格。
2. **run 体发射**：把布局 + op 序列发射成 Component 的 run 体字节，与 `crates/sico-codegen-wasm` 的 oracle 逐字节对拍。
3. 之后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。

> 提醒（承接 [STEP-0491](./STEP-0491-m22-w2-s5-frontend-record-field-projection.md) §4、STEP-0494 §5、STEP-0495 §5 与 [`../handoff-m22.md`](../handoff-m22.md) §4/§5）：guest 前端的**形状前沿**与**语法/语义纪律**（返回类型必须写 `returns X:`；同函数内 `let` 不可重名、**分支之间也不可重名**；match 分支绑定与 `let` 单元共用命名空间；**形参/局部名不得取语言关键字**；token 计数边界）共同构成"新代码必须能被 guest 自己降低"的全部已知约束。
