# STEP-0508 m22-w2 S5 调用位置的调用：**前端刀**（**已实现，本机全绿**）

> - status: **implementation complete locally（本机全绿，未提交未推送）**
> - date: 2026-10-22
> - owners: autonomous-agent
> - depends: [STEP-0507](./STEP-0507-m22-w2-s5-multi-function-component.md)（多函数模块的组件级派生）、[STEP-0506](./STEP-0506-m22-w2-s5-error-arm-component.md)（`error` 臂整组件）、[STEP-0495](./STEP-0495-m22-w2-s5-result-variant.md)（`ok`/`error` 变体）
> - gate: 门 3 的 S5 波次 = guest IR 摄取 + run 体发射 —— **本片是前端的第八刀（调用位置的调用）**
> - numbering: M22 线顺延为 **STEP-0508**（本线已占用 STEP-0491..0499、STEP-0501..0507；已核对 `docs/steps/` 与并行 worktree `E:\github\sico-m24` 占用的 STEP-0330–0337，**均无 0508**）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| backend 的**生产输出**已测得（真值只取自生产输出） | ✅ 见 §1（IR 形状 + oracle 组件 **1025 B**） |
| 有**有界出口测试** | ✅ 见 §3（fixture 的 **IR 与 Rust 前端逐字节相同**） |
| 前端是否需要刀已判定 | ✅ 见 §4（construct 的**字段值形状表**缺「调用」这一种形状） |

⇒ 取号合法。**实现已完成**，见 §6。

## 1. fixture 与真值

### 1.1 fixture（把 helper 的返回值放进 `ScriptOutput` 的一个字段）

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
  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: helper(I64.literal(7))))
end function
```

### 1.2 Rust IR（backend 生产输出，逐指令实测）

`run` 的 block 0：

| # | result | ty | operation |
|---|---|---|---|
| 1 | 1 | bytes | `project`（`input.stdin` ⇒ `stdout`） |
| 2 | 2 | bytes | `project`（`input.stdin` ⇒ `stderr`） |
| 3 | 3 | i64 | `const_i64 7`（**实参先发射**，range 619..620） |
| 4 | 4 | i64 | **`call`** `{function: 1, arguments: [3]}`，range 600..622 |
| 5 | 5 | named(ScriptOutput) | `construct`（`stdout`→1、`stderr`→2、**`exit_code`→4**） |
| 6 | 6 | result(ok,error) | `variant ok [5]` |

### 1.3 组件级真值（下一刀的出口目标）

oracle `compile_script_program` 对同一 fixture 的生产输出 = **1025 B**（无调用的同形 fixture 是 **1017 B** ⇒ 差 **8 B**）。本片**不**闭合这一层（见 §7）。

### 1.4 实施前的 guest 行为（实测）

- `--emit-ir-split` ⇒ `ERR:E-SH-IR-CONSTRUCT-VALUE`（construct 的字段值形状表没有「调用」）。
- `--emit-component-hex` ⇒ `unsupported codegen source shape`。

## 2. 形状规则（全部实测，不是推导）

| 规则 | 实测值 |
|---|---|
| `call.data.function` | 被调者的**声明 id**（1-based）。实测 `1` = `helper`（第一个声明的函数）⇒ **不是** 0-based 的序号 |
| `call.data.arguments` | 实参的 **value id**；本例 `[3]` |
| `call.ty` | 被调者的**声明返回类型**（本例 `i64`） |
| `call.range` | 从**被调者名字起点**到**右括号之后**（600..622 = `helper(I64.literal(7))`） |
| **id 记账** | 实参指令**先发射**（各占 1 个 id）⇒ 调用占下一个 id ⇒ **字段引用调用的 id**（4）⇒ construct 紧随其后（5）⇒ variant 随后（6） |

⚠️ **实参 id 的 base 不是参数数**：在 construct 的字段位置，实参指令接在**该字段当前的结果 id** 之后（本例 3），而不是「参数个数」之后。return 表达式的调用才用「参数个数」当 base（那里恰好相等）。

## 3. 出口测试（有界）

1. **固件钉（IR 级）**：guest 前端把 §1.1 fixture 降成的 IR 与 **Rust 前端**的 canonical IR **逐字节相同** ⇒ `sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly`。
2. **回归**：STEP-0506/0507 的四个组件钉与全部既有 lowering 钉**仍绿** ⇒ 固件全量绿。
3. **fail-closed**：形状超出本片支持的（例如未声明的被调者、checked-result 返回的被调者）⇒ **typed 拒绝**，不得静默降级。

## 4. 前端事实（决定本片要哪一刀）

| 形状 | 实施前 | 实施后 |
|---|---|---|
| construct 的字段值 = **调用** | ❌ `ERR:E-SH-IR-CONSTRUCT-VALUE` | ✅ **IR 逐字节等同** |
| construct 的字段值 = 字符串 / 字段投影 / 枚举 case / `I64`·`U64` 字面量 | ✅ | ✅（不变） |
| **表达式位置**（return）的调用 | ⚠️ 已有路径，但返回类型**写死** `{"kind":"u64"}` | 不变（**仍未钉**） |
| **语句位置**的调用 | ⚠️ 已有路径，返回类型同样写死 | 不变（**仍未钉**） |

⇒ 本片**只**补 construct 字段值这一个位置；return / 语句位置的调用路径**不在本片范围**（它们的返回类型仍写死，属后续刀）。

## 5. 实施提纲（已完成）

1. `construct_field_call_count`：字段值是不是调用形状、消耗几个 id（0 = 不是调用形状）。
2. `construct_field_call_ir`：调用形状的字段值 ⇒ **实参指令 + 调用指令**的 JSON；实参 id 的 base = 字段当前的结果 id。
3. `construct_lowering`：字段值路径分成「调用」与「既有形状」两支；`cl_next_result` 按**实际消耗**推进，字段引用调用的 id。
4. `argument_values` ⇒ `argument_values_at`（多一个 `base` 形参）+ 保留旧签名的包装（return 表达式仍用「参数个数」当 base，行为不变）。
5. `construct_lowering` 的返回值改为 **`<指令>\n<construct 的 id>`**：字段值可能消耗多个 id ⇒ 调用方**不能**再用「参数数 + 字段数」推 construct 的 id ⇒ `rcf_*`（return construct）与 `rvf_*`（return ok/error）两个调用方改为**读真实 id**。
6. `argument_instructions` 的**嵌套调用**实参也改用 `argument_values_at(..., seg_base)`（原先用「参数数」，与外层的 id 记账不自洽）。

## 6. 实施记录（2026-10-22）

### 6.1 改动清单

| 文件 | 改动 |
|---|---|
| `selfhost/parser.sico` | 新增 `construct_field_call_count` / `construct_field_call_ir`；`construct_lowering` 的字段值两分支 + 真实 id 推进 + 打包返回值；`argument_values` → `argument_values_at` + 包装；`rcf_*` / `rvf_*` 读真实 construct id；嵌套调用实参的 base 修正 |
| `runner/sico-runner/tests/selfhost_compiler.rs` | 新钉 `sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly` + fixture；两个既有闭包钉（`argument_instructions` / `checked_arithmetic_match_ir`）的函数清单补入 `argument_values_at` |
| `tools/validate-selfhost-comment-discipline.py`（**新**） | 把 §6.3 的注释纪律做成**受版本控制的只读校验器**：报告「函数体内注释数」与「注释里的违禁词」；按 guest 实际降低方式分档（`parser.sico` 整模块 ⇒ 违禁词也算违规；`codegen.sico` 逐函数 ⇒ 只有函数体内的注释算违规） |
| `tools/run-ci.ps1` | 一键 CI 新增 `selfhost comment discipline` 步 ⇒ **CI 由 12 步变 13 步** |

### 6.2 过程中撞到的三处真实偏差（记录在案）

1. **`function` 多加了一**：guest 的 `function_declaration_ordinal` 返回的是 **1-based 的声明 id**（`next_index(count, count)` = count+1），我按 0-based 又加了一次 ⇒ guest 发 `2`、Rust 发 `1`。
2. **实参 id 的 base 写死**：`argument_values` 把 base 写成「调用者的参数数」（return 表达式的形状），在 construct 字段位置必须是**字段当前的结果 id** ⇒ guest 发 `arguments:[1]`、Rust 发 `[3]` ⇒ 拆出 `argument_values_at`。
3. **construct 的 id 被调用方另算**：`rcf_*` / `rvf_*` 用「参数数 + 字段数」推 construct 的 id ⇒ 调用字段多消耗 id 时**整体错位**（guest 的 variant 落在 5、Rust 在 6）⇒ `construct_lowering` 改为把真实 id 一起返回。

### 6.3 自造缺陷 #1：函数体**内部**的注释让整份 `parser.sico` 退化

**现象**：新钉绿、但两个既有回归红（`sico_guest_arena_accommodates_full_parser_selfhost_input` 与 `sico_compiler_emits_the_module_ir_split_across_both_channels_byte_exactly`），都是 `ERR:E-SH-IR-STATEMENT`；闭包钉红在 `unresolved call target argument_values_at`（闭包清单缺新函数，属另一处、已单独补齐）。

**定位**（可复现，逐层收敛）：

1. 按「函数闭包」逐个喂给 guest 前端（闭包 = 该函数的**传递被调集**；guest 从**输入词表**里查被调者的声明返回类型，所以闭包必须自带被调者）。实测：`argument_values` / `argument_values_at` / `argument_instructions` / `construct_field_call_count` / `construct_field_call_ir` **全部 exit=0**；`construct_lowering` / `record_construct_function_ir` / `record_variant_function_ir` **红** ⇒ 收敛到 `construct_lowering`。
2. 在 guest 的 `general_while_function_ir` 里给**每个** `ERR:E-SH-IR-STATEMENT` 站点打上**源码行号**标记（临时诊断，已还原）⇒ 命中 `general_while_function_ir` 的**语句分派兜底**（不是任何形状检查）⇒ 说明 guest 遇到了**一个它不认识的语句词**。
3. 把该兜底再改成**回报无法识别的词 + 所属函数名** ⇒ 回报 **`construct_lowering` / `The`** ⇒ 罪魁是**注释里的第一个词**。

**根因**：guest 的词法器 `lex_words` **不跳过 `//` 注释**——注释里的**每个词**都会进词表。于是

- 写在**函数体内部**的注释，其词会被 guest 的**语句扫描器**当成源码语句 ⇒ 落到「无法识别的语句」兜底 ⇒ `ERR:E-SH-IR-STATEMENT`；
- 写在**函数之间**的注释，其词会被 guest 的**结构扫描**（`count_until_end_function` 统计 `let` / `set` / `match` / `if` / `while`）当成源码词 ⇒ 改变模块路由对该函数的**分类**。

**证据**：整份 `selfhost/parser.sico`（2.1 万行）里，**函数体内部的注释只有我新加的这 3 行**；其余全部注释都在函数之间。

**修法**：把 3 行注释**移到函数体之外**（`construct_lowering` 之前），并把 `argument_values_at` 注释里的 `return` 一词换成 `expression`（`return` 是结构扫描词）。移出后逐个闭包**全部 exit=0**，整份 `parser.sico` 重新自宿通过。

⇒ **本线新增一条硬约束**（与 STEP-0506/0507 记录的「先过 cell」「同签名桩」并列）：

> **`selfhost/parser.sico` 的注释只能写在函数之间，且不得出现 `let` / `set` / `match` / `if` / `while` / `for` / `return` / `end` / `function` / `record` / `enum` / `case` / `break` / `continue` / `use` / `module` 这 17 个词。**
>
> 校验：`tools/validate-selfhost-comment-discipline.py`（**受版本控制、只读**）—— 报告「函数体内注释数」与「注释里的违禁词」，并**已接入一键 CI**（`tools/run-ci.ps1` 的 `selfhost comment discipline` 步）。

### 6.4 自举形状前沿

新代码必须能被 **guest 自己**逐字节降低。实测（逐个闭包，exit=0 即通过）：

| 新增 / 改写的函数 | 闭包大小 | 结果 |
|---|---|---|
| `construct_field_call_count` | 15 个被调 | ✅ exit=0 |
| `construct_field_call_ir` | 28 个被调 | ✅ exit=0 |
| `argument_values_at` | 19 个被调 | ✅ exit=0 |
| `argument_values`（包装） | 20 个被调 | ✅ exit=0 |
| `argument_instructions`（改 base） | 24 个被调 | ✅ exit=0 |
| `construct_lowering`（两分支 + 真实 id） | 42 个被调 | ✅ exit=0（**修掉 §6.3 后**） |
| `record_construct_function_ir`（读真实 id） | 51 个被调 | ✅ exit=0 |
| `record_variant_function_ir`（读真实 id） | 53 个被调 | ✅ exit=0 |

⇒ **本片没有推进 guest 前端的形状边界**：新代码全部落在既有子集内；唯一的失败原因是**注释位置**（§6.3），不是语句形状。

## 7. 诚实性

### 7.1 本片**只**闭合 IR 层

组件发射**仍然 typed 拒绝**：guest 的 codegen 还没有「`call` 指令」这一种操作数（`cgs_op_hex` 不认 `call`）⇒ `--emit-component-hex` 对本 fixture 仍是 `unsupported codegen source shape`。**整组件 1025 B 的逐字节等同是下一刀的目标**（真值已测得，见 §1.3）。

### 7.2 支持面收窄

- 只覆盖**一个 fixture**：实参是 `I64.literal(...)`；实参还可以是被调者之外的**调用者参数**（既有 `argument_instructions` 的形状表），其它实参形状沿用既有子集。
- 被调者的返回类型必须是**标量**（`declared_return_kind` 能给出 kind）；named / result / option 返回 ⇒ typed 拒绝。
- 未声明的被调者 ⇒ typed 拒绝（`ERR:E-SH-IR-CONSTRUCT-VALUE` 路径，不静默当作别的形状）。

### 7.3 其余缺口（承自本线）

- **表达式位置**（return）与**语句位置**的调用路径：返回类型仍**写死** `{"kind":"u64"}`，**未被本片改动**，也未被钉。
- **整文件自举**仍未关闭（guest 不能整体降低 `codegen.sico`：`List[Text]` 类型形状 ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- oracle 生产代码零改动；独立 CI 尚未运行；未提交未推送。
- **M22 整体仍为 NO-GO**（门 3 未动）。

## 8. 验证记录（2026-10-22）

| 验证 | 命令 / 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | **109 passed / 0 failed**（STEP-0507 的 108 + 本片新钉 1） |
| 新钉 | `sico_compiler_lowers_the_call_in_a_construct_field_byte_exactly` | ✅ guest IR == Rust IR（2198 B == 2198 B） |
| 整份 `parser.sico` 自宿 | `sico_guest_arena_accommodates_full_parser_selfhost_input` | ✅ 绿（**§6.3 修回**） |
| 两个闭包钉 | `argument_instructions` / `checked_arithmetic_match_ir` | ✅ 绿（闭包清单补 `argument_values_at`） |
| 组件钉回归 | STEP-0504 / 0506 / 0507 的组件钉 | ✅ 仍绿 |
| 注释纪律 | `python tools/validate-selfhost-comment-discipline.py` | ✅ `SELFHOST_COMMENT_DISCIPLINE_GREEN files=2` |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §8.1 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | ✅ `LINE_ENDING_AUDIT_GREEN`（11 files） |

### 8.1 电池结果（如实记录）

本机验证电池 **11 步**：首轮 **10/11 绿**，唯一红步**是环境敏感项**（**不是**代码缺陷）：

| 步 | 秒 | exit |
|---|---|---|
| `fmt-runner` | 6 | 0 |
| `ci-one-key` | 419 | 0（**12 步**版；接入注释纪律校验步后复跑 = **13 步全 PASS**） |
| `validate-0245` | 10 | **1** ← 环境敏感 |
| `validate-0262` | 176 | 0 |
| `probe-frontend-frontier` | 151 | 0 |
| `probe-ir-plan-frontier` | 34 | 0 |
| `validate-0124` | 1 | 0 |
| `validate-0131` | 7 | 0 |
| `validate-0156` | 1 | 0 |
| `git-diff-check` | 0 | 0 |
| `line-ending-audit` | 0 | 0 |

**红点定性**：`validate-0245` 的 3 个 `selfhost_checker` 用例报 `Launch("页面文件太小，无法完成操作。 (os error 1455)")` —— 这是**宿主 Windows 提交限额耗尽**（一个 python 常驻 **5.45 GB**、msedge 1.3 GB；每个 guest 预留 2 GiB 线性内存），**不是**诊断分区不匹配（期望值仍是 `E3002` / `E2001` / `LEXICAL`）。**只降并发**（`RUST_TEST_THREADS=1`，**不改任何断言、不改任何预算常量**）复跑 ⇒ **`validate-step-0245.ps1` exit=0**（内含固件 **109 passed** + clippy）。

**本片顺带把这条注释纪律做成受版本控制的校验器**：`tools/validate-selfhost-comment-discipline.py`（**只读**）⇒ 报告「函数体内注释数」与「注释里的违禁词」，并**接入一键 CI**（`tools/run-ci.ps1` 新增 `selfhost comment discipline` 步 ⇒ **CI 由 12 步变 13 步**）。

### 8.2 未做的事

- **组件层（codegen 刀）未做**：`cgs_op_hex` 仍不认 `call` ⇒ 本 fixture 的组件发射仍 typed 拒绝。
- **未提交、未推送**；**独立 CI 尚未运行**。

